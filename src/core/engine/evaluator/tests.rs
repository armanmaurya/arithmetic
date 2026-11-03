//! End-to-end checks: input text → the text the screen would show.

use crate::core::engine::{CalcError, Context, EvalError, evaluate_expression};

/// Evaluates each line in turn in one context (so `ans`, variables and
/// memory carry over), with the given settings.
fn run(settings: &[(&str, &str)], lines: &[(&str, &str)]) {
    let mut ctx = Context::with_seed(42);
    for (key, value) in settings {
        ctx.settings.apply(key, value).unwrap();
    }
    for (input, expected) in lines {
        let shown = match evaluate_expression(input, &mut ctx) {
            Ok(v) => ctx.format(&v),
            Err(e) => format!("error: {e}"),
        };
        assert_eq!(shown, *expected, "input: {input:?}, settings: {settings:?}");
    }
}

fn check(lines: &[(&str, &str)]) {
    run(&[], lines);
}

fn eval_err(input: &str) -> EvalError {
    match evaluate_expression(input, &mut Context::with_seed(1)) {
        Err(CalcError::Eval(e)) => e,
        other => panic!("{input:?}: expected an evaluation error, got {other:?}"),
    }
}

#[test]
fn arithmetic() {
    check(&[
        ("7+8", "15"),
        ("12×4", "48"),
        ("100÷4", "25"),
        ("1+2×3", "7"),
        ("8−3−2", "3"),
        ("2^3^2", "512"),
        ("-2^2", "-4"),
        ("2^-1", "0.5"),
        ("5!", "120"),
        ("0!", "1"),
        ("50%", "0.5"),
        ("2π", "6.283185307"),
        ("2(3+4)", "14"),
        ("√16", "4"),
        ("ln(e)", "1"),
        ("log(1000)", "3"),
        ("0.1+0.2−0.3", "0"),
        ("1÷3", "0.3333333333"),
        ("2^0.5", "1.414213562"),
        ("(−8)^(1÷3)", "-2"),
        ("(−8)^(2÷3)", "4"),
        ("1.5E3+1", "1501"),
        ("0x1F+1", "32"),
    ]);
}

#[test]
fn trig_and_angles() {
    check(&[
        ("sin(0)", "0"),
        ("sin(90)", "1"),
        ("cos(90)", "0"),
        ("cos(450)", "0"),
        ("cos(90)×10^20", "0"),
        ("tan(45)", "1"),
        ("sin(30)", "0.5"),
        ("asin(1)", "90"),
        ("sin⁻¹(0.5)", "30"),
        ("sin 30", "0.5"),
        ("sin −30", "-0.5"),
        ("log 10^3", "3"),
        ("sin 60+cos 30", "1.732050808"),
        ("sinh(0)", "0"),
        ("acosh(1)", "0"),
        ("pol(3, 4)", "{5, 53.13010235}"),
        ("rec(2, 90)", "{0, 2}"),
    ]);
    run(
        &[("angle", "rad")],
        &[
            ("sin(π÷2)", "1"),
            ("sin(π)", "0"),
            ("cos(3π÷2)", "0"),
            ("tan(π)", "0"),
            ("tan(π÷2)", "error: Domain error"),
            ("sin(1E300)", "error: Domain error"),
            ("sin(1E14)", "-0.2094083075"),
            ("sin(1E-20)", "1E-20"),
            ("solve(cos(X) = X, 0)", "0.7390851332"),
            ("cos(π)", "-1"),
            ("asin(1)", "1.570796327"),
            ("sin(90°)", "1"),
            ("30°", "0.5235987756"),
        ],
    );
    run(
        &[("angle", "grad")],
        &[("sin(100)", "1"), ("cos(200)", "-1"), ("asin(1)", "100")],
    );
    run(
        &[("dms", "on")],
        &[("12.5", "12°30′0″"), ("10°30′+1°45′", "12°15′0″")],
    );
}

#[test]
fn variables_ans_and_statements() {
    check(&[
        ("5→A", "5"),
        ("A×2", "10"),
        ("ans+1", "11"),
        ("B", "0"),
        ("1→B : B+1→B : B×10", "20"),
        ("B", "2"),
        ("2A", "10"),
        // a statement starting with an operator carries on from Ans
        ("2+3 : ×4", "20"),
        ("×2", "40"),
        ("+1", "41"),
        ("−1", "-1"),
        ("6 : nCr2", "15"),
    ]);
}

