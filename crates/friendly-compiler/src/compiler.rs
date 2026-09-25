//! The friendly input compiler — a COMMAND SHIM, not a language.
//!
//! Input is one line. If it begins with a known command word followed by a space
//! (or is exactly a command word), we treat it as friendly and rewrite it to Sage
//! source. Simple assignments are also rewritten to echo their assigned value in
//! the tape. Otherwise input BYPASSES untouched as raw Sage.

use crate::error::CompileError;
use crate::result::CompileResult;
use crate::scanner::{self, BalanceError, BalanceErrorKind};
use crate::variables;
use std::collections::{HashMap, HashSet};

/// Compile one line of friendly or raw input.
pub fn compile(raw_input: &str) -> CompileResult {
    let input_clean = raw_input.replace('\n', " ");
    let input_trimmed = scanner::trimmed_shim(&input_clean);

    if input_trimmed.is_empty() {
        return CompileResult::Bypass {
            raw_sage: String::new(),
        };
    }

    // 1. Check for unit conversion clause (e.g. `32 mi + 5 km to km` or `100 km / 2 hr in mph`)
    if let Some(conv) = crate::unit_parser::parse_unit_conversion(input_trimmed) {
        let desugared_inner = crate::unit_parser::desugar_unit_juxtaposition(&conv.expression);
        let inner_compiled = compile(&desugared_inner);
        let inner_sage = match inner_compiled {
            CompileResult::Success { generated_sage, .. } => generated_sage,
            CompileResult::Bypass { raw_sage } => raw_sage,
            other => return other,
        };
        let required = free_variables(&inner_sage);
        return CompileResult::Success {
            generated_sage: format!("__casette_convert({inner_sage}, {})", conv.target_unit),
            required_variables: required,
        };
    }

    // 2. Desugar natural unit juxtaposition (e.g. `32 mi` -> `32*mi`, `52 mi^2` -> `52*mi^2`)
    let desugared = crate::unit_parser::desugar_unit_juxtaposition(input_trimmed);
    let input = desugared.as_str();

    let cmd_match = leading_command(input);
    let cmd = match cmd_match {
        Some(c) => c,
        None => {
            if let Some(matrix) = matlab_matrix_shorthand(input) {
                return matrix;
            }
            if let Some(assignment) = simple_assignment_echo(input) {
                return assignment;
            }
            if crate::units::contains_any_unit(input) {
                return CompileResult::Success {
                    generated_sage: format!("__casette_convert({input})"),
                    required_variables: free_variables(input),
                };
            }
            return CompileResult::Bypass {
                raw_sage: input.to_string(),
            };
        }
    };

    let rest = scanner::trimmed_shim(&input[cmd.matched_prefix.len()..]);

    // `var = 3` is an assignment, not the `var` command
    if cmd.keyword == Keyword::VarDeclaration && rest.starts_with('=') {
        if let Some(assignment) = simple_assignment_echo(input) {
            return assignment;
        }
        return CompileResult::Bypass {
            raw_sage: input.to_string(),
        };
    }

    // `forget` is a no-argument command
    if cmd.keyword == Keyword::Forget {
        return forget_form(rest);
    }

    // A bare command word with no argument
    if rest.is_empty() {
        return CompileResult::Error(CompileError::new(
            format!("`{}` needs an expression.", cmd.keyword.canonical()),
            Some(input.len()),
            Some(cmd.keyword.example_usage().to_string()),
        ));
    }

    // Balance check on the whole remainder before clause parsing
    if let Some(bal) = scanner::balance_error(rest) {
        let base = input.len() - rest.len();
        return CompileResult::Error(balance_compile_error(&bal, base));
    }

    match cmd.keyword {
        Keyword::Factor => wrap_call("factor", rest),
        Keyword::Expand => wrap_call("expand", rest),
        Keyword::Simplify => simplify(rest),
        Keyword::Solve => solve(rest),
        Keyword::Derivative => derivative(rest),
        Keyword::Integral => integral(rest, false),
        Keyword::DoubleIntegral => integral(rest, true),
        Keyword::Limit => limit_form(rest),
        Keyword::Taylor => taylor(rest),
        Keyword::Plot => plot(rest),
        Keyword::ImplicitPlot => implicit_plot(rest),
        Keyword::ParametricPlot => parametric_plot(rest),
        Keyword::Sum => series_range(rest, "sum", "sum"),
        Keyword::Product => series_range(rest, "product", "product"),
        Keyword::Matrix => matrix_form(rest),
        Keyword::Vector => vector_form(rest),
        Keyword::Eigenvalues => matrix_method(rest, "eigenvalues"),
        Keyword::Rref => matrix_method(rest, "rref"),
        Keyword::Det => matrix_method(rest, "det"),
        Keyword::Inverse => matrix_method(rest, "inverse"),
        Keyword::Transpose => matrix_method(rest, "transpose"),
        Keyword::Rank => matrix_method(rest, "rank"),
        Keyword::Eigenvectors => matrix_method(rest, "eigenvectors_right"),
        Keyword::Gradient => gradient(rest),
        Keyword::Hessian => hessian(rest),
        Keyword::Jacobian => jacobian(rest),
        Keyword::Subs => subs(rest),
        Keyword::Numeric => numeric(rest),
        Keyword::Latex => latex_form(rest),
        Keyword::VarDeclaration => var_form(rest),
        Keyword::Assume => assume_form(rest),
        Keyword::Forget => forget_form(rest),
        Keyword::Choose => choose_form(rest),
        Keyword::Gcd => gcd_lcm_form(rest, "gcd"),
        Keyword::Lcm => gcd_lcm_form(rest, "lcm"),
        Keyword::Factorial => wrap_call("factorial", rest),
        Keyword::IsPrime => wrap_call("is_prime", rest),
        Keyword::FactorInteger => wrap_call("factor", rest),
        Keyword::Mean => mean_form(rest),
        Keyword::NormalPdf => stats_form(rest, StatsKind::NormalPdf),
        Keyword::NormalCdf => stats_form(rest, StatsKind::NormalCdf),
        Keyword::NormalBetween => stats_form(rest, StatsKind::NormalBetween),
        Keyword::NormalInv => stats_form(rest, StatsKind::NormalInv),
        Keyword::BinomialPmf => stats_form(rest, StatsKind::BinomialPmf),
        Keyword::BinomialCdf => stats_form(rest, StatsKind::BinomialCdf),
        Keyword::BinomialBetween => stats_form(rest, StatsKind::BinomialBetween),
        Keyword::BinomialAtMost => stats_form(rest, StatsKind::BinomialAtMost),
        Keyword::BinomialAtLeast => stats_form(rest, StatsKind::BinomialAtLeast),
        Keyword::PoissonPmf => stats_form(rest, StatsKind::PoissonPmf),
        Keyword::PoissonCdf => stats_form(rest, StatsKind::PoissonCdf),
        Keyword::PoissonBetween => stats_form(rest, StatsKind::PoissonBetween),
        Keyword::PoissonAtMost => stats_form(rest, StatsKind::PoissonAtMost),
        Keyword::PoissonAtLeast => stats_form(rest, StatsKind::PoissonAtLeast),
        Keyword::ExponentialPdf => stats_form(rest, StatsKind::ExponentialPdf),
        Keyword::ExponentialCdf => stats_form(rest, StatsKind::ExponentialCdf),
        Keyword::ExponentialBetween => stats_form(rest, StatsKind::ExponentialBetween),
        Keyword::ExponentialInv => stats_form(rest, StatsKind::ExponentialInv),
        Keyword::UniformPdf => stats_form(rest, StatsKind::UniformPdf),
        Keyword::UniformCdf => stats_form(rest, StatsKind::UniformCdf),
        Keyword::UniformBetween => stats_form(rest, StatsKind::UniformBetween),
        Keyword::UniformInv => stats_form(rest, StatsKind::UniformInv),
    }
}

/// Free variables of a Sage expression using the documented heuristic.
pub fn free_variables(expression: &str) -> Vec<String> {
    variables::free_variables(expression, &[])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Factor,
    Expand,
    Simplify,
    Solve,
    Derivative,
    Integral,
    DoubleIntegral,
    Limit,
    Taylor,
    Plot,
    ImplicitPlot,
    ParametricPlot,
    Sum,
    Product,
    Matrix,
    Vector,
    Eigenvalues,
    Rref,
    Det,
    Inverse,
    Transpose,
    Rank,
    Eigenvectors,
    Gradient,
    Hessian,
    Jacobian,
    Subs,
    Numeric,
    Latex,
    VarDeclaration,
    Assume,
    Forget,
    Choose,
    Gcd,
    Lcm,
    Factorial,
    IsPrime,
    FactorInteger,
    Mean,
    NormalPdf,
    NormalCdf,
    NormalBetween,
    NormalInv,
    BinomialPmf,
    BinomialCdf,
    BinomialBetween,
    BinomialAtMost,
    BinomialAtLeast,
    PoissonPmf,
    PoissonCdf,
    PoissonBetween,
    PoissonAtMost,
    PoissonAtLeast,
    ExponentialPdf,
    ExponentialCdf,
    ExponentialBetween,
    ExponentialInv,
    UniformPdf,
    UniformCdf,
    UniformBetween,
    UniformInv,
}

