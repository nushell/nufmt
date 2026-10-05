//! Lexical scanning of string literals and comments.
//!
//! Comment extraction and the parse-error repair passes both work on raw
//! source text, so they need to know which bytes belong to a string literal
//! or a comment. This module is the one place that decides that.
//!
//! String interpolations get special care: the `(...)` parts of `$"..."` and
//! `$'...'` hold nested expressions that may contain quotes of their own
//! (e.g. `$"(ansi "#ff0000")red"`), so they are walked the same way
//! `nu_parser::parse_string_interpolation` walks them (issue #220).
//!
//! A `#` inside a word (`nixpkgs#hello`, `.#default`) is part of the word,
//! not a comment, as in `nu_parser::lex`. Callers that scan part of the
//! source must start the slice at a line start or at a region boundary, so
//! that the first byte is classified the same way as in the whole source.

/// What a [`Region`] of source contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RegionKind {
    /// A string literal, including its delimiters.
    String,
    /// A `#` comment, up to (not including) the end of the line.
    Comment,
}

/// A byte range of source that holds a string literal or a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Region {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) kind: RegionKind,
}

/// Find every string literal and comment in `source`, in order.
///
/// An unterminated string literal extends to the end of the source.
pub(super) fn scan_regions(source: &[u8]) -> Vec<Region> {
    let mut regions = Vec::new();
    let mut i = 0;

    while i < source.len() {
        if let Some(end) = string_literal_end(source, i) {
            regions.push(Region {
                start: i,
                end,
                kind: RegionKind::String,
            });
            i = end;
            continue;
        }

        if source[i] == b'#' && starts_comment(source, i) {
            let start = i;
            while i < source.len() && source[i] != b'\n' {
                i += 1;
            }
            regions.push(Region {
                start,
                end: i,
                kind: RegionKind::Comment,
            });
            continue;
        }

        i += 1;
    }

    regions
}

/// Return `true` when the `#` at `source[i]` starts a comment.
///
/// `nu_parser::lex` only starts a comment at a token boundary or after
/// whitespace, so `a#b` is one word. The contents of `(...)`, `[...]` and
/// `{...}` are lexed again on their own, which makes a `#` right after an
/// opening delimiter or a list comma start a comment as well.
fn starts_comment(source: &[u8], i: usize) -> bool {
    i == 0
        || source[i - 1].is_ascii_whitespace()
        || matches!(source[i - 1], b';' | b'|' | b'(' | b'[' | b'{' | b',')
}

/// Return `true` when `text` contains `byte` outside its string literals
/// and comments.
pub(super) fn has_code_byte(text: &[u8], byte: u8) -> bool {
    let mut cursor = 0;
    for region in scan_regions(text) {
        if text[cursor..region.start].contains(&byte) {
            return true;
        }
        cursor = region.end;
    }
    text[cursor..].contains(&byte)
}

/// Return the region that contains byte `pos` (`start <= pos < end`).
///
/// `regions` must be sorted and non-overlapping, as [`scan_regions`]
/// returns them.
pub(super) fn region_at(regions: &[Region], pos: usize) -> Option<&Region> {
    let after = regions.partition_point(|region| region.start <= pos);
    regions[..after].last().filter(|region| pos < region.end)
}

/// Return the positions in `text` of each `byte` that is outside string
/// literals, comments, and brackets (`()`, `[]`, `{}`, `<>`), such as the
/// commas that separate closure parameters in `x: record<a: int, b: int>, y`.
pub(super) fn top_level_positions(text: &[u8], byte: u8) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut regions = scan_regions(text).into_iter().peekable();
    let mut depth = 0usize;
    let mut idx = 0;
    while idx < text.len() {
        if let Some(region) = regions.next_if(|region| region.start == idx) {
            idx = region.end;
            continue;
        }
        match text[idx] {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' | b'>' => depth = depth.saturating_sub(1),
            found if found == byte && depth == 0 => positions.push(idx),
            _ => {}
        }
        idx += 1;
    }
    positions
}

/// Return the index in `text` of the `|` that closes a closure's parameter
/// list, where `text` starts right after the opening `|`. A `|` inside a
/// string literal, a comment, or brackets, as in `{|x = "a|b"| $x }`, does
/// not count.
pub(super) fn closing_param_pipe(text: &[u8]) -> Option<usize> {
    let mut regions = scan_regions(text).into_iter().peekable();
    let mut depth = 0usize;
    let mut idx = 0;
    while idx < text.len() {
        if let Some(region) = regions.next_if(|region| region.start == idx) {
            idx = region.end;
            continue;
        }
        match text[idx] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b'|' if depth == 0 => return Some(idx),
            _ => {}
        }
        idx += 1;
    }
    None
}

