//! Expression formatting.
//!
//! Dispatches each [`Expr`] variant to the appropriate formatting logic,
//! and handles cell paths, subexpressions, binary operations, and ranges.

use super::Formatter;
use nu_protocol::{
    ast::{CellPath, Expr, Expression, FullCellPath, PathMember},
    Span,
};

impl<'a> Formatter<'a> {
    /// Format an expression by dispatching on its variant.
    pub(super) fn format_expression(&mut self, expr: &Expression) {
        match &expr.expr {
            // Literals and simple values — preserve original source
            Expr::Int(_)
            | Expr::Float(_)
            | Expr::Bool(_)
            | Expr::Nothing
            | Expr::DateTime(_)
            | Expr::String(_)
            | Expr::RawString(_)
            | Expr::Binary(_)
            | Expr::Filepath(_, _)
            | Expr::Directory(_, _)
            | Expr::Var(_)
            | Expr::VarDecl(_)
            | Expr::Operator(_)
            | Expr::StringInterpolation(_)
            | Expr::GlobInterpolation(_, _)
            | Expr::ImportPattern(_)
            | Expr::Overlay(_) => {
                self.write_expr_span(expr);
            }

            Expr::Garbage => {
                if !(self.try_write_redundant_pipeline_subexpr_without_outer_parens(expr)
                    || self.try_write_spacing_normalized_pipe_closure_garbage(expr))
                {
                    self.write_expr_span(expr);
                }
            }

            // Glob patterns — normalise empty-brace globs like `{ }` to `{}`
            // (the parser treats `{ }` in unknown-command args as a glob).
            Expr::GlobPattern(_, _) => {
                let content = &self.source[expr.span.start..expr.span.end];
                if content.starts_with(b"{")
                    && content.ends_with(b"}")
                    && content[1..content.len() - 1]
                        .iter()
                        .all(|b| b.is_ascii_whitespace())
                {
                    self.write("{}");
                } else {
                    self.write_expr_span(expr);
                }
            }

            Expr::Signature(sig) => {
                if self.has_comments_in_span(expr.span.start, expr.span.end) {
                    self.write_expr_span(expr);
                    self.last_pos = expr.span.end;
                    self.mark_comments_written_in_span(expr.span.start, expr.span.end);
                } else {
                    self.format_signature(sig);
                }
            }

            Expr::Call(call) => self.format_call(call),
            Expr::ExternalCall(head, args) => self.format_external_call(head, args),
            Expr::BinaryOp(lhs, op, rhs) => self.format_binary_op(lhs, op, rhs),
            Expr::UnaryNot(inner) => {
                self.write("not ");
                self.format_expression(inner);
            }

            Expr::Block(block_id) => {
                self.format_block_expression(*block_id, expr.span, false);
            }
            Expr::Closure(block_id) => {
                self.format_closure_expression(*block_id, expr.span);
            }
            Expr::Subexpression(block_id) => {
                self.format_subexpression(*block_id, expr.span);
            }

            Expr::List(items) => self.format_list(items),
            Expr::Record(items) => self.format_record(items, expr.span),
            Expr::Table(table) => self.format_table(&table.columns, &table.rows),

            Expr::Range(range) => self.format_range(range),
            Expr::CellPath(cell_path) => self.format_cell_path(cell_path),
            Expr::FullCellPath(full_path) => {
                self.format_full_cell_path(full_path);
            }

            Expr::RowCondition(_) => self.write_expr_span(expr),

            Expr::Keyword(keyword) => {
                self.write_span(keyword.span);
                self.space();
                self.format_block_or_expr(&keyword.expr);
            }

            Expr::ValueWithUnit(_) => {
                // Preserve original span — the parser normalises units
                // (e.g. 1kb → 1000b internally).
                self.write_expr_span(expr);
            }

            Expr::MatchBlock(matches) => self.format_match_block(matches),

            Expr::Collect(_, inner) => self.format_expression(inner),

            Expr::AttributeBlock(attr_block) => {
                for attr in &attr_block.attributes {
                    self.write_attribute_span(&attr.expr);
                    self.newline();
                }
                self.format_expression(&attr_block.item);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Binary operations and ranges
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a binary operation (`lhs op rhs`).
    pub(super) fn format_binary_op(&mut self, lhs: &Expression, op: &Expression, rhs: &Expression) {
        self.preserve_subexpr_parens_depth += 1;
        self.format_expression(lhs);
        self.preserve_subexpr_parens_depth -= 1;

        // Always add space around binary operators for valid Nushell syntax
        self.write(" ");
        self.format_expression(op);
        self.write(" ");

        // For assignment operators, unwrap Subexpression RHS to avoid double parens
        if let Expr::Operator(nu_protocol::ast::Operator::Assignment(_)) = &op.expr {
            if let Expr::Subexpression(block_id) = &rhs.expr {
                let block = self.working_set.get_block(*block_id);
                self.format_block(block);
                return;
            }
        }

        self.preserve_subexpr_parens_depth += 1;
        self.format_expression(rhs);
        self.preserve_subexpr_parens_depth -= 1;
    }

    /// Format a range expression (e.g. `1..5`, `1..2..10`).
    pub(super) fn format_range(&mut self, range: &nu_protocol::ast::Range) {
        let op = range.operator.to_string();
        if let Some(from) = &range.from {
            self.format_expression(from);
        }
        self.write(&op);
        if let Some(next) = &range.next {
            self.format_expression(next);
            // For step ranges (start..step..end), write the operator again before end
            self.write(&op);
        }
        if let Some(to) = &range.to {
            self.format_expression(to);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Cell paths
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a bare cell-path literal (e.g. `name.0`).
    pub(super) fn format_cell_path(&mut self, cell_path: &CellPath) {
        for (i, member) in cell_path.members.iter().enumerate() {
            if i > 0 {
                self.write(".");
            }
            self.format_cell_path_member(member);
        }
    }

    /// Format a full cell-path (expression + tail, e.g. `$record.name`).
    pub(super) fn format_full_cell_path(&mut self, cell_path: &FullCellPath) {
        self.format_expression(&cell_path.head);
        for member in &cell_path.tail {
            self.write(".");
            self.format_cell_path_member(member);
        }
    }

    /// Format a single cell-path member (string key or integer index, with
    /// optional `?` suffix).
    fn format_cell_path_member(&mut self, member: &PathMember) {
        match member {
            PathMember::String { val, optional, .. } => {
                self.write(val);
                if *optional {
                    self.write("?");
                }
            }
            PathMember::Int { val, optional, .. } => {
                self.write(&val.to_string());
                if *optional {
                    self.write("?");
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Subexpressions
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a subexpression (`(…)`).
    ///
    /// Decides whether to keep or strip the parentheses based on context
    /// (conditional, precedence, multiline, `$in` shorthand, etc.).
    pub(super) fn format_subexpression(&mut self, block_id: nu_protocol::BlockId, span: Span) {
        let block = self.working_set.get_block(block_id);
        let source_has_newline =
            span.end > span.start && self.source[span.start..span.end].contains(&b'\n');

        let has_explicit_parens = self.source.get(span.start) == Some(&b'(')
            && self.source.get(span.end - 1) == Some(&b')');

        if !has_explicit_parens {
            self.format_block(block);
            return;
        }

        // Preserve explicit multiline parenthesised expressions (author intent).
        if source_has_newline {
            self.write_span(span);
            return;
        }

        if self.conditional_context_depth > 0 {
            if self.preserve_subexpr_parens_depth > 0 {
                self.write("(");
                self.format_block(block);
                self.write(")");
                return;
            }

            let can_drop_parens =
                block.pipelines.len() == 1 && block.pipelines[0].elements.len() == 1;
            if can_drop_parens {
                self.format_block(block);
                return;
            }
            self.write("(");
            self.format_block(block);
            self.write(")");
            return;
        }

        // String interpolations inside subexpressions don't need parentheses
        if block.pipelines.len() == 1 && block.pipelines[0].elements.len() == 1 {
            if let Expr::StringInterpolation(_) = &block.pipelines[0].elements[0].expr.expr {
                self.format_block(block);
                return;
            }
        }

        // Preserve explicit parens in precedence-sensitive contexts
        if self.preserve_subexpr_parens_depth == 0
            && block.pipelines.len() == 1
            && !block.pipelines[0].elements.is_empty()
        {
            let first_element = &block.pipelines[0].elements[0];
            let element_text = String::from_utf8_lossy(
                &self.source[first_element.expr.span.start..first_element.expr.span.end],
            );
            if element_text == "$in" {
                self.format_block(block);
                return;
            }
        }

        // Set inline comment boundary at the closing paren so that comments
        // appearing after `)` on the same line are not captured inside (issue #133).
        let saved_bound = self.inline_comment_upper_bound;
        let closing_paren_pos = span.end.saturating_sub(1);
        self.inline_comment_upper_bound = Some(closing_paren_pos);

        self.write("(");
        let is_simple = !source_has_newline
            && block.pipelines.len() == 1
            && !self.pipeline_requires_multiline(&block.pipelines[0]);

        if is_simple {
            self.format_block(block);
        } else {
            self.newline();
            self.indent_level += 1;
            self.force_pipeline_multiline_depth += 1;
            self.format_block(block);
            self.force_pipeline_multiline_depth =
                self.force_pipeline_multiline_depth.saturating_sub(1);
            self.newline();
            self.indent_level -= 1;
            self.write_indent();
        }
        self.write(")");

        self.inline_comment_upper_bound = saved_bound;
    }

    /// Format an expression that could be a block or a regular expression.
    pub(super) fn format_block_or_expr(&mut self, expr: &Expression) {
        match &expr.expr {
            Expr::Block(block_id) => {
                self.format_block_expression(*block_id, expr.span, true);
            }
            Expr::Closure(block_id) => {
                self.format_closure_expression(*block_id, expr.span);
            }
            _ => self.format_expression(expr),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────────────

    /// Best-effort normalisation for a narrow garbage case:
    /// `((head) | tail)` -> `head | tail`.
    fn try_write_redundant_pipeline_subexpr_without_outer_parens(
        &mut self,
        expr: &Expression,
    ) -> bool {
        let raw = &self.source[expr.span.start..expr.span.end];
        if raw.contains(&b'\n') || raw.len() < 5 {
            return false;
        }

        if !(raw.starts_with(b"((") && raw.ends_with(b")") && raw.contains(&b'|')) {
            return false;
        }

        let Some(inner) = raw.get(1..raw.len() - 1) else {
            return false;
        };
        let Some(pipe_idx) = inner.iter().position(|b| *b == b'|') else {
            return false;
        };

        let left = &inner[..pipe_idx];
        let right = &inner[pipe_idx + 1..];
        let left_trimmed = left.trim_ascii();

        if !(left_trimmed.starts_with(b"(") && left_trimmed.ends_with(b")")) {
            return false;
        }

        let Some(unwrapped_left) = left_trimmed.get(1..left_trimmed.len() - 1) else {
            return false;
        };
        if unwrapped_left.is_empty() {
            return false;
        }

        self.write_bytes(unwrapped_left);
        self.write(" | ");
        self.write_bytes(right.trim_ascii());
        true
    }

    fn try_write_spacing_normalized_pipe_closure_garbage(&mut self, expr: &Expression) -> bool {
        let raw = self.get_span_content(expr.span);
        let trimmed = raw.trim_ascii();
        if trimmed.len() < 4 || trimmed.contains(&b'\n') {
            return false;
        }

        if !(trimmed.starts_with(b"{") && trimmed.ends_with(b"}")) {
            return false;
        }

        let inner = trimmed[1..trimmed.len() - 1].trim_ascii();
        if inner.first() != Some(&b'|') {
            return false;
        }

        let Some(second_pipe) = inner[1..]
            .iter()
            .position(|byte| *byte == b'|')
            .map(|pos| pos + 1)
        else {
            return false;
        };

        let params = &inner[1..second_pipe];
        let body = inner[second_pipe + 1..].trim_ascii();

        self.write("{|");
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

        if !body.is_empty() {
            self.space();
            self.write_bytes(body);
            self.write(" ");
        }

        self.write("}");
        true
    }

    /// Check if an expression is a simple primitive (used by collection
    /// formatting to decide inline vs. multiline layout).
    pub(super) fn is_simple_expr(&self, expr: &Expression) -> bool {
        match &expr.expr {
            Expr::Int(_)
            | Expr::Float(_)
            | Expr::Bool(_)
            | Expr::String(_)
            | Expr::RawString(_)
            | Expr::Nothing
            | Expr::Var(_)
            | Expr::StringInterpolation(_)
            | Expr::Filepath(_, _)
            | Expr::Directory(_, _)
            | Expr::GlobPattern(_, _)
            | Expr::DateTime(_) => true,
            Expr::FullCellPath(full_path) => {
                matches!(
                    &full_path.head.expr,
                    Expr::Var(_) | Expr::Garbage | Expr::Int(_) | Expr::String(_)
                ) && full_path.tail.iter().all(|member| {
                    matches!(member, PathMember::String { .. } | PathMember::Int { .. })
                })
            }
            _ => false,
        }
    }
}
