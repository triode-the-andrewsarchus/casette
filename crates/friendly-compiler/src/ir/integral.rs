//! Integral formula IR.

use crate::compiler::{self, Keyword};
use crate::scanner;
use crate::variables;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IntegralFormulaIR {
    pub expression: String,
    pub variable: String,
    pub lower_bound: Option<String>,
    pub upper_bound: Option<String>,
    pub has_explicit_variable: bool,
}

impl IntegralFormulaIR {
    pub fn new(
        expression: impl Into<String>,
        variable: impl Into<String>,
        lower_bound: Option<String>,
        upper_bound: Option<String>,
        has_explicit_variable: bool,
    ) -> Self {
        Self {
            expression: expression.into(),
            variable: variable.into(),
            lower_bound,
            upper_bound,
            has_explicit_variable,
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        if command.keyword != Keyword::Integral {
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

        let parts = scanner::split_top_level_commas(&body);
        let expression = parts
            .first()
            .map(|s| scanner::trimmed_shim(s))
            .unwrap_or("")
            .to_string();
        let range = parts
            .iter()
            .skip(1)
            .find_map(|s| compiler::parse_partial_range(s));
        let free_variables = variables::free_variables(&expression, &[]);
        let inferred_variable = if free_variables.len() == 1 {
            free_variables[0].clone()
        } else {
            String::new()
        };

        let range_variable = range.as_ref().and_then(|r| {
            if r.variable.is_empty() {
                None
            } else {
                Some(r.variable.clone())
            }
        });
        let variable = range_variable
            .or(explicit_variable.clone())
            .unwrap_or(inferred_variable);

        Some(Self {
            expression,
            variable,
            lower_bound: range.as_ref().and_then(|r| r.lower.clone()),
            upper_bound: range.as_ref().and_then(|r| r.upper.clone()),
            has_explicit_variable: explicit_variable.is_some() || range.is_some(),
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_expression = scanner::trimmed_shim(&self.expression);
        let mut input = if trimmed_expression.is_empty() {
            "integral".to_string()
        } else {
            format!("integral {trimmed_expression}")
        };

        let trimmed_var = scanner::trimmed_shim(&self.variable);
        let has_bounds = self.lower_bound.is_some() || self.upper_bound.is_some();

        if has_bounds {
            let lower = self
                .lower_bound
                .as_deref()
                .map(scanner::trimmed_shim)
                .unwrap_or("");
            let upper = self
                .upper_bound
                .as_deref()
                .map(scanner::trimmed_shim)
                .unwrap_or("");
            input.push_str(&format!(", {trimmed_var}={lower}..{upper}"));
        } else if self.has_explicit_variable && !trimmed_var.is_empty() {
            input.push_str(&format!(", wrt {trimmed_var}"));
        }

        input
    }
}
