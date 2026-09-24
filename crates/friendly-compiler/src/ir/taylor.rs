//! Taylor formula IR.

use crate::compiler::{self, Keyword};
use crate::scanner;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaylorFormulaIR {
    pub expression: String,
    pub variable: String,
    pub center: String,
    pub order: String,
}

impl TaylorFormulaIR {
    pub fn new(
        expression: impl Into<String>,
        variable: impl Into<String>,
        center: impl Into<String>,
        order: impl Into<String>,
    ) -> Self {
        Self {
            expression: expression.into(),
            variable: variable.into(),
            center: center.into(),
            order: order.into(),
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        if command.keyword != Keyword::Taylor {
            return None;
        }

        let body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        let parts = scanner::split_top_level_commas(body);
        let expression = parts
            .first()
            .map(|s| scanner::trimmed_shim(s))
            .unwrap_or("")
            .to_string();

        let mut variable = String::new();
        let mut center = String::new();
        if parts.len() >= 2 {
            let clause = scanner::trimmed_shim(&parts[1]);
            if let Some(eq) = clause.find('=') {
                if !clause.contains("..") {
                    variable = scanner::trimmed_shim(&clause[..eq]).to_string();
                    center = scanner::trimmed_shim(&clause[eq + 1..]).to_string();
                } else {
                    variable = clause.to_string();
                }
            } else {
                variable = clause.to_string();
            }
        }

        let mut order = String::new();
        if parts.len() >= 3 {
            let clause = scanner::trimmed_shim(&parts[2]);
            if let Some(eq) = clause.find('=') {
                if scanner::trimmed_shim(&clause[..eq]).eq_ignore_ascii_case("order") {
                    order = scanner::trimmed_shim(&clause[eq + 1..]).to_string();
                } else {
                    order = clause.to_string();
                }
            } else {
                order = clause.to_string();
            }
        }

        Some(Self {
            expression,
            variable,
            center,
            order,
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_expression = scanner::trimmed_shim(&self.expression);
        let mut input = if trimmed_expression.is_empty() {
            "taylor".to_string()
        } else {
            format!("taylor {trimmed_expression}")
        };

        let v = scanner::trimmed_shim(&self.variable);
        let c = scanner::trimmed_shim(&self.center);
        let o = scanner::trimmed_shim(&self.order);

        if !v.is_empty() || !c.is_empty() || !o.is_empty() {
            input.push_str(&format!(", {v}={c}"));
        }
        if !o.is_empty() {
            input.push_str(&format!(", order={o}"));
        }

        input
    }
}
