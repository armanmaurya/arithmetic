//! Everything that differs between calculators lives here as a setting, so
//! the engine never needs to know which calculator it is imitating.
//!
//! Hosts and skins change settings with plain strings through
//! [`Settings::apply`], e.g. `apply("angle", "rad")`. See [`Settings::KEYS`].

use std::f64::consts::PI;
use std::fmt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AngleMode {
    #[default]
    Degrees,
    Radians,
    Gradians,
}

impl AngleMode {
    /// The size of a full turn in this unit.
    pub fn full_turn(self) -> f64 {
        match self {
            Self::Degrees => 360.0,
            Self::Radians => 2.0 * PI,
            Self::Gradians => 400.0,
        }
    }

    pub fn to_radians(self, x: f64) -> f64 {
        match self {
            Self::Degrees => x.to_radians(),
            Self::Radians => x,
            Self::Gradians => x * PI / 200.0,
        }
    }

    pub fn from_radians(self, r: f64) -> f64 {
        match self {
            Self::Degrees => r.to_degrees(),
            Self::Radians => r,
            Self::Gradians => r * 200.0 / PI,
        }
    }

    pub fn from_degrees(self, d: f64) -> f64 {
        match self {
            Self::Degrees => d,
            Self::Radians => d.to_radians(),
            Self::Gradians => d * 400.0 / 360.0,
        }
    }
}

/// How results are written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Notation {
    /// Plain digits, switching to scientific notation for very large or
    /// small values.
    #[default]
    Normal,
    /// Always this many decimal places.
    Fixed(u8),
    /// Always scientific notation with this many significant digits.
    Scientific(u8),
    /// Scientific notation with the exponent a multiple of 3.
    Engineering,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Base {
    Binary,
    Octal,
    #[default]
    Decimal,
    Hexadecimal,
}

/// What `%` means after `+` or `−`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PercentStyle {
    /// `x%` is always `x ÷ 100`: `200 + 10%` = 200.1.
    #[default]
    Fraction,
    /// `a ± b%` means `a ± (b% of a)`: `200 + 10%` = 220, as on many
    /// desk calculators.
    OfLeftOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaySettings {
    pub notation: Notation,
    /// Integers are shown in this base (`0x1F`, `0b101`, `0o17`).
    pub base: Base,
    /// Show values like 0.75 as `3/4` where a small fraction fits exactly.
    pub fractions: bool,
    /// Show values as degrees, minutes and seconds: `12°30′15″`.
    pub dms: bool,
    /// Significant digits in normal notation (1–15).
    pub significant_digits: u8,
    /// Normal notation switches to scientific below 10^normal_min:
    /// −7 by default, −2 for Casio's Norm 1, −9 for Norm 2.
    pub normal_min: i8,
    /// Show complex numbers in polar form, `r∠θ`, instead of `a+bi`.
    pub polar: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            notation: Notation::Normal,
            base: Base::Decimal,
            fractions: false,
            dms: false,
            significant_digits: 10,
            normal_min: -7,
            polar: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub angle: AngleMode,
    pub display: DisplaySettings,
    /// Let real operations produce complex results, e.g. `√(−4)` = `2i`.
    /// Complex input (using `i`) works either way.
    pub complex: bool,
    /// Implicit multiplication binds tighter than `×` and `÷`, so
    /// `6÷2(1+2)` is 1 instead of 9. Calculators disagree on this.
    pub implicit_mul_first: bool,
    pub percent: PercentStyle,
    /// Word size for the bitwise functions (`and`, `not`, `shl`, …), 1–64.
    pub word_bits: u8,
    /// The base plain number literals are read in. In hexadecimal, `A`–`F`
    /// are digits, so `1F` is 31. Prefixed literals (`0x1F`) always work.
    pub input_base: Base,
    /// Treat whole numbers as `word_bits`-bit two's-complement words:
    /// binary, octal and hexadecimal show −1 as `0xFF…F`, and read such
    /// literals back as negative numbers.
    pub word_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            angle: AngleMode::Degrees,
            display: DisplaySettings::default(),
            complex: false,
            implicit_mul_first: false,
            percent: PercentStyle::Fraction,
            word_bits: 64,
            input_base: Base::Decimal,
            word_mode: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingError {
    UnknownKey(String),
    BadValue { key: String, value: String },
}

impl fmt::Display for SettingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKey(key) => write!(f, "unknown setting '{key}'"),
            Self::BadValue { key, value } => write!(f, "invalid value '{value}' for '{key}'"),
        }
    }
}

