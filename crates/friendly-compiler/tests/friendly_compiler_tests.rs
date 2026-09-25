use friendly_compiler::{compile, CompileError, CompileResult};

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

fn expect_bypass(input: &str) {
    let r = compile(input);
    match r {
        CompileResult::Bypass { ref raw_sage } => {
            assert_eq!(raw_sage, input.trim(), "input: {input}");
        }
        _ => panic!("expected Bypass for '{input}', got {:?}", r),
    }
}

fn expect_error(input: &str) -> CompileError {
    let r = compile(input);
    match r {
        CompileResult::Error(e) => e,
        _ => panic!("expected Error for '{input}', got {:?}", r),
    }
}

fn expect_ambiguous(input: &str) -> Vec<String> {
    let r = compile(input);
    match r {
        CompileResult::Ambiguous { candidates } => candidates,
        _ => panic!("expected Ambiguous for '{input}', got {:?}", r),
    }
}

// MARK: - Spec forms (exact reference translations)

#[test]
fn test_spec_forms() {
    expect_success("factor x^4 - 1", "factor(x^4 - 1)");
    expect_success("expand (x + 1)^5", "expand((x + 1)^5)");
    expect_success(
        "simplify sin(x)^2 + cos(x)^2",
        "(sin(x)^2 + cos(x)^2).simplify_full()",
    );
    expect_success("solve x^2 + 5*x + 6 = 0", "solve(x^2 + 5*x + 6 == 0, x)");
    expect_success(
        "solve x^2 + 5*x + 6 = 0 for x",
        "solve(x^2 + 5*x + 6 == 0, x)",
    );
    expect_success("derivative sin(x^2)", "derivative(sin(x^2), x)");
    expect_success("derivative sin(x^2) wrt x", "derivative(sin(x^2), x)");
    expect_success("integral x^2", "integrate(x^2, x)");
    expect_success("integral x*y, wrt y", "integrate(x*y, y)");
    expect_success("integral x^2, x=0..1", "integrate(x^2, (x, 0, 1))");
    expect_success(
        "double integral x*y, x=0..1, y=0..x",
        "integrate(integrate(x*y, (y, 0, x)), (x, 0, 1))",
    );
    expect_success("limit sin(x)/x, x->0", "limit(sin(x)/x, x=0)");
    expect_success("taylor sin(x), x=0, order=7", "taylor(sin(x), x, 0, 7)");
    expect_success("plot sin(x), x=-pi..pi", "plot(sin(x), (x, -pi, pi))");
    expect_success("matrix [[1,2],[3,4]]", "matrix([[1,2],[3,4]])");
    expect_success("matrix [1, 2, 3 ; 2, 3, 4 ]", "matrix([[1,2,3],[2,3,4]])");
    expect_success("[1, 2, 3 ; 2, 3, 4 ]", "matrix([[1,2,3],[2,3,4]])");
    expect_success(
        "A = [1, 2, 3 ; 2, 3, 4 ]",
        "A = matrix([[1,2,3],[2,3,4]])\nA",
    );
    expect_success(
        "eigenvalues [[1,2],[3,4]]",
        "matrix([[1,2],[3,4]]).eigenvalues()",
    );
    expect_success(
        "eigenvalues [1,2;3,4]",
        "matrix([[1,2],[3,4]]).eigenvalues()",
    );
    expect_success("rref [[1,2],[3,4]]", "matrix([[1,2],[3,4]]).rref()");
    expect_success("rref [1,2,3;4,5,6]", "matrix([[1,2,3],[4,5,6]]).rref()");
}

// MARK: - Double integral nesting

#[test]
fn test_double_integral_nesting() {
    expect_success(
        "double integral x*y, x=0..1, y=0..x",
        "integrate(integrate(x*y, (y, 0, x)), (x, 0, 1))",
    );
    expect_success(
        "double integral 1, x=0..2, y=0..3",
        "integrate(integrate(1, (y, 0, 3)), (x, 0, 2))",
    );

    let r = compile("double integral x*y, x=0..1, y=0..x");
    if let CompileResult::Success {
        required_variables, ..
    } = r
    {
        assert_eq!(required_variables, vec!["x", "y"]);
    } else {
        panic!("expected success");
    }

    expect_error("double integral x*y, x=0..1");
}

// MARK: - Bypass

