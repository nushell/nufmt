//! Block, pipeline, and closure formatting.
//!
//! Handles formatting of blocks (top-level and nested), pipelines,
//! pipeline elements, redirections, block expressions, and closures.

use super::Formatter;
use nu_protocol::{
    ast::{
        Argument, Block, Expr, Expression, PipelineElement, PipelineRedirection, RedirectionTarget,
    },
    Span,
};

#[derive(Copy, Clone, Eq, PartialEq)]
enum LetFamily {
    Variable,
    Constant,
}

impl<'a> Formatter<'a> {
    // ─────────────────────────────────────────────────────────────────────────
    // Block and pipeline formatting
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a block (a sequence of pipelines).
    pub(super) fn format_block(&mut self, block: &Block) {
        let num_pipelines = block.pipelines.len();
        for (i, pipeline) in block.pipelines.iter().enumerate() {
            if let Some(first_elem) = pipeline.elements.first() {
                self.write_comments_before(first_elem.expr.span.start);
            }

            // Pipelines inside indented blocks (e.g. def bodies) need
            // explicit indentation when they start on a fresh line.
            if self.at_line_start {
                self.write_indent();
            }

            self.format_pipeline(pipeline);

            if let Some(last_elem) = pipeline.elements.last() {
                let end_pos = self.get_element_end_pos(last_elem);
                self.write_inline_comment_bounded(end_pos, self.inline_comment_upper_bound);
                self.last_pos = end_pos;
            }

            if i < num_pipelines - 1 {
                let separator_newlines = self.separator_newlines_between_top_level_pipelines(
                    pipeline,
                    &block.pipelines[i + 1],
                );

                for _ in 0..separator_newlines {
                    self.newline();
                }
            }
        }
    }

    /// Decide how many newline characters to emit between adjacent pipelines.
    ///
    /// At top-level this respects `margin` while keeping adjacent `use`
    /// statements and same-family `let`/`const` groups compact.
    fn separator_newlines_between_top_level_pipelines(
        &self,
        current: &nu_protocol::ast::Pipeline,
        next: &nu_protocol::ast::Pipeline,
    ) -> usize {
        if self.indent_level != 0 && self.config.margin > 1 {
            return 1;
        }

        if self.indent_level == 0 && self.is_use_pipeline(current) && self.is_use_pipeline(next) {
            return 1;
        }

        if self.indent_level == 0 {
            if let (Some(current_family), Some(next_family)) = (
                self.pipeline_let_family(current),
                self.pipeline_let_family(next),
            ) {
                if current_family == next_family {
                    if self.pipeline_call_span_has_newline(current)
                        || self.pipeline_call_span_has_newline(next)
                    {
                        return self.config.margin.saturating_add(1);
                    }

                    if self.has_comment_between_pipelines(current, next) {
                        // Let the margin/blank-line logic below decide spacing for
                        // comment-delimited groups.
                    } else {
                        return 1;
                    }
                } else {
                    return self.config.margin.saturating_add(1);
                }
            }
        }

        if self.config.margin == 1 && !self.config.margin_is_explicit {
            if self.has_blank_line_between_pipelines(current, next) {
                return 2;
            }

            return 1;
        }

        self.config.margin.saturating_add(1)
    }

    /// Whether a pipeline is a top-level `use` command.
    fn is_use_pipeline(&self, pipeline: &nu_protocol::ast::Pipeline) -> bool {
        matches!(
            self.pipeline_decl_name(pipeline),
            Some("use" | "export use")
        )
    }

    fn pipeline_decl_name(&self, pipeline: &nu_protocol::ast::Pipeline) -> Option<&str> {
        let first = pipeline.elements.first()?;
        let Expr::Call(call) = &first.expr.expr else {
            return None;
        };

        Some(self.working_set.get_decl(call.decl_id).name())
    }

    fn pipeline_let_family(&self, pipeline: &nu_protocol::ast::Pipeline) -> Option<LetFamily> {
        match self.pipeline_decl_name(pipeline)? {
            "let" | "let-env" | "mut" => Some(LetFamily::Variable),
            "const" | "export const" => Some(LetFamily::Constant),
            _ => None,
        }
    }

