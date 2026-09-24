//! Lightweight structural scanning — NOT a math parser.
//!
//! Per the spec, we do not build a full expression parser. We only need enough
//! structure to (a) validate balanced parens/brackets and (b) split a payload on
//! top-level delimiters (not nested inside (), [], {}). Expression payloads are
//! otherwise passed through verbatim.

/// A balance / bracket problem kind found while scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BalanceErrorKind {
    /// An opener that never closed.
    Unclosed(char),
    /// A closer with no matching opener.
    UnexpectedClose(char),
    /// Mismatched opener and closer (e.g. `(..]`).
    Mismatched { open: char, close: char },
}

/// A balance / bracket problem with its 0-based UTF-8 byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalanceError {
    pub kind: BalanceErrorKind,
    pub position: usize,
}

const PAIRS: &[(char, char)] = &[('(', ')'), ('[', ']'), ('{', '}')];
const CLOSERS: &[char] = &[')', ']', '}'];

fn matching_close(ch: char) -> Option<char> {
    PAIRS.iter().find(|(o, _)| *o == ch).map(|(_, c)| *c)
}

fn is_closer(ch: char) -> bool {
    CLOSERS.contains(&ch)
}

/// Trims ASCII spaces and tabs.
pub fn trimmed_shim(s: &str) -> &str {
    s.trim_matches(|c| c == ' ' || c == '\t')
}

/// Validates that all `()`, `[]`, `{}` in `s` are balanced and correctly nested.
///
/// Returns the first balance problem found, or `None` if balanced.
pub fn balance_error(s: &str) -> Option<BalanceError> {
    let mut stack: Vec<(char, usize)> = Vec::new();

    for (offset, ch) in s.char_indices() {
        if let Some(_) = matching_close(ch) {
            stack.push((ch, offset));
        } else if is_closer(ch) {
            match stack.pop() {
                None => {
                    return Some(BalanceError {
                        kind: BalanceErrorKind::UnexpectedClose(ch),
                        position: offset,
                    });
                }
                Some((open, _)) => {
                    let expected = matching_close(open).unwrap();
                    if expected != ch {
                        return Some(BalanceError {
                            kind: BalanceErrorKind::Mismatched { open, close: ch },
                            position: offset,
                        });
                    }
                }
            }
        }
    }

    if let Some((leftover, pos)) = stack.first() {
        return Some(BalanceError {
            kind: BalanceErrorKind::Unclosed(*leftover),
            position: *pos,
        });
    }

    None
}

/// Splits `s` on top-level commas (depth 0 w.r.t. (), [], {}), trimming each piece.
pub fn split_top_level_commas(s: &str) -> Vec<String> {
    split_top_level(s, ',')
}

/// Splits `s` on top-level semicolons (depth 0 w.r.t. (), [], {}), trimming each piece.
pub fn split_top_level_semicolons(s: &str) -> Vec<String> {
    split_top_level(s, ';')
}

fn split_top_level(s: &str, delimiter: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth: isize = 0;
    let mut current = String::new();

    for ch in s.chars() {
        if matching_close(ch).is_some() {
            depth += 1;
            current.push(ch);
        } else if is_closer(ch) {
            depth -= 1;
            current.push(ch);
        } else if ch == delimiter && depth == 0 {
            parts.push(trimmed_shim(&current).to_string());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    parts.push(trimmed_shim(&current).to_string());
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balanced() {
        assert_eq!(balance_error("(x + 1) * [2 + {3}]"), None);
    }

    #[test]
    fn test_unclosed() {
        let err = balance_error("sin(x").unwrap();
        assert_eq!(err.kind, BalanceErrorKind::Unclosed('('));
        assert_eq!(err.position, 3);
    }

    #[test]
    fn test_unexpected_close() {
        let err = balance_error("x + 1)").unwrap();
        assert_eq!(err.kind, BalanceErrorKind::UnexpectedClose(')'));
        assert_eq!(err.position, 5);
    }

    #[test]
    fn test_mismatched() {
        let err = balance_error("(x + 1]").unwrap();
        assert_eq!(
            err.kind,
            BalanceErrorKind::Mismatched {
                open: '(',
                close: ']'
            }
        );
        assert_eq!(err.position, 6);
    }

    #[test]
    fn test_split_top_level() {
        let parts = split_top_level_commas("f(x, y), g([1, 2]), z");
        assert_eq!(parts, vec!["f(x, y)", "g([1, 2])", "z"]);
    }
}
