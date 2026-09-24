//! Assumption formula IR.

use crate::compiler::{self, Keyword};
use crate::scanner;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssumeFormulaIR {
    pub condition: String,
}

impl AssumeFormulaIR {
    pub fn new(condition: impl Into<String>) -> Self {
        Self {
            condition: condition.into(),
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        if command.keyword != Keyword::Assume {
            return None;
        }

        let condition = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        Some(Self {
            condition: condition.to_string(),
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed = scanner::trimmed_shim(&self.condition);
        if trimmed.is_empty() {
            "assume".to_string()
        } else {
            format!("assume {trimmed}")
        }
    }
}
