//! Unary formula IR.

use crate::compiler::{self, Keyword};
use crate::scanner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnaryKind {
    Expand,
    Factor,
    Simplify,
    Latex,
    Factorial,
    IsPrime,
    FactorInteger,
    Mean,
}

impl UnaryKind {
    pub fn command(&self) -> &'static str {
        match self {
            Self::Expand => "expand",
            Self::Factor => "factor",
            Self::Simplify => "simplify",
            Self::Latex => "latex",
            Self::Factorial => "factorial",
            Self::IsPrime => "is_prime",
            Self::FactorInteger => "factor_integer",
            Self::Mean => "mean",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Expand => "EXPAND",
            Self::Factor => "FACTOR",
            Self::Simplify => "SIMPLIFY",
            Self::Latex => "LATEX",
            Self::Factorial => "FACTORIAL",
            Self::IsPrime => "IS PRIME",
            Self::FactorInteger => "FACTOR INTEGER",
            Self::Mean => "MEAN",
        }
    }

    pub fn from_keyword(kw: Keyword) -> Option<Self> {
        match kw {
            Keyword::Expand => Some(Self::Expand),
            Keyword::Factor => Some(Self::Factor),
            Keyword::Simplify => Some(Self::Simplify),
            Keyword::Latex => Some(Self::Latex),
            Keyword::Factorial => Some(Self::Factorial),
            Keyword::IsPrime => Some(Self::IsPrime),
            Keyword::FactorInteger => Some(Self::FactorInteger),
            Keyword::Mean => Some(Self::Mean),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnaryFormulaIR {
    pub kind: UnaryKind,
    pub expression: String,
}

impl UnaryFormulaIR {
    pub fn new(kind: UnaryKind, expression: impl Into<String>) -> Self {
        Self {
            kind,
            expression: expression.into(),
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        let kind = UnaryKind::from_keyword(command.keyword)?;
        let expression = scanner::trimmed_shim(&input[command.matched_prefix.len()..]).to_string();

        Some(Self { kind, expression })
    }

    pub fn friendly_input(&self) -> String {
        let expr = scanner::trimmed_shim(&self.expression);
        if expr.is_empty() {
            self.kind.command().to_string()
        } else {
            format!("{} {expr}", self.kind.command())
        }
    }
}
