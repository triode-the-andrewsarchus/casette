//! Limit formula IR.

use crate::compiler::{self, Keyword};
use crate::scanner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LimitDirection {
    Both,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LimitFormulaIR {
    pub expression: String,
    pub variable: String,
    pub point: String,
    pub direction: LimitDirection,
}

impl LimitFormulaIR {
    pub fn new(
        expression: impl Into<String>,
        variable: impl Into<String>,
        point: impl Into<String>,
        direction: LimitDirection,
    ) -> Self {
        Self {
            expression: expression.into(),
            variable: variable.into(),
            point: point.into(),
            direction,
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        if command.keyword != Keyword::Limit {
            return None;
        }

        let body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        let parts = scanner::split_top_level_commas(body);
        let expression = parts
            .first()
            .map(|s| scanner::trimmed_shim(s))
            .unwrap_or("")
            .to_string();

        let mut approach_parts: Vec<String> = parts.into_iter().skip(1).collect();
        let mut direction = LimitDirection::Both;

        if approach_parts.len() >= 2 {
            let tail = scanner::trimmed_shim(approach_parts.last().unwrap()).to_lowercase();
            match tail.as_str() {
                "left" => {
                    direction = LimitDirection::Left;
                    approach_parts.pop();
                }
                "right" => {
                    direction = LimitDirection::Right;
                    approach_parts.pop();
                }
                _ => {}
            }
        }

        let approach = approach_parts.join(", ");
        let approach_trimmed = scanner::trimmed_shim(&approach);

        let (variable, point) = if let Some(arrow_idx) = approach_trimmed.find("->") {
            let v = scanner::trimmed_shim(&approach_trimmed[..arrow_idx]).to_string();
            let p = scanner::trimmed_shim(&approach_trimmed[arrow_idx + 2..]).to_string();
            (v, p)
        } else {
            (approach_trimmed.to_string(), String::new())
        };

        Some(Self {
            expression,
            variable,
            point,
            direction,
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_expression = scanner::trimmed_shim(&self.expression);
        let mut input = if trimmed_expression.is_empty() {
            "limit".to_string()
        } else {
            format!("limit {trimmed_expression}")
        };

        let v = scanner::trimmed_shim(&self.variable);
        let p = scanner::trimmed_shim(&self.point);
        if !v.is_empty() || !p.is_empty() || self.direction != LimitDirection::Both {
            input.push_str(&format!(", {v}->{p}"));
        }
        match self.direction {
            LimitDirection::Both => {}
            LimitDirection::Left => input.push_str(", left"),
            LimitDirection::Right => input.push_str(", right"),
        }

        input
    }
}
