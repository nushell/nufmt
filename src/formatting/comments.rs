//! Comment extraction and formatting.
//!
//! Extracts `#`-prefixed comments from Nushell source while respecting string
//! boundaries, and provides methods on [`Formatter`] to emit them at the
//! correct locations in the output.

use super::Formatter;
use super::scan::{RegionKind, scan_regions};
use nu_protocol::Span;

/// Extract all comments from source code, returning their spans and content.
///
/// `#` characters inside string literals (including the nested expressions
/// of string interpolations) are not treated as comment starts.
pub(super) fn extract_comments(source: &[u8]) -> Vec<(Span, Vec<u8>)> {
    scan_regions(source)
        .into_iter()
        .filter(|region| region.kind == RegionKind::Comment)
        .map(|region| {
            (
                Span::new(region.start, region.end),
                source[region.start..region.end].to_vec(),
            )
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Formatter comment methods
// ─────────────────────────────────────────────────────────────────────────────

impl<'a> Formatter<'a> {
    /// Return true when the source slice contains at least one blank line
    /// (two newline boundaries with only whitespace between them).
    fn source_has_blank_line(&self, start: usize, end: usize) -> bool {
        if start >= end || end > self.source.len() {
            return false;
        }

        let mut previous_newline: Option<usize> = None;
        for (offset, byte) in self.source[start..end].iter().enumerate() {
            if *byte != b'\n' {
                continue;
            }

            if let Some(prev) = previous_newline {
                let gap_start = start + prev + 1;
                let gap_end = start + offset;
                if self.source[gap_start..gap_end]
                    .iter()
                    .all(|b| b.is_ascii_whitespace())
                {
                    return true;
                }
            }

            previous_newline = Some(offset);
        }

        false
    }

    /// Ensure `self.output` ends with at least `min_newlines` newline bytes,
    /// emitting additional ones as needed without over-indenting.
    fn ensure_trailing_newlines(&mut self, min_newlines: usize) {
        if self.output.is_empty() || min_newlines == 0 {
            return;
        }

        // Count contiguous trailing newlines so we can top up to the requested
        // separation without over-emitting line breaks.
        let existing = self
            .output
            .iter()
            .rev()
            .take_while(|&&byte| byte == b'\n')
            .count();

        for _ in existing..min_newlines {
            self.newline();
        }
    }

    /// Emit all comments that fall between `last_pos` and `pos`, each on its
    /// own line with the current indentation.
    pub(super) fn write_comments_before(&mut self, pos: usize) {
        let mut comments_to_write: Vec<_> = self
            .comments
            .iter()
            .enumerate()
            .filter(|(i, (span, _))| {
                !self.written_comments[*i] && span.start >= self.last_pos && span.end <= pos
            })
            .map(|(i, (span, content))| (i, span.start, content.clone()))
            .collect();

        comments_to_write.sort_by_key(|(_, start, _)| *start);

        let Some((_, first_start, _)) = comments_to_write.first() else {
            return;
        };

        let leading_newlines = if self.source_has_blank_line(self.last_pos, *first_start) {
            2
        } else {
            1
        };
        // Preserve spacing before a standalone comment group.
        self.ensure_trailing_newlines(leading_newlines);

        let mut prev_comment_end: Option<usize> = None;
        for (idx, start, content) in &comments_to_write {
            self.written_comments[*idx] = true;

            if let Some(prev_end) = prev_comment_end {
                let between_newlines = if self.source_has_blank_line(prev_end, *start) {
                    2
                } else {
                    1
                };
                self.ensure_trailing_newlines(between_newlines);
            }

            if !self.at_line_start
                && let Some(&last) = self.output.last()
                && last != b'\n'
            {
                self.newline();
            }
            self.write_indent();
            self.output.extend(content);
            self.newline();

            prev_comment_end = Some(start + content.len());
        }

        if let Some(last_comment_end) = prev_comment_end {
            self.last_pos = last_comment_end;
            if self.source_has_blank_line(last_comment_end, pos) {
                // Preserve a blank separator when comments are followed by a
                // spaced-apart statement group.
                self.ensure_trailing_newlines(2);
            }
        }
    }

    /// Emit the standalone comments that sit between the last statement of a
    /// delimited body and its closing delimiter at `close_pos` (issue #232).
    ///
    /// Returns `true` when any comment was written. The output is then at the
    /// start of a fresh line, with no blank line left before the delimiter.
    pub(super) fn write_trailing_body_comments(&mut self, close_pos: usize) -> bool {
        let before = self.output.len();
        self.write_comments_before(close_pos);
        if self.output.len() == before {
            return false;
        }

        while self.output.ends_with(b"\n\n") {
            self.output.pop();
        }
        true
    }

    /// Emit an inline comment (on the same line) that appears after `after_pos`,
    /// optionally bounded by an upper position limit.
    ///
    /// When `upper` is `Some(pos)`, comments starting at or after `pos` are
    /// ignored. This prevents capturing comments that belong outside a
    /// surrounding delimiter (e.g. after a closing parenthesis).
    pub(super) fn write_inline_comment_bounded(&mut self, after_pos: usize, upper: Option<usize>) {
        let line_end = self.source[after_pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(self.source.len(), |p| after_pos + p);

        let effective_end = upper.map_or(line_end, |u| u.min(line_end));

        let found = self
            .comments
            .iter()
            .enumerate()
            .find(|(i, (span, _))| {
                !self.written_comments[*i] && span.start >= after_pos && span.start < effective_end
            })
            .map(|(i, (span, content))| (i, *span, content.clone()));

        if let Some((idx, span, content)) = found {
            self.written_comments[idx] = true;
            self.write(" ");
            self.output.extend(&content);
            self.last_pos = span.end;
        }
    }

    /// Check whether the given span range contains any comments.
    pub(super) fn has_comments_in_span(&self, start: usize, end: usize) -> bool {
        self.comments
            .iter()
            .any(|(span, _)| span.start >= start && span.end <= end)
    }

    /// Move the comment cursor to `body_start`, the first byte inside a
    /// delimited body, unless that would skip a comment not yet written.
    ///
    /// The blank-line check before the body's first comment then looks only
    /// inside the body, not at the blank lines before the statement that
    /// holds it.
    pub(super) fn skip_to_body_start(&mut self, body_start: usize) {
        if self.last_pos < body_start && !self.has_unwritten_comments_in(self.last_pos, body_start)
        {
            self.last_pos = body_start;
        }
    }

    /// Return `true` when a comment inside `start..end` has not been written.
    pub(super) fn has_unwritten_comments_in(&self, start: usize, end: usize) -> bool {
        let first = self
            .comments
            .partition_point(|(span, _)| span.start < start);
        self.comments[first..]
            .iter()
            .zip(&self.written_comments[first..])
            .take_while(|((span, _), _)| span.start < end)
            .any(|((span, _), written)| !written && span.end <= end)
    }

    /// Mark all comments within the given span range as already written.
    pub(super) fn mark_comments_written_in_span(&mut self, start: usize, end: usize) {
        // Comments are extracted in source order, so the ones inside the
        // range are contiguous.
        let first = self
            .comments
            .partition_point(|(span, _)| span.start < start);
        for (i, (span, _)) in self.comments.iter().enumerate().skip(first) {
            if span.start >= end {
                break;
            }
            if span.end <= end {
                self.written_comments[i] = true;
            }
        }
    }
}
