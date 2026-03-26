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

            self.format_pipeline(pipeline);

            if let Some(last_elem) = pipeline.elements.last() {
                let end_pos = self.get_element_end_pos(last_elem);
                self.write_inline_comment(end_pos);
                self.last_pos = end_pos;
            }

            if i < num_pipelines - 1 {
                let separator_newlines = if self.indent_level == 0 && self.config.margin > 1 {
                    self.config.margin.saturating_add(1)
                } else {
                    1
                };

                for _ in 0..separator_newlines {
                    self.newline();
                }
            }
        }
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
    pub(super) fn format_pipeline(&mut self, pipeline: &nu_protocol::ast::Pipeline) {
        for (i, element) in pipeline.elements.iter().enumerate() {
            if i > 0 {
                self.write(" | ");
            }
            self.format_pipeline_element(element);
        }
    }

    /// Format a single pipeline element (expression + optional redirection).
    fn format_pipeline_element(&mut self, element: &PipelineElement) {
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
        let block = self.working_set.get_block(block_id);

        let source_has_newline = with_braces
            && span.end > span.start
            && self.source[span.start..span.end].contains(&b'\n');

        if with_braces {
            self.write("{");
        }

        let is_simple = block.pipelines.len() == 1
            && block.pipelines[0].elements.len() == 1
            && !self.block_has_nested_structures(block)
            && !source_has_newline;
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
            if with_braces {
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

        // Normalise parameter whitespace
        let params = &content[first_pipe + 1..second_pipe];
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

        self.write("|");

        let block = self.working_set.get_block(block_id);
        let is_simple = block.pipelines.len() == 1
            && block.pipelines[0].elements.len() == 1
            && !self.block_has_nested_structures(block);

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
}
