//! Statistics formula IR.

use crate::compiler::{self, StatsKind};
use crate::scanner;
use crate::variables;
use std::collections::{HashMap, HashSet};

pub struct NamedField {
    pub key: &'static str,
    pub title: &'static str,
    pub prompt: &'static str,
}

impl StatsKind {
    pub fn named_fields(&self) -> &'static [NamedField] {
        match self {
            Self::NormalPdf | Self::NormalCdf | Self::NormalBetween | Self::NormalInv => &[
                NamedField {
                    key: "mean",
                    title: "mean",
                    prompt: "0",
                },
                NamedField {
                    key: "sd",
                    title: "sd",
                    prompt: "1",
                },
            ],
            Self::BinomialPmf
            | Self::BinomialCdf
            | Self::BinomialBetween
            | Self::BinomialAtMost
            | Self::BinomialAtLeast => &[
                NamedField {
                    key: "n",
                    title: "n",
                    prompt: "10",
                },
                NamedField {
                    key: "p",
                    title: "p",
                    prompt: ".5",
                },
            ],
            Self::PoissonPmf
            | Self::PoissonCdf
            | Self::PoissonBetween
            | Self::PoissonAtMost
            | Self::PoissonAtLeast => &[NamedField {
                key: "lambda",
                title: "lambda",
                prompt: "3",
            }],
            Self::ExponentialPdf
            | Self::ExponentialCdf
            | Self::ExponentialBetween
            | Self::ExponentialInv => &[NamedField {
                key: "rate",
                title: "rate",
                prompt: "2",
            }],
            Self::UniformPdf | Self::UniformCdf | Self::UniformBetween | Self::UniformInv => &[
                NamedField {
                    key: "min",
                    title: "min",
                    prompt: "0",
                },
                NamedField {
                    key: "max",
                    title: "max",
                    prompt: "1",
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatsFormulaIR {
    pub kind: StatsKind,
    pub positional: Vec<String>,
    pub named: HashMap<String, String>,
}

impl StatsFormulaIR {
    pub fn new(kind: StatsKind, positional: Vec<String>, named: HashMap<String, String>) -> Self {
        Self {
            kind,
            positional,
            named,
        }
    }

    pub fn parse(raw_input: &str) -> Option<Self> {
        let input_clean = raw_input.replace('\n', " ");
        let input = scanner::trimmed_shim(&input_clean);

        let command = compiler::leading_command(input)?;
        let kind = match command.keyword {
            compiler::Keyword::NormalPdf => StatsKind::NormalPdf,
            compiler::Keyword::NormalCdf => StatsKind::NormalCdf,
            compiler::Keyword::NormalBetween => StatsKind::NormalBetween,
            compiler::Keyword::NormalInv => StatsKind::NormalInv,
            compiler::Keyword::BinomialPmf => StatsKind::BinomialPmf,
            compiler::Keyword::BinomialCdf => StatsKind::BinomialCdf,
            compiler::Keyword::BinomialBetween => StatsKind::BinomialBetween,
            compiler::Keyword::BinomialAtMost => StatsKind::BinomialAtMost,
            compiler::Keyword::BinomialAtLeast => StatsKind::BinomialAtLeast,
            compiler::Keyword::PoissonPmf => StatsKind::PoissonPmf,
            compiler::Keyword::PoissonCdf => StatsKind::PoissonCdf,
            compiler::Keyword::PoissonBetween => StatsKind::PoissonBetween,
            compiler::Keyword::PoissonAtMost => StatsKind::PoissonAtMost,
            compiler::Keyword::PoissonAtLeast => StatsKind::PoissonAtLeast,
            compiler::Keyword::ExponentialPdf => StatsKind::ExponentialPdf,
            compiler::Keyword::ExponentialCdf => StatsKind::ExponentialCdf,
            compiler::Keyword::ExponentialBetween => StatsKind::ExponentialBetween,
            compiler::Keyword::ExponentialInv => StatsKind::ExponentialInv,
            compiler::Keyword::UniformPdf => StatsKind::UniformPdf,
            compiler::Keyword::UniformCdf => StatsKind::UniformCdf,
            compiler::Keyword::UniformBetween => StatsKind::UniformBetween,
            compiler::Keyword::UniformInv => StatsKind::UniformInv,
            _ => return None,
        };

        let body = scanner::trimmed_shim(&input[command.matched_prefix.len()..]);
        let parts = scanner::split_top_level_commas(body);
        let allowed: HashSet<&'static str> = kind.named_fields().iter().map(|f| f.key).collect();

        let mut positional = Vec::new();
        let mut named = HashMap::new();

        for part in parts {
            let trimmed = scanner::trimmed_shim(&part);
            if let Some((name, value)) = named_argument(trimmed) {
                if allowed.contains(name.as_str()) {
                    named.insert(name, value);
                    continue;
                }
            }
            positional.push(trimmed.to_string());
        }

        Some(Self {
            kind,
            positional,
            named,
        })
    }

    pub fn friendly_input(&self) -> String {
        let trimmed_positional: Vec<&str> = self
            .positional
            .iter()
            .map(|s| scanner::trimmed_shim(s))
            .collect();
        let last_positional_idx = trimmed_positional.iter().rposition(|s| !s.is_empty());
        let rendered_positional: Vec<&str> = match last_positional_idx {
            Some(idx) => trimmed_positional[..=idx].to_vec(),
            None => Vec::new(),
        };

        let mut rendered_named: Vec<String> = Vec::new();
        for field in self.kind.named_fields() {
            if let Some(val) = self.named.get(field.key) {
                rendered_named.push(format!("{}={}", field.key, scanner::trimmed_shim(val)));
            }
        }

        let mut parts: Vec<String> = rendered_positional
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        parts.extend(rendered_named);

        if parts.is_empty() {
            self.kind.command().to_string()
        } else {
            format!("{} {}", self.kind.command(), parts.join(", "))
        }
    }
}

fn named_argument(text: &str) -> Option<(String, String)> {
    let eq_idx = text.find('=')?;
    let name = scanner::trimmed_shim(&text[..eq_idx]);
    if !variables::is_plausible_variable(name) {
        return None;
    }
    let value = scanner::trimmed_shim(&text[eq_idx + 1..]);
    Some((name.to_string(), value.to_string()))
}
