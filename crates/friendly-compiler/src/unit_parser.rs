//! Natural physical unit parsing using `logos` and `nom`.
//!
//! Handles:
//! - Unit conversion clauses: `<expr> to <target>` and `<expr> in <target>`
//! - Natural quantity juxtaposition: `32 mi` -> `32*mi`, `52 mi^2` -> `52*mi^2`
//! - Automatic unit simplification: wrapping unit expressions in `__casette_convert(...)`

use crate::units;
use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t]+")]
pub enum Token<'a> {
    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice())]
    Number(&'a str),

    #[token("to")]
    To,

    #[token("in")]
    In,

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice())]
    Ident(&'a str),

    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("^")]
    Caret,
    #[token("=")]
    Eq,
    #[token("==")]
    EqEq,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
}

/// A parsed unit conversion clause (e.g. `100 km / 2 hr in mph`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitConversion {
    pub expression: String,
    pub target_unit: String,
}

/// Checks if an input ends with a unit conversion clause (`... to <unit>` or `... in <unit>`).
///
/// If found, returns the expression and the target unit name.
#[must_use]
pub fn parse_unit_conversion(input: &str) -> Option<UnitConversion> {
    let trimmed = input.trim();

    // Look for trailing " to <unit>" or " in <unit>"
    for keyword in &[" to ", " in "] {
        if let Some(pos) = trimmed.rfind(keyword) {
            let expr = trimmed[..pos].trim();
            let target = trimmed[pos + keyword.len()..].trim();

            // Target must be a recognized unit or compound unit (like mph, km/hr)
            if !expr.is_empty() && is_valid_target_unit(target) {
                return Some(UnitConversion {
                    expression: expr.to_string(),
                    target_unit: target.to_string(),
                });
            }
        }
    }
    None
}

/// Checks if a target string is a valid target unit for conversion.
#[must_use]
pub fn is_valid_target_unit(target: &str) -> bool {
    let clean = target.trim();
    if clean.is_empty() {
        return false;
    }
    // Single unit (e.g. "mi", "km", "mph")
    if units::is_known_unit(clean) {
        return true;
    }
    // Compound unit separated by / or * (e.g. "km/hr", "m/s", "kg*m/s^2")
    let pieces: Vec<&str> = clean
        .split(|c: char| c == '/' || c == '*' || c == '^' || c.is_ascii_digit())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    !pieces.is_empty() && pieces.iter().all(|p| units::is_known_unit(p))
}

/// Transforms natural unit juxtaposition (e.g. `32 mi`, `52 mi^2`, `9.8 m/s^2`)
/// into explicit multiplication expressions (`32*mi`, `52*mi^2`).
#[must_use]
pub fn desugar_unit_juxtaposition(input: &str) -> String {
    let mut lex = Token::lexer(input);
    let mut tokens: Vec<(Token, std::ops::Range<usize>)> = Vec::new();

    while let Some(res) = lex.next() {
        if let Ok(tok) = res {
            tokens.push((tok, lex.span()));
        }
    }

    if tokens.len() < 2 {
        return input.to_string();
    }

    let mut result = String::with_capacity(input.len() + 16);
    let mut last_idx = 0;

    for i in 0..tokens.len() - 1 {
        let (tok, span) = &tokens[i];
        let (next_tok, next_span) = &tokens[i + 1];

        result.push_str(&input[last_idx..span.end]);
        last_idx = span.end;

        // Check for juxtaposition pattern:
        // Number followed by a known unit Ident (e.g. "32" followed by "mi")
        if let Token::Number(_) = tok {
            if let Token::Ident(name) = next_tok {
                if units::is_known_unit(name) {
                    result.push('*');
                    last_idx = next_span.start;
                }
            }
        }
    }

    result.push_str(&input[last_idx..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desugar_juxtaposition() {
        assert_eq!(desugar_unit_juxtaposition("32 mi"), "32*mi");
        assert_eq!(desugar_unit_juxtaposition("52 mi^2"), "52*mi^2");
        assert_eq!(desugar_unit_juxtaposition("32*mi"), "32*mi");
        assert_eq!(desugar_unit_juxtaposition("32 mi + 5 km"), "32*mi + 5*km");
        assert_eq!(
            desugar_unit_juxtaposition("solve 32 mi * x^2 = 52 mi^2"),
            "solve 32*mi * x^2 = 52*mi^2"
        );
    }

    #[test]
    fn test_parse_conversion() {
        let conv = parse_unit_conversion("32 mi + 5 km to miles").unwrap();
        assert_eq!(conv.expression, "32 mi + 5 km");
        assert_eq!(conv.target_unit, "miles");

        let conv2 = parse_unit_conversion("100 km / 2 hr in mph").unwrap();
        assert_eq!(conv2.expression, "100 km / 2 hr");
        assert_eq!(conv2.target_unit, "mph");

        assert!(parse_unit_conversion("solve x^2 = 4").is_none());
    }
}
