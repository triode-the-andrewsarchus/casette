//! Numeric approximation formula IR.

use crate::compiler::{self, Keyword};
use crate::scanner;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NumericFormulaIR {
    pub expression: String,
    pub digits: String,
}

impl NumericFormulaIR {
    pub fn new(expression: impl Into<String>, digits: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            digits: digits.into(),
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        if command.keyword != Keyword::Numeric {
            return None;
        }

        let body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        let parts = scanner::split_top_level_commas(body);
        let expression = parts
            .first()
            .map(|s| scanner::trimmed_shim(s))
            .unwrap_or("")
            .to_string();
        let digits = parts
            .get(1)
            .map(|s| scanner::trimmed_shim(s))
            .unwrap_or("")
            .to_string();

        Some(Self { expression, digits })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_expression = scanner::trimmed_shim(&self.expression);
        let mut input = if trimmed_expression.is_empty() {
            "numeric".to_string()
        } else {
            format!("numeric {trimmed_expression}")
        };

        let d = scanner::trimmed_shim(&self.digits);
        if !d.is_empty() {
            input.push_str(&format!(", {d}"));
        }

        input
    }
}