impl std::error::Error for SettingError {}

impl Settings {
    /// Every key `apply` and `get` understand, with the values they take.
    pub const KEYS: &'static [(&'static str, &'static str)] = &[
        ("angle", "deg | rad | grad"),
        ("notation", "normal | fix:N | sci:N | eng"),
        ("base", "dec | bin | oct | hex"),
        ("fractions", "on | off"),
        ("dms", "on | off"),
        ("digits", "1–15"),
        (
            "normal-min",
            "−15–0: normal notation switches to scientific below 10^N",
        ),
        ("complex", "on | off"),
        ("implicit-mul-first", "on | off"),
        ("percent", "fraction | of-left"),
        ("word-bits", "1–64"),
        ("input-base", "dec | bin | oct | hex"),
        ("word-mode", "on | off"),
        ("complex-form", "rect | polar"),
    ];

    /// Changes one setting from strings, e.g. `apply("notation", "fix:2")`.
    pub fn apply(&mut self, key: &str, value: &str) -> Result<(), SettingError> {
        let bad = || SettingError::BadValue {
            key: key.to_string(),
            value: value.to_string(),
        };
        let value = value.trim();
        match key {
            "angle" => {
                self.angle = match value {
                    "deg" => AngleMode::Degrees,
                    "rad" => AngleMode::Radians,
                    "grad" => AngleMode::Gradians,
                    _ => return Err(bad()),
                }
            }
            "notation" => {
                let digits = |s: &str| s.parse::<u8>().ok().filter(|d| (0..=15).contains(d));
                self.display.notation = match value.split_once(':') {
                    None if value == "normal" => Notation::Normal,
                    None if value == "eng" => Notation::Engineering,
                    Some(("fix", n)) => Notation::Fixed(digits(n).ok_or_else(bad)?),
                    Some(("sci", n)) => {
                        Notation::Scientific(digits(n).filter(|d| *d >= 1).ok_or_else(bad)?)
                    }
                    _ => return Err(bad()),
                }
            }
            "base" => self.display.base = parse_base(value).ok_or_else(bad)?,
            "input-base" => self.input_base = parse_base(value).ok_or_else(bad)?,
            "word-mode" => self.word_mode = parse_bool(value).ok_or_else(bad)?,
            "complex-form" => {
                self.display.polar = match value {
                    "rect" => false,
                    "polar" => true,
                    _ => return Err(bad()),
                }
            }
            "fractions" => self.display.fractions = parse_bool(value).ok_or_else(bad)?,
            "dms" => self.display.dms = parse_bool(value).ok_or_else(bad)?,
            "digits" => {
                self.display.significant_digits = value
                    .parse::<u8>()
                    .ok()
                    .filter(|d| (1..=15).contains(d))
                    .ok_or_else(bad)?
            }
            "normal-min" => {
                self.display.normal_min = value
                    .replace('−', "-")
                    .parse::<i8>()
                    .ok()
                    .filter(|n| (-15..=0).contains(n))
                    .ok_or_else(bad)?
            }
            "complex" => self.complex = parse_bool(value).ok_or_else(bad)?,
            "implicit-mul-first" => self.implicit_mul_first = parse_bool(value).ok_or_else(bad)?,
            "percent" => {
                self.percent = match value {
                    "fraction" => PercentStyle::Fraction,
                    "of-left" => PercentStyle::OfLeftOperand,
                    _ => return Err(bad()),
                }
            }
            "word-bits" => {
                self.word_bits = value
                    .parse::<u8>()
                    .ok()
                    .filter(|b| (1..=64).contains(b))
                    .ok_or_else(bad)?
            }
            _ => return Err(SettingError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    /// The current value of a setting, in the same form `apply` takes.
    pub fn get(&self, key: &str) -> Option<String> {
        let on_off = |b: bool| if b { "on" } else { "off" }.to_string();
        Some(match key {
            "angle" => match self.angle {
                AngleMode::Degrees => "deg",
                AngleMode::Radians => "rad",
                AngleMode::Gradians => "grad",
            }
            .to_string(),
            "notation" => match self.display.notation {
                Notation::Normal => "normal".to_string(),
                Notation::Fixed(n) => format!("fix:{n}"),
                Notation::Scientific(n) => format!("sci:{n}"),
                Notation::Engineering => "eng".to_string(),
            },
            "base" => base_name(self.display.base).to_string(),
            "input-base" => base_name(self.input_base).to_string(),
            "word-mode" => on_off(self.word_mode),
            "complex-form" => if self.display.polar { "polar" } else { "rect" }.to_string(),
            "fractions" => on_off(self.display.fractions),
            "dms" => on_off(self.display.dms),
            "digits" => self.display.significant_digits.to_string(),
            "normal-min" => self.display.normal_min.to_string(),
            "complex" => on_off(self.complex),
            "implicit-mul-first" => on_off(self.implicit_mul_first),
            "percent" => match self.percent {
                PercentStyle::Fraction => "fraction",
                PercentStyle::OfLeftOperand => "of-left",
            }
            .to_string(),
            "word-bits" => self.word_bits.to_string(),
            _ => return None,
        })
    }
}

fn parse_base(value: &str) -> Option<Base> {
    Some(match value {
        "dec" => Base::Decimal,
        "bin" => Base::Binary,
        "oct" => Base::Octal,
        "hex" => Base::Hexadecimal,
        _ => return None,
    })
}

fn base_name(base: Base) -> &'static str {
    match base {
        Base::Decimal => "dec",
        Base::Binary => "bin",
        Base::Octal => "oct",
        Base::Hexadecimal => "hex",
    }
}

impl Base {
    pub fn radix(self) -> u32 {
        match self {
            Self::Binary => 2,
            Self::Octal => 8,
            Self::Decimal => 10,
            Self::Hexadecimal => 16,
        }
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "on" | "true" | "1" | "yes" => Some(true),
        "off" | "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_and_get_round_trip() {
        let mut s = Settings::default();
        let changes = [
            ("angle", "grad"),
            ("notation", "fix:3"),
            ("base", "hex"),
            ("fractions", "on"),
            ("dms", "on"),
            ("digits", "12"),
            ("complex", "on"),
            ("implicit-mul-first", "on"),
            ("percent", "of-left"),
            ("word-bits", "32"),
            ("input-base", "hex"),
            ("word-mode", "on"),
            ("complex-form", "polar"),
            ("normal-min", "-2"),
        ];
        for (key, value) in changes {
            s.apply(key, value).unwrap();
            assert_eq!(s.get(key).as_deref(), Some(value), "{key}");
        }
        assert_eq!(Settings::KEYS.len(), changes.len());
    }

    #[test]
    fn rejects_bad_input() {
        let mut s = Settings::default();
        assert!(matches!(
            s.apply("colour", "red"),
            Err(SettingError::UnknownKey(_))
        ));
        assert!(matches!(
            s.apply("angle", "turns"),
            Err(SettingError::BadValue { .. })
        ));
        assert!(s.apply("digits", "0").is_err());
        assert!(s.apply("word-bits", "65").is_err());
        assert!(s.apply("notation", "sci:0").is_err());
        assert_eq!(s, Settings::default());
    }
}
