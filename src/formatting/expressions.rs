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
            | Expr::GlobPattern(_, _)
            | Expr::Var(_)
            | Expr::VarDecl(_)
            | Expr::Operator(_)
            | Expr::StringInterpolation(_)
            | Expr::GlobInterpolation(_, _)
            | Expr::ImportPattern(_)
            | Expr::Overlay(_)
            | Expr::Garbage => {
                self.write_expr_span(expr);
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

        self.write("(");
        let is_simple = !source_has_newline
            && block.pipelines.len() == 1
            && block.pipelines[0].elements.len() <= 3;

        if is_simple {
            self.format_block(block);
        } else {
            self.newline();
            self.indent_level += 1;
            self.format_block(block);
            self.newline();
            self.indent_level -= 1;
            self.write_indent();
        }
        self.write(")");
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
            | Expr::Filepath(_, _)
            | Expr::Directory(_, _)
            | Expr::GlobPattern(_, _)
            | Expr::DateTime(_) => true,
            Expr::FullCellPath(full_path) => {
                full_path.tail.is_empty()
                    && matches!(
                        &full_path.head.expr,
                        Expr::Var(_) | Expr::Garbage | Expr::Int(_) | Expr::String(_)
                    )
            }
            _ => false,
        }
    }
}
