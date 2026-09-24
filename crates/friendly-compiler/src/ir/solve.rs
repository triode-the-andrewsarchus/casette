//! Solve formula IR.

use crate::compiler::{self, Keyword};
use crate::scanner;
use crate::variables;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SolveFormulaIR {
    pub equation: String,
    pub variable: String,
    pub has_explicit_variable: bool,
}

impl SolveFormulaIR {
    pub fn new(
        equation: impl Into<String>,
        variable: impl Into<String>,
        has_explicit_variable: bool,
    ) -> Self {
        Self {
            equation: equation.into(),
            variable: variable.into(),
            has_explicit_variable,
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        if command.keyword != Keyword::Solve {
            return None;
        }

        let mut body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]).to_string();
        let mut explicit_variable: Option<String> = None;
        if let Some(for_clause) = compiler::trailing_clause(&body, "for") {
            explicit_variable =
                Some(scanner::trimmed_shim(&body[for_clause.value_range.clone()]).to_string());
            body = scanner::trimmed_shim(&body[..for_clause.clause_start]).to_string();
        }

        let equation = scanner::trimmed_shim(&body).to_string();
        let free_variables = variables::free_variables(&equation, &[]);
        let inferred_variable = if free_variables.len() == 1 {
            free_variables[0].clone()
        } else {
            String::new()
        };
        let variable = explicit_variable.clone().unwrap_or(inferred_variable);

        Some(Self {
            equation,
            variable,
            has_explicit_variable: explicit_variable.is_some(),
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_equation = scanner::trimmed_shim(&self.equation);
        let mut input = if trimmed_equation.is_empty() {
            "solve".to_string()
        } else {
            format!("solve {trimmed_equation}")
        };

        let trimmed_var = scanner::trimmed_shim(&self.variable);
        if self.has_explicit_variable && !trimmed_var.is_empty() {
            input.push_str(&format!(" for {trimmed_var}"));
        }

        input
    }
}