#[test]
fn test_bypass() {
    expect_bypass("2+2");
    expect_bypass("factor(x^4-1)");
    expect_success("A = matrix([[1,2],[3,4]])", "A = matrix([[1,2],[3,4]])\nA");
    expect_success("A = [1, 2, 3]", "A = [1, 2, 3]\nA");
    expect_bypass("sin(pi/3)");
    expect_bypass("factorial(5)");
    expect_bypass("x.diff()");
    expect_bypass("foobar x^2");

    if let CompileResult::Bypass { raw_sage } = compile("") {
        assert_eq!(raw_sage, "");
    } else {
        panic!("empty should bypass");
    }

    expect_bypass("104729");
    expect_bypass("plotting_helper(3)");
}

// MARK: - Errors

#[test]
fn test_errors() {
    let e = expect_error("integral x^2, x=0..");
    assert!(e.message.contains("incomplete"));
    assert!(e.suggestion.is_some());

    let e = expect_error("plot sin(x), x=0");
    assert!(e.message.contains(".."));

    let e = expect_error("solve x^2 + 1");
    assert!(e.message.contains("equation"));

    let e = expect_error("factor");
    assert!(e.message.contains("expression"));

    let e = expect_error("plot sin(x)");
    assert!(e.suggestion.as_ref().unwrap().contains("plot"));

    expect_error("matrix (1,2)");
    expect_error("matrix [1,2; ]");
    expect_error("matrix [1,,2; 3,4,5]");
    expect_error("A = [1,,2; 3,4,5]");

    let e = expect_error("expand (x+1");
    assert!(e.message.contains("never closed"));
    assert!(e.position.is_some());

    let e = expect_error("matrix [[1,2],[3,4]");
    assert!(e.message.contains("never closed"));

    let e = expect_error("factor (x^4 - 1]");
    assert!(e.message.contains("mismatch"));

    let e = expect_error("factor x^4 - 1)");
    assert!(e.message.contains("no matching opener"));

    expect_error("taylor sin(x), x=0");

    let e = expect_error("taylor sin(x), x=0, order=abc");
    assert!(e.message.contains("whole number"));

    expect_error("limit sin(x)/x, x=0");
    expect_error("derivative 5");

    let e = expect_error("integral x^2, x=0..1, x=2..3");
    assert!(e.message.contains("at most one range"));
}

// MARK: - Ambiguity

#[test]
fn test_ambiguity() {
    let c = expect_ambiguous("solve x*y = 1");
    assert_eq!(c, vec!["solve(x*y == 1, x)", "solve(x*y == 1, y)"]);

    let c = expect_ambiguous("derivative x*y");
    assert_eq!(c.len(), 2);

    let c = expect_ambiguous("integral x*y");
    assert_eq!(c.len(), 2);

    expect_success("solve x*y = 1 for x", "solve(x*y == 1, x)");
    expect_success("derivative x*y wrt y", "derivative(x*y, y)");
}

// MARK: - Spacing & Casing

#[test]
fn test_spacing_and_casing() {
    expect_success("factor    x^4 - 1", "factor(x^4 - 1)");
    expect_success("   factor x^4 - 1   ", "factor(x^4 - 1)");
    expect_success("factor\tx^4 - 1", "factor(x^4 - 1)");
    expect_success("integral x^2 , x = 0 .. 1", "integrate(x^2, (x, 0, 1))");
    expect_success("limit sin(x)/x , x -> 0", "limit(sin(x)/x, x=0)");
    expect_success("FACTOR x^4 - 1", "factor(x^4 - 1)");
    expect_success("Expand (x + 1)^5", "expand((x + 1)^5)");
    expect_success(
        "double integral  x*y ,  x=0..1 ,  y=0..x",
        "integrate(integrate(x*y, (y, 0, x)), (x, 0, 1))",
    );
    expect_success("factor x^4 - 1\n", "factor(x^4 - 1)");
}

// MARK: - Synonyms

#[test]
fn test_synonyms() {
    expect_success("diff sin(x^2) wrt x", "derivative(sin(x^2), x)");
    expect_success("integrate x^2, x=0..1", "integrate(x^2, (x, 0, 1))");
    expect_success(
        "double integrate x*y, x=0..1, y=0..x",
        "integrate(integrate(x*y, (y, 0, x)), (x, 0, 1))",
    );
}

// MARK: - Required variables

#[test]
fn test_required_variables() {
    if let CompileResult::Success {
        required_variables, ..
    } = compile("integral x^2, x=0..1")
    {
        assert_eq!(required_variables, vec!["x"]);
    } else {
        panic!("expected success");
    }

    if let CompileResult::Success {
        required_variables, ..
    } = compile("plot sin(x), x=-pi..pi")
    {
        assert_eq!(required_variables, vec!["x"]);
    } else {
        panic!("expected success");
    }

    if let CompileResult::Success {
        required_variables, ..
    } = compile("matrix [[1,2],[3,4]]")
    {
        assert!(required_variables.is_empty());
    } else {
        panic!("expected success");
    }

    if let CompileResult::Success {
        required_variables, ..
    } = compile("factor pi*x")
    {
        assert_eq!(required_variables, vec!["x"]);
    } else {
        panic!("expected success");
    }

    if let CompileResult::Success {
        required_variables, ..
    } = compile("factor sin(x) + cos(y)")
    {
        assert_eq!(required_variables, vec!["x", "y"]);
    } else {
        panic!("expected success");
    }
}

