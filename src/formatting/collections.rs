//! Collection formatting: lists, records, tables, and match blocks.
//!
//! Handles inline-vs-multiline layout decisions and
//! comma / whitespace normalisation for each collection type.

use super::Formatter;
use nu_protocol::{
    ast::{Expr, Expression, ListItem, MatchPattern, Pattern, RecordItem},
    Span,
};

impl<'a> Formatter<'a> {
    // ─────────────────────────────────────────────────────────────────────────
    // Lists
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a list expression, choosing inline or multiline layout.
    pub(super) fn format_list(&mut self, items: &[ListItem]) {
        if items.is_empty() {
            self.write("[]");
            return;
        }

        let uses_commas = self.list_uses_commas(items);
        let source_has_newline = self.list_has_source_newline(items);

        let all_simple = items.iter().all(|item| match item {
            ListItem::Item(expr) | ListItem::Spread(_, expr) => self.is_simple_expr(expr),
        });

        let inline_single_item = items.len() == 1 && all_simple;
        let should_preserve_multiline = source_has_newline && items.len() > 1;
        let should_inline =
            inline_single_item || (all_simple && items.len() <= 5 && !should_preserve_multiline);

        if should_inline {
            self.write("[");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    if uses_commas {
                        self.write(", ");
                    } else {
                        self.write(" ");
                    }
                }
                self.format_list_item(item);
            }
            self.write("]");
        } else {
            self.write("[");
            self.newline();
            self.indent_level += 1;

            let mut idx = 0;
            while idx < items.len() {
                self.write_indent();

                if idx + 1 < items.len() {
                    let current = &items[idx];
                    let next = &items[idx + 1];
                    if self.should_pair_flag_value_items(current, next)
                        && self.paired_list_items_fit_on_line(current, next, uses_commas)
                    {
                        self.format_list_item(current);
                        if uses_commas {
                            self.write(", ");
                        } else {
                            self.write(" ");
                        }
                        self.format_list_item(next);
                        self.newline();
                        idx += 2;
                        continue;
                    }
                }

                self.format_list_item(&items[idx]);
                self.newline();
                idx += 1;
            }

            self.indent_level -= 1;
            self.write_indent();
            self.write("]");
        }
    }

    /// Detect whether items are comma-separated in the original source.
    fn list_uses_commas(&self, items: &[ListItem]) -> bool {
        if items.len() < 2 {
            return false;
        }

        let (_, mut prev_end) = self.list_item_bounds(&items[0]);
        for item in items.iter().skip(1) {
            let (start, end) = self.list_item_bounds(item);
            if start > prev_end && self.source[prev_end..start].contains(&b',') {
                return true;
            }
            prev_end = end;
        }

        false
    }

    /// Format a single list item (regular or spread).
    fn format_list_item(&mut self, item: &ListItem) {
        match item {
            ListItem::Item(expr) => self.format_expression(expr),
            ListItem::Spread(_, expr) => {
                self.write("...");
                self.format_expression(expr);
            }
        }
    }

    fn list_item_bounds(&self, item: &ListItem) -> (usize, usize) {
        match item {
            ListItem::Item(expr) | ListItem::Spread(_, expr) => (expr.span.start, expr.span.end),
        }
    }

    fn list_has_source_newline(&self, items: &[ListItem]) -> bool {
        if items.len() < 2 {
            return false;
        }

        let (_, mut prev_end) = self.list_item_bounds(&items[0]);
        for item in items.iter().skip(1) {
            let (start, end) = self.list_item_bounds(item);
            if prev_end < start && self.source[prev_end..start].contains(&b'\n') {
                return true;
            }
            prev_end = end;
        }

        false
    }

    fn should_pair_flag_value_items(&self, current: &ListItem, next: &ListItem) -> bool {
        self.list_item_is_flag_string(current) && !self.list_item_is_flag_string(next)
    }

    fn list_item_is_flag_string(&self, item: &ListItem) -> bool {
        let expr = match item {
            ListItem::Item(expr) => expr,
            ListItem::Spread(_, _) => return false,
        };

        if !matches!(expr.expr, Expr::String(_)) {
            return false;
        }

        let raw = self.get_span_content(expr.span);
        let trimmed = raw.trim_ascii();
        if trimmed.len() < 3 || trimmed.first() != Some(&b'"') || trimmed.last() != Some(&b'"') {
            return false;
        }

        let inner = &trimmed[1..trimmed.len() - 1];
        inner.starts_with(b"-")
    }

    fn paired_list_items_fit_on_line(
        &self,
        current: &ListItem,
        next: &ListItem,
        uses_commas: bool,
    ) -> bool {
        let left_len = self.probe_format(|p| p.format_list_item(current)).len();
        let right_len = self.probe_format(|p| p.format_list_item(next)).len();

        let separator_len = if uses_commas { 2 } else { 1 };
        let indent_len = self.config.indent * self.indent_level;

        indent_len + left_len + separator_len + right_len <= self.config.line_length
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Records
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a record expression, choosing inline or multiline layout.
    pub(super) fn format_record(&mut self, items: &[RecordItem], span: Span) {
        if items.is_empty() {
            self.write("{}");
            return;
        }

        let source_has_newline =
            span.end > span.start && self.source[span.start..span.end].contains(&b'\n');

        let preserve_compact = self.record_preserve_compact_style(span);

        let all_simple = items.iter().all(|item| match item {
            RecordItem::Pair(k, v) => self.is_simple_expr(k) && self.is_simple_expr(v),
            RecordItem::Spread(_, expr) => self.is_simple_expr(expr),
        });

        let has_nested_complex = items.iter().any(|item| match item {
            RecordItem::Pair(_, v) => matches!(
                &v.expr,
                Expr::Record(_)
                    | Expr::List(_)
                    | Expr::Closure(_)
                    | Expr::Block(_)
                    | Expr::StringInterpolation(_)
                    | Expr::Subexpression(_)
            ),
            RecordItem::Spread(_, _) => false,
        });

        // Records with 2+ items and complex values should be multiline when nested
        let nested_multiline = self.indent_level > 0 && items.len() >= 2 && has_nested_complex;

        let preserve_multiline_top_level = source_has_newline && self.indent_level == 0;

        if all_simple && items.len() <= 3 && !nested_multiline && !preserve_multiline_top_level {
            // Inline format
            let record_start = self.output.len();
            self.write("{");
            if preserve_compact {
                self.write(" ");
            }
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.format_record_item(item, preserve_compact);
            }
            if !preserve_compact
                && self.output.len() > record_start + 1
                && self.output[record_start + 1] == b' '
            {
                self.output.remove(record_start + 1);
            }
            if preserve_compact {
                self.write(" ");
            }
            self.write("}");
        } else {
            // Multiline format
            self.write("{");
            self.newline();
            self.indent_level += 1;

            let closing_brace_pos = span.end.saturating_sub(1);
            for item in items {
                let item_start = match item {
                    RecordItem::Pair(key, _) => key.span.start,
                    RecordItem::Spread(_, expr) => expr.span.start,
                };
                self.write_comments_before(item_start);
                self.write_indent();
                self.format_record_item(item, preserve_compact);

                // Capture inline comments on the same line as the record item.
                let item_end = match item {
                    RecordItem::Pair(_, v) => v.span.end,
                    RecordItem::Spread(_, expr) => expr.span.end,
                };
                self.write_inline_comment_bounded(item_end, Some(closing_brace_pos));
                // Advance beyond the current item so inter-item comments are
                // searched from the correct source window.
                self.last_pos = self.last_pos.max(item_end);
                self.newline();
            }

            self.write_comments_before(closing_brace_pos);
            self.indent_level -= 1;
            self.write_indent();
            self.write("}");
        }
    }

    /// Determine whether a record should preserve compact colon style
    /// (used for repaired malformed records like `{ name:Alice, age:30 }`).
    fn record_preserve_compact_style(&self, span: Span) -> bool {
        if !self.allow_compact_recovered_record_style {
            return false;
        }

        if span.end <= span.start || span.end > self.source.len() {
            return false;
        }

        let slice = &self.source[span.start..span.end];
        slice.starts_with(b"{ ")
            && slice.ends_with(b" }")
            && slice.contains(&b',')
            && !slice.windows(2).any(|window| window == b": ")
    }

    /// Format a single record item (key-value pair or spread).
    fn format_record_item(&mut self, item: &RecordItem, compact_colon: bool) {
        match item {
            RecordItem::Pair(key, value) => {
                self.format_record_key(key);
                if compact_colon {
                    self.write(":");
                } else {
                    self.write(": ");
                }
                self.format_expression(value);
            }
            RecordItem::Spread(_, expr) => {
                self.write("...");
                self.format_expression(expr);
            }
        }
    }

    /// Format a record key, trimming any surrounding whitespace from the
    /// source span.
    fn format_record_key(&mut self, key: &Expression) {
        let span_contents = self.get_span_content(key.span);

        let start = span_contents
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .unwrap_or(span_contents.len());
        let end = span_contents
            .iter()
            .rposition(|byte| !byte.is_ascii_whitespace())
            .map(|idx| idx + 1)
            .unwrap_or(0);

        if start < end {
            self.write_bytes(&span_contents[start..end]);
        } else {
            self.format_expression(key);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tables
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a table expression (`[[col1, col2]; [val1, val2]]`).
    pub(super) fn format_table(&mut self, columns: &[Expression], rows: &[Box<[Expression]>]) {
        self.write("[");

        // Header row
        self.write("[");
        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.format_expression(col);
        }
        self.write("]");

        // Data rows
        if !rows.is_empty() {
            self.write("; ");
            for (i, row) in rows.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write("[");
                for (j, cell) in row.iter().enumerate() {
                    if j > 0 {
                        self.write(", ");
                    }
                    self.format_expression(cell);
                }
                self.write("]");
            }
        }

        self.write("]");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Match blocks
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a match block (`match $val { pattern => expr }`).
    pub(super) fn format_match_block(&mut self, matches: &[(MatchPattern, Expression)]) {
        let rendered_lhs: Vec<Vec<u8>> = matches
            .iter()
            .map(|(pattern, expr)| self.render_match_arm_lhs(pattern, expr))
            .collect();

        let should_align = self.should_preserve_match_arm_alignment(matches)
            && rendered_lhs.iter().all(|lhs| !lhs.contains(&b'\n'));
        let max_lhs_len = if should_align {
            rendered_lhs.iter().map(Vec::len).max().unwrap_or(0)
        } else {
            0
        };

        self.write("{");
        self.newline();
        self.indent_level += 1;

        for ((_pattern, expr), lhs) in matches.iter().zip(rendered_lhs.iter()) {
            self.write_indent();
            self.write_bytes(lhs);

            if should_align {
                let spaces = max_lhs_len.saturating_sub(lhs.len()) + 1;
                for _ in 0..spaces {
                    self.output.push(b' ');
                }
            } else {
                self.write(" ");
            }

            self.write("=> ");
            self.format_block_or_expr(expr);
            self.newline();
        }

        self.indent_level -= 1;
        self.write_indent();
        self.write("}");
    }

    /// Format a match pattern (value, variable, list, record, or, etc.).
    pub(super) fn format_match_pattern(&mut self, pattern: &MatchPattern) {
        match &pattern.pattern {
            Pattern::Expression(expr) => self.format_expression(expr),
            Pattern::Value(_) | Pattern::Variable(_) | Pattern::Rest(_) | Pattern::Garbage => {
                self.write_span(pattern.span);
            }
            Pattern::Or(patterns) => {
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        self.write(" | ");
                    }
                    self.format_match_pattern(p);
                }
            }
            Pattern::List(patterns) => {
                self.write("[");
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.format_match_pattern(p);
                }
                self.write("]");
            }
            Pattern::Record(entries) => {
                self.write("{");
                for (i, (key, pat)) in entries.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(key);
                    self.write(": ");
                    self.format_match_pattern(pat);
                }
                self.write("}");
            }
            Pattern::IgnoreRest => self.write(".."),
            Pattern::IgnoreValue => self.write("_"),
        }
    }

    fn render_match_arm_lhs(&self, pattern: &MatchPattern, rhs: &Expression) -> Vec<u8> {
        self.probe_format(|probe| {
            probe.format_match_pattern_for_arm(pattern, rhs);
            if let Some(guard) = &pattern.guard {
                probe.write(" if ");
                probe.format_expression(guard);
            }
        })
    }

    fn should_preserve_match_arm_alignment(&self, matches: &[(MatchPattern, Expression)]) -> bool {
        let spacing_hints: Vec<usize> = matches
            .iter()
            .filter_map(|(pattern, expr)| self.source_spaces_before_match_arrow(pattern, expr))
            .collect();

        if spacing_hints.len() < 2 || spacing_hints.iter().all(|spaces| *spaces <= 1) {
            return false;
        }

        let arrow_columns: Vec<usize> = matches
            .iter()
            .filter_map(|(pattern, expr)| self.source_match_arrow_column(pattern, expr))
            .collect();

        if arrow_columns.len() < 2 {
            return false;
        }

        let first = arrow_columns[0];
        arrow_columns.iter().all(|col| *col == first)
    }

    fn source_match_arrow_column(
        &self,
        pattern: &MatchPattern,
        expr: &Expression,
    ) -> Option<usize> {
        if pattern.span.end >= expr.span.start || expr.span.start > self.source.len() {
            return None;
        }

        let between = &self.source[pattern.span.end..expr.span.start];
        let arrow_idx = between.windows(2).position(|pair| pair == b"=>")?;
        let line_start = self.source[..pattern.span.start]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |idx| idx + 1);

        Some(pattern.span.end.saturating_sub(line_start) + arrow_idx)
    }

    fn source_spaces_before_match_arrow(
        &self,
        pattern: &MatchPattern,
        expr: &Expression,
    ) -> Option<usize> {
        if pattern.span.end >= expr.span.start || expr.span.start > self.source.len() {
            return None;
        }

        let between = &self.source[pattern.span.end..expr.span.start];
        let arrow_idx = between.windows(2).position(|pair| pair == b"=>")?;
        let before_arrow = &between[..arrow_idx];

        Some(
            before_arrow
                .iter()
                .rev()
                .take_while(|byte| byte.is_ascii_whitespace())
                .count(),
        )
    }

    fn format_match_pattern_for_arm(&mut self, pattern: &MatchPattern, rhs: &Expression) {
        if let Pattern::Expression(expr) = &pattern.pattern {
            if self.should_unquote_identifier_safe_match_pattern(expr, rhs) {
                let raw = self.get_span_content(expr.span);
                let trimmed = raw.trim_ascii();
                let inner = &trimmed[1..trimmed.len() - 1];
                self.write_bytes(inner);
                return;
            }
        }

        self.format_match_pattern(pattern);
    }

    fn should_unquote_identifier_safe_match_pattern(
        &self,
        expr: &Expression,
        rhs: &Expression,
    ) -> bool {
        if !matches!(expr.expr, Expr::String(_)) {
            return false;
        }

        if matches!(
            rhs.expr,
            Expr::Subexpression(_) | Expr::Block(_) | Expr::Closure(_)
        ) {
            return false;
        }

        let rhs_raw = self.get_span_content(rhs.span);
        let rhs_trimmed = rhs_raw.trim_ascii();
        if rhs_trimmed.starts_with(b"(") && rhs_trimmed.ends_with(b")") {
            return false;
        }

        let raw = self.get_span_content(expr.span);
        let trimmed = raw.trim_ascii();
        if trimmed.len() < 3 || trimmed.first() != Some(&b'"') || trimmed.last() != Some(&b'"') {
            return false;
        }

        let inner = &trimmed[1..trimmed.len() - 1];
        if inner.is_empty() || inner.contains(&b'\\') {
            return false;
        }

        let first = inner[0];
        if !(first.is_ascii_alphabetic() || first == b'_') {
            return false;
        }

        if !inner
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            return false;
        }

        true
    }
}