impl Keyword {
    pub const ALL: &'static [Keyword] = &[
        Keyword::Factor,
        Keyword::Expand,
        Keyword::Simplify,
        Keyword::Solve,
        Keyword::Derivative,
        Keyword::Integral,
        Keyword::DoubleIntegral,
        Keyword::Limit,
        Keyword::Taylor,
        Keyword::Plot,
        Keyword::ImplicitPlot,
        Keyword::ParametricPlot,
        Keyword::Sum,
        Keyword::Product,
        Keyword::Matrix,
        Keyword::Vector,
        Keyword::Eigenvalues,
        Keyword::Rref,
        Keyword::Det,
        Keyword::Inverse,
        Keyword::Transpose,
        Keyword::Rank,
        Keyword::Eigenvectors,
        Keyword::Gradient,
        Keyword::Hessian,
        Keyword::Jacobian,
        Keyword::Subs,
        Keyword::Numeric,
        Keyword::Latex,
        Keyword::VarDeclaration,
        Keyword::Assume,
        Keyword::Forget,
        Keyword::Choose,
        Keyword::Gcd,
        Keyword::Lcm,
        Keyword::Factorial,
        Keyword::IsPrime,
        Keyword::FactorInteger,
        Keyword::Mean,
        Keyword::NormalPdf,
        Keyword::NormalCdf,
        Keyword::NormalBetween,
        Keyword::NormalInv,
        Keyword::BinomialPmf,
        Keyword::BinomialCdf,
        Keyword::BinomialBetween,
        Keyword::BinomialAtMost,
        Keyword::BinomialAtLeast,
        Keyword::PoissonPmf,
        Keyword::PoissonCdf,
        Keyword::PoissonBetween,
        Keyword::PoissonAtMost,
        Keyword::PoissonAtLeast,
        Keyword::ExponentialPdf,
        Keyword::ExponentialCdf,
        Keyword::ExponentialBetween,
        Keyword::ExponentialInv,
        Keyword::UniformPdf,
        Keyword::UniformCdf,
        Keyword::UniformBetween,
        Keyword::UniformInv,
    ];

    pub fn phrases(&self) -> &'static [&'static str] {
        match self {
            Keyword::Factor => &["factor"],
            Keyword::Expand => &["expand"],
            Keyword::Simplify => &["simplify"],
            Keyword::Solve => &["solve"],
            Keyword::Derivative => &["derivative", "diff"],
            Keyword::Integral => &["integral", "integrate"],
            Keyword::DoubleIntegral => &["double integral", "double integrate"],
            Keyword::Limit => &["limit"],
            Keyword::Taylor => &["taylor"],
            Keyword::Plot => &["plot"],
            Keyword::ImplicitPlot => &["implicit_plot", "implicit plot"],
            Keyword::ParametricPlot => &["parametric_plot", "parametric plot"],
            Keyword::Sum => &["sum"],
            Keyword::Product => &["product"],
            Keyword::Matrix => &["matrix"],
            Keyword::Vector => &["vector"],
            Keyword::Eigenvalues => &["eigenvalues", "eigenvalue"],
            Keyword::Rref => &["rref"],
            Keyword::Det => &["det", "determinant"],
            Keyword::Inverse => &["inverse"],
            Keyword::Transpose => &["transpose"],
            Keyword::Rank => &["rank"],
            Keyword::Eigenvectors => &["eigenvectors", "eigenvector"],
            Keyword::Gradient => &["gradient", "grad"],
            Keyword::Hessian => &["hessian"],
            Keyword::Jacobian => &["jacobian"],
            Keyword::Subs => &["subs", "substitute"],
            Keyword::Numeric => &["numeric", "approx", "decimal"],
            Keyword::Latex => &["latex"],
            Keyword::VarDeclaration => &["var"],
            Keyword::Assume => &["assume"],
            Keyword::Forget => &["forget"],
            Keyword::Choose => &["choose"],
            Keyword::Gcd => &["gcd"],
            Keyword::Lcm => &["lcm"],
            Keyword::Factorial => &["factorial"],
            Keyword::IsPrime => &["is_prime"],
            Keyword::FactorInteger => &["factor_integer", "prime_factorization"],
            Keyword::Mean => &["mean"],
            Keyword::NormalPdf => &["normal_pdf"],
            Keyword::NormalCdf => &["normal_cdf"],
            Keyword::NormalBetween => &["normal_between"],
            Keyword::NormalInv => &["normal_inv"],
            Keyword::BinomialPmf => &["binomial_pmf"],
            Keyword::BinomialCdf => &["binomial_cdf"],
            Keyword::BinomialBetween => &["binomial_between"],
            Keyword::BinomialAtMost => &["binomial_at_most"],
            Keyword::BinomialAtLeast => &["binomial_at_least"],
            Keyword::PoissonPmf => &["poisson_pmf"],
            Keyword::PoissonCdf => &["poisson_cdf"],
            Keyword::PoissonBetween => &["poisson_between"],
            Keyword::PoissonAtMost => &["poisson_at_most"],
            Keyword::PoissonAtLeast => &["poisson_at_least"],
            Keyword::ExponentialPdf => &["exponential_pdf"],
            Keyword::ExponentialCdf => &["exponential_cdf"],
            Keyword::ExponentialBetween => &["exponential_between"],
            Keyword::ExponentialInv => &["exponential_inv"],
            Keyword::UniformPdf => &["uniform_pdf"],
            Keyword::UniformCdf => &["uniform_cdf"],
            Keyword::UniformBetween => &["uniform_between"],
            Keyword::UniformInv => &["uniform_inv"],
        }
    }

    pub fn canonical(&self) -> &'static str {
        self.phrases()[0]
    }

    pub fn example_usage(&self) -> &'static str {
        match self {
            Keyword::Factor => "Try: factor x^4 - 1",
            Keyword::Expand => "Try: expand (x + 1)^5",
            Keyword::Simplify => "Try: simplify sin(x)^2 + cos(x)^2",
            Keyword::Solve => "Try: solve x^2 + 5*x + 6 = 0  (optionally: ... for x)",
            Keyword::Derivative => "Try: derivative sin(x^2)  (optionally: ... wrt x)",
            Keyword::Integral => "Try: integral x^2  or  integral x^2, x=0..1",
            Keyword::DoubleIntegral => "Try: double integral x*y, x=0..1, y=0..x",
            Keyword::Limit => "Try: limit sin(x)/x, x->0",
            Keyword::Taylor => "Try: taylor sin(x), x=0, order=7",
            Keyword::Plot => "Try: plot sin(x), x=-pi..pi",
            Keyword::ImplicitPlot => "Try: implicit_plot x^2 + y^2 = 1, x=-2..2, y=-2..2",
            Keyword::ParametricPlot => "Try: parametric_plot (cos(t), sin(t)), t=0..2*pi",
            Keyword::Sum => "Try: sum k^2, k=1..n",
            Keyword::Product => "Try: product 1 + 1/k, k=1..n",
            Keyword::Matrix => "Try: matrix [1,2; 3,4]",
            Keyword::Vector => "Try: vector [1,2,3]",
            Keyword::Eigenvalues => "Try: eigenvalues [1,2; 3,4]",
            Keyword::Rref => "Try: rref [1,2; 3,4]",
            Keyword::Det => "Try: det [1,2; 3,4]",
            Keyword::Inverse => "Try: inverse [1,2; 3,4]",
            Keyword::Transpose => "Try: transpose [1,2; 3,4]",
            Keyword::Rank => "Try: rank [1,2; 3,4]",
            Keyword::Eigenvectors => "Try: eigenvectors [1,2; 3,4]",
            Keyword::Gradient => "Try: gradient x^2 + y^2  or  gradient x^2 + y^2, [x,y]",
            Keyword::Hessian => "Try: hessian x^2 + y^2",
            Keyword::Jacobian => "Try: jacobian [x^2+y, sin(x*y)], [x,y]",
            Keyword::Subs => "Try: subs x^2 + y, x=3",
            Keyword::Numeric => "Try: numeric pi  or  numeric pi, 50",
            Keyword::Latex => "Try: latex integral(sin(x), x)",
            Keyword::VarDeclaration => "Try: var a b c",
            Keyword::Assume => "Try: assume x > 0  or  assume x real",
            Keyword::Forget => "Try: forget",
            Keyword::Choose => "Try: choose 10, 3",
            Keyword::Gcd => "Try: gcd 12, 18  or  gcd [12, 18, 24]",
            Keyword::Lcm => "Try: lcm 12, 18  or  lcm [12, 18, 24]",
            Keyword::Factorial => "Try: factorial 5",
            Keyword::IsPrime => "Try: is_prime 104729",
            Keyword::FactorInteger => "Try: factor_integer 3600",
            Keyword::Mean => "Try: mean [1, 2, 3]",
            Keyword::NormalPdf => "Try: normal_pdf 0, mean=0, sd=1",
            Keyword::NormalCdf => "Try: normal_cdf 1.96",
            Keyword::NormalBetween => "Try: normal_between -1, 1",
            Keyword::NormalInv => "Try: normal_inv .975",
            Keyword::BinomialPmf => "Try: binomial_pmf 3, n=10, p=.5",
            Keyword::BinomialCdf => "Try: binomial_cdf 3, n=10, p=.5",
            Keyword::BinomialBetween => "Try: binomial_between 3, 7, n=10, p=.5",
            Keyword::BinomialAtMost => "Try: binomial_at_most 3, n=10, p=.5",
            Keyword::BinomialAtLeast => "Try: binomial_at_least 8, n=10, p=.5",
            Keyword::PoissonPmf => "Try: poisson_pmf 2, lambda=3",
            Keyword::PoissonCdf => "Try: poisson_cdf 2, lambda=3",
            Keyword::PoissonBetween => "Try: poisson_between 1, 4, lambda=3",
            Keyword::PoissonAtMost => "Try: poisson_at_most 2, lambda=3",
            Keyword::PoissonAtLeast => "Try: poisson_at_least 5, lambda=3",
            Keyword::ExponentialPdf => "Try: exponential_pdf 1, rate=2",
            Keyword::ExponentialCdf => "Try: exponential_cdf 1, rate=2",
            Keyword::ExponentialBetween => "Try: exponential_between 1, 2, rate=2",
            Keyword::ExponentialInv => "Try: exponential_inv .95, rate=2",
            Keyword::UniformPdf => "Try: uniform_pdf .5, min=0, max=1",
            Keyword::UniformCdf => "Try: uniform_cdf .5, min=0, max=1",
            Keyword::UniformBetween => "Try: uniform_between .2, .8, min=0, max=1",
            Keyword::UniformInv => "Try: uniform_inv .95, min=0, max=1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandMatch {
    pub keyword: Keyword,
    pub matched_prefix: String,
}

