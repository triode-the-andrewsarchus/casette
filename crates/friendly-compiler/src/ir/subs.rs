//! Substitution formula IR.

use crate::compiler::{self, Keyword};
use crate::scanner;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SubsFormulaIR {
    pub expression: String,
    pub bindings: String,
}

impl SubsFormulaIR {
    pub fn new(expression: impl Into<String>, bindings: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            bindings: bindings.into(),
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        if command.keyword != Keyword::Subs {
            return None;
        }

        let body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        let parts = scanner::split_top_level_commas(body);
        let expression = parts
            .first()
            .map(|s| scanner::trimmed_shim(s))
            .unwrap_or("")
            .to_string();
        let bindings = parts.into_iter().skip(1).collect::<Vec<_>>().join(", ");

        Some(Self {
            expression,
            bindings: scanner::trimmed_shim(&bindings).to_string(),
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_expression = scanner::trimmed_shim(&self.expression);
        let mut input = if trimmed_expression.is_empty() {
            "subs".to_string()
        } else {
            format!("subs {trimmed_expression}")
        };

        let b = scanner::trimmed_shim(&self.bindings);
        if !b.is_empty() {
            input.push_str(&format!(", {b}"));
        }

        input
    }
}
