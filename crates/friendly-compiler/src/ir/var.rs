//! Variable declaration formula IR.

use crate::compiler::{self, Keyword};
use crate::scanner;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VarFormulaIR {
    pub names: String,
}

impl VarFormulaIR {
    pub fn new(names: impl Into<String>) -> Self {
        Self {
            names: names.into(),
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        if command.keyword != Keyword::VarDeclaration {
            return None;
        }

        let names = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        if names.starts_with('=') {
            return None;
        }

        Some(Self {
            names: names.to_string(),
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed = scanner::trimmed_shim(&self.names);
        if trimmed.is_empty() {
            "var".to_string()
        } else {
            format!("var {trimmed}")
        }
    }
}