pub fn leading_command(input: &str) -> Option<CommandMatch> {
    let lower = input.to_lowercase();
    let mut best: Option<(Keyword, &'static str)> = None;

    for &kw in Keyword::ALL {
        for &phrase in kw.phrases() {
            if matches_command(&lower, phrase, input)
                && (best.is_none() || phrase.len() > best.unwrap().1.len())
            {
                best = Some((kw, phrase));
            }
        }
    }

    best.map(|(kw, phrase)| CommandMatch {
        keyword: kw,
        matched_prefix: input[..phrase.len()].to_string(),
    })
}

fn matches_command(lower: &str, phrase: &str, original: &str) -> bool {
    if !lower.starts_with(phrase) {
        return false;
    }
    if lower.len() == phrase.len() {
        return true;
    }
    let next = original[phrase.len()..].chars().next();
    matches!(next, Some(' ') | Some('\t'))
}

fn wrap_call(func: &str, payload: &str) -> CompileResult {
    let expr = scanner::trimmed_shim(payload);
    let sage = format!("{func}({expr})");
    CompileResult::Success {
        generated_sage: sage,
        required_variables: variables::free_variables(expr, &[]),
    }
}

fn simplify(payload: &str) -> CompileResult {
    let expr = scanner::trimmed_shim(payload);
    let sage = format!("({expr}).simplify_full()");
    CompileResult::Success {
        generated_sage: sage,
        required_variables: variables::free_variables(expr, &[]),
    }
}

fn solve(payload: &str) -> CompileResult {
    let mut body = payload.to_string();
    let mut for_var: Option<String> = None;

    if let Some(range) = trailing_clause(&body, "for") {
        for_var = Some(scanner::trimmed_shim(&body[range.value_range.clone()]).to_string());
        body = scanner::trimmed_shim(&body[..range.clause_start]).to_string();
    }

    let eq = match split_equation(&body) {
        Some(e) => e,
        None => {
            return CompileResult::Error(CompileError::new(
                "`solve` needs an equation with `=`.",
                None,
                Some("Try: solve x^2 + 5*x + 6 = 0  (optionally: ... for x)".to_string()),
            ));
        }
    };
    let relation = format!("{} == {}", eq.lhs, eq.rhs);
    let mut vars = variables::free_variables(&format!("{} {}", eq.lhs, eq.rhs), &[]);

    if let Some(v) = for_var {
        if !v.is_empty() {
            if !variables::is_plausible_variable(&v) {
                return CompileResult::Error(CompileError::new(
                    format!("`for {v}` is not a valid variable name."),
                    None,
                    Some("Try: solve x^2 + 5*x + 6 = 0 for x".to_string()),
                ));
            }
            if !vars.contains(&v) {
                vars.insert(0, v.clone());
            }
            return CompileResult::Success {
                generated_sage: format!("solve({relation}, {v})"),
                required_variables: vars,
            };
        }
    }

    match vars.len() {
        0 => CompileResult::Success {
            generated_sage: format!("solve({relation}, [])"),
            required_variables: Vec::new(),
        },
        1 => {
            let v = vars[0].clone();
            CompileResult::Success {
                generated_sage: format!("solve({relation}, {v})"),
                required_variables: vars,
            }
        }
        _ => {
            let candidates = vars
                .iter()
                .map(|v| format!("solve({relation}, {v})"))
                .collect();
            CompileResult::Ambiguous { candidates }
        }
    }
}

fn derivative(payload: &str) -> CompileResult {
    let mut body = payload.to_string();
    let mut wrt_var: Option<String> = None;

    if let Some(range) = trailing_clause(&body, "wrt") {
        wrt_var = Some(scanner::trimmed_shim(&body[range.value_range.clone()]).to_string());
        body = scanner::trimmed_shim(&body[..range.clause_start]).to_string();
    }

    let mut order: Option<String> = None;
    let parts = scanner::split_top_level_commas(&body);
    if parts.len() >= 2 {
        if let Some(last) = parts.last() {
            let last_trimmed = scanner::trimmed_shim(last);
            if !last_trimmed.is_empty() && last_trimmed.chars().all(|c| c.is_ascii_digit()) {
                order = Some(last_trimmed.to_string());
                body = parts[..parts.len() - 1].join(",");
            }
        }
    }

    let expr = scanner::trimmed_shim(&body);
    let vars = variables::free_variables(expr, &[]);

    let make_call = |v: &str| -> String {
        if let Some(ref o) = order {
            format!("derivative({expr}, {v}, {o})")
        } else {
            format!("derivative({expr}, {v})")
        }
    };

    if let Some(v) = wrt_var {
        if !v.is_empty() {
            if !variables::is_plausible_variable(&v) {
                return CompileResult::Error(CompileError::new(
                    format!("`wrt {v}` is not a valid variable name."),
                    None,
                    Some("Try: derivative sin(x^2) wrt x".to_string()),
                ));
            }
            let mut req = vars;
            if !req.contains(&v) {
                req.insert(0, v.clone());
            }
            return CompileResult::Success {
                generated_sage: make_call(&v),
                required_variables: req,
            };
        }
    }

    match vars.len() {
        0 => CompileResult::Error(CompileError::new(
            "`derivative` has no variable to differentiate by.",
            None,
            Some(format!("Add one: derivative {expr} wrt x")),
        )),
        1 => CompileResult::Success {
            generated_sage: make_call(&vars[0]),
            required_variables: vars,
        },
        _ => {
            let candidates = vars.iter().map(|v| make_call(v)).collect();
            CompileResult::Ambiguous { candidates }
        }
    }
}

fn integral(payload: &str, double: bool) -> CompileResult {
    let mut payload_str = payload.to_string();
    let mut wrt_var: Option<String> = None;

    if let Some(wrt) = trailing_clause(&payload_str, "wrt") {
        wrt_var = Some(scanner::trimmed_shim(&payload_str[wrt.value_range.clone()]).to_string());
        let before = &payload_str[..wrt.clause_start];
        payload_str = dropping_integral_trailing_comma(before).to_string();
    }

    let parts = scanner::split_top_level_commas(&payload_str);
    let integrand = parts
        .first()
        .map(|s| scanner::trimmed_shim(s))
        .unwrap_or("");
    if integrand.is_empty() {
        return CompileResult::Error(CompileError::new(
            "`integral` needs an expression.",
            None,
            Some(if double {
                "Try: double integral x*y, x=0..1, y=0..x".to_string()
            } else {
                "Try: integral x^2  or  integral x^2, x=0..1".to_string()
            }),
        ));
    }

    let range_clauses = &parts[1..];

    if double {
        if range_clauses.len() != 2 {
            return CompileResult::Error(CompileError::new(
                "`double integral` needs two ranges (outer first, inner last).",
                None,
                Some("Try: double integral x*y, x=0..1, y=0..x".to_string()),
            ));
        }
        let outer = match parse_range(&range_clauses[0]) {
            Some(r) => r,
            None => return CompileResult::Error(range_error(&range_clauses[0], "x=0..1")),
        };
        let inner = match parse_range(&range_clauses[1]) {
            Some(r) => r,
            None => return CompileResult::Error(range_error(&range_clauses[1], "y=0..x")),
        };

        let inner_sage = format!(
            "integrate({integrand}, ({}, {}, {}))",
            inner.variable, inner.lower, inner.upper
        );
        let sage = format!(
            "integrate({inner_sage}, ({}, {}, {}))",
            outer.variable, outer.lower, outer.upper
        );

        let var_source = format!(
            "{integrand} {} {} {} {}",
            inner.lower, inner.upper, outer.lower, outer.upper
        );
        let req_raw = variables::free_variables(&var_source, &[&outer.variable, &inner.variable]);
        let mut ordered = vec![outer.variable, inner.variable];
        ordered.extend(req_raw);
        let req = ordered_unique(ordered);

        return CompileResult::Success {
            generated_sage: sage,
            required_variables: req,
        };
    }

    match range_clauses.len() {
        0 => {
            let vars = variables::free_variables(integrand, &[]);
            if let Some(wrt) = wrt_var {
                if !wrt.is_empty() {
                    if !variables::is_plausible_variable(&wrt) {
                        return CompileResult::Error(CompileError::new(
                            format!("`wrt {wrt}` is not a valid variable name."),
                            None,
                            Some(format!("Try: integral {integrand}, wrt x")),
                        ));
                    }
                    let mut all_vars = vec![wrt.clone()];
                    all_vars.extend(vars);
                    return CompileResult::Success {
                        generated_sage: format!("integrate({integrand}, {wrt})"),
                        required_variables: ordered_unique(all_vars),
                    };
                }
            }
            match vars.len() {
                0 => CompileResult::Error(CompileError::new(
                    "`integral` has no variable to integrate by.",
                    None,
                    Some(format!(
                        "Add one: integral {integrand}, x=0..1  (or give a variable)"
                    )),
                )),
                1 => CompileResult::Success {
                    generated_sage: format!("integrate({integrand}, {})", vars[0]),
                    required_variables: vars,
                },
                _ => {
                    let candidates = vars
                        .iter()
                        .map(|v| format!("integrate({integrand}, {v})"))
                        .collect();
                    CompileResult::Ambiguous { candidates }
                }
            }
        }
        1 => {
            let r = match parse_range(&range_clauses[0]) {
                Some(r) => r,
                None => return CompileResult::Error(range_error(&range_clauses[0], "x=0..1")),
            };
            let sage = format!(
                "integrate({integrand}, ({}, {}, {}))",
                r.variable, r.lower, r.upper
            );
            let var_source = format!("{integrand} {} {}", r.lower, r.upper);
            let req_raw = variables::free_variables(&var_source, &[&r.variable]);
            let mut all_vars = vec![r.variable];
            all_vars.extend(req_raw);
            CompileResult::Success {
                generated_sage: sage,
                required_variables: ordered_unique(all_vars),
            }
        }
        _ => CompileResult::Error(CompileError::new(
            format!(
                "`integral` takes at most one range (got {}).",
                range_clauses.len()
            ),
            None,
            Some("For a nested integral use: double integral x*y, x=0..1, y=0..x".to_string()),
        )),
    }
}

fn limit_form(payload: &str) -> CompileResult {
    let parts = scanner::split_top_level_commas(payload);
    if parts.len() < 2 {
        return CompileResult::Error(CompileError::new(
            "`limit` needs an approach clause `var->value`.",
            None,
            Some("Try: limit sin(x)/x, x->0".to_string()),
        ));
    }
    let expr = scanner::trimmed_shim(&parts[0]);

    let mut dir: Option<&'static str> = None;
    let mut approach_parts: Vec<String> = parts[1..].to_vec();
    if approach_parts.len() >= 2 {
        let tail = scanner::trimmed_shim(approach_parts.last().unwrap()).to_lowercase();
        match tail.as_str() {
            "left" => {
                dir = Some("-");
                approach_parts.pop();
            }
            "right" => {
                dir = Some("+");
                approach_parts.pop();
            }
            _ => {
                return CompileResult::Error(CompileError::new(
                    "`limit` direction must be `left` or `right`.",
                    None,
                    Some("Try: limit sin(x)/x, x->0, right".to_string()),
                ));
            }
        }
    }

    let approach = approach_parts.join(", ");
    let approach_trimmed = scanner::trimmed_shim(&approach);

    let arrow_idx = match approach_trimmed.find("->") {
        Some(idx) => idx,
        None => {
            return CompileResult::Error(CompileError::new(
                "`limit` approach must use `->`, e.g. `x->0`.",
                None,
                Some("Try: limit sin(x)/x, x->0".to_string()),
            ));
        }
    };

    let v = scanner::trimmed_shim(&approach_trimmed[..arrow_idx]);
    let point = scanner::trimmed_shim(&approach_trimmed[arrow_idx + 2..]);

    if !variables::is_plausible_variable(v) {
        return CompileResult::Error(CompileError::new(
            format!("`limit` approach variable `{v}` is not valid."),
            None,
            Some("Try: limit sin(x)/x, x->0".to_string()),
        ));
    }

    if point.is_empty() {
        return CompileResult::Error(CompileError::new(
            "`limit` approach point is missing after `->`.",
            None,
            Some("Try: limit sin(x)/x, x->0".to_string()),
        ));
    }

    let sage = match dir {
        Some(d) => format!("limit({expr}, {v}={point}, dir='{d}')"),
        None => format!("limit({expr}, {v}={point})"),
    };

    let mut all_vars = vec![v.to_string()];
    all_vars.extend(variables::free_variables(&format!("{expr} {point}"), &[v]));
    CompileResult::Success {
        generated_sage: sage,
        required_variables: ordered_unique(all_vars),
    }
}

fn taylor(payload: &str) -> CompileResult {
    let parts = scanner::split_top_level_commas(payload);
    if parts.len() < 3 {
        return CompileResult::Error(CompileError::new(
            "`taylor` needs an expansion point and order: `expr, var=pt, order=n`.",
            None,
            Some("Try: taylor sin(x), x=0, order=7".to_string()),
        ));
    }
    let expr = scanner::trimmed_shim(&parts[0]);

    let center = scanner::trimmed_shim(&parts[1]);
    let eq_idx = match center.find('=') {
        Some(idx) if !center.contains("..") => idx,
        _ => {
            return CompileResult::Error(CompileError::new(
                "`taylor` expansion point must be `var=value`.",
                None,
                Some("Try: taylor sin(x), x=0, order=7".to_string()),
            ));
        }
    };
    let v = scanner::trimmed_shim(&center[..eq_idx]);
    let pt = scanner::trimmed_shim(&center[eq_idx + 1..]);
    if !variables::is_plausible_variable(v) || pt.is_empty() {
        return CompileResult::Error(CompileError::new(
            format!("`taylor` expansion point `{center}` is malformed."),
            None,
            Some("Try: taylor sin(x), x=0, order=7".to_string()),
        ));
    }

    let order_clause = scanner::trimmed_shim(&parts[2]);
    let oeq = match order_clause.find('=') {
        Some(idx) if scanner::trimmed_shim(&order_clause[..idx]).to_lowercase() == "order" => idx,
        _ => {
            return CompileResult::Error(CompileError::new(
                "`taylor` needs `order=n` as the last clause.",
                None,
                Some("Try: taylor sin(x), x=0, order=7".to_string()),
            ));
        }
    };
    let order = scanner::trimmed_shim(&order_clause[oeq + 1..]);
    if order.is_empty() || !order.chars().all(|c| c.is_ascii_digit()) {
        return CompileResult::Error(CompileError::new(
            format!("`taylor` order must be a whole number (got `{order}`)."),
            None,
            Some("Try: taylor sin(x), x=0, order=7".to_string()),
        ));
    }

    let sage = format!("taylor({expr}, {v}, {pt}, {order})");
    let mut all_vars = vec![v.to_string()];
    all_vars.extend(variables::free_variables(&format!("{expr} {pt}"), &[v]));
    CompileResult::Success {
        generated_sage: sage,
        required_variables: ordered_unique(all_vars),
    }
}

fn series_range(payload: &str, func: &str, command: &str) -> CompileResult {
    let example = format!("Try: {command} k^2, k=1..n");
    let parts = scanner::split_top_level_commas(payload);
    let expr = parts
        .first()
        .map(|s| scanner::trimmed_shim(s))
        .unwrap_or("");
    if expr.is_empty() {
        return CompileResult::Error(CompileError::new(
            format!("`{command}` needs an expression."),
            None,
            Some(example),
        ));
    }

    let range_clauses = &parts[1..];
    if range_clauses.len() > 1 {
        return CompileResult::Error(CompileError::new(
            format!(
                "`{command}` takes one range `k=1..n` (got {}).",
                range_clauses.len()
            ),
            None,
            Some(example),
        ));
    }
    let clause = match range_clauses.first() {
        Some(c) => c,
        None => {
            return CompileResult::Error(CompileError::new(
                format!("`{command}` needs a range `k=1..n`."),
                None,
                Some(example),
            ));
        }
    };

    let r = match parse_range(clause) {
        Some(r) => r,
        None => return CompileResult::Error(range_error(clause, "k=1..n")),
    };

    let generated = format!("{func}({expr}, {}, {}, {})", r.variable, r.lower, r.upper);
    let mut all_vars = vec![r.variable.clone()];
    all_vars.extend(variables::free_variables(
        &format!("{expr} {} {}", r.lower, r.upper),
        &[&r.variable],
    ));
    CompileResult::Success {
        generated_sage: generated,
        required_variables: ordered_unique(all_vars),
    }
}

fn plot(payload: &str) -> CompileResult {
    let parts = scanner::split_top_level_commas(payload);
    let expr = parts
        .first()
        .map(|s| scanner::trimmed_shim(s))
        .unwrap_or("");
    if expr.is_empty() {
        return CompileResult::Error(CompileError::new(
            "`plot` needs an expression.",
            None,
            Some("Try: plot sin(x), x=-pi..pi".to_string()),
        ));
    }
    if parts.len() < 2 {
        return CompileResult::Error(CompileError::new(
            "`plot` needs a range `var=a..b`.",
            None,
            Some("Try: plot sin(x), x=-pi..pi".to_string()),
        ));
    }
    let r = match parse_range(&parts[1]) {
        Some(r) => r,
        None => return CompileResult::Error(range_error(&parts[1], "x=-pi..pi")),
    };

    let sage = format!("plot({expr}, ({}, {}, {}))", r.variable, r.lower, r.upper);
    let mut all_vars = vec![r.variable.clone()];
    all_vars.extend(variables::free_variables(
        &format!("{expr} {} {}", r.lower, r.upper),
        &[&r.variable],
    ));
    CompileResult::Success {
        generated_sage: sage,
        required_variables: ordered_unique(all_vars),
    }
}

fn implicit_plot(payload: &str) -> CompileResult {
    let example = "Try: implicit_plot x^2 + y^2 = 1, x=-2..2, y=-2..2";
    let parts = scanner::split_top_level_commas(payload);
    let raw_equation = parts
        .first()
        .map(|s| scanner::trimmed_shim(s))
        .unwrap_or("");
    if raw_equation.is_empty() {
        return CompileResult::Error(CompileError::new(
            "`implicit_plot` needs an equation.",
            None,
            Some(example.to_string()),
        ));
    }

    let relation = if raw_equation.contains("==") {
        raw_equation.to_string()
    } else if let Some(eq) = split_equation(raw_equation) {
        format!("{} == {}", eq.lhs, eq.rhs)
    } else {
        format!("{raw_equation} == 0")
    };

    let range_clauses = &parts[1..];
    if range_clauses.len() != 2 {
        return CompileResult::Error(CompileError::new(
            "`implicit_plot` needs two ranges (`x=...`, then `y=...`).",
            None,
            Some(example.to_string()),
        ));
    }

    let x_range = match parse_range(&range_clauses[0]) {
        Some(r) => r,
        None => return CompileResult::Error(range_error(&range_clauses[0], "x=-2..2")),
    };
    let y_range = match parse_range(&range_clauses[1]) {
        Some(r) => r,
        None => return CompileResult::Error(range_error(&range_clauses[1], "y=-2..2")),
    };

    let sage = format!(
        "implicit_plot({relation}, ({}, {}, {}), ({}, {}, {}))",
        x_range.variable,
        x_range.lower,
        x_range.upper,
        y_range.variable,
        y_range.lower,
        y_range.upper
    );

    let var_source = format!(
        "{relation} {} {} {} {}",
        x_range.lower, x_range.upper, y_range.lower, y_range.upper
    );
    let mut all_vars = vec![x_range.variable.clone(), y_range.variable.clone()];
    all_vars.extend(variables::free_variables(
        &var_source,
        &[&x_range.variable, &y_range.variable],
    ));
    CompileResult::Success {
        generated_sage: sage,
        required_variables: ordered_unique(all_vars),
    }
}

fn parametric_plot(payload: &str) -> CompileResult {
    let example = "Try: parametric_plot (cos(t), sin(t)), t=0..2*pi";
    let parts = scanner::split_top_level_commas(payload);
    let pair_text = parts
        .first()
        .map(|s| scanner::trimmed_shim(s))
        .unwrap_or("");
    if pair_text.is_empty() || !pair_text.starts_with('(') || !pair_text.ends_with(')') {
        return CompileResult::Error(CompileError::new(
            "`parametric_plot` needs a coordinate pair `(x(t), y(t))`.",
            None,
            Some(example.to_string()),
        ));
    }

    let inner = &pair_text[1..pair_text.len() - 1];
    let raw_coords = scanner::split_top_level_commas(inner);
    let coords: Vec<&str> = raw_coords
        .iter()
        .map(|s| scanner::trimmed_shim(s))
        .filter(|s| !s.is_empty())
        .collect();
    if coords.len() != 2 {
        return CompileResult::Error(CompileError::new(
            "`parametric_plot` needs a coordinate pair `(x(t), y(t))`.",
            None,
            Some(example.to_string()),
        ));
    }
    let (x_expr, y_expr) = (coords[0], coords[1]);

    let range_clauses = &parts[1..];
    if range_clauses.len() != 1 {
        return CompileResult::Error(CompileError::new(
            "`parametric_plot` needs one range `var=a..b`.",
            None,
            Some(example.to_string()),
        ));
    }
    let r = match parse_range(&range_clauses[0]) {
        Some(r) => r,
        None => return CompileResult::Error(range_error(&range_clauses[0], "t=0..2*pi")),
    };

    let sage = format!(
        "parametric_plot(({x_expr}, {y_expr}), ({}, {}, {}))",
        r.variable, r.lower, r.upper
    );
    let mut all_vars = vec![r.variable.clone()];
    all_vars.extend(variables::free_variables(
        &format!("{x_expr} {y_expr} {} {}", r.lower, r.upper),
        &[&r.variable],
    ));
    CompileResult::Success {
        generated_sage: sage,
        required_variables: ordered_unique(all_vars),
    }
}

fn matrix_form(payload: &str) -> CompileResult {
    match normalize_matrix_payload(payload, false) {
        Some(body) => CompileResult::Success {
            generated_sage: format!("matrix({body})"),
            required_variables: Vec::new(),
        },
        None => CompileResult::Error(CompileError::new(
            "`matrix` needs a bracketed list of rows.",
            None,
            Some("Try: matrix [1,2; 3,4]".to_string()),
        )),
    }
}

fn matrix_method(payload: &str, method: &str) -> CompileResult {
    let body = scanner::trimmed_shim(payload);
    if body.starts_with('[') {
        match normalize_matrix_payload(body, false) {
            Some(normalized) => CompileResult::Success {
                generated_sage: format!("matrix({normalized}).{method}()"),
                required_variables: Vec::new(),
            },
            None => CompileResult::Error(CompileError::new(
                format!("`{method}` needs a bracketed list of rows."),
                None,
                Some(format!("Try: {method} [1,2; 3,4]")),
            )),
        }
    } else {
        CompileResult::Success {
            generated_sage: format!("({body}).{method}()"),
            required_variables: variables::free_variables(body, &[]),
        }
    }
}

fn vector_form(payload: &str) -> CompileResult {
    let body = scanner::trimmed_shim(payload);
    if body.starts_with('[') && body.ends_with(']') && single_outer_bracket_body(body).is_some() {
        CompileResult::Success {
            generated_sage: format!("vector({body})"),
            required_variables: variables::free_variables(body, &[]),
        }
    } else {
        CompileResult::Error(CompileError::new(
            "`vector` needs a bracketed list of entries.",
            None,
            Some("Try: vector [1,2,3]".to_string()),
        ))
    }
}

fn gradient(payload: &str) -> CompileResult {
    let parts = scanner::split_top_level_commas(payload);
    let expr = parts
        .first()
        .map(|s| scanner::trimmed_shim(s))
        .unwrap_or("");
    if expr.is_empty() {
        return CompileResult::Error(CompileError::new(
            "`gradient` needs an expression.",
            None,
            Some("Try: gradient x^2 + y^2  or  gradient x^2 + y^2, [x,y]".to_string()),
        ));
    }
    let rest = &parts[1..];
    if rest.is_empty() {
        return CompileResult::Success {
            generated_sage: format!("({expr}).gradient()"),
            required_variables: variables::free_variables(expr, &[]),
        };
    }
    let list_text = rest.join(", ");
    match bracketed_variable_list(scanner::trimmed_shim(&list_text)) {
        Some(vars) => {
            let mut all_vars = vars.clone();
            let bound_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
            all_vars.extend(variables::free_variables(expr, &bound_refs));
            CompileResult::Success {
                generated_sage: format!("({expr}).gradient([{}])", vars.join(", ")),
                required_variables: ordered_unique(all_vars),
            }
        }
        None => CompileResult::Error(CompileError::new(
            "`gradient` variables must be a bracketed list like `[x, y]`.",
            None,
            Some("Try: gradient x^2 + y^2, [x,y]".to_string()),
        )),
    }
}

fn hessian(payload: &str) -> CompileResult {
    let parts = scanner::split_top_level_commas(payload);
    let expr = parts
        .first()
        .map(|s| scanner::trimmed_shim(s))
        .unwrap_or("");
    if expr.is_empty() {
        return CompileResult::Error(CompileError::new(
            "`hessian` needs an expression.",
            None,
            Some("Try: hessian x^2 + y^2".to_string()),
        ));
    }
    if parts.len() != 1 {
        return CompileResult::Error(CompileError::new(
            "`hessian` takes just an expression.",
            None,
            Some("Try: hessian x^2 + y^2".to_string()),
        ));
    }
    CompileResult::Success {
        generated_sage: format!("({expr}).hessian()"),
        required_variables: variables::free_variables(expr, &[]),
    }
}

fn jacobian(payload: &str) -> CompileResult {
    let example = "Try: jacobian [x^2+y, sin(x*y)], [x,y]";
    let parts = scanner::split_top_level_commas(payload);
    let functions_text = parts
        .first()
        .map(|s| scanner::trimmed_shim(s))
        .unwrap_or("");
    if functions_text.is_empty() {
        return CompileResult::Error(CompileError::new(
            "`jacobian` needs a bracketed list of functions.",
            None,
            Some(example.to_string()),
        ));
    }
    if !functions_text.starts_with('[')
        || !functions_text.ends_with(']')
        || single_outer_bracket_body(functions_text).is_none()
    {
        return CompileResult::Error(CompileError::new(
            "`jacobian` functions must be a bracketed list like `[x^2+y, sin(x*y)]`.",
            None,
            Some(example.to_string()),
        ));
    }
    let rest = &parts[1..];
    if rest.is_empty() {
        return CompileResult::Error(CompileError::new(
            "`jacobian` needs a bracketed variable list.",
            None,
            Some(example.to_string()),
        ));
    }
    let list_text = rest.join(", ");
    match bracketed_variable_list(scanner::trimmed_shim(&list_text)) {
        Some(vars) => {
            let mut all_vars = vars.clone();
            let bound_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
            all_vars.extend(variables::free_variables(functions_text, &bound_refs));
            CompileResult::Success {
                generated_sage: format!("jacobian({functions_text}, [{}])", vars.join(", ")),
                required_variables: ordered_unique(all_vars),
            }
        }
        None => CompileResult::Error(CompileError::new(
            "`jacobian` variables must be a bracketed list like `[x, y]`.",
            None,
            Some(example.to_string()),
        )),
    }
}

fn subs(payload: &str) -> CompileResult {
    let parts = scanner::split_top_level_commas(payload);
    let expr = parts
        .first()
        .map(|s| scanner::trimmed_shim(s))
        .unwrap_or("");
    if expr.is_empty() {
        return CompileResult::Error(CompileError::new(
            "`subs` needs an expression.",
            None,
            Some("Try: subs x^2 + y, x=3".to_string()),
        ));
    }
    let binding_clauses = &parts[1..];
    if binding_clauses.is_empty() {
        return CompileResult::Error(CompileError::new(
            "`subs` needs at least one binding `var=value`.",
            None,
            Some("Try: subs x^2 + y, x=3".to_string()),
        ));
    }
    let mut bindings: Vec<(String, String)> = Vec::new();
    for clause in binding_clauses {
        let c = scanner::trimmed_shim(clause);
        let eq_idx = match c.find('=') {
            Some(idx) if !c.contains("..") => idx,
            _ => {
                return CompileResult::Error(CompileError::new(
                    format!("`subs` binding `{c}` must be `var=value`."),
                    None,
                    Some("Try: subs x^2 + y, x=3".to_string()),
                ));
            }
        };
        let name = scanner::trimmed_shim(&c[..eq_idx]);
        let value = scanner::trimmed_shim(&c[eq_idx + 1..]);
        if !variables::is_plausible_variable(name) || value.is_empty() {
            return CompileResult::Error(CompileError::new(
                format!("`subs` binding `{c}` must be `var=value`."),
                None,
                Some("Try: subs x^2 + y, x=3".to_string()),
            ));
        }
        bindings.push((name.to_string(), value.to_string()));
    }

    let binding_sage = bindings
        .iter()
        .map(|(n, v)| format!("{n}={v}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut all_vars: Vec<String> = bindings.iter().map(|(n, _)| n.clone()).collect();
    let values_str = bindings
        .iter()
        .map(|(_, v)| v.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    all_vars.extend(variables::free_variables(
        &format!("{expr} {values_str}"),
        &[],
    ));

    CompileResult::Success {
        generated_sage: format!("({expr}).subs({binding_sage})"),
        required_variables: ordered_unique(all_vars),
    }
}

fn numeric(payload: &str) -> CompileResult {
    let parts = scanner::split_top_level_commas(payload);
    let expr = parts
        .first()
        .map(|s| scanner::trimmed_shim(s))
        .unwrap_or("");
    if expr.is_empty() {
        return CompileResult::Error(CompileError::new(
            "`numeric` needs an expression.",
            None,
            Some("Try: numeric pi  or  numeric pi, 50".to_string()),
        ));
    }
    let rest = &parts[1..];
    if rest.is_empty() {
        return CompileResult::Success {
            generated_sage: format!("N({expr})"),
            required_variables: variables::free_variables(expr, &[]),
        };
    }
    if rest.len() != 1 {
        return CompileResult::Error(CompileError::new(
            "`numeric` takes at most a digit count after the expression.",
            None,
            Some("Try: numeric pi, 50".to_string()),
        ));
    }
    let digits = scanner::trimmed_shim(&rest[0]);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return CompileResult::Error(CompileError::new(
            format!("`numeric` digits must be a whole number (got `{digits}`)."),
            None,
            Some("Try: numeric pi, 50".to_string()),
        ));
    }
    CompileResult::Success {
        generated_sage: format!("N({expr}, digits={digits})"),
        required_variables: variables::free_variables(expr, &[]),
    }
}

fn latex_form(payload: &str) -> CompileResult {
    let expr = scanner::trimmed_shim(payload);
    CompileResult::Success {
        generated_sage: format!("latex({expr})"),
        required_variables: variables::free_variables(expr, &[]),
    }
}

fn var_form(payload: &str) -> CompileResult {
    let example = "`var` names must be identifiers. Try: var a b c";
    let clauses: Vec<String> = scanner::split_top_level_commas(payload)
        .into_iter()
        .map(|s| scanner::trimmed_shim(&s).to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let names: Vec<String> = if clauses.len() <= 1 {
        let first = clauses.first().map(|s| s.as_str()).unwrap_or("");
        first.split_whitespace().map(|s| s.to_string()).collect()
    } else {
        clauses
    };

    if names.is_empty() || !names.iter().all(|n| variables::is_plausible_variable(n)) {
        return CompileResult::Error(CompileError::new(
            example,
            None,
            Some("Try: var a b c".to_string()),
        ));
    }

    CompileResult::Success {
        generated_sage: format!("var('{}')", names.join(" ")),
        required_variables: Vec::new(),
    }
}

fn assume_form(payload: &str) -> CompileResult {
    let example = "Try: assume x > 0  or  assume x real";
    let body = scanner::trimmed_shim(payload);

    if top_level_comparison_operator(body).is_some() {
        return CompileResult::Success {
            generated_sage: format!("assume({body})"),
            required_variables: variables::free_variables(body, &[]),
        };
    }

    if split_equation(body).is_some() {
        return CompileResult::Error(CompileError::new(
            "`assume` needs a comparison like `x > 0`, not `=`.",
            None,
            Some(example.to_string()),
        ));
    }

    let mut words: Vec<&str> = body.split_whitespace().collect();
    if words.len() == 3 && words[1].eq_ignore_ascii_case("is") {
        words = vec![words[0], words[2]];
    }

    if words.len() != 2 || !variables::is_plausible_variable(words[0]) {
        return CompileResult::Error(CompileError::new(
            "`assume` is malformed.",
            None,
            Some(example.to_string()),
        ));
    }

    let variable = words[0];
    let property = words[1].to_lowercase();
    let known_properties: HashSet<&'static str> = [
        "real",
        "integer",
        "rational",
        "complex",
        "imaginary",
        "even",
        "odd",
        "noninteger",
    ]
    .into_iter()
    .collect();

    let sage = match property.as_str() {
        "positive" => format!("assume({variable} > 0)"),
        "negative" => format!("assume({variable} < 0)"),
        p if known_properties.contains(p) => format!("assume({variable}, '{p}')"),
        _ => {
            return CompileResult::Error(CompileError::new(
                format!("`assume` property `{}` is not recognized.", words[1]),
                None,
                Some(example.to_string()),
            ));
        }
    };

    CompileResult::Success {
        generated_sage: sage,
        required_variables: vec![variable.to_string()],
    }
}

fn top_level_comparison_operator(s: &str) -> Option<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut depth: isize = 0;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        let next = if i + 1 < chars.len() {
            chars[i + 1]
        } else {
            ' '
        };
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '<' | '>' if depth == 0 => {
                return Some(if next == '=' {
                    format!("{ch}=")
                } else {
                    ch.to_string()
                });
            }
            '=' if depth == 0 && next == '=' => return Some("==".to_string()),
            '!' if depth == 0 && next == '=' => return Some("!=".to_string()),
            _ => {}
        }
        i += 1;
    }
    None
}

