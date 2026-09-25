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

/// Returns true if an identifier is a command keyword, clause keyword, or common function name,
/// which should not be treated as an algebraic factor juxtaposed with a unit.
fn is_command_or_keyword(name: &str) -> bool {
    matches!(
        name,
        "solve"
            | "factor"
            | "expand"
            | "simplify"
            | "derivative"
            | "diff"
            | "integral"
            | "int"
            | "double_integral"
            | "dint"
            | "limit"
            | "taylor"
            | "plot"
            | "implicit_plot"
            | "parametric_plot"
            | "sum"
            | "product"
            | "matrix"
            | "vector"
            | "eigenvalues"
            | "rref"
            | "det"
            | "inverse"
            | "inv"
            | "transpose"
            | "rank"
            | "eigenvectors"
            | "gradient"
            | "hessian"
            | "jacobian"
            | "subs"
            | "numeric"
            | "eval"
            | "latex"
            | "var"
            | "assume"
            | "forget"
            | "choose"
            | "gcd"
            | "lcm"
            | "factorial"
            | "is_prime"
            | "factor_integer"
            | "mean"
            | "normal_pdf"
            | "normal_cdf"
            | "normal_between"
            | "normal_inv"
            | "binomial_pmf"
            | "binomial_cdf"
            | "binomial_between"
            | "binomial_at_most"
            | "binomial_at_least"
            | "poisson_pmf"
            | "poisson_cdf"
            | "poisson_between"
            | "poisson_at_most"
            | "poisson_at_least"
            | "exponential_pdf"
            | "exponential_cdf"
            | "exponential_between"
            | "exponential_inv"
            | "uniform_pdf"
            | "uniform_cdf"
            | "uniform_between"
            | "uniform_inv"
            | "for"
            | "to"
            | "in"
            | "sin"
            | "cos"
            | "tan"
            | "exp"
            | "log"
            | "ln"
            | "sqrt"
            | "abs"
    )
}

fn is_multiplier_token(tok: &Token) -> bool {
    match tok {
        Token::Number(_) => true,
        Token::Ident(name) => !is_command_or_keyword(name),
        _ => false,
    }
}