/// Return `true` when `text` is one parenthesized group: the `(` it starts
/// with is closed by the `)` it ends with, as in `(a | b)` but not
/// `(a) + (b)`. Parens inside string literals and comments are ignored.
pub(super) fn parens_enclose(text: &[u8]) -> bool {
    if !(text.starts_with(b"(") && text.ends_with(b")")) {
        return false;
    }

    let mut regions = scan_regions(text).into_iter().peekable();
    let mut depth = 0usize;
    let mut idx = 0;
    while idx < text.len() {
        if let Some(region) = regions.next_if(|region| region.start == idx) {
            idx = region.end;
            continue;
        }

        match text[idx] {
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return idx == text.len() - 1;
                }
            }
            _ => {}
        }
        idx += 1;
    }

    false
}

/// If a string literal starts at `source[start]`, return the index just past
/// its end, or `source.len()` when it is unterminated.
fn string_literal_end(source: &[u8], start: usize) -> Option<usize> {
    match source[start] {
        // Only double-quoted strings process backslash escapes. Backtick
        // strings are bare words that may contain quotes and `#`.
        b'"' => Some(quoted_string_end(source, start + 1, b'"')),
        b'\'' | b'`' => Some(quoted_string_end(source, start + 1, source[start])),
        b'$' => match source.get(start + 1) {
            Some(&quote @ (b'"' | b'\'')) => Some(interpolation_end(source, start + 2, quote)),
            _ => None,
        },
        // Raw string: r#'...'# (or r##'...'##, ...). Without this the `#` in
        // the opener would start a bogus comment.
        b'r' => raw_string_open_hashes(source, start).map(|hashes| {
            let body_start = start + 1 + hashes + 1; // 'r' + N*'#' + '\''
            find_raw_string_end(source, body_start, hashes).unwrap_or(source.len())
        }),
        _ => None,
    }
}

/// Return the index just past the closing `quote` of a plain string whose
/// body starts at `body_start`.
fn quoted_string_end(source: &[u8], body_start: usize, quote: u8) -> usize {
    let mut i = body_start;
    while i < source.len() {
        let byte = source[i];
        if quote == b'"' && byte == b'\\' {
            i += 2;
            continue;
        }
        i += 1;
        if byte == quote {
            return i;
        }
    }
    source.len()
}