fn forget_form(payload: &str) -> CompileResult {
    let body = scanner::trimmed_shim(payload).to_lowercase();
    if body.is_empty() || body == "all" || body == "assumptions" {
        CompileResult::Success {
            generated_sage: "forget()".to_string(),
            required_variables: Vec::new(),
        }
    } else {
        CompileResult::Error(CompileError::new(
            "`forget` clears all assumptions.",
            None,
            Some("Try: forget".to_string()),
        ))
    }
}

fn choose_form(payload: &str) -> CompileResult {
    let example = "Try: choose 10, 3";
    let parts: Vec<String> = scanner::split_top_level_commas(payload)
        .into_iter()
        .map(|s| scanner::trimmed_shim(&s).to_string())
        .collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return CompileResult::Error(CompileError::new(
            "`choose` needs two values `n, k`.",
            None,
            Some(example.to_string()),
        ));
    }
    CompileResult::Success {
        generated_sage: format!("binomial({}, {})", parts[0], parts[1]),
        required_variables: variables::free_variables(&format!("{} {}", parts[0], parts[1]), &[]),
    }
}

fn gcd_lcm_form(payload: &str, func: &str) -> CompileResult {
    let example = format!("Try: {func} 12, 18  or  {func} [12, 18, 24]");
    let parts: Vec<String> = scanner::split_top_level_commas(payload)
        .into_iter()
        .map(|s| scanner::trimmed_shim(&s).to_string())
        .collect();
    let all_vars = variables::free_variables(payload, &[]);
    match parts.len() {
        1 => {
            let body = &parts[0];
            if !body.starts_with('[') {
                return CompileResult::Error(CompileError::new(
                    format!("`{func}` needs two values or a bracketed list."),
                    None,
                    Some(example),
                ));
            }
            CompileResult::Success {
                generated_sage: format!("{func}({body})"),
                required_variables: all_vars,
            }
        }
        2 => CompileResult::Success {
            generated_sage: format!("{func}({}, {})", parts[0], parts[1]),
            required_variables: all_vars,
        },
        _ => CompileResult::Success {
            generated_sage: format!("{func}([{}])", parts.join(", ")),
            required_variables: all_vars,
        },
    }
}

