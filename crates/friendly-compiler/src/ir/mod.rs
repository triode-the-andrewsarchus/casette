//! Formula bar intermediate representations (IR) for autocomplete and chip editing.

pub mod assume;
pub mod binary;
pub mod derivative;
pub mod integral;
pub mod limit;
pub mod matrix;
pub mod numeric;
pub mod plot;
pub mod series_range;
pub mod solve;
pub mod stats;
pub mod subs;
pub mod taylor;
pub mod unary;
pub mod var;
pub mod vector_calculus;

pub use assume::AssumeFormulaIR;
pub use binary::BinaryFormulaIR;
pub use derivative::DerivativeFormulaIR;
pub use integral::IntegralFormulaIR;
pub use limit::LimitFormulaIR;
pub use matrix::MatrixFormulaIR;
pub use numeric::NumericFormulaIR;
pub use plot::{ImplicitPlotFormulaIR, ParametricPlotFormulaIR, PlotFormulaIR};
pub use series_range::SeriesRangeFormulaIR;
pub use solve::SolveFormulaIR;
pub use stats::StatsFormulaIR;
pub use subs::SubsFormulaIR;
pub use taylor::TaylorFormulaIR;
pub use unary::UnaryFormulaIR;
pub use var::VarFormulaIR;
pub use vector_calculus::VectorCalculusFormulaIR;

/// The dispatch point for Casette's completion UIs: one case per formula family.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FormulaIR {
    Integral(IntegralFormulaIR),
    Unary(UnaryFormulaIR),
    Solve(SolveFormulaIR),
    Derivative(DerivativeFormulaIR),
    Limit(LimitFormulaIR),
    Taylor(TaylorFormulaIR),
    SeriesRange(SeriesRangeFormulaIR),
    Plot(PlotFormulaIR),
    ImplicitPlot(ImplicitPlotFormulaIR),
    ParametricPlot(ParametricPlotFormulaIR),
    MatrixOp(MatrixFormulaIR),
    VectorCalculus(VectorCalculusFormulaIR),
    Subs(SubsFormulaIR),
    Numeric(NumericFormulaIR),
    VarDeclaration(VarFormulaIR),
    Assume(AssumeFormulaIR),
    Binary(BinaryFormulaIR),
    Stats(StatsFormulaIR),
}

impl FormulaIR {
    /// Attempts to parse the draft string into a recognized formula IR family.
    pub fn parse(draft: &str) -> Option<Self> {
        if let Some(ir) = IntegralFormulaIR::parse(draft) {
            return Some(Self::Integral(ir));
        }
        if let Some(ir) = UnaryFormulaIR::parse(draft) {
            return Some(Self::Unary(ir));
        }
        if let Some(ir) = SolveFormulaIR::parse(draft) {
            return Some(Self::Solve(ir));
        }
        if let Some(ir) = DerivativeFormulaIR::parse(draft) {
            return Some(Self::Derivative(ir));
        }
        if let Some(ir) = LimitFormulaIR::parse(draft) {
            return Some(Self::Limit(ir));
        }
        if let Some(ir) = TaylorFormulaIR::parse(draft) {
            return Some(Self::Taylor(ir));
        }
        if let Some(ir) = SeriesRangeFormulaIR::parse(draft) {
            return Some(Self::SeriesRange(ir));
        }
        if let Some(ir) = ImplicitPlotFormulaIR::parse(draft) {
            return Some(Self::ImplicitPlot(ir));
        }
        if let Some(ir) = ParametricPlotFormulaIR::parse(draft) {
            return Some(Self::ParametricPlot(ir));
        }
        if let Some(ir) = PlotFormulaIR::parse(draft) {
            return Some(Self::Plot(ir));
        }
        if let Some(ir) = MatrixFormulaIR::parse(draft) {
            return Some(Self::MatrixOp(ir));
        }
        if let Some(ir) = VectorCalculusFormulaIR::parse(draft) {
            return Some(Self::VectorCalculus(ir));
        }
        if let Some(ir) = SubsFormulaIR::parse(draft) {
            return Some(Self::Subs(ir));
        }
        if let Some(ir) = NumericFormulaIR::parse(draft) {
            return Some(Self::Numeric(ir));
        }
        if let Some(ir) = VarFormulaIR::parse(draft) {
            return Some(Self::VarDeclaration(ir));
        }
        if let Some(ir) = AssumeFormulaIR::parse(draft) {
            return Some(Self::Assume(ir));
        }
        if let Some(ir) = BinaryFormulaIR::parse(draft) {
            return Some(Self::Binary(ir));
        }
        if let Some(ir) = StatsFormulaIR::parse(draft) {
            return Some(Self::Stats(ir));
        }
        None
    }

    /// Renders back to friendly input (never Sage).
    pub fn friendly_input(&self) -> String {
        match self {
            Self::Integral(ir) => ir.friendly_input(),
            Self::Unary(ir) => ir.friendly_input(),
            Self::Solve(ir) => ir.friendly_input(),
            Self::Derivative(ir) => ir.friendly_input(),
            Self::Limit(ir) => ir.friendly_input(),
            Self::Taylor(ir) => ir.friendly_input(),
            Self::SeriesRange(ir) => ir.friendly_input(),
            Self::Plot(ir) => ir.friendly_input(),
            Self::ImplicitPlot(ir) => ir.friendly_input(),
            Self::ParametricPlot(ir) => ir.friendly_input(),
            Self::MatrixOp(ir) => ir.friendly_input(),
            Self::VectorCalculus(ir) => ir.friendly_input(),
            Self::Subs(ir) => ir.friendly_input(),
            Self::Numeric(ir) => ir.friendly_input(),
            Self::VarDeclaration(ir) => ir.friendly_input(),
            Self::Assume(ir) => ir.friendly_input(),
            Self::Binary(ir) => ir.friendly_input(),
            Self::Stats(ir) => ir.friendly_input(),
        }
    }
}
