//! Series range formula IR (sum / product).

use crate::compiler::{self, Keyword};
use crate::scanner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SeriesRangeKind {
    Sum,
    Product,
}

impl SeriesRangeKind {
    pub fn command(&self) -> &'static str {
        match self {
            Self::Sum => "sum",
            Self::Product => "product",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Sum => "SUM",
            Self::Product => "PRODUCT",
        }
    }

    pub fn from_keyword(kw: Keyword) -> Option<Self> {
        match kw {
            Keyword::Sum => Some(Self::Sum),
            Keyword::Product => Some(Self::Product),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SeriesRangeFormulaIR {
    pub kind: SeriesRangeKind,
    pub expression: String,
    pub index_variable: String,
    pub lower_bound: String,
    pub upper_bound: String,
}

impl SeriesRangeFormulaIR {
    pub fn new(
        kind: SeriesRangeKind,
        expression: impl Into<String>,
        index_variable: impl Into<String>,
        lower_bound: impl Into<String>,
        upper_bound: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            expression: expression.into(),
            index_variable: index_variable.into(),
            lower_bound: lower_bound.into(),
            upper_bound: upper_bound.into(),
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        let kind = SeriesRangeKind::from_keyword(command.keyword)?;

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

        Some(Self {
            kind,
            expression,
            index_variable: range
                .as_ref()
                .map(|r| r.variable.clone())
                .unwrap_or_default(),
            lower_bound: range
                .as_ref()
                .and_then(|r| r.lower.clone())
                .unwrap_or_default(),
            upper_bound: range
                .as_ref()
                .and_then(|r| r.upper.clone())
                .unwrap_or_default(),
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_expression = scanner::trimmed_shim(&self.expression);
        let mut input = if trimmed_expression.is_empty() {
            self.kind.command().to_string()
        } else {
            format!("{} {trimmed_expression}", self.kind.command())
        };

        let v = scanner::trimmed_shim(&self.index_variable);
        let lo = scanner::trimmed_shim(&self.lower_bound);
        let hi = scanner::trimmed_shim(&self.upper_bound);

        if !v.is_empty() || !lo.is_empty() || !hi.is_empty() {
            let index = if v.is_empty() { "k" } else { v };
            input.push_str(&format!(", {index}={lo}..{hi}"));
        }

        input
    }
}
