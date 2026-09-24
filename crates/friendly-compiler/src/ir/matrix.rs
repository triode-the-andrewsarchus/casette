//! Matrix formula IR.

use crate::compiler::{self, Keyword};
use crate::scanner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MatrixKind {
    Matrix,
    Vector,
    Det,
    Inverse,
    Transpose,
    Rank,
    Rref,
    Eigenvalues,
    Eigenvectors,
}

impl MatrixKind {
    pub fn command(&self) -> &'static str {
        match self {
            Self::Matrix => "matrix",
            Self::Vector => "vector",
            Self::Det => "det",
            Self::Inverse => "inverse",
            Self::Transpose => "transpose",
            Self::Rank => "rank",
            Self::Rref => "rref",
            Self::Eigenvalues => "eigenvalues",
            Self::Eigenvectors => "eigenvectors",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Matrix => "MATRIX",
            Self::Vector => "VECTOR",
            Self::Det => "DET",
            Self::Inverse => "INVERSE",
            Self::Transpose => "TRANSPOSE",
            Self::Rank => "RANK",
            Self::Rref => "RREF",
            Self::Eigenvalues => "EIGENVALUES",
            Self::Eigenvectors => "EIGENVECTORS",
        }
    }

    pub fn from_keyword(kw: Keyword) -> Option<Self> {
        match kw {
            Keyword::Matrix => Some(Self::Matrix),
            Keyword::Vector => Some(Self::Vector),
            Keyword::Det => Some(Self::Det),
            Keyword::Inverse => Some(Self::Inverse),
            Keyword::Transpose => Some(Self::Transpose),
            Keyword::Rank => Some(Self::Rank),
            Keyword::Rref => Some(Self::Rref),
            Keyword::Eigenvalues => Some(Self::Eigenvalues),
            Keyword::Eigenvectors => Some(Self::Eigenvectors),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MatrixFormulaIR {
    pub kind: MatrixKind,
    pub expression: String,
}

impl MatrixFormulaIR {
    pub fn new(kind: MatrixKind, expression: impl Into<String>) -> Self {
        Self {
            kind,
            expression: expression.into(),
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        let kind = MatrixKind::from_keyword(command.keyword)?;
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
