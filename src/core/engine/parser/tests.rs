use super::*;
use crate::core::engine::lexer::tokenize;

fn tree(src: &str) -> String {
    let tokens = tokenize(src).unwrap_or_else(|e| panic!("{src:?} failed to lex: {e}"));
    parse(&tokens)
        .unwrap_or_else(|e| panic!("{src:?} failed to parse: {e}"))
        .to_string()
}

fn tree_with(src: &str, options: ParseOptions) -> String {
    let tokens = tokenize(src).unwrap();
    parse_with(&tokens, options).unwrap().to_string()
}

fn parse_err(src: &str) -> ParseError {
    let tokens = tokenize(src).unwrap();
    parse(&tokens).expect_err(src)
}

#[test]
fn structure() {
    let cases: &[(&str, &str)] = &[
        // precedence
        ("1+2×3", "(+ 1 (* 2 3))"),
        ("1×2+3", "(+ (* 1 2) 3)"),
        ("2×3^2", "(* 2 (^ 3 2))"),
        // associativity
        ("8−3−2", "(- (- 8 3) 2)"),
        ("8÷4÷2", "(/ (/ 8 4) 2)"),
        ("2^3^2", "(^ 2 (^ 3 2))"),
        // unary minus binds looser than ^
        ("-2^2", "(neg (^ 2 2))"),
        ("2^-1", "(^ 2 (neg 1))"),
        ("−3!", "(neg (! 3))"),
        ("5−−3", "(- 5 (neg 3))"),
        ("2×−3", "(* 2 (neg 3))"),
        ("+4", "(+ ans 4)"),
        ("×2", "(* ans 2)"),
        ("1 : ^2", "(: 1 (^ ans 2))"),
        // postfix
        ("3!%", "(% (! 3))"),
        ("√4!", "(sqrt (! 4))"),
        ("sin −30", "(sin (neg 30))"),
        ("log 10^3", "(log (^ 10 3))"),
        ("sin 30+cos 9", "(+ (sin 30) (cos 9))"),
        ("√2×3", "(* (sqrt 2) 3)"),
        // functions and constants
        ("sin(30)", "(sin 30)"),
        ("sin⁻¹(1)", "(asin 1)"),
        ("√9", "(sqrt 9)"),
        ("ln(e)", "(ln e)"),
        // implicit multiplication
        ("2π", "(* 2 π)"),
        ("2(3+4)", "(* 2 (+ 3 4))"),
        ("(2)(3)", "(* 2 3)"),
        ("5sin(30)", "(* 5 (sin 30))"),
        ("ee", "(* e e)"),
        ("3!2", "(* (! 3) 2)"),
        ("2π^2", "(* 2 (^ π 2))"),
        // brackets
        ("(1+2)×3", "(* (+ 1 2) 3)"),
        ("((4))", "4"),
        ("(2+3", "(+ 2 3)"),
        ("sin(30", "(sin 30)"),
        // several arguments, equations, no arguments
        ("nCr(5, 2)", "(nCr 5 2)"),
        // nPr / nCr between values, x-th root
        ("7nCr3", "(nCr 7 3)"),
        ("2×7nPr2+1", "(+ (* 2 (nPr 7 2)) 1)"),
        ("2nCr(5, 2)", "(* 2 (nCr 5 2))"),
        ("3ˣ√27", "(root 27 3)"),
        ("2+3ˣ√8^1", "(+ 2 (root (^ 8 1) 3))"),
        ("solve(X^2 = 4, 1)", "(solve (= (^ X 2) 4) 1)"),
        ("rand", "(rand)"),
        ("2rand", "(* 2 (rand))"),
        ("convert(1, _km, _m)", "(convert 1 _km _m)"),
        // matrices and lists
        ("[1, 2; 3, 4]", "(matrix (row 1 2) (row 3 4))"),
        ("{1, 2, 3}", "(list 1 2 3)"),
        ("{}", "(list)"),
        ("2{1, 2}", "(* 2 (list 1 2))"),
        // angles in degrees, minutes and seconds
        ("30°", "(° 30)"),
        ("12°30′", "(° (+ 12 0.5))"),
        ("sin 30°", "(sin (° 30))"),
        ("-1°30'0\"", "(neg (° (+ (+ 1 0.5) 0)))"),
        // storing and statements
        ("5→A", "(→ 5 A)"),
        ("1→A : A+1", "(: (→ 1 A) (+ A 1))"),
    ];
    for (src, expected) in cases {
        assert_eq!(tree(src), *expected, "input: {src:?}");
    }
}

#[test]
fn errors() {
    let cases: &[(&str, ParseError)] = &[
        ("", ParseError::UnexpectedEnd { span: 0..0 }),
        ("2+", ParseError::UnexpectedEnd { span: 2..2 }),
        ("2+×3", ParseError::UnexpectedToken { span: 2..4 }),
        ("()", ParseError::UnexpectedToken { span: 1..2 }),
        ("2)", ParseError::UnmatchedBracket { span: 1..2 }),
        ("(1))", ParseError::UnmatchedBracket { span: 3..4 }),
        ("1}", ParseError::UnmatchedBracket { span: 1..2 }),
        ("nCr(5)", ParseError::WrongArgumentCount { span: 0..6 }),
        ("rand(1)", ParseError::WrongArgumentCount { span: 0..7 }),
        ("[1, 2; 3]", ParseError::RaggedMatrix { span: 0..9 }),
        ("5→ans", ParseError::UnexpectedToken { span: 4..7 }),
        ("′", ParseError::UnexpectedToken { span: 0..3 }),
        ("2+×", ParseError::UnexpectedToken { span: 2..4 }),
        ("sin", ParseError::UnexpectedEnd { span: 3..3 }),
        ("2+!", ParseError::UnexpectedToken { span: 2..3 }),
    ];
    for (src, expected) in cases {
        assert_eq!(parse_err(src), *expected, "input: {src:?}");
    }
}

#[test]
fn implicit_multiplication_first() {
    let first = ParseOptions {
        implicit_mul_first: true,
    };
    assert_eq!(tree("6÷2(1+2)"), "(* (/ 6 2) (+ 1 2))");
    assert_eq!(tree_with("6÷2(1+2)", first), "(/ 6 (* 2 (+ 1 2)))");
    assert_eq!(tree_with("1÷2π", first), "(/ 1 (* 2 π))");
    assert_eq!(tree_with("2×3+4", first), "(+ (* 2 3) 4)");
}

#[test]
fn deep_nesting_is_an_error_not_a_crash() {
    let src = "(".repeat(10_000) + "1";
    assert!(matches!(parse_err(&src), ParseError::TooDeep { .. }));
    let src = "-".repeat(10_000) + "1";
    assert!(matches!(parse_err(&src), ParseError::TooDeep { .. }));
}