    fn has_blank_line_between_pipelines(
        &self,
        current: &nu_protocol::ast::Pipeline,
        next: &nu_protocol::ast::Pipeline,
    ) -> bool {
        let current_end = current
            .elements
            .last()
            .map_or(0, |element| self.get_element_end_pos(element));
        let next_start = next
            .elements
            .first()
            .map_or(current_end, |element| element.expr.span.start);

        if current_end >= next_start {
            return false;
        }

        let between = &self.source[current_end..next_start];
        let mut previous_newline: Option<usize> = None;
        for (idx, byte) in between.iter().enumerate() {
            if *byte != b'\n' {
                continue;
            }

            if let Some(prev) = previous_newline {
                if between[prev + 1..idx]
                    .iter()
                    .all(|b| b.is_ascii_whitespace())
                {
                    return true;
                }
            }

            previous_newline = Some(idx);
        }

        false
    }

    fn has_comment_between_pipelines(
        &self,
        current: &nu_protocol::ast::Pipeline,
        next: &nu_protocol::ast::Pipeline,
    ) -> bool {
        let current_end = current
            .elements
            .last()
            .map_or(0, |element| self.get_element_end_pos(element));
        let next_start = next
            .elements
            .first()
            .map_or(current_end, |element| element.expr.span.start);

        current_end < next_start && self.source[current_end..next_start].contains(&b'#')
    }

    fn pipeline_call_span_has_newline(&self, pipeline: &nu_protocol::ast::Pipeline) -> bool {
        let Some(first) = pipeline.elements.first() else {
            return false;
        };
        let Some(last) = pipeline.elements.last() else {
            return false;
        };

        let Expr::Call(call) = &first.expr.expr else {
            return false;
        };

        let start = call.head.start;
        let end = self.get_element_end_pos(last);
        start < end && self.source[start..end].contains(&b'\n')
    }

    /// Get the end position of a pipeline element, including redirections.
    fn get_element_end_pos(&self, element: &PipelineElement) -> usize {
        element
            .redirection
            .as_ref()
            .map_or(element.expr.span.end, |redir| match redir {
                PipelineRedirection::Single { target, .. } => target.span().end,
                PipelineRedirection::Separate { out, err } => out.span().end.max(err.span().end),
            })
    }

    /// Format a pipeline (elements joined by `|`).
    ///
    /// Preserves multi-line pipeline layout: if the original source has line
    /// breaks between pipeline elements, the formatted output keeps each
    /// stage on its own line with `| ` prefix.
    pub(super) fn format_pipeline(&mut self, pipeline: &nu_protocol::ast::Pipeline) {
        if pipeline.elements.len() <= 1 {
            if let Some(element) = pipeline.elements.first() {
                self.format_pipeline_element(element);
            }
            return;
        }

        // Detect whether the source places pipeline elements on separate lines.
        let source_is_multiline = pipeline.elements.windows(2).any(|pair| {
            let prev_end = self.get_element_end_pos(&pair[0]);
            let next_start = pair[1].expr.span.start;
            prev_end < next_start && self.source[prev_end..next_start].contains(&b'\n')
        });
        let is_multiline = source_is_multiline
            || (self.force_pipeline_multiline_depth > 0
                && self.pipeline_requires_multiline(pipeline));

        for (i, element) in pipeline.elements.iter().enumerate() {
            if i > 0 {
                if is_multiline {
                    self.newline();
                    self.write_indent();
                    self.write("| ");
                } else {
                    self.write(" | ");
                }
            }
            self.format_pipeline_element(element);
        }
    }