#[test]
fn functions() {
    check(&[
        ("nCr(5, 2)", "10"),
        ("nPr(5, 2)", "20"),
        ("7nPr4", "840"),
        ("10nCr4", "210"),
        ("7ˣ√123", "1.988647795"),
        ("3ˣ√-27", "-3"),
        ("gcd(12, 18, 24)", "6"),
        ("lcm(4, 6)", "12"),
        ("mod(-7, 3)", "2"),
        ("abs(-3)", "3"),
        ("round(2.567, 2)", "2.57"),
        ("floor(-2.5)", "-3"),
        ("int(-2.5)", "-2"),
        ("frac(2.75)", "0.75"),
        ("log(8, 2)", "3"),
        ("root(27, 3)", "3"),
        ("root(-32, 5)", "-2"),
        ("cbrt(-8)", "-2"),
        ("gamma(5)", "24"),
        ("isprime(97)", "1"),
        ("exp(0)", "1"),
        ("hypot(3, 4)", "5"),
        // bitwise on 64-bit words by default
        ("and(12, 10)", "8"),
        ("or(12, 10)", "14"),
        ("xor(12, 10)", "6"),
        ("not(0)", "-1"),
        ("shl(1, 4)", "16"),
    ]);
    run(
        &[("word-bits", "8")],
        &[
            ("not(0)", "-1"),
            ("shr(-1, 4)", "15"),
            ("and(255, 15)", "15"),
        ],
    );
    run(
        &[("base", "hex")],
        &[("0xFF + 1", "0x100"), ("and(0xF0, 0x3C)", "0x30")],
    );
}

#[test]
fn lists_and_statistics() {
    check(&[
        ("{1, 2, 3} + 1", "{2, 3, 4}"),
        ("{1, 2} × {3, 4}", "{3, 8}"),
        ("sin({0, 90})", "{0, 1}"),
        ("sum({1, 2, 3, 4})", "10"),
        ("mean(1, 2, 3, 4)", "2.5"),
        ("median({5, 1, 3})", "3"),
        ("sd(2, 4, 4, 4, 5, 5, 7, 9)", "2.138089935"),
        ("sdp(2, 4, 4, 4, 5, 5, 7, 9)", "2"),
        ("min({4, -1, 7})", "-1"),
        ("max([1, 2; 3, 4])", "4"),
        ("count({})", "0"),
        ("sort({3, 1, 2})", "{1, 2, 3}"),
        ("linreg({1, 2, 3}, {3, 5, 7})", "{1, 2}"),
        ("quadreg({0, 1, 2, 3}, {1, 2, 5, 10})", "{1, 0, 1}"),
        ("corr({1, 2, 3}, {2, 4, 6})", "1"),
        ("expreg({0, 1, 2}, {1, e, e^2})", "{1, 1}"),
        ("normcdf(-1, 1)", "0.6826894921"),
        ("invnorm(0.975)", "1.959963985"),
        ("normpdf(0)", "0.3989422804"),
        ("binompdf(10, 0.5, 5)", "0.24609375"),
        ("binomcdf(10, 0.5, 10)", "1"),
        ("poissonpdf(2, 0)", "0.1353352832"),
    ]);
}

#[test]
fn matrices_and_vectors() {
    check(&[
        ("[1, 2; 3, 4] + [1, 1; 1, 1]", "[2, 3; 4, 5]"),
        ("[1, 2; 3, 4] × [5; 6]", "[17; 39]"),
        ("2[1, 2]", "[2, 4]"),
        ("det([1, 2; 3, 4])", "-2"),
        ("inv([2, 0; 0, 4])", "[0.5, 0; 0, 0.25]"),
        ("trn([1, 2; 3, 4])", "[1, 3; 2, 4]"),
        ("[1, 1; 0, 1]^3", "[1, 3; 0, 1]"),
        ("iden(2)", "[1, 0; 0, 1]"),
        ("trace([1, 2; 3, 4])", "5"),
        ("linsolve([2, 1; 1, 3], [3; 5])", "{0.8, 1.4}"),
        ("dot([1, 2, 3], [4, 5, 6])", "32"),
        ("cross([1, 0, 0], [0, 1, 0])", "[0, 0, 1]"),
        ("norm([3, 4])", "5"),
        ("angle([1, 0], [0, 1])", "90"),
        ("unit([3, 4])", "[0.6, 0.8]"),
    ]);
}

