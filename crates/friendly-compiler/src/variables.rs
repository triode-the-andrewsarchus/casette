//! Free-variable inference for the `required_variables` field.
//!
//! Scans an expression for identifiers that are NOT immediately followed by '('
//! (so they are not function calls), are NOT known Sage constants (pi, e, I, ...),
//! and are plausible symbolic variables.

use std::collections::HashSet;

const RESERVED: &[&str] = &[
    "pi",
    "e",
    "I",
    "i",
    "oo",
    "infinity",
    "Infinity",
    "NaN",
    "euler_gamma",
    "True",
    "False",
    "None",
    "sqrt",
    "sin",
    "cos",
    "tan",
    "exp",
    "log",
    "ln",
    "abs",
];

fn is_reserved(name: &str) -> bool {
    RESERVED.contains(&name)
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

/// Returns true if `name` is a plausible symbolic variable.
pub fn is_plausible_variable(name: &str) -> bool {
    match name.chars().next() {
        Some(first) if is_ident_start(first) => !is_reserved(name),
        _ => false,
    }
}

/// Identifiers that are explicitly bound by the command (e.g. integration variable)
/// are passed in `bound` so they are reported even though we'd otherwise also find
/// them in the body — and so a plain `x` bound variable is always required even if
/// it only appears in the range.
pub fn free_variables(expression: &str, bound: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();

    let mut add = |name: &str| {
        if !seen.contains(name) {
            seen.insert(name.to_string());
            found.push(name.to_string());
        }
    };

    for &b in bound {
        if is_plausible_variable(b) {
            add(b);
        }
    }

    let chars: Vec<char> = expression.chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        let ch = chars[idx];
        if is_ident_start(ch) {
            let mut j = idx + 1;
            while j < chars.len() && is_ident_continue(chars[j]) {
                j += 1;
            }
            let name: String = chars[idx..j].iter().collect();

            // Check if call: next non-space char is '('
            let mut k = j;
            while k < chars.len() && (chars[k] == ' ' || chars[k] == '\t') {
                k += 1;
            }
            let is_call = k < chars.len() && chars[k] == '(';

            if !is_call && !is_reserved(&name) && is_plausible_variable(&name) {
                add(&name);
            }
            idx = j;
        } else {
            idx += 1;
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_variables() {
        assert_eq!(free_variables("sin(x) + y^2 - pi", &[]), vec!["x", "y"]);
        assert_eq!(
            free_variables("f(a, b) + c", &["t"]),
            vec!["t", "a", "b", "c"]
        );
    }
}
