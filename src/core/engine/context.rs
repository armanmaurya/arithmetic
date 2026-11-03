use crate::core::engine::{
    evaluator::EvalError, format, lexer::Var, settings::Settings, value::Value,
};
use std::collections::BTreeMap;

/// Most expression evaluations one calculation may use inside `solve`,
/// `integrate`, `sigma` and friends. Stops nested integrals from running
/// for minutes.
pub(crate) const MAX_WORK: u64 = 2_000_000;

/// Everything the calculator remembers between calculations: settings,
/// variables, the last answer (`ans`), memory (`M`) and the random
/// number generator.
#[derive(Debug, Clone)]
pub struct Context {
    pub settings: Settings,
    variables: BTreeMap<Var, Value>,
    rng: u64,
    work: u64,
}

impl Default for Context {
    /// Default settings and a random generator seeded from the clock.
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x9E37_79B9_7F4A_7C15, |d| d.as_nanos() as u64);
        Self::with_seed(seed)
    }
}

impl Context {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    /// A context whose random numbers are always the same, for tests.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            settings: Settings::default(),
            variables: BTreeMap::new(),
            rng: seed | 1, // xorshift needs a non-zero state
            work: 0,
        }
    }

    /// A variable's value. Variables that were never set are 0.
    pub fn get(&self, var: Var) -> Value {
        self.variables
            .get(&var)
            .cloned()
            .unwrap_or(Value::Number(0.0))
    }

    pub fn set(&mut self, var: Var, value: Value) {
        self.variables.insert(var, value);
    }

    /// `M+` (positive `delta`) and `M−` (negative `delta`).
    pub fn memory_add(&mut self, delta: f64) -> Result<(), EvalError> {
        let current = self.get(Var::M).as_number().unwrap_or(0.0);
        let total = current + delta;
        if !total.is_finite() {
            return Err(EvalError::InvalidResult);
        }
        self.set(Var::M, Value::Number(total));
        Ok(())
    }

    pub fn memory_clear(&mut self) {
        self.variables.remove(&Var::M);
    }

    /// Whether `M` holds something other than 0, e.g. to light an "M" sign.
    pub fn memory_is_set(&self) -> bool {
        self.get(Var::M) != Value::Number(0.0)
    }

    /// Clears every variable, including `ans` and `M`. Settings stay.
    pub fn clear_variables(&mut self) {
        self.variables.clear();
    }

    /// A value as text, following the display settings.
    pub fn format(&self, value: &Value) -> String {
        format::format_with(value, &self.settings)
    }

    /// A uniformly distributed number in [0, 1) (xorshift64*).
    pub fn next_random(&mut self) -> f64 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        let bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11; // top 53 bits
        bits as f64 / (1u64 << 53) as f64
    }

    pub(crate) fn reset_work(&mut self) {
        self.work = 0;
    }

    /// Records `amount` units of work, failing once the budget is spent.
    pub(crate) fn spend(&mut self, amount: u64) -> Result<(), EvalError> {
        self.work += amount;
        if self.work > MAX_WORK {
            Err(EvalError::LimitExceeded)
        } else {
            Ok(())
        }
    }
}
