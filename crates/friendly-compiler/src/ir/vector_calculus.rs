//! Vector calculus formula IR (gradient, hessian, jacobian).

use crate::compiler::{self, Keyword};
use crate::scanner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VectorCalculusKind {
    Gradient,
    Hessian,
    Jacobian,
}

impl VectorCalculusKind {
    pub fn command(&self) -> &'static str {
        match self {
            Self::Gradient => "gradient",
            Self::Hessian => "hessian",
            Self::Jacobian => "jacobian",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Gradient => "GRADIENT",
            Self::Hessian => "HESSIAN",
            Self::Jacobian => "JACOBIAN",
        }
    }

    pub fn from_keyword(kw: Keyword) -> Option<Self> {
        match kw {
            Keyword::Gradient => Some(Self::Gradient),
            Keyword::Hessian => Some(Self::Hessian),
            Keyword::Jacobian => Some(Self::Jacobian),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VectorCalculusFormulaIR {
    pub kind: VectorCalculusKind,
    pub expression: String,
    pub variables: String,
}

impl VectorCalculusFormulaIR {
    pub fn new(
        kind: VectorCalculusKind,
        expression: impl Into<String>,
        variables: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            expression: expression.into(),
            variables: variables.into(),
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        let kind = VectorCalculusKind::from_keyword(command.keyword)?;

        let body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        let parts = scanner::split_top_level_commas(body);
        let expression = parts
            .first()
            .map(|s| scanner::trimmed_shim(s))
            .unwrap_or("")
            .to_string();

        let variables = if kind == VectorCalculusKind::Hessian {
            String::new()
        } else {
            parts.into_iter().skip(1).collect::<Vec<_>>().join(", ")
        };

        Some(Self {
            kind,
            expression,
            variables: scanner::trimmed_shim(&variables).to_string(),
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_expression = scanner::trimmed_shim(&self.expression);
        let mut input = if trimmed_expression.is_empty() {
            self.kind.command().to_string()
        } else {
            format!("{} {trimmed_expression}", self.kind.command())
        };

        if self.kind != VectorCalculusKind::Hessian {
            let vars = scanner::trimmed_shim(&self.variables);
            if !vars.is_empty() {
                input.push_str(&format!(", {vars}"));
            }
        }

        input
    }
}