fn mean_form(payload: &str) -> CompileResult {
    let example = "Try: mean [1, 2, 3]";
    let parts: Vec<String> = scanner::split_top_level_commas(payload)
        .into_iter()
        .map(|s| scanner::trimmed_shim(&s).to_string())
        .collect();
    if parts.len() != 1 || parts[0].is_empty() {
        return CompileResult::Error(CompileError::new(
            "`mean` takes one list of data.",
            None,
            Some(example.to_string()),
        ));
    }
    let data = &parts[0];
    CompileResult::Success {
        generated_sage: format!("sum({data})/len({data})"),
        required_variables: variables::free_variables(data, &[]),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StatsKind {
    NormalPdf,
    NormalCdf,
    NormalBetween,
    NormalInv,
    BinomialPmf,
    BinomialCdf,
    BinomialBetween,
    BinomialAtMost,
    BinomialAtLeast,
    PoissonPmf,
    PoissonCdf,
    PoissonBetween,
    PoissonAtMost,
    PoissonAtLeast,
    ExponentialPdf,
    ExponentialCdf,
    ExponentialBetween,
    ExponentialInv,
    UniformPdf,
    UniformCdf,
    UniformBetween,
    UniformInv,
}

impl StatsKind {
    pub fn command(&self) -> &'static str {
        match self {
            StatsKind::NormalPdf => "normal_pdf",
            StatsKind::NormalCdf => "normal_cdf",
            StatsKind::NormalBetween => "normal_between",
            StatsKind::NormalInv => "normal_inv",
            StatsKind::BinomialPmf => "binomial_pmf",
            StatsKind::BinomialCdf => "binomial_cdf",
            StatsKind::BinomialBetween => "binomial_between",
            StatsKind::BinomialAtMost => "binomial_at_most",
            StatsKind::BinomialAtLeast => "binomial_at_least",
            StatsKind::PoissonPmf => "poisson_pmf",
            StatsKind::PoissonCdf => "poisson_cdf",
            StatsKind::PoissonBetween => "poisson_between",
            StatsKind::PoissonAtMost => "poisson_at_most",
            StatsKind::PoissonAtLeast => "poisson_at_least",
            StatsKind::ExponentialPdf => "exponential_pdf",
            StatsKind::ExponentialCdf => "exponential_cdf",
            StatsKind::ExponentialBetween => "exponential_between",
            StatsKind::ExponentialInv => "exponential_inv",
            StatsKind::UniformPdf => "uniform_pdf",
            StatsKind::UniformCdf => "uniform_cdf",
            StatsKind::UniformBetween => "uniform_between",
            StatsKind::UniformInv => "uniform_inv",
        }
    }

    pub fn positional_count(&self) -> usize {
        match self {
            StatsKind::NormalBetween
            | StatsKind::BinomialBetween
            | StatsKind::PoissonBetween
            | StatsKind::ExponentialBetween
            | StatsKind::UniformBetween => 2,
            _ => 1,
        }
    }

    pub fn allowed_named(&self) -> &'static [&'static str] {
        match self {
            StatsKind::NormalPdf
            | StatsKind::NormalCdf
            | StatsKind::NormalBetween
            | StatsKind::NormalInv => &["mean", "sd"],
            StatsKind::BinomialPmf
            | StatsKind::BinomialCdf
            | StatsKind::BinomialBetween
            | StatsKind::BinomialAtMost
            | StatsKind::BinomialAtLeast => &["n", "p"],
            StatsKind::PoissonPmf
            | StatsKind::PoissonCdf
            | StatsKind::PoissonBetween
            | StatsKind::PoissonAtMost
            | StatsKind::PoissonAtLeast => &["lambda"],
            StatsKind::ExponentialPdf
            | StatsKind::ExponentialCdf
            | StatsKind::ExponentialBetween
            | StatsKind::ExponentialInv => &["rate"],
            StatsKind::UniformPdf
            | StatsKind::UniformCdf
            | StatsKind::UniformBetween
            | StatsKind::UniformInv => &["min", "max"],
        }
    }

    pub fn required_named(&self) -> &'static [&'static str] {
        match self {
            StatsKind::NormalPdf
            | StatsKind::NormalCdf
            | StatsKind::NormalBetween
            | StatsKind::NormalInv => &[],
            _ => self.allowed_named(),
        }
    }

    pub fn example(&self) -> &'static str {
        match self {
            StatsKind::NormalPdf => "Try: normal_pdf 0, mean=0, sd=1",
            StatsKind::NormalCdf => "Try: normal_cdf 1.96",
            StatsKind::NormalBetween => "Try: normal_between -1, 1",
            StatsKind::NormalInv => "Try: normal_inv .975",
            StatsKind::BinomialPmf => "Try: binomial_pmf 3, n=10, p=.5",
            StatsKind::BinomialCdf => "Try: binomial_cdf 3, n=10, p=.5",
            StatsKind::BinomialBetween => "Try: binomial_between 3, 7, n=10, p=.5",
            StatsKind::BinomialAtMost => "Try: binomial_at_most 3, n=10, p=.5",
            StatsKind::BinomialAtLeast => "Try: binomial_at_least 8, n=10, p=.5",
            StatsKind::PoissonPmf => "Try: poisson_pmf 2, lambda=3",
            StatsKind::PoissonCdf => "Try: poisson_cdf 2, lambda=3",
            StatsKind::PoissonBetween => "Try: poisson_between 1, 4, lambda=3",
            StatsKind::PoissonAtMost => "Try: poisson_at_most 2, lambda=3",
            StatsKind::PoissonAtLeast => "Try: poisson_at_least 5, lambda=3",
            StatsKind::ExponentialPdf => "Try: exponential_pdf 1, rate=2",
            StatsKind::ExponentialCdf => "Try: exponential_cdf 1, rate=2",
            StatsKind::ExponentialBetween => "Try: exponential_between 1, 2, rate=2",
            StatsKind::ExponentialInv => "Try: exponential_inv .95, rate=2",
            StatsKind::UniformPdf => "Try: uniform_pdf .5, min=0, max=1",
            StatsKind::UniformCdf => "Try: uniform_cdf .5, min=0, max=1",
            StatsKind::UniformBetween => "Try: uniform_between .2, .8, min=0, max=1",
            StatsKind::UniformInv => "Try: uniform_inv .95, min=0, max=1",
        }
    }
}