/// Transforms natural unit juxtaposition (e.g. `32 mi`, `x sec`, `52 mi^2`, `9.8 m/s^2`)
/// into explicit multiplication expressions (`(32*mi)`, `(x*sec)`, `(52*mi^2)`).
///
/// Also wraps explicit quantities after division (`/ 2*hr`, `/ x*sec`) into `/(2*hr)`
/// so operator precedence does not invert division into multiplication.
#[must_use]
pub fn desugar_unit_juxtaposition(input: &str) -> String {
    let mut lex = Token::lexer(input);
    let mut tokens: Vec<(Token, std::ops::Range<usize>)> = Vec::new();

    while let Some(res) = lex.next() {
        if let Ok(tok) = res {
            tokens.push((tok, lex.span()));
        }
    }

    if tokens.is_empty() {
        return input.to_string();
    }

    let mut result = String::with_capacity(input.len() + 32);
    let mut last_idx = 0;
    let mut i = 0;

    while i < tokens.len() {
        let is_after_slash = i > 0 && matches!(tokens[i - 1].0, Token::Slash);

        if is_multiplier_token(&tokens[i].0) {
            // Case 1: Multiplier (Number or Variable) [whitespace] Ident (where Ident is a known unit, e.g. "32 mi", "x sec")
            if i + 1 < tokens.len() {
                if let Token::Ident(name) = tokens[i + 1].0 {
                    if units::is_known_unit(name) {
                        let end_tok_idx = if i + 3 < tokens.len()
                            && matches!(tokens[i + 2].0, Token::Caret)
                            && matches!(tokens[i + 3].0, Token::Number(_))
                        {
                            i + 3
                        } else {
                            i + 1
                        };
                        let end_pos = tokens[end_tok_idx].1.end;

                        let start_pos = tokens[i].1.start;
                        let left_end = tokens[i].1.end;
                        let unit_start = tokens[i + 1].1.start;
                        let unit_piece = if name == "N" {
                            "newton"
                        } else {
                            &input[unit_start..end_pos]
                        };

                        result.push_str(&input[last_idx..start_pos]);
                        result.push('(');
                        result.push_str(&input[start_pos..left_end]);
                        result.push('*');
                        result.push_str(unit_piece);
                        result.push(')');

                        last_idx = end_pos;
                        i = end_tok_idx + 1;
                        continue;
                    }
                }
            }

            // Case 2: After slash, Multiplier * Ident (e.g. / 2*hr or / x*sec)
            if is_after_slash && i + 2 < tokens.len() && matches!(tokens[i + 1].0, Token::Star) {
                if let Token::Ident(name) = tokens[i + 2].0 {
                    if units::is_known_unit(name) {
                        let end_tok_idx = if i + 4 < tokens.len()
                            && matches!(tokens[i + 3].0, Token::Caret)
                            && matches!(tokens[i + 4].0, Token::Number(_))
                        {
                            i + 4
                        } else {
                            i + 2
                        };

                        let start_pos = tokens[i].1.start;
                        let end_pos = tokens[end_tok_idx].1.end;

                        result.push_str(&input[last_idx..start_pos]);
                        result.push('(');
                        result.push_str(&input[start_pos..end_pos]);
                        result.push(')');

                        last_idx = end_pos;
                        i = end_tok_idx + 1;
                        continue;
                    }
                }
            }
        }

        // Case 3: Parenthesized expression followed by unit: (expr) unit -> (expr)*(unit)
        if matches!(tokens[i].0, Token::RParen) && i + 1 < tokens.len() {
            if let Token::Ident(name) = tokens[i + 1].0 {
                if units::is_known_unit(name) {
                    let end_tok_idx = if i + 3 < tokens.len()
                        && matches!(tokens[i + 2].0, Token::Caret)
                        && matches!(tokens[i + 3].0, Token::Number(_))
                    {
                        i + 3
                    } else {
                        i + 1
                    };

                    let rparen_end = tokens[i].1.end;
                    let unit_start = tokens[i + 1].1.start;
                    let end_pos = tokens[end_tok_idx].1.end;
                    let unit_piece = if name == "N" {
                        "newton"
                    } else {
                        &input[unit_start..end_pos]
                    };

                    result.push_str(&input[last_idx..rparen_end]);
                    result.push('*');
                    result.push('(');
                    result.push_str(unit_piece);
                    result.push(')');

                    last_idx = end_pos;
                    i = end_tok_idx + 1;
                    continue;
                }
            }
        }

        i += 1;
    }

    result.push_str(&input[last_idx..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desugar_juxtaposition() {
        assert_eq!(desugar_unit_juxtaposition("32 mi"), "(32*mi)");
        assert_eq!(desugar_unit_juxtaposition("52 mi^2"), "(52*mi^2)");
        assert_eq!(desugar_unit_juxtaposition("32*mi"), "32*mi");
        assert_eq!(desugar_unit_juxtaposition("100 km / 2 hr"), "(100*km) / (2*hr)");
        assert_eq!(desugar_unit_juxtaposition("100*km / 2*hr"), "100*km / (2*hr)");
        assert_eq!(desugar_unit_juxtaposition("32 mi + 5 km"), "(32*mi) + (5*km)");
        assert_eq!(
            desugar_unit_juxtaposition("solve 32 mi * x^2 = 52 mi^2"),
            "solve (32*mi) * x^2 = (52*mi^2)"
        );
        assert_eq!(
            desugar_unit_juxtaposition("solve 3 hr + x sec = 12 days"),
            "solve (3*hr) + (x*sec) = (12*days)"
        );
        assert_eq!(desugar_unit_juxtaposition("(x + 1) hr"), "(x + 1)*(hr)");
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
