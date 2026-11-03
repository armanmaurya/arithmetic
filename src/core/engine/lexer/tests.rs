use super::*;
use TokenKind as T;

fn kinds(src: &str) -> Vec<TokenKind> {
    tokenize(src)
        .unwrap_or_else(|e| panic!("{src:?} failed: {e}"))
        .into_iter()
        .map(|t| t.kind)
        .collect()
}

fn f(name: &str) -> TokenKind {
    T::Func(FuncId::lookup(name).unwrap())
}

fn c(name: &str) -> TokenKind {
    T::Constant(ConstId::lookup(name).unwrap())
}

#[test]
fn valid_input() {
    let cases: &[(&str, &[TokenKind])] = &[
        ("", &[]),
        ("   ", &[]),
        ("42", &[T::Number(42.0)]),
        ("12.5", &[T::Number(12.5)]),
        (".5", &[T::Number(0.5)]),
        ("5.", &[T::Number(5.0)]),
        ("1 + 2", &[T::Number(1.0), T::Plus, T::Number(2.0)]),
        ("3!%", &[T::Number(3.0), T::Bang, T::Percent]),
        (
            "2^(1)",
            &[
                T::Number(2.0),
                T::Caret,
                T::LParen,
                T::Number(1.0),
                T::RParen,
            ],
        ),
        ("√9", &[f("sqrt"), T::Number(9.0)]),
        ("2π", &[T::Number(2.0), c("π")]),
        ("ee", &[c("e"), c("e")]),
        (
            "log(ln(e))",
            &[
                f("log"),
                T::LParen,
                f("ln"),
                T::LParen,
                c("e"),
                T::RParen,
                T::RParen,
            ],
        ),
        // scientific notation; "2E" alone is 2 × the variable E
        ("1.5E3", &[T::Number(1500.0)]),
        ("2E−4", &[T::Number(0.0002)]),
        ("2E", &[T::Number(2.0), T::Variable(Var::E)]),
        // other bases; "0x" alone is 0 × x
        ("0x1F", &[T::Number(31.0)]),
        ("0b101", &[T::Number(5.0)]),
        ("0d15", &[T::Number(15.0)]),
        ("0o17", &[T::Number(15.0)]),
        ("0x", &[T::Number(0.0), T::Variable(Var::X)]),
        // variables, storing, statements
        ("5→A", &[T::Number(5.0), T::Store, T::Variable(Var::A)]),
        (
            "ans:M",
            &[T::Variable(Var::Ans), T::Colon, T::Variable(Var::M)],
        ),
        ("sinx", &[f("sin"), T::Variable(Var::X)]),
        ("exp", &[f("exp")]),
        // matrices and lists
        (
            "[1;2]",
            &[
                T::LBracket,
                T::Number(1.0),
                T::Semicolon,
                T::Number(2.0),
                T::RBracket,
            ],
        ),
        (
            "{1,2}",
            &[
                T::LBrace,
                T::Number(1.0),
                T::Comma,
                T::Number(2.0),
                T::RBrace,
            ],
        ),
        // angles
        (
            "12°30′15″",
            &[
                T::Number(12.0),
                T::Degree,
                T::Number(30.0),
                T::Minute,
                T::Number(15.0),
                T::Second,
            ],
        ),
        // complex numbers, physical constants and units
        ("3+4i", &[T::Number(3.0), T::Plus, T::Number(4.0), c("i")]),
        ("_c", &[c("_c")]),
        ("_km", &[T::Unit(UnitId::lookup("_km").unwrap())]),
    ];
    for (src, expected) in cases {
        assert_eq!(kinds(src), *expected, "input: {src:?}");
    }
}

#[test]
fn spellings_are_normalised() {
    let same: &[(&str, &str)] = &[
        ("−", "-"),
        ("×", "*"),
        ("÷", "/"),
        ("π", "pi"),
        ("√", "sqrt"),
        ("sin⁻¹", "asin"),
        ("cos⁻¹", "acos"),
        ("tan⁻¹", "atan"),
        ("→", "->"),
        ("′", "'"),
        ("x", "X"),
    ];
    for (a, b) in same {
        assert_eq!(kinds(a), kinds(b), "{a:?} vs {b:?}");
    }
}

