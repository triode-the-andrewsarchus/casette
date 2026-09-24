//! Plot formula IRs (2D Cartesian plot, implicit plot, parametric plot).

use crate::compiler::{self, Keyword};
use crate::scanner;
use crate::variables;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlotFormulaIR {
    pub expression: String,
    pub variable: String,
    pub lower_bound: Option<String>,
    pub upper_bound: Option<String>,
    pub has_explicit_variable: bool,
}

impl PlotFormulaIR {
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
        if command.keyword != Keyword::Plot {
            return None;
        }

        let body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        let parts = scanner::split_top_level_commas(body);
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
        let variable = range_variable.unwrap_or(inferred_variable);

        Some(Self {
            expression,
            variable,
            lower_bound: range.as_ref().and_then(|r| r.lower.clone()),
            upper_bound: range.as_ref().and_then(|r| r.upper.clone()),
            has_explicit_variable: range.is_some(),
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_expression = scanner::trimmed_shim(&self.expression);
        let mut input = if trimmed_expression.is_empty() {
            "plot".to_string()
        } else {
            format!("plot {trimmed_expression}")
        };

        let has_bounds = self.lower_bound.is_some() || self.upper_bound.is_some();
        let trimmed_var = scanner::trimmed_shim(&self.variable);
        if has_bounds || !trimmed_var.is_empty() {
            let v = if trimmed_var.is_empty() {
                "x"
            } else {
                trimmed_var
            };
            let lo = self
                .lower_bound
                .as_deref()
                .map(scanner::trimmed_shim)
                .unwrap_or("");
            let hi = self
                .upper_bound
                .as_deref()
                .map(scanner::trimmed_shim)
                .unwrap_or("");
            input.push_str(&format!(", {v}={lo}..{hi}"));
        }

        input
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImplicitPlotFormulaIR {
    pub equation: String,
    pub x_variable: String,
    pub x_lower: Option<String>,
    pub x_upper: Option<String>,
    pub y_variable: String,
    pub y_lower: Option<String>,
    pub y_upper: Option<String>,
}

impl ImplicitPlotFormulaIR {
    pub fn new(
        equation: impl Into<String>,
        x_variable: impl Into<String>,
        x_lower: Option<String>,
        x_upper: Option<String>,
        y_variable: impl Into<String>,
        y_lower: Option<String>,
        y_upper: Option<String>,
    ) -> Self {
        Self {
            equation: equation.into(),
            x_variable: x_variable.into(),
            x_lower,
            x_upper,
            y_variable: y_variable.into(),
            y_lower,
            y_upper,
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        if command.keyword != Keyword::ImplicitPlot {
            return None;
        }

        let body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        let parts = scanner::split_top_level_commas(body);
        let equation = parts
            .first()
            .map(|s| scanner::trimmed_shim(s))
            .unwrap_or("")
            .to_string();

        let ranges: Vec<_> = parts
            .iter()
            .skip(1)
            .map(|s| compiler::parse_partial_range(s))
            .collect();
        let x_range = ranges.first().and_then(|r| r.as_ref());
        let y_range = ranges.get(1).and_then(|r| r.as_ref());

        Some(Self {
            equation,
            x_variable: x_range.map(|r| r.variable.clone()).unwrap_or_default(),
            x_lower: x_range.and_then(|r| r.lower.clone()),
            x_upper: x_range.and_then(|r| r.upper.clone()),
            y_variable: y_range.map(|r| r.variable.clone()).unwrap_or_default(),
            y_lower: y_range.and_then(|r| r.lower.clone()),
            y_upper: y_range.and_then(|r| r.upper.clone()),
        })
    }

    fn x_has_content(&self) -> bool {
        !scanner::trimmed_shim(&self.x_variable).is_empty()
            || self.x_lower.is_some()
            || self.x_upper.is_some()
    }

    fn y_has_content(&self) -> bool {
        !scanner::trimmed_shim(&self.y_variable).is_empty()
            || self.y_lower.is_some()
            || self.y_upper.is_some()
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_equation = scanner::trimmed_shim(&self.equation);
        let mut input = if trimmed_equation.is_empty() {
            "implicit_plot".to_string()
        } else {
            format!("implicit_plot {trimmed_equation}")
        };

        if self.x_has_content() || self.y_has_content() {
            let x_var = scanner::trimmed_shim(&self.x_variable);
            let x_v = if x_var.is_empty() { "x" } else { x_var };
            let y_var = scanner::trimmed_shim(&self.y_variable);
            let y_v = if y_var.is_empty() { "y" } else { y_var };

            let x_lo = self
                .x_lower
                .as_deref()
                .map(scanner::trimmed_shim)
                .unwrap_or("");
            let x_hi = self
                .x_upper
                .as_deref()
                .map(scanner::trimmed_shim)
                .unwrap_or("");
            let y_lo = self
                .y_lower
                .as_deref()
                .map(scanner::trimmed_shim)
                .unwrap_or("");
            let y_hi = self
                .y_upper
                .as_deref()
                .map(scanner::trimmed_shim)
                .unwrap_or("");

            input.push_str(&format!(", {x_v}={x_lo}..{x_hi}"));
            input.push_str(&format!(", {y_v}={y_lo}..{y_hi}"));
        }

        input
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ParametricPlotFormulaIR {
    pub x_expression: String,
    pub y_expression: String,
    pub variable: String,
    pub lower_bound: Option<String>,
    pub upper_bound: Option<String>,
    pub has_explicit_variable: bool,
}

impl ParametricPlotFormulaIR {
    pub fn new(
        x_expression: impl Into<String>,
        y_expression: impl Into<String>,
        variable: impl Into<String>,
        lower_bound: Option<String>,
        upper_bound: Option<String>,
        has_explicit_variable: bool,
    ) -> Self {
        Self {
            x_expression: x_expression.into(),
            y_expression: y_expression.into(),
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
        if command.keyword != Keyword::ParametricPlot {
            return None;
        }

        let body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        let parts = scanner::split_top_level_commas(body);
        let pair_text = parts
            .first()
            .map(|s| scanner::trimmed_shim(s))
            .unwrap_or("");

        let mut x_expression = String::new();
        let mut y_expression = String::new();

        if pair_text.starts_with('(') {
            let mut inner = scanner::trimmed_shim(&pair_text[1..]);
            if inner.ends_with(')') {
                inner = scanner::trimmed_shim(&inner[..inner.len() - 1]);
            }
            let coords = scanner::split_top_level_commas(inner);
            if let Some(first) = coords.first() {
                x_expression = scanner::trimmed_shim(first).to_string();
            }
            if coords.len() >= 2 {
                y_expression = scanner::trimmed_shim(&coords[1]).to_string();
            }
        } else {
            x_expression = pair_text.to_string();
        }

        let range = parts
            .iter()
            .skip(1)
            .find_map(|s| compiler::parse_partial_range(s));

        Some(Self {
            x_expression,
            y_expression,
            variable: range
                .as_ref()
                .map(|r| r.variable.clone())
                .unwrap_or_default(),
            lower_bound: range.as_ref().and_then(|r| r.lower.clone()),
            upper_bound: range.as_ref().and_then(|r| r.upper.clone()),
            has_explicit_variable: range.is_some(),
        })
    }

    pub fn friendly_input(&self) -> String {
        let x = scanner::trimmed_shim(&self.x_expression);
        let y = scanner::trimmed_shim(&self.y_expression);
        let mut input = format!("parametric_plot ({x}, {y})");

        let lo = self
            .lower_bound
            .as_deref()
            .map(scanner::trimmed_shim)
            .unwrap_or("");
        let hi = self
            .upper_bound
            .as_deref()
            .map(scanner::trimmed_shim)
            .unwrap_or("");
        let v = scanner::trimmed_shim(&self.variable);

        if self.lower_bound.is_some() || self.upper_bound.is_some() || !v.is_empty() {
            let index = if v.is_empty() { "t" } else { v };
            input.push_str(&format!(", {index}={lo}..{hi}"));
        }

        input
    }
}