fn stats_form(payload: &str, kind: StatsKind) -> CompileResult {
    let parts: Vec<String> = scanner::split_top_level_commas(payload)
        .into_iter()
        .map(|s| scanner::trimmed_shim(&s).to_string())
        .collect();

    let mut positional: Vec<String> = Vec::new();
    let mut named: HashMap<String, String> = HashMap::new();
    let mut saw_named = false;

    for part in parts {
        if part.is_empty() {
            continue;
        }
        if let Some((name, value)) = simple_named_argument(&part) {
            saw_named = true;
            if !kind.allowed_named().contains(&name.as_str()) {
                return CompileResult::Error(CompileError::new(
                    format!("`{}` does not take `{name}=...`.", kind.command()),
                    None,
                    Some(kind.example().to_string()),
                ));
            }
            if value.is_empty() {
                return CompileResult::Error(CompileError::new(
                    format!("`{}` needs a value for `{name}`.", kind.command()),
                    None,
                    Some(kind.example().to_string()),
                ));
            }
            named.insert(name, value);
        } else {
            if saw_named {
                return CompileResult::Error(CompileError::new(
                    format!(
                        "`{}` positional values must come before named parameters.",
                        kind.command()
                    ),
                    None,
                    Some(kind.example().to_string()),
                ));
            }
            positional.push(part);
        }
    }

    if positional.len() != kind.positional_count() || !positional.iter().all(|s| !s.is_empty()) {
        let msg = format!(
            "`{}` needs {} positional value{}.",
            kind.command(),
            kind.positional_count(),
            if kind.positional_count() == 1 {
                ""
            } else {
                "s"
            }
        );
        return CompileResult::Error(CompileError::new(
            msg,
            None,
            Some(kind.example().to_string()),
        ));
    }

    for &required in kind.required_named() {
        if !named.contains_key(required) {
            return CompileResult::Error(CompileError::new(
                format!("`{}` needs `{required}=...`.", kind.command()),
                None,
                Some(kind.example().to_string()),
            ));
        }
    }

    let mut rendered_named: Vec<String> = Vec::new();
    for &friendly_name in kind.allowed_named() {
        if let Some(val) = named.get(friendly_name) {
            rendered_named.push(format!("{}={val}", sage_argument_name(friendly_name)));
        }
    }

    let mut args = positional.clone();
    args.extend(rendered_named);

    let mut var_source = positional;
    for &n in kind.allowed_named() {
        if let Some(val) = named.get(n) {
            var_source.push(val.clone());
        }
    }

    CompileResult::Success {
        generated_sage: format!("{}({})", kind.command(), args.join(", ")),
        required_variables: variables::free_variables(&var_source.join(" "), &[]),
    }
}