#[test]
fn longest_name_wins() {
    assert_eq!(kinds("sin⁻¹(1)")[0], f("asin"));
    assert_eq!(kinds("sinh(1)")[0], f("sinh"));
    assert_eq!(kinds("integrate")[0], f("integrate"));
    assert_eq!(kinds("esin(30)")[..2], [c("e"), f("sin")]);
}

#[test]
fn spans_slice_back_to_lexemes() {
    let src = "12.5×sin⁻¹(30) − √ 2";
    let lexemes: Vec<&str> = tokenize(src)
        .unwrap()
        .iter()
        .map(|t| &src[t.span.clone()])
        .collect();
    assert_eq!(
        lexemes,
        ["12.5", "×", "sin⁻¹", "(", "30", ")", "−", "√", "2"]
    );
}

#[test]
fn errors() {
    let cases: &[(&str, LexError)] = &[
        ("1.2.3", LexError::BadNumber { span: 0..5 }),
        (".", LexError::BadNumber { span: 0..1 }),
        (
            "2$",
            LexError::UnexpectedChar {
                ch: '$',
                span: 1..2,
            },
        ),
        (
            "×#",
            LexError::UnexpectedChar {
                ch: '#',
                span: 2..3,
            },
        ),
        // A multi-byte character: the span covers all 3 bytes of '€'.
        (
            "€",
            LexError::UnexpectedChar {
                ch: '€',
                span: 0..3,
            },
        ),
        (
            "foo",
            LexError::UnknownName {
                name: "foo".into(),
                span: 0..3,
            },
        ),
        // `_` names must match a whole word
        (
            "_kmx",
            LexError::UnknownName {
                name: "_kmx".into(),
                span: 0..4,
            },
        ),
    ];
    for (src, expected) in cases {
        assert_eq!(tokenize(src).as_ref(), Err(expected), "input: {src:?}");
    }
    // a hex literal too large for 64 bits
    assert_eq!(
        tokenize("0x11111111111111111"),
        Err(LexError::BadNumber { span: 0..19 })
    );
}

#[test]
fn too_large_number_is_rejected() {
    // f64 tops out near 1.8e308; Rust would parse this as infinity.
    let huge = "9".repeat(400);
    assert_eq!(tokenize(&huge), Err(LexError::BadNumber { span: 0..400 }));
}

/// Checks properties that must hold for *every* input, on thousands of
/// random strings built from characters the lexer finds interesting:
/// - it never panics
/// - on success, tokens are non-empty, in order, don't overlap, and only
///   whitespace sits between them
/// - on failure, the error's span is inside the input
#[test]
fn random_inputs_keep_invariants() {
    const ALPHABET: &[char] = &[
        '0', '1', '9', '.', '+', '-', '−', '*', '×', '/', '÷', '^', '%', '!', '(', ')', ' ', '\t',
        's', 'i', 'n', 'c', 'o', 't', 'a', 'l', 'g', 'q', 'r', 'p', 'e', 'x', '⁻', '¹', 'π', '√',
        '$', '€', 'é', '🙂', '_', 'k', 'm', '°', '′', '{', '}', '[', ']', ';', ':', ',', 'b', 'E',
        '→', '=',
    ];
    // A tiny deterministic random number generator (xorshift), so failures
    // are reproducible and no extra crate is needed.
    let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };

    for _ in 0..20_000 {
        let len = (next() % 16) as usize;
        let src: String = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();

        match tokenize(&src) {
            Ok(tokens) => {
                let mut prev_end = 0;
                for t in &tokens {
                    assert!(t.span.start < t.span.end, "empty span in {src:?}");
                    let gap = src
                        .get(prev_end..t.span.start)
                        .expect("gap off a char boundary");
                    assert!(
                        gap.chars().all(char::is_whitespace),
                        "skipped text in {src:?}"
                    );
                    assert!(src.get(t.span.clone()).is_some(), "bad span in {src:?}");
                    prev_end = t.span.end;
                }
                let tail = &src[prev_end..];
                assert!(
                    tail.chars().all(char::is_whitespace),
                    "unlexed tail in {src:?}"
                );
            }
            Err(e) => {
                assert!(
                    src.get(e.span()).is_some(),
                    "bad error span {e:?} in {src:?}"
                );
            }
        }
    }
}