    /// Format a single pipeline element (expression + optional redirection).
    pub(super) fn format_pipeline_element(&mut self, element: &PipelineElement) {
        self.format_expression(&element.expr);
        if let Some(ref redirection) = element.redirection {
            self.format_redirection(redirection);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Redirections
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a redirection.
    fn format_redirection(&mut self, redir: &PipelineRedirection) {
        match redir {
            PipelineRedirection::Single { target, .. } => {
                self.space();
                self.format_redirection_target(target);
            }
            PipelineRedirection::Separate { out, err } => {
                self.space();
                self.format_redirection_target(out);
                self.space();
                self.format_redirection_target(err);
            }
        }
    }

    /// Format a redirection target (file or pipe).
    fn format_redirection_target(&mut self, target: &RedirectionTarget) {
        match target {
            RedirectionTarget::File { expr, span, .. } => {
                self.write_span(*span);
                self.space();
                self.format_expression(expr);
            }
            RedirectionTarget::Pipe { span } => {
                self.write_span(*span);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Block expressions
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a block expression with optional braces.
    ///
    /// Simple single-pipeline blocks are kept inline; complex or multiline
    /// blocks are indented on new lines.
    pub(super) fn format_block_expression(
        &mut self,
        block_id: nu_protocol::BlockId,
        span: Span,
        with_braces: bool,
    ) {
        if with_braces && self.try_format_pipe_closure_block_from_span(span) {
            return;
        }

        let block = self.working_set.get_block(block_id);

        let source_has_newline = with_braces
            && span.end > span.start
            && self.source[span.start..span.end].contains(&b'\n');

        if with_braces {
            self.write("{");
        }

        // Reset conditional_context_depth inside block bodies so that
        // subexpressions nested inside `if` / `try` blocks keep their
        // explicit parentheses (fixes issue #131).
        let saved_conditional_depth = self.conditional_context_depth;
        self.conditional_context_depth = 0;

        let is_simple = block.pipelines.len() == 1
            && block.pipelines[0].elements.len() == 1
            && !self.block_has_nested_structures(block)
            && !source_has_newline;
        let has_comments_in_block_span = self.has_comments_in_span(span.start, span.end);
        let preserve_compact_record_like =
            with_braces && is_simple && self.block_expression_looks_like_compact_record(span);

        if is_simple && with_braces {
            if !preserve_compact_record_like {
                self.write(" ");
            }
            self.format_block(block);
            if !preserve_compact_record_like {
                self.write(" ");
            }
        } else if block.pipelines.is_empty() {
            if with_braces && has_comments_in_block_span {
                self.newline();
                self.indent_level += 1;
                self.write_comments_before(span.end.saturating_sub(1));
                self.indent_level -= 1;
                self.write_indent();
            } else if with_braces {
                self.write(" ");
            }
        } else {
            self.newline();
            self.indent_level += 1;
            self.format_block(block);
            self.newline();
            self.indent_level -= 1;
            self.write_indent();
        }

        if with_braces {
            self.write("}");
        }

        self.conditional_context_depth = saved_conditional_depth;
    }

    /// Detect whether a braced block looks like a compact record literal
    /// (e.g. `{name:"Alice"}`), which should preserve its tight formatting.
    fn block_expression_looks_like_compact_record(&self, span: Span) -> bool {
        if span.end <= span.start + 1 || span.end > self.source.len() {
            return false;
        }

        let slice = &self.source[span.start..span.end];
        if !slice.starts_with(b"{") || !slice.ends_with(b"}") {
            return false;
        }

        if slice.starts_with(b"{ ") || slice.ends_with(b" }") {
            return false;
        }

        slice[1..slice.len() - 1].contains(&b':')
    }

    /// Check if a block has nested structures that require multiline formatting.
    pub(super) fn block_has_nested_structures(&self, block: &Block) -> bool {
        block
            .pipelines
            .iter()
            .flat_map(|p| &p.elements)
            .any(|e| self.expr_is_complex(&e.expr))
    }

    /// Decide whether a pipeline should be expanded across multiple lines.
    pub(super) fn pipeline_requires_multiline(
        &self,
        pipeline: &nu_protocol::ast::Pipeline,
    ) -> bool {
        if pipeline.elements.len() > 3 {
            return true;
        }

        if pipeline
            .elements
            .iter()
            .any(|element| self.expr_contains_nested_pipeline(&element.expr))
        {
            return true;
        }

        let Some(first) = pipeline.elements.first() else {
            return false;
        };
        let Some(last) = pipeline.elements.last() else {
            return false;
        };

        let start = first.expr.span.start;
        let end = self.get_element_end_pos(last);
        if start >= end {
            return false;
        }

        let estimated_inline_len = self.config.indent * self.indent_level + (end - start);
        estimated_inline_len > self.config.line_length
    }

    fn expr_contains_nested_pipeline(&self, expr: &Expression) -> bool {
        match &expr.expr {
            Expr::Subexpression(block_id) | Expr::Block(block_id) | Expr::Closure(block_id) => {
                let block = self.working_set.get_block(*block_id);
                block.pipelines.iter().any(|pipeline| {
                    pipeline.elements.len() > 1
                        || pipeline
                            .elements
                            .iter()
                            .any(|element| self.expr_contains_nested_pipeline(&element.expr))
                })
            }
            Expr::Call(call) => call.arguments.iter().any(|arg| match arg {
                Argument::Positional(inner)
                | Argument::Unknown(inner)
                | Argument::Spread(inner) => self.expr_contains_nested_pipeline(inner),
                Argument::Named(named) => named
                    .2
                    .as_ref()
                    .is_some_and(|inner| self.expr_contains_nested_pipeline(inner)),
            }),
            Expr::Keyword(keyword) => self.expr_contains_nested_pipeline(&keyword.expr),
            Expr::BinaryOp(lhs, _, rhs) => {
                self.expr_contains_nested_pipeline(lhs) || self.expr_contains_nested_pipeline(rhs)
            }
            _ => false,
        }
    }

    /// Check if an expression is complex enough to warrant multiline formatting.
    pub(super) fn expr_is_complex(&self, expr: &Expression) -> bool {
        match &expr.expr {
            Expr::Block(_) | Expr::Closure(_) => true,
            Expr::List(items) => items.len() > 3,
            Expr::Record(items) => items.len() > 2,
            Expr::Call(call) => call.arguments.iter().any(|arg| match arg {
                Argument::Positional(e) | Argument::Unknown(e) | Argument::Spread(e) => {
                    self.expr_is_complex(e)
                }
                Argument::Named(n) => n.2.as_ref().is_some_and(|e| self.expr_is_complex(e)),
            }),
            _ => false,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Closure expressions
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a closure expression (`{|params| body}`), extracting and
    /// normalising parameters from the raw source.
    pub(super) fn format_closure_expression(&mut self, block_id: nu_protocol::BlockId, span: Span) {
        let content = self.get_span_content(span);
        let has_params = content
            .iter()
            .skip(1) // Skip '{'
            .find(|b| !b.is_ascii_whitespace())
            .is_some_and(|ch| *ch == b'|');

        if !has_params {
            self.format_block_expression(block_id, span, true);
            return;
        }

        let Some(first_pipe) = content.iter().position(|&b| b == b'|') else {
            self.write_bytes(&content);
            return;
        };

        let Some(second_pipe) = content[first_pipe + 1..]
            .iter()
            .position(|&b| b == b'|')
            .map(|p| first_pipe + 1 + p)
        else {
            self.write_bytes(&content);
            return;
        };

        self.write("{|");
        self.write_normalized_closure_params(&content[first_pipe + 1..second_pipe]);

        self.write("|");

        let block = self.working_set.get_block(block_id);
        let has_comments = self.has_comments_in_span(span.start, span.end);
        let is_simple = block.pipelines.len() == 1
            && block.pipelines[0].elements.len() == 1
            && !self.block_has_nested_structures(block)
            && !has_comments;

        if is_simple {
            self.space();
            self.format_block(block);
            self.write(" }");
        } else {
            self.newline();
            self.indent_level += 1;
            self.format_block(block);
            self.newline();
            self.indent_level -= 1;
            self.write_indent();
            self.write("}");
        }
    }

    fn write_normalized_closure_params(&mut self, params: &[u8]) {
        let mut params_iter = params.split(|&b| b == b',').peekable();

        while let Some(param) = params_iter.next() {
            let mut sub_parts = param.splitn(2, |&b| b == b':');

            if let (Some(param_name), Some(type_hint)) = (sub_parts.next(), sub_parts.next()) {
                self.write_bytes(param_name.trim_ascii());
                self.write_bytes(b": ");
                self.write_bytes(type_hint.trim_ascii());
            } else {
                self.write_bytes(param.trim_ascii());
            }

            if params_iter.peek().is_some() {
                self.write_bytes(b", ");
            }
        }
    }

    /// Normalise closure-like blocks parsed as regular block expressions,
    /// such as `{ |line| $line }`.
    fn try_format_pipe_closure_block_from_span(&mut self, span: Span) -> bool {
        if span.end <= span.start + 2 || span.end > self.source.len() {
            return false;
        }

        let raw = &self.source[span.start..span.end];
        if !raw.starts_with(b"{") || !raw.ends_with(b"}") {
            return false;
        }

        let inner = raw[1..raw.len() - 1].trim_ascii();
        if inner.first() != Some(&b'|') || inner.contains(&b'\n') {
            return false;
        }

        let Some(second_pipe_rel) = inner[1..]
            .iter()
            .position(|byte| *byte == b'|')
            .map(|pos| pos + 1)
        else {
            return false;
        };

        let params = &inner[1..second_pipe_rel];
        let body = inner[second_pipe_rel + 1..].trim_ascii();

        self.write("{|");
        self.write_normalized_closure_params(params);
        self.write("|");

        if !body.is_empty() {
            self.space();
            self.write_bytes(body);
            self.write(" ");
        }

        self.write("}");
        true
    }
}