fn simple_named_argument(text: &str) -> Option<(String, String)> {
    let eq_idx = text.find('=')?;
    let name = scanner::trimmed_shim(&text[..eq_idx]);
    if !variables::is_plausible_variable(name) {
        return None;
    }
    let value = scanner::trimmed_shim(&text[eq_idx + 1..]);
    Some((name.to_string(), value.to_string()))
}

fn sage_argument_name(friendly_name: &str) -> &str {
    match friendly_name {
        "lambda" => "lambda_",
        "min" => "low",
        "max" => "high",
        _ => friendly_name,
    }
}

fn bracketed_variable_list(text: &str) -> Option<Vec<String>> {
    let body = scanner::trimmed_shim(text);
    if !body.starts_with('[') || !body.ends_with(']') {
        return None;
    }
    let inside = single_outer_bracket_body(body)?;
    let entries: Vec<String> = scanner::split_top_level_commas(inside)
        .into_iter()
        .map(|s| scanner::trimmed_shim(&s).to_string())
        .collect();
    if entries.is_empty() || !entries.iter().all(|s| variables::is_plausible_variable(s)) {
        return None;
    }
    Some(entries)
}

fn matlab_matrix_shorthand(input: &str) -> Option<CompileResult> {
    if let Some(assignment) = split_equation(input) {
        if variables::is_plausible_variable(&assignment.lhs) {
            let offset = utf8_offset(&assignment.rhs, input);
            let lhs = assignment.lhs.clone();
            if let Some(res) = matlab_matrix_generated_sage(&assignment.rhs, offset, &|m| {
                format!("{lhs} = matrix({m})\n{lhs}")
            }) {
                return Some(res);
            }
        }
    }

    matlab_matrix_generated_sage(input, 0, &|m| format!("matrix({m})"))
}

