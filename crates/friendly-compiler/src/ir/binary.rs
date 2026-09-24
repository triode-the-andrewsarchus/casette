//! Two-argument formula IR (choose, gcd, lcm).

use crate::compiler::{self, Keyword};
use crate::scanner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BinaryKind {
    Choose,
    Gcd,
    Lcm,
}

impl BinaryKind {
    pub fn command(&self) -> &'static str {
        match self {
            Self::Choose => "choose",
            Self::Gcd => "gcd",
            Self::Lcm => "lcm",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Choose => "CHOOSE",
            Self::Gcd => "GCD",
            Self::Lcm => "LCM",
        }
    }

    pub fn from_keyword(kw: Keyword) -> Option<Self> {
        match kw {
            Keyword::Choose => Some(Self::Choose),
            Keyword::Gcd => Some(Self::Gcd),
            Keyword::Lcm => Some(Self::Lcm),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BinaryFormulaIR {
    pub kind: BinaryKind,
    pub first: String,
    pub second: String,
}

impl BinaryFormulaIR {
    pub fn new(kind: BinaryKind, first: impl Into<String>, second: impl Into<String>) -> Self {
        Self {
            kind,
            first: first.into(),
            second: second.into(),
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        let kind = BinaryKind::from_keyword(command.keyword)?;

        let body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        let parts = scanner::split_top_level_commas(body);
        let first = parts
            .first()
            .map(|s| scanner::trimmed_shim(s))
            .unwrap_or("")
            .to_string();
        let second = parts.into_iter().skip(1).collect::<Vec<_>>().join(", ");

        Some(Self {
            kind,
            first,
            second: scanner::trimmed_shim(&second).to_string(),
        })
    }

    pub fn friendly_input(&self) -> String {
        let f = scanner::trimmed_shim(&self.first);
        let mut input = if f.is_empty() {
            self.kind.command().to_string()
        } else {
            format!("{} {f}", self.kind.command())
        };

        let s = scanner::trimmed_shim(&self.second);
        if !s.is_empty() {
            input.push_str(&format!(", {s}"));
        }

        input
    }
}