/// Return the index just past the closing `quote` of a `$"..."` or `$'...'`
/// interpolation whose body starts at `body_start`.
///
/// Inside a `(...)` expression, quotes delimit nested strings and parens
/// nest, so a quote there never closes the interpolation itself.
fn interpolation_end(source: &[u8], body_start: usize, quote: u8) -> usize {
    // Closing delimiters of the open expression parts and nested strings.
    let mut expected_closers: Vec<u8> = Vec::new();
    let mut i = body_start;

    while i < source.len() {
        let byte = source[i];
        i += 1;

        match expected_closers.last().copied() {
            None => {
                if quote == b'"' && byte == b'\\' {
                    i += 1;
                } else if byte == quote {
                    return i;
                } else if byte == b'(' {
                    expected_closers.push(b')');
                }
            }
            // Inside a nested string: only its own quote ends it, and a
            // nested double-quoted string has escapes of its own.
            Some(closer) if closer != b')' => {
                if closer == b'"' && byte == b'\\' {
                    i += 1;
                } else if byte == closer {
                    expected_closers.pop();
                }
            }
            // Inside an expression part.
            Some(_) => match byte {
                b'"' | b'\'' | b'`' => expected_closers.push(byte),
                b'(' => expected_closers.push(b')'),
                b')' => {
                    expected_closers.pop();
                }
                _ => {}
            },
        }
    }

    source.len()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Render each region as `kind:text` for compact assertions.
    fn regions(source: &str) -> Vec<String> {
        scan_regions(source.as_bytes())
            .into_iter()
            .map(|region| {
                let kind = match region.kind {
                    RegionKind::String => "str",
                    RegionKind::Comment => "comment",
                };
                format!("{kind}:{}", &source[region.start..region.end])
            })
            .collect()
    }

    #[test]
    fn plain_strings_and_comments() {
        assert_eq!(
            regions(r#"echo "a # b" 'c' # done"#),
            ["str:\"a # b\"", "str:'c'", "comment:# done"]
        );
    }

    #[test]
    fn double_quoted_escapes_do_not_close_the_string() {
        assert_eq!(regions(r#""a\"b" # c"#), ["str:\"a\\\"b\"", "comment:# c"]);
    }

    #[test]
    fn single_quoted_strings_are_raw() {
        assert_eq!(regions(r"'a\' # c"), ["str:'a\\'", "comment:# c"]);
    }

    #[test]
    fn backtick_strings_hide_quotes_and_hashes() {
        assert_eq!(
            regions("cd `it's #1` # real"),
            ["str:`it's #1`", "comment:# real"]
        );
    }

    #[test]
    fn raw_strings_hide_hashes() {
        assert_eq!(
            regions("r#'a # b'# # real"),
            ["str:r#'a # b'#", "comment:# real"]
        );
    }

    #[test]
    fn interpolation_with_nested_double_quotes_issue220() {
        assert_eq!(regions(r#"$"("a:")""#), [r#"str:$"("a:")""#]);
        assert_eq!(
            regions(r##"$"(ansi "#ff0000")red" # real"##),
            [r##"str:$"(ansi "#ff0000")red""##, "comment:# real"]
        );
    }

    #[test]
    fn interpolation_with_nested_parens_and_strings() {
        assert_eq!(
            regions(r#"$"a (f ("x)" | g ')')) b" # c"#),
            [r#"str:$"a (f ("x)" | g ')')) b""#, "comment:# c"]
        );
    }

    #[test]
    fn nested_interpolation() {
        assert_eq!(
            regions(r#"$"(echo $"(1)")" # c"#),
            [r#"str:$"(echo $"(1)")""#, "comment:# c"]
        );
    }

    #[test]
    fn escaped_quote_in_nested_string_does_not_end_it() {
        assert_eq!(
            regions(r#"$"(echo "a\"b")" # c"#),
            [r#"str:$"(echo "a\"b")""#, "comment:# c"]
        );
    }

    #[test]
    fn escaped_paren_in_double_quoted_interpolation_is_literal() {
        assert_eq!(regions(r#"$"\(" # c"#), [r#"str:$"\(""#, "comment:# c"]);
    }

    #[test]
    fn single_quoted_interpolation() {
        assert_eq!(
            regions(r#"$'(echo "x" ')') y' # c"#),
            [r#"str:$'(echo "x" ')') y'"#, "comment:# c"]
        );
    }

    #[test]
    fn unterminated_strings_extend_to_eof() {
        assert_eq!(regions("\"abc # c"), ["str:\"abc # c"]);
        assert_eq!(regions("$\"(abc # c"), ["str:$\"(abc # c"]);
    }

    #[test]
    fn parens_enclose_ignores_parens_in_strings_and_comments() {
        assert!(parens_enclose(b"(a)"));
        assert!(parens_enclose(b"(a | b)"));
        assert!(parens_enclose(br#"((pwd) | str replace ")" "(")"#));
        assert!(parens_enclose(b"(a # )\n)"));
        assert!(!parens_enclose(b"(a) + (b)"));
        assert!(!parens_enclose(b"(ls | length) + (ls | length)"));
        assert!(!parens_enclose(b"((a)"));
        assert!(!parens_enclose(b"a"));
    }

    #[test]
    fn dollar_without_quote_is_not_a_string() {
        assert_eq!(regions("$x # c"), ["comment:# c"]);
    }

    #[test]
    fn hash_inside_a_word_is_not_a_comment() {
        assert_eq!(regions("ls foo#bar # real"), ["comment:# real"]);
        assert_eq!(
            regions("^nix run nixpkgs#hello \"a # b\""),
            ["str:\"a # b\""]
        );
        assert_eq!(regions("(1)#x .#default"), Vec::<String>::new());
    }

    #[test]
    fn hash_after_a_separator_or_opening_delimiter_is_a_comment() {
        assert_eq!(regions("# a\nx;# b"), ["comment:# a", "comment:# b"]);
        assert_eq!(regions("[# c\n1,# d\n]"), ["comment:# c", "comment:# d"]);
        assert_eq!(regions("{# c\n}"), ["comment:# c"]);
        assert_eq!(regions("ls |# c"), ["comment:# c"]);
    }

    #[test]
    fn closing_param_pipe_skips_strings_and_brackets() {
        assert_eq!(closing_param_pipe(b"x| $x }"), Some(1));
        assert_eq!(closing_param_pipe(br#"x = "a|b"| $x }"#), Some(9));
        assert_eq!(closing_param_pipe(b"f = {|y| $y}| 1 }"), Some(12));
        assert_eq!(closing_param_pipe(b"x # a|b\n| 1 }"), Some(8));
        assert_eq!(closing_param_pipe(b"x"), None);
    }

    #[test]
    fn top_level_positions_skip_brackets_and_strings() {
        assert_eq!(top_level_positions(b"a, b", b','), [1]);
        assert_eq!(
            top_level_positions(b"r: record<a: int, b: int>, y", b','),
            [25]
        );
        assert_eq!(top_level_positions(br#"x = "a,b", y"#, b','), [9]);
        assert_eq!(
            top_level_positions(br#"x = "a:b""#, b':'),
            Vec::<usize>::new()
        );
        assert_eq!(top_level_positions(b"r: record<a: int>", b':'), [1]);
    }

    #[test]
    fn region_at_finds_the_containing_region() {
        let source = b"a \"bc\" d # e";
        let found = scan_regions(source);
        assert_eq!(region_at(&found, 0), None);
        assert_eq!(
            region_at(&found, 2).map(|r| r.kind),
            Some(RegionKind::String)
        );
        assert_eq!(
            region_at(&found, 5).map(|r| r.kind),
            Some(RegionKind::String)
        );
        assert_eq!(region_at(&found, 6), None);
        assert_eq!(
            region_at(&found, 9).map(|r| r.kind),
            Some(RegionKind::Comment)
        );
    }
}