#[test]
fn equations_and_calculus() {
    check(&[
        ("roots(1, -3, 2)", "{1, 2}"),
        ("roots(1, 0, -1, 0)", "{-1, 0, 1}"),
        ("roots(2, -4)", "{2}"),
        ("solve(X^2 = 2, 1)", "1.414213562"),
        ("X", "1.414213562"),
        ("solve(2X − 6)", "3"),
        ("solve(X^3 = X + 1, 1)", "1.324717957"),
        ("integrate(X^2, 0, 3)", "9"),
        ("∫(1÷X, 1, e)", "1"),
        ("derivative(X^3, 2)", "12"),
        ("d/dx(sin(X), 0)", "0.01745329252"),
        ("sigma(X, 1, 100)", "5050"),
        ("Σ(X^2, 1, 3)", "14"),
        ("product(X, 1, 5)", "120"),
        ("table(X^2, 1, 3)", "{{1, 1}, {2, 4}, {3, 9}}"),
    ]);
    // X is restored after calculus, but set by solve.
    check(&[("7→X", "7"), ("integrate(X, 0, 1)", "0.5"), ("X", "7")]);
}

#[test]
fn complex_numbers() {
    check(&[
        ("(3+4i)×(1−2i)", "11-2i"),
        ("abs(3+4i)", "5"),
        ("i^2", "-1"),
        ("re(3+4i)", "3"),
        ("im(3+4i)", "4"),
        ("conj(3+4i)", "3-4i"),
        ("arg(i)", "90"),
        ("√(−4)", "error: Keep it real"),
        ("roots(1, 0, 1)", "error: No real roots"),
    ]);
    run(
        &[("complex", "on")],
        &[
            ("√(−4)", "2i"),
            ("ln(−1)", "3.141592654i"),
            ("roots(1, 0, 1)", "{i, -i}"),
            (
                "roots(1, 0, 0, -1)",
                "{1, -0.5+0.8660254038i, -0.5-0.8660254038i}",
            ),
            ("(−8)^(1÷3)", "1+1.732050808i"),
        ],
    );
}

#[test]
fn units_and_constants() {
    check(&[
        ("convert(1, _in, _cm)", "2.54"),
        ("convert(100, _degC, _degF)", "212"),
        ("convert({1, 2}, _km, _m)", "{1000, 2000}"),
        ("convert(1, _m, _s)", "error: Unit error"),
        ("_km", "error: Unit error"),
        ("_c", "299792458"),
        ("_h×_c", "1.986445857E-25"),
    ]);
}

#[test]
fn settings_change_behaviour() {
    run(&[("implicit-mul-first", "on")], &[("6÷2(1+2)", "1")]);
    check(&[("6÷2(1+2)", "9")]);
    run(
        &[("percent", "of-left")],
        &[("200+10%", "220"), ("200−10%", "180"), ("50%", "0.5")],
    );
    check(&[("200+10%", "200.1")]);
    run(&[("notation", "fix:2")], &[("1÷3", "0.33")]);
    run(&[("notation", "sci:3")], &[("1234", "1.23E3")]);
    run(&[("notation", "eng")], &[("12345", "12.345E3")]);
    run(&[("fractions", "on")], &[("1÷4+1÷2", "3/4")]);
    run(&[("digits", "4")], &[("1÷3", "0.3333")]);
}

#[test]
fn bases_words_angles_and_polar() {
    run(
        &[("input-base", "hex"), ("base", "hex")],
        &[
            ("1F+1", "0x20"),
            ("FF×A", "0x9F6"),
            ("0b11", "0x3"),
            ("Ans", "0x3"),
        ],
    );
    run(
        &[("input-base", "bin")],
        &[("101+1", "6"), ("12", "error: Format error")],
    );
    run(
        &[
            ("input-base", "hex"),
            ("base", "hex"),
            ("word-mode", "on"),
            ("word-bits", "32"),
        ],
        &[
            ("0-1", "0xFFFFFFFF"),
            ("FFFFFFFF+2", "0x1"),
            ("not(0)", "0xFFFFFFFF"),
        ],
    );
    run(
        &[("word-mode", "on"), ("word-bits", "8")],
        &[("0xFF", "-1"), ("0x100", "error: Format error")],
    );
    check(&[("πʳ", "180"), ("200ᵍ", "180"), ("90°", "90")]);
    run(&[("angle", "rad")], &[("180°", "3.141592654"), ("2ʳ", "2")]);
    check(&[("2∠90", "2i"), ("(1∠0)+(1∠90)", "1+i"), ("abs(3∠45)", "3")]);
    run(
        &[("complex-form", "polar")],
        &[("3+4i", "5∠53.13010235"), ("i", "1∠90"), ("2∠30", "2∠30")],
    );
    run(
        &[("word-mode", "on"), ("word-bits", "32")],
        &[("d10+h10+b10+o10", "36"), ("hFF", "255")],
    );
    check(&[
        ("{1, rep(2, 3)}", "{1, 2, 2, 2}"),
        ("mean({55, rep(54, 3)})", "54.25"),
    ]);
    run(
        &[("normal-min", "-2")],
        &[("1÷300", "3.333333333E-3"), ("0.05", "0.05")],
    );
    run(&[("normal-min", "-9")], &[("1÷3E7", "0.00000003333333333")]);
    run(
        &[("normal-min", "-9"), ("digits", "3")],
        &[("1÷3000000", "0.000000333")],
    );
    check(&[
        ("item({5, 6, 7}, 2)", "6"),
        ("item([1, 2, 3], 3)", "3"),
        ("item({1}, 2)", "error: Domain error"),
    ]);
    check(&[("convert(1, _kgfcm2, _Pa)", "98066.5"), ("_t0", "273.15")]);
}

