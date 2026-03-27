//! Comment extraction and formatting.
//!
//! Extracts `#`-prefixed comments from Nushell source while respecting string
//! boundaries, and provides methods on [`Formatter`] to emit them at the
//! correct locations in the output.

use super::Formatter;
use nu_protocol::Span;

/// Extract all comments from source code, returning their spans and content.
///
/// Tracks string state so that `#` characters inside quoted strings are not
/// treated as comment starts.
pub(super) fn extract_comments(source: &[u8]) -> Vec<(Span, Vec<u8>)> {
    let mut comments = Vec::new();
    let mut i = 0;
    let mut in_string = false;
    let mut string_char = b'"';

    while i < source.len() {
        let c = source[i];

        // Track string state to avoid matching # inside strings
        if !in_string && (c == b'"' || c == b'\'') {
            in_string = true;
            string_char = c;
            i += 1;
            continue;
        }

        if in_string {
            if c == b'\\' && i + 1 < source.len() {
                i += 2; // Skip escaped character
                continue;
            }
            if c == string_char {
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Found a comment
        if c == b'#' {
            let start = i;
            while i < source.len() && source[i] != b'\n' {
                i += 1;
            }
            comments.push((Span::new(start, i), source[start..i].to_vec()));
        }

        i += 1;
    }

    comments
}

// ─────────────────────────────────────────────────────────────────────────────
// Formatter comment methods
// ─────────────────────────────────────────────────────────────────────────────

impl<'a> Formatter<'a> {
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

        let mut prev_comment_end: Option<usize> = None;
        for (idx, start, content) in &comments_to_write {
            self.written_comments[*idx] = true;

            // Preserve blank lines between comment groups by checking
            // whether the original source has a blank line between the
            // previous comment and this one.
            if let Some(prev_end) = prev_comment_end {
                let between = &self.source[prev_end..*start];
                let newline_count = between.iter().filter(|&&b| b == b'\n').count();
                if newline_count >= 2 {
                    // There was at least one blank line in the source
                    // between the two comments — emit one now.
                    if !self.at_line_start {
                        self.newline();
                    }
                    self.newline();
                }
            }

            if !self.at_line_start {
                if let Some(&last) = self.output.last() {
                    if last != b'\n' {
                        self.newline();
                    }
                }
            }
            self.write_indent();
            self.output.extend(content);
            self.newline();

            prev_comment_end = Some(start + content.len());
        }
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

    /// Mark all comments within the given span range as already written.
    pub(super) fn mark_comments_written_in_span(&mut self, start: usize, end: usize) {
        for (i, (span, _)) in self.comments.iter().enumerate() {
            if span.start >= start && span.end <= end {
                self.written_comments[i] = true;
            }
        }
    }
}
