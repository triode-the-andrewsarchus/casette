//! Derivative formula IR.

use crate::compiler::{self, Keyword};
use crate::scanner;
use crate::variables;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DerivativeFormulaIR {
    pub expression: String,
    pub variable: String,
    pub order: Option<String>,
    pub has_explicit_variable: bool,
}

impl DerivativeFormulaIR {
    pub fn new(
        expression: impl Into<String>,
        variable: impl Into<String>,
        order: Option<String>,
        has_explicit_variable: bool,
    ) -> Self {
        Self {
            expression: expression.into(),
            variable: variable.into(),
            order,
            has_explicit_variable,
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        if command.keyword != Keyword::Derivative {
            return None;
        }

        let mut body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]).to_string();
        let mut explicit_variable: Option<String> = None;
        if let Some(wrt) = compiler::trailing_clause(&body, "wrt") {
            explicit_variable =
                Some(scanner::trimmed_shim(&body[wrt.value_range.clone()]).to_string());
            let before = scanner::trimmed_shim(&body[..wrt.clause_start]);
            let stripped = if let Some(s) = before.strip_suffix(',') {
                scanner::trimmed_shim(s)
            } else {
                before
            };
            body = stripped.to_string();
        }

        let mut order: Option<String> = None;
        let parts = scanner::split_top_level_commas(&body);
        if parts.len() >= 2 {
            if let Some(last) = parts.last() {
                let last_trimmed = scanner::trimmed_shim(last);
                if !last_trimmed.is_empty() && last_trimmed.chars().all(|c| c.is_ascii_digit()) {
                    order = Some(last_trimmed.to_string());
                    body = parts[..parts.len() - 1].join(",");
                }
            }
        }

        let expression = scanner::trimmed_shim(&body).to_string();
        let free_variables = variables::free_variables(&expression, &[]);
        let inferred_variable = if free_variables.len() == 1 {
            free_variables[0].clone()
        } else {
            String::new()
        };
        let variable = explicit_variable.clone().unwrap_or(inferred_variable);

        Some(Self {
            expression,
            variable,
            order,
            has_explicit_variable: explicit_variable.is_some(),
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_expression = scanner::trimmed_shim(&self.expression);
        let mut input = if trimmed_expression.is_empty() {
            "derivative".to_string()
        } else {
            format!("derivative {trimmed_expression}")
        };

        if let Some(ref o) = self.order {
            input.push_str(&format!(", {o}"));
        }

        let trimmed_var = scanner::trimmed_shim(&self.variable);
        if self.has_explicit_variable && !trimmed_var.is_empty() {
            input.push_str(&format!(" wrt {trimmed_var}"));
        }

        input
    }
}
