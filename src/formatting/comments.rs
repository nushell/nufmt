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

    while i < source.len() {
        let c = source[i];

        // Raw string: r#'...'# (or r##'...'##, r###'...'###, ...). Skip the whole
        // literal so that neither the `#` in its delimiters nor any `#` in its
        // body is ever mistaken for a comment. Without this, the `#` in `r#'`
        // starts a bogus comment and the closing `'#` opens a phantom string
        // that swallows every real comment until the next stray apostrophe.
        if c == b'r' {
            if let Some(hashes) = raw_string_open_hashes(source, i) {
                let body_start = i + 1 + hashes + 1; // 'r' + N*'#' + '\''
                i = find_raw_string_end(source, body_start, hashes).unwrap_or(source.len());
                continue;
            }
        }

        // Quoted string. Single-quoted strings are raw (no escapes); only
        // double-quoted strings process backslash escapes. Consuming each string
        // inline (rather than a persistent flag) means an unterminated string can
        // never bleed across the rest of the file.
        if c == b'"' || c == b'\'' {
            let quote = c;
            i += 1;
            while i < source.len() {
                let b = source[i];
                if quote == b'"' && b == b'\\' && i + 1 < source.len() {
                    i += 2;
                    continue;
                }
                i += 1;
                if b == quote {
                    break;
                }
            }
            continue;
        }

        // Found a comment
        if c == b'#' {
            let start = i;
            while i < source.len() && source[i] != b'\n' {
                i += 1;
            }
            comments.push((Span::new(start, i), source[start..i].to_vec()));
            continue;
        }

        i += 1;
    }

    comments
}

/// If `source[i..]` begins a raw-string opener (`r#'`, `r##'`, ...), return the
/// number of `#` hashes; otherwise `None`. Requires `source[i] == b'r'`.
fn raw_string_open_hashes(source: &[u8], i: usize) -> Option<usize> {
    let mut j = i + 1;
    let mut hashes = 0;
    while j < source.len() && source[j] == b'#' {
        hashes += 1;
        j += 1;
    }
    if hashes >= 1 && source.get(j) == Some(&b'\'') {
        Some(hashes)
    } else {
        None
    }
}

/// Find the byte index just past a raw-string closer (`'` followed by `hashes`
/// `#`), scanning from `body_start`. Returns `None` if unterminated.
fn find_raw_string_end(source: &[u8], body_start: usize, hashes: usize) -> Option<usize> {
    let mut j = body_start;
    while j < source.len() {
        if source[j] == b'\'' {
            let close = j + 1 + hashes;
            if source.len() >= close && source[j + 1..close].iter().all(|&b| b == b'#') {
                return Some(close);
            }
        }
        j += 1;
    }
    None
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

        if let Some(last_comment_end) = prev_comment_end {
            self.last_pos = last_comment_end;
            if self.source_has_blank_line(last_comment_end, pos) {
                // Preserve a blank separator when comments are followed by a
                // spaced-apart statement group.
                self.ensure_trailing_newlines(2);
            }
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
