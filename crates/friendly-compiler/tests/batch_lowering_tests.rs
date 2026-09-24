use friendly_compiler::{compile, CompileResult};

fn expect_success(input: &str, expected_sage: &str) {
    let r = compile(input);
    match r {
        CompileResult::Success {
            ref generated_sage, ..
        } => {
            assert_eq!(generated_sage, expected_sage, "input: {input}");
        }
        _ => panic!("expected Success for '{input}', got {:?}", r),
    }
}

fn expect_error(input: &str) {
    let r = compile(input);
    match r {
        CompileResult::Error(_) => {}
        _ => panic!("expected Error for '{input}', got {:?}", r),
    }
}

fn required_vars(input: &str) -> Vec<String> {
    let r = compile(input);
    match r {
        CompileResult::Success {
            required_variables, ..
        } => required_variables,
        _ => panic!("expected Success for '{input}', got {:?}", r),
    }
}

// MARK: - Batch B: Sum, Product, Derivative Order, Limit Direction

#[test]
fn test_batch_b_lowering() {
    expect_success("sum k^2, k=1..n", "sum(k^2, k, 1, n)");
    expect_success("product 1 + 1/k, k=1..n", "product(1 + 1/k, k, 1, n)");
    expect_success("sum k^2, k=1..10", "sum(k^2, k, 1, 10)");
    assert_eq!(required_vars("sum k^2, k=1..n"), vec!["k", "n"]);
    expect_error("sum k^2");
    expect_error("sum");
    expect_error("sum k^2, k=1..");
    expect_error("sum k^2, k=1..n, j=1..m");
    expect_success("sum #3 * k, k=1..10", "sum(#3 * k, k, 1, 10)");

    expect_success("derivative sin(x), 2", "derivative(sin(x), x, 2)");
    expect_success("derivative x*y, 2 wrt y", "derivative(x*y, y, 2)");
    expect_success("derivative sin(x^2)", "derivative(sin(x^2), x)");
    expect_success("derivative #14, 2 wrt x", "derivative(#14, x, 2)");

    expect_success(
        "limit sin(x)/x, x->0, left",
        "limit(sin(x)/x, x=0, dir='-')",
    );
    expect_success(
        "limit sin(x)/x, x->0, right",
        "limit(sin(x)/x, x=0, dir='+')",
    );
    expect_success(
        "limit sin(x)/x, x->0, RIGHT",
        "limit(sin(x)/x, x=0, dir='+')",
    );
    expect_success("limit sin(x)/x, x->0", "limit(sin(x)/x, x=0)");
    expect_error("limit sin(x)/x, x->0, sideways");
}

// MARK: - Batch C: Implicit Plot, Parametric Plot

#[test]
fn test_batch_c_lowering() {
    expect_success(
        "implicit_plot x^2 + y^2 = 1, x=-2..2, y=-2..2",
        "implicit_plot(x^2 + y^2 == 1, (x, -2, 2), (y, -2, 2))",
    );
    expect_success(
        "implicit_plot x^2 + y^2 == 1, x=-2..2, y=-2..2",
        "implicit_plot(x^2 + y^2 == 1, (x, -2, 2), (y, -2, 2))",
    );
    expect_success(
        "implicit_plot x^2 + y^2 - 1, x=-2..2, y=-2..2",
        "implicit_plot(x^2 + y^2 - 1 == 0, (x, -2, 2), (y, -2, 2))",
    );
    expect_success(
        "implicit plot x^2 + y^2 = 1, x=-2..2, y=-2..2",
        "implicit_plot(x^2 + y^2 == 1, (x, -2, 2), (y, -2, 2))",
    );
    expect_error("implicit_plot x^2 + y^2 = 1, x=-2..2");
    expect_error("implicit_plot x^2 + y^2 = 1, x=-2, y=-2..2");
    expect_error("implicit_plot");

    assert_eq!(
        required_vars("implicit_plot x^2 + y^2 = 1, x=-2..2, y=-2..2"),
        vec!["x", "y"]
    );
    assert_eq!(
        required_vars("implicit_plot x^2 + y^2 = r, x=-r..r, y=-r..r"),
        vec!["x", "y", "r"]
    );

    expect_success(
        "parametric_plot (cos(t), sin(t)), t=0..2*pi",
        "parametric_plot((cos(t), sin(t)), (t, 0, 2*pi))",
    );
    expect_success(
        "parametric plot (cos(t), sin(t)), t=0..2*pi",
        "parametric_plot((cos(t), sin(t)), (t, 0, 2*pi))",
    );
    expect_error("parametric_plot cos(t), sin(t), t=0..2*pi");
    expect_error("parametric_plot (cos(t), sin(t))");
    assert_eq!(
        required_vars("parametric_plot (cos(t), sin(t)), t=0..2*pi"),
        vec!["t"]
    );
}

// MARK: - Batch D: Linear Algebra (vector, det, inverse, transpose, rank, eigenvectors)

#[test]
fn test_batch_d_lowering() {
    expect_success("det [1,2; 3,4]", "matrix([[1,2],[3,4]]).det()");
    expect_success("det [[1,2],[3,4]]", "matrix([[1,2],[3,4]]).det()");
    expect_success("det A", "(A).det()");
    expect_success("det A*B", "(A*B).det()");

    expect_success("inverse [1,2; 3,4]", "matrix([[1,2],[3,4]]).inverse()");
    expect_success("transpose [1,2; 3,4]", "matrix([[1,2],[3,4]]).transpose()");
    expect_success("rank [1,2; 3,4]", "matrix([[1,2],[3,4]]).rank()");
    expect_success(
        "eigenvectors [1,2; 3,4]",
        "matrix([[1,2],[3,4]]).eigenvectors_right()",
    );

    expect_success("vector [1, 2, 3]", "vector([1, 2, 3])");
    expect_success("vector [x, y, z]", "vector([x, y, z])");
    expect_error("vector 1, 2, 3");
}