fn matlab_matrix_generated_sage(
    payload: &str,
    base_offset: usize,
    wrap: &dyn Fn(&str) -> String,
) -> Option<CompileResult> {
    let body = scanner::trimmed_shim(payload);
    if !body.starts_with('[') || !body.contains(';') {
        return None;
    }

    if let Some(bal) = scanner::balance_error(body) {
        return Some(CompileResult::Error(balance_compile_error(
            &bal,
            base_offset,
        )));
    }

    match normalize_matrix_payload(body, true) {
        Some(normalized) => Some(CompileResult::Success {
            generated_sage: wrap(&normalized),
            required_variables: variables::free_variables(&normalized, &[]),
        }),
        None => Some(CompileResult::Error(CompileError::new(
            "MATLAB-style matrix literal is malformed.",
            None,
            Some("Try: [1,2; 3,4] or A = [1,2; 3,4]".to_string()),
        ))),
    }
}

fn simple_assignment_echo(input: &str) -> Option<CompileResult> {
    let assignment = split_equation(input)?;
    if !variables::is_plausible_variable(&assignment.lhs) {
        return None;
    }
    Some(CompileResult::Success {
        generated_sage: format!(
            "{} = {}\n{}",
            assignment.lhs, assignment.rhs, assignment.lhs
        ),
        required_variables: variables::free_variables(&assignment.rhs, &[]),
    })
}

fn normalize_matrix_payload(payload: &str, requires_top_level_semicolon: bool) -> Option<String> {
    let body = scanner::trimmed_shim(payload);
    if !body.starts_with('[') {
        return None;
    }
    let inside = single_outer_bracket_body(body)?;
    let inner = scanner::trimmed_shim(inside);

    if inner.starts_with('[') {
        return if requires_top_level_semicolon {
            None
        } else {
            Some(body.to_string())
        };
    }

    let rows = scanner::split_top_level_semicolons(inside);
    if requires_top_level_semicolon && rows.len() <= 1 {
        return None;
    }
    if rows.is_empty() || !rows.iter().all(|r| !r.is_empty()) {
        return None;
    }

    let mut sage_rows = Vec::new();
    for row in rows {
        let cells = scanner::split_top_level_commas(&row);
        if cells.is_empty() || !cells.iter().all(|c| !c.is_empty()) {
            return None;
        }
        sage_rows.push(format!("[{}]", cells.join(",")));
    }

    Some(format!("[{}]", sage_rows.join(",")))
}

fn single_outer_bracket_body(body: &str) -> Option<&str> {
    if !body.starts_with('[') || !body.ends_with(']') {
        return None;
    }
    let mut depth = 0;
    for (offset, ch) in body.char_indices() {
        if ch == '[' {
            depth += 1;
        } else if ch == ']' {
            depth -= 1;
            if depth == 0 && offset != body.len() - 1 {
                return None;
            }
        }
    }
    Some(&body[1..body.len() - 1])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    pub variable: String,
    pub lower: String,
    pub upper: String,
}

pub fn parse_range(clause: &str) -> Option<Range> {
    let c = scanner::trimmed_shim(clause);
    let eq_idx = c.find('=')?;
    let v = scanner::trimmed_shim(&c[..eq_idx]);
    let range_part = scanner::trimmed_shim(&c[eq_idx + 1..]);
    if !variables::is_plausible_variable(v) {
        return None;
    }
    let dot_range = range_part.find("..")?;
    let lower = scanner::trimmed_shim(&range_part[..dot_range]);
    let upper = scanner::trimmed_shim(&range_part[dot_range + 2..]);
    if lower.is_empty() || upper.is_empty() {
        return None;
    }
    Some(Range {
        variable: v.to_string(),
        lower: lower.to_string(),
        upper: upper.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialRange {
    pub variable: String,
    pub lower: Option<String>,
    pub upper: Option<String>,
}

pub fn parse_partial_range(clause: &str) -> Option<PartialRange> {
    let c = scanner::trimmed_shim(clause);
    let variable: String;
    let range_part: String;

    if let Some(eq_idx) = c.find('=') {
        let after = &c[eq_idx + 1..];
        if after.starts_with('=') {
            return None;
        }
        if eq_idx > 0 {
            let prev = c[..eq_idx].chars().last().unwrap();
            if prev == '<' || prev == '>' || prev == '!' {
                return None;
            }
        }
        variable = scanner::trimmed_shim(&c[..eq_idx]).to_string();
        range_part = scanner::trimmed_shim(after).to_string();
        if !variable.is_empty() && !variables::is_plausible_variable(&variable) {
            return None;
        }
    } else if c.contains("..") {
        variable = String::new();
        range_part = c.to_string();
    } else {
        return None;
    }

    let lower_text: String;
    let upper_text: String;
    if let Some(dot_range) = range_part.find("..") {
        lower_text = scanner::trimmed_shim(&range_part[..dot_range]).to_string();
        upper_text = scanner::trimmed_shim(&range_part[dot_range + 2..]).to_string();
    } else {
        lower_text = scanner::trimmed_shim(&range_part).to_string();
        upper_text = String::new();
    }

    Some(PartialRange {
        variable,
        lower: if lower_text.is_empty() {
            None
        } else {
            Some(lower_text)
        },
        upper: if upper_text.is_empty() {
            None
        } else {
            Some(upper_text)
        },
    })
}

fn range_error(clause: &str, example: &str) -> CompileError {
    let c = scanner::trimmed_shim(clause);
    if c.contains("..") && c.ends_with("..") {
        CompileError::new(
            format!("Range `{c}` is incomplete — missing the upper bound after `..`."),
            None,
            Some(format!("Complete it, e.g. `{example}`.")),
        )
    } else if !c.contains("..") {
        CompileError::new(
            format!("Range `{c}` is missing `..` between the bounds."),
            None,
            Some(format!("Use `var=lo..hi`, e.g. `{example}`.")),
        )
    } else {
        CompileError::new(
            format!("Range `{c}` is malformed."),
            None,
            Some(format!("Use `var=lo..hi`, e.g. `{example}`.")),
        )
    }
}

pub struct TrailingClause {
    pub clause_start: usize,
    pub value_range: std::ops::Range<usize>,
}

pub fn trailing_clause(s: &str, keyword: &str) -> Option<TrailingClause> {
    let lower = s.to_lowercase();
    let needle = format!(" {keyword} ");

    let idx = lower.rfind(&needle)?;
    let start_offset = idx;
    let end_offset = idx + needle.len();
    let value = scanner::trimmed_shim(&s[end_offset..]);
    if value.is_empty() {
        return None;
    }
    Some(TrailingClause {
        clause_start: start_offset,
        value_range: end_offset..s.len(),
    })
}

pub struct Equation {
    pub lhs: String,
    pub rhs: String,
}

pub fn split_equation(s: &str) -> Option<Equation> {
    let chars: Vec<char> = s.chars().collect();
    let mut depth = 0;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '=' if depth == 0 => {
                let prev = if i > 0 { chars[i - 1] } else { ' ' };
                let next = if i + 1 < chars.len() {
                    chars[i + 1]
                } else {
                    ' '
                };
                if next == '=' {
                    i += 2;
                    continue;
                }
                if prev == '=' || prev == '<' || prev == '>' || prev == '!' {
                    i += 1;
                    continue;
                }
                let lhs: String = chars[..i].iter().collect();
                let rhs: String = chars[i + 1..].iter().collect();
                let lhs_trimmed = scanner::trimmed_shim(&lhs);
                let rhs_trimmed = scanner::trimmed_shim(&rhs);
                if lhs_trimmed.is_empty() || rhs_trimmed.is_empty() {
                    return None;
                }
                return Some(Equation {
                    lhs: lhs_trimmed.to_string(),
                    rhs: rhs_trimmed.to_string(),
                });
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn balance_compile_error(bal: &BalanceError, base_offset: usize) -> CompileError {
    let pos = base_offset + bal.position;
    match bal.kind {
        BalanceErrorKind::Unclosed(ch) => {
            let close = match ch {
                '(' => ")",
                '[' => "]",
                _ => "}",
            };
            CompileError::new(
                format!("Unbalanced `{ch}` — it is never closed."),
                Some(pos),
                Some(format!("Add a matching `{close}`.")),
            )
        }
        BalanceErrorKind::UnexpectedClose(ch) => CompileError::new(
            format!("Unexpected `{ch}` — there is no matching opener."),
            Some(pos),
            Some("Remove it or add the missing opener.".to_string()),
        ),
        BalanceErrorKind::Mismatched { open, close } => {
            let want = match open {
                '(' => ")",
                '[' => "]",
                _ => "}",
            };
            CompileError::new(
                format!("Bracket mismatch — `{open}` is closed by `{close}`."),
                Some(pos),
                Some(format!("Close it with `{want}` instead.")),
            )
        }
    }
}

fn ordered_unique(items: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in items {
        if !item.is_empty() && !seen.contains(&item) {
            seen.insert(item.clone());
            out.push(item);
        }
    }
    out
}

fn utf8_offset(needle: &str, haystack: &str) -> usize {
    haystack.rfind(needle).unwrap_or(0)
}

fn dropping_integral_trailing_comma(s: &str) -> &str {
    let trimmed = scanner::trimmed_shim(s);
    if let Some(stripped) = trimmed.strip_suffix(',') {
        scanner::trimmed_shim(stripped)
    } else {
        trimmed
    }
}