// MARK: - No implicit multiplication

#[test]
fn test_no_implicit_multiplication() {
    expect_success("factor 2*x", "factor(2*x)");
    expect_success("factor 2x", "factor(2x)");
}

// MARK: - Tape reference compilation

#[test]
fn test_tape_references_compilation() {
    use friendly_compiler::{compile_with_refs, TapeReferenceTable};

    let mut refs = TapeReferenceTable::empty();
    refs.insert(1, "1/(1 - x^2)".into());
    refs.insert(57, "x^4 - 1".into());

    // 1. Plot referencing #1
    let r = compile_with_refs("plot #1, x=-10..10", &refs);
    if let CompileResult::Success {
        generated_sage,
        required_variables,
    } = r
    {
        assert_eq!(generated_sage, "plot(__casette_tape_refs[1], (x, -10, 10))");
        // __casette_tape_refs must NOT be treated as a free variable needing declaration
        assert_eq!(required_variables, vec!["x"]);
    } else {
        panic!("expected success for plot #1, got {:?}", r);
    }

    // 2. Factor referencing #57
    let r2 = compile_with_refs("expand #57", &refs);
    if let CompileResult::Success {
        generated_sage,
        required_variables,
    } = r2
    {
        assert_eq!(generated_sage, "expand(__casette_tape_refs[57])");
        assert!(required_variables.is_empty());
    } else {
        panic!("expected success for expand #57, got {:?}", r2);
    }

    // 3. Missing reference yields compile error
    let r3 = compile_with_refs("plot #2, x=-10..10", &refs);
    assert!(matches!(r3, CompileResult::Error(_)));
}

#[test]
fn test_solve_with_units() {
    let res = compile("solve 32*mi*x^2 = 52*mi^2");
    if let CompileResult::Success {
        generated_sage,
        required_variables,
    } = res
    {
        assert_eq!(generated_sage, "solve(32*mi*x^2 == 52*mi^2, x)");
        assert_eq!(required_variables, vec!["x"]);
    } else {
        panic!("expected success for solve with units, got: {:?}", res);
    }

    // Natural juxtaposition without *
    let res_jux = compile("solve 32 mi * x^2 = 52 mi^2");
    if let CompileResult::Success {
        generated_sage,
        required_variables,
    } = res_jux
    {
        assert_eq!(generated_sage, "solve((32*mi) * x^2 == (52*mi^2), x)");
        assert_eq!(required_variables, vec!["x"]);
    } else {
        panic!("expected success for juxtaposition solve, got: {:?}", res_jux);
    }

    // Variable juxtaposed with unit (x sec)
    let res_time = compile("solve 3 hr + x sec = 12 days");
    if let CompileResult::Success {
        generated_sage,
        required_variables,
    } = res_time
    {
        assert_eq!(generated_sage, "solve((3*hr) + (x*sec) == (12*days), x)");
        assert_eq!(required_variables, vec!["x"]);
    } else {
        panic!("expected success for time equation with x sec, got: {:?}", res_time);
    }
}

#[test]
fn test_unit_conversions() {
    let res1 = compile("32 mi + 5 km to miles");
    if let CompileResult::Success {
        generated_sage,
        required_variables,
    } = res1
    {
        assert_eq!(generated_sage, "__casette_convert((32*mi) + (5*km), miles)");
        assert!(required_variables.is_empty());
    } else {
        panic!("expected success for conversion to miles, got: {:?}", res1);
    }

    let res2 = compile("100 km / 2 hr in mph");
    if let CompileResult::Success {
        generated_sage,
        required_variables,
    } = res2
    {
        assert_eq!(generated_sage, "__casette_convert((100*km) / (2*hr), mph)");
        assert!(required_variables.is_empty());
    } else {
        panic!("expected success for speed conversion, got: {:?}", res2);
    }
}

#[test]
fn test_bare_unit_expressions() {
    let res = compile("50*km / (32*mi)");
    if let CompileResult::Success {
        generated_sage,
        required_variables,
    } = res
    {
        assert_eq!(generated_sage, "__casette_convert(50*km / (32*mi))");
        assert!(required_variables.is_empty());
    } else {
        panic!("expected success for bare unit ratio, got: {:?}", res);
    }
}