// MARK: - Batch E: Calculus & Operations (gradient, hessian, jacobian, subs, numeric, latex)

#[test]
fn test_batch_e_lowering() {
    expect_success("gradient x^2 + y^2", "(x^2 + y^2).gradient()");
    expect_success("gradient x^2 + y^2, [x, y]", "(x^2 + y^2).gradient([x, y])");
    expect_success("grad x^2 + y^2", "(x^2 + y^2).gradient()");
    expect_error("gradient x^2 + y^2, x, y");

    expect_success("hessian x^2 + y^2", "(x^2 + y^2).hessian()");
    expect_error("hessian x^2 + y^2, [x, y]");

    expect_success(
        "jacobian [x^2+y, sin(x*y)], [x, y]",
        "jacobian([x^2+y, sin(x*y)], [x, y])",
    );
    expect_error("jacobian [x^2+y, sin(x*y)]");

    expect_success("subs x^2 + y, x=3", "(x^2 + y).subs(x=3)");
    expect_success("subs x^2 + y, x=3, y=4", "(x^2 + y).subs(x=3, y=4)");
    expect_error("subs x^2 + y");
    expect_error("subs x^2 + y, x");

    expect_success("numeric pi", "N(pi)");
    expect_success("numeric pi, 50", "N(pi, digits=50)");
    expect_success("approx pi", "N(pi)");
    expect_success("decimal pi", "N(pi)");
    expect_error("numeric pi, 50, 60");
    expect_error("numeric pi, abc");

    expect_success("latex sin(x)", "latex(sin(x))");
}

// MARK: - Batch F: Assumptions & Number Theory (var, assume, forget, choose, gcd, lcm, factorial, is_prime, factor_integer, mean)

#[test]
fn test_batch_f_lowering() {
    expect_success("var a b c", "var('a b c')");
    expect_success("var a, b, c", "var('a b c')");
    expect_error("var");
    expect_error("var 1 2 3");

    expect_success("assume x > 0", "assume(x > 0)");
    expect_success("assume x >= 0", "assume(x >= 0)");
    expect_success("assume x == 0", "assume(x == 0)");
    expect_success("assume x != 0", "assume(x != 0)");
    expect_success("assume x real", "assume(x, 'real')");
    expect_success("assume x is real", "assume(x, 'real')");
    expect_success("assume x positive", "assume(x > 0)");
    expect_success("assume x negative", "assume(x < 0)");
    expect_error("assume x = 0");
    expect_error("assume x flibbertigibbet");

    expect_success("forget", "forget()");
    expect_success("forget all", "forget()");
    expect_success("forget assumptions", "forget()");
    expect_error("forget x");

    expect_success("choose 10, 3", "binomial(10, 3)");
    expect_error("choose 10");

    expect_success("gcd 12, 18", "gcd(12, 18)");
    expect_success("gcd [12, 18, 24]", "gcd([12, 18, 24])");
    expect_success("gcd 12, 18, 24", "gcd([12, 18, 24])");

    expect_success("lcm 12, 18", "lcm(12, 18)");
    expect_success("factorial 5", "factorial(5)");
    expect_success("is_prime 104729", "is_prime(104729)");
    expect_success("factor_integer 3600", "factor(3600)");
    expect_success("prime_factorization 3600", "factor(3600)");

    expect_success("mean [1, 2, 3]", "sum([1, 2, 3])/len([1, 2, 3])");
    expect_error("mean 1, 2, 3");
}

// MARK: - Batch G: Statistics Distributions

#[test]
fn test_batch_g_lowering() {
    expect_success("normal_pdf 0, mean=0, sd=1", "normal_pdf(0, mean=0, sd=1)");
    expect_success("normal_pdf 0", "normal_pdf(0)");
    expect_success("normal_cdf 1.96", "normal_cdf(1.96)");
    expect_success("normal_between -1, 1", "normal_between(-1, 1)");
    expect_success("normal_inv .975", "normal_inv(.975)");

    expect_success("binomial_pmf 3, n=10, p=.5", "binomial_pmf(3, n=10, p=.5)");
    expect_success("binomial_cdf 3, n=10, p=.5", "binomial_cdf(3, n=10, p=.5)");
    expect_success(
        "binomial_between 3, 7, n=10, p=.5",
        "binomial_between(3, 7, n=10, p=.5)",
    );
    expect_success(
        "binomial_at_most 3, n=10, p=.5",
        "binomial_at_most(3, n=10, p=.5)",
    );
    expect_success(
        "binomial_at_least 8, n=10, p=.5",
        "binomial_at_least(8, n=10, p=.5)",
    );
    expect_error("binomial_pmf 3, n=10"); // missing required p

    expect_success("poisson_pmf 2, lambda=3", "poisson_pmf(2, lambda_=3)");
    expect_success("exponential_pdf 1, rate=2", "exponential_pdf(1, rate=2)");
    expect_success(
        "uniform_pdf .5, min=0, max=1",
        "uniform_pdf(.5, low=0, high=1)",
    );
}