#[test]
fn results_read_back_as_input() {
    // Whatever the screen shows can be used in the next calculation.
    let settings: &[&[(&str, &str)]] = &[
        &[],
        &[("fractions", "on")],
        &[("dms", "on")],
        &[("base", "hex")],
        &[("base", "bin")],
        &[("notation", "eng")],
        &[("complex", "on")],
        &[("complex", "on"), ("complex-form", "polar")],
        &[("base", "hex"), ("word-mode", "on"), ("word-bits", "16")],
    ];
    let inputs = [
        "1÷3",
        "2÷3×1000",
        "1E20÷3",
        "-7÷4",
        "255",
        "12.5",
        "√(-2)",
        "3+4i",
        "{1, 2.5}",
        "[1, 2; 3, 4]",
    ];
    for s in settings {
        for input in inputs {
            let mut ctx = Context::with_seed(1);
            for (k, v) in *s {
                ctx.settings.apply(k, v).unwrap();
            }
            let Ok(first) = evaluate_expression(input, &mut ctx) else {
                continue;
            };
            let shown = ctx.format(&first);
            let again = evaluate_expression(&shown, &mut ctx).unwrap_or_else(|e| {
                panic!("{shown:?} (from {input:?}, {s:?}) can't be read back: {e:?}")
            });
            assert_eq!(ctx.format(&again), shown, "from {input:?}, {s:?}");
        }
    }
}

#[test]
fn errors() {
    let cases: &[(&str, EvalError)] = &[
        ("1÷0", EvalError::DivisionByZero),
        ("5÷(2−2)", EvalError::DivisionByZero),
        ("0^-1", EvalError::DivisionByZero),
        ("√(−4)", EvalError::NegativeRoot),
        ("ln(0)", EvalError::Domain),
        ("log(−1)", EvalError::Domain),
        ("asin(2)", EvalError::Domain),
        ("tan(90)", EvalError::Domain),
        ("tan(−270)", EvalError::Domain),
        ("2.5!", EvalError::Domain),
        ("(−1)!", EvalError::Domain),
        ("nCr(2, 5)", EvalError::Domain),
        ("171!", EvalError::InvalidResult),
        ("10^400", EvalError::InvalidResult),
        ("(−8)^0.3", EvalError::Domain),
        ("sin([1, 2])", EvalError::TypeMismatch),
        ("det(5)", EvalError::TypeMismatch),
        ("[1, 2] + [1, 2, 3]", EvalError::DimensionMismatch),
        ("{1, 2} + {1, 2, 3}", EvalError::DimensionMismatch),
        ("det([1, 2])", EvalError::DimensionMismatch),
        ("inv([1, 2; 2, 4])", EvalError::SingularMatrix),
        ("sin(1 = 2)", EvalError::MisplacedEquation),
        ("solve(X^2 = -1)", EvalError::NoSolution),
        ("sigma(X, 1, 1E7)", EvalError::LimitExceeded),
        (
            "sigma(sigma(1, 1, 2000), 1, 2000)",
            EvalError::LimitExceeded,
        ),
        ("mean({})", EvalError::Domain),
    ];
    for (src, expected) in cases {
        assert_eq!(eval_err(src), *expected, "input: {src:?}");
    }
}

#[test]
fn random_numbers_are_in_range() {
    let mut ctx = Context::with_seed(7);
    for _ in 0..1000 {
        let r = evaluate_expression("rand", &mut ctx)
            .unwrap()
            .as_number()
            .unwrap();
        assert!((0.0..1.0).contains(&r));
        let n = evaluate_expression("randint(1, 6)", &mut ctx)
            .unwrap()
            .as_number()
            .unwrap();
        assert!([1.0, 2.0, 3.0, 4.0, 5.0, 6.0].contains(&n));
    }
}

#[test]
fn preview_changes_nothing() {
    let mut ctx = Context::with_seed(1);
    evaluate_expression("5→A", &mut ctx).unwrap();
    let preview = crate::core::engine::preview_expression("A+1→A", &ctx).unwrap();
    assert_eq!(ctx.format(&preview), "6");
    assert_eq!(ctx.format(&ctx.get(crate::core::engine::Var::A)), "5");
}
