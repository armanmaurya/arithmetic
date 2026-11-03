//! Every function the calculator knows, one definition each.
//!
//! A definition holds the name, other spellings, how many arguments it
//! takes and the code that runs it. The lexer, the parser and the evaluator
//! all read this one table, so adding a function means adding one entry to
//! the `DEFS` list of the matching family file, nothing else.
//!
//! - **Eager** functions get their arguments already evaluated.
//! - **Lazy** functions get the unevaluated trees, for things that evaluate
//!   an expression many times with different `X` (`solve`, `integrate`, …)
//!   or take arguments that aren't values (units in `convert`).

mod calculus;
mod elementary;
mod integer;
mod linalg;
mod stats;

use crate::core::engine::{context::Context, evaluator::EvalError, parser::Expr, value::Value};
use std::sync::OnceLock;

pub type EagerFn = fn(&[Value], &mut Context) -> Result<Value, EvalError>;
pub type LazyFn = fn(&[Expr], &mut Context) -> Result<Value, EvalError>;

#[derive(Clone, Copy)]
pub enum Body {
    Eager(EagerFn),
    Lazy(LazyFn),
}

pub struct FunctionDef {
    /// The main spelling, used when printing a tree.
    pub name: &'static str,
    /// Other spellings, e.g. `√` for `sqrt`.
    pub aliases: &'static [&'static str],
    pub min_args: usize,
    /// [`VARIADIC`] for no upper limit.
    pub max_args: usize,
    /// One line for help screens, e.g. `"nCr(n, r): combinations"`.
    pub summary: &'static str,
    pub body: Body,
}

pub const VARIADIC: usize = usize::MAX;

/// A short way to write a table entry.
const fn eager(
    name: &'static str,
    aliases: &'static [&'static str],
    args: (usize, usize),
    summary: &'static str,
    run: EagerFn,
) -> FunctionDef {
    FunctionDef {
        name,
        aliases,
        min_args: args.0,
        max_args: args.1,
        summary,
        body: Body::Eager(run),
    }
}

const fn lazy(
    name: &'static str,
    aliases: &'static [&'static str],
    args: (usize, usize),
    summary: &'static str,
    run: LazyFn,
) -> FunctionDef {
    FunctionDef {
        name,
        aliases,
        min_args: args.0,
        max_args: args.1,
        summary,
        body: Body::Lazy(run),
    }
}

fn registry() -> &'static [&'static FunctionDef] {
    static ALL: OnceLock<Vec<&'static FunctionDef>> = OnceLock::new();
    ALL.get_or_init(|| {
        [
            elementary::DEFS,
            integer::DEFS,
            stats::DEFS,
            linalg::DEFS,
            calculus::DEFS,
        ]
        .into_iter()
        .flatten()
        .collect()
    })
}

/// Every function definition, e.g. for a help screen.
pub fn all() -> impl Iterator<Item = &'static FunctionDef> {
    registry().iter().copied()
}

/// A function, as the lexer hands it to the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(u16);

impl FuncId {
    pub fn def(self) -> &'static FunctionDef {
        registry()[self.0 as usize]
    }

    pub fn name(self) -> &'static str {
        self.def().name
    }

    /// Every function with every spelling.
    pub(crate) fn all_names() -> impl Iterator<Item = (&'static str, Self)> {
        registry().iter().enumerate().flat_map(|(i, def)| {
            let id = Self(i as u16);
            std::iter::once(def.name)
                .chain(def.aliases.iter().copied())
                .map(move |name| (name, id))
        })
    }

    pub fn lookup(name: &str) -> Option<Self> {
        Self::all_names()
            .find(|(n, _)| *n == name)
            .map(|(_, id)| id)
    }
}

// Helpers shared by the family files.

/// Applies a real function to a value, or to every item of a list.
fn real(v: &Value, f: impl Fn(f64) -> Result<f64, EvalError>) -> Result<Value, EvalError> {
    crate::core::engine::value::broadcast(v, &mut |x| Ok(Value::Number(f(x.as_number()?)?)))
}

/// Like [`real`] for two arguments.
fn real2(
    a: &Value,
    b: &Value,
    f: impl Fn(f64, f64) -> Result<f64, EvalError>,
) -> Result<Value, EvalError> {
    crate::core::engine::value::broadcast2(a, b, &mut |x, y| {
        Ok(Value::Number(f(x.as_number()?, y.as_number()?)?))
    })
}

/// The `i`-th argument as a real number, or `default` if it wasn't given.
fn arg_or(args: &[Value], i: usize, default: f64) -> Result<f64, EvalError> {
    args.get(i).map_or(Ok(default), Value::as_number)
}

/// A whole number, allowing a little floating-point noise (so that
/// `(0.1+0.2)×10` counts as 3).
fn whole(x: f64) -> Result<i64, EvalError> {
    let rounded = x.round();
    if (x - rounded).abs() > 1e-9 * x.abs().max(1.0) || rounded.abs() > 9.0e15 {
        return Err(EvalError::Domain);
    }
    Ok(rounded as i64)
}

/// Fails on infinity and NaN.
fn finite(x: f64) -> Result<f64, EvalError> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(EvalError::InvalidResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_arity_is_sane() {
        let mut names: Vec<_> = FuncId::all_names().map(|(n, _)| n).collect();
        let len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), len, "a function name is used twice");
        for def in all() {
            assert!(def.min_args <= def.max_args, "{}", def.name);
            assert!(!def.summary.is_empty(), "{}", def.name);
        }
    }

    #[test]
    fn lookup() {
        let sqrt = FuncId::lookup("sqrt").unwrap();
        assert_eq!(FuncId::lookup("√"), Some(sqrt));
        assert_eq!(sqrt.name(), "sqrt");
        assert_eq!(FuncId::lookup("nope"), None);
    }
}
