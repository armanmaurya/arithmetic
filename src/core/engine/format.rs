//! Turns values into text, following the display settings. Every output can
//! be read back by the lexer, so a result can always be used in the next
//! calculation.

use crate::core::engine::{
    settings::{AngleMode, Base, DisplaySettings, Notation, Settings},
    value::{Complex, Value},
};

/// Everything formatting depends on: the display settings plus the parts
/// of the other settings that change how a value is written.
#[derive(Clone)]
struct Style {
    d: DisplaySettings,
    /// For the angle of polar complex numbers.
    angle: AngleMode,
    /// `Some(bits)` in word mode: negative whole numbers in other bases are
    /// written as two's-complement words.
    word_bits: Option<u8>,
}

impl Style {
    fn plain(d: &DisplaySettings) -> Self {
        Self {
            d: d.clone(),
            angle: AngleMode::Degrees,
            word_bits: None,
        }
    }
}

/// A number in the default display style (10 significant digits).
pub fn format_number(x: f64) -> String {
    format_real(x, &Style::plain(&DisplaySettings::default()))
}

/// A value written with only the display settings (polar angles in
/// degrees, no word mode). [`format_with`] takes every setting into account.
pub fn format_value(value: &Value, display: &DisplaySettings) -> String {
    format_styled(value, &Style::plain(display))
}

/// A value written the way the settings say, as the calculator shows it.
pub fn format_with(value: &Value, settings: &Settings) -> String {
    let style = Style {
        d: settings.display.clone(),
        angle: settings.angle,
        word_bits: settings.word_mode.then_some(settings.word_bits),
    };
    format_styled(value, &style)
}

fn format_styled(value: &Value, style: &Style) -> String {
    match value {
        Value::Number(n) => format_real(*n, style),
        Value::Complex(c) => format_complex(*c, style),
        Value::Matrix(m) => {
            let rows: Vec<String> = (0..m.rows())
                .map(|r| {
                    let cells: Vec<String> = (0..m.cols())
                        .map(|c| format_real(m.get(r, c), style))
                        .collect();
                    cells.join(", ")
                })
                .collect();
            format!("[{}]", rows.join("; "))
        }
        Value::List(items) => {
            let items: Vec<String> = items.iter().map(|v| format_styled(v, style)).collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}

fn format_real(x: f64, style: &Style) -> String {
    let d = &style.d;
    if !x.is_finite() {
        return "Value error".into();
    }
    if d.base != Base::Decimal
        && let Some(s) = in_base(x, d.base, style.word_bits)
    {
        return s;
    }
    if d.dms {
        return dms(x);
    }
    if d.fractions
        && let Some(s) = fraction(x)
    {
        return s;
    }
    match d.notation {
        Notation::Normal => normal(x, d.significant_digits, d.normal_min),
        Notation::Fixed(places) => fixed(x, places),
        Notation::Scientific(digits) => scientific(x, digits, false),
        Notation::Engineering => engineering(x, d.significant_digits),
    }
}

fn format_complex(c: Complex, style: &Style) -> String {
    // Bases and degree-minute-second make no sense for complex parts.
    let mut parts = style.clone();
    parts.d.base = Base::Decimal;
    parts.d.dms = false;

    if style.d.polar {
        let r = format_real(c.abs(), &parts);
        let theta = format_real(style.angle.from_radians(c.arg()), &parts);
        return format!("{r}∠{theta}");
    }
    let mut im = format_real(c.im.abs(), &parts);
    if im.contains('/') {
        im = format!("({im})"); // so "(1/2)i" doesn't read back as 1/(2i)
    }
    let im = if im == "1" {
        "i".to_string()
    } else {
        format!("{im}i")
    };
    if c.re == 0.0 {
        let sign = if c.im < 0.0 { "-" } else { "" };
        return format!("{sign}{im}");
    }
    let sign = if c.im < 0.0 { "-" } else { "+" };
    format!("{}{sign}{im}", format_real(c.re, &parts))
}

/// Rounds to `digits` significant digits, switching to scientific notation
/// when the number doesn't fit or is below 10^`min`.
fn normal(x: f64, digits: u8, min: i8) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let abs = x.abs();
    if abs >= 10f64.powi(digits as i32) || abs < 10f64.powi(min as i32) {
        return scientific(x, digits, true);
    }
    let magnitude = abs.log10().floor() as i32;
    let decimals = (digits as i32 - 1 - magnitude).max(0) as usize;
    trim_zeros(&format!("{x:.decimals$}"))
}

fn fixed(x: f64, places: u8) -> String {
    if x.abs() >= 1e15 {
        return scientific(x, 10, true);
    }
    let s = format!("{x:.*}", places as usize);
    // Don't show "-0.00".
    if s.starts_with('-') && s[1..].chars().all(|c| c == '0' || c == '.') {
        s[1..].to_string()
    } else {
        s
    }
}

fn scientific(x: f64, digits: u8, trim: bool) -> String {
    let s = format!("{x:.*e}", digits.saturating_sub(1) as usize);
    let (mantissa, exponent) = s.split_once('e').unwrap_or((&s, "0"));
    let mantissa = if trim {
        trim_zeros(mantissa)
    } else {
        mantissa.to_string()
    };
    format!("{mantissa}E{exponent}")
}

fn engineering(x: f64, digits: u8) -> String {
    if x == 0.0 {
        return "0E0".into();
    }
    let mut exponent = x.abs().log10().floor() as i32;
    exponent = exponent.div_euclid(3) * 3;
    let mut mantissa = x / 10f64.powi(exponent);
    let integer_digits = (mantissa.abs().log10().floor() as i32 + 1).max(1);
    let decimals = (digits as i32 - integer_digits).max(0) as usize;
    let mut text = format!("{mantissa:.decimals$}");
    // Rounding can carry into a fourth integer digit (999.99… → 1000).
    if text
        .trim_start_matches('-')
        .split('.')
        .next()
        .map_or(0, str::len)
        > 3
    {
        exponent += 3;
        mantissa /= 1000.0;
        text = format!("{mantissa:.decimals$}");
    }
    format!("{}E{exponent}", trim_zeros(&text))
}

fn trim_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

/// Whole numbers in another base, with a prefix the lexer reads back:
/// `0b101`, `0o17`, `0x1F`. Other values return `None` and stay decimal.
/// With `word_bits`, negative numbers are written as two's-complement
/// words (−1 in 8 bits is `0xFF`); numbers that don't fit the word stay
/// decimal.
fn in_base(x: f64, base: Base, word_bits: Option<u8>) -> Option<String> {
    if x.fract() != 0.0 || x.abs() >= 9.2e18 {
        return None;
    }
    let n = x as i64;
    let (sign, magnitude) = match word_bits {
        Some(bits) => {
            let n = n as i128;
            if n < -(1i128 << (bits - 1)) || n >= (1i128 << bits) {
                return None;
            }
            ("", (n.rem_euclid(1i128 << bits)) as u64)
        }
        None => (if n < 0 { "-" } else { "" }, n.unsigned_abs()),
    };
    Some(match base {
        Base::Binary => format!("{sign}0b{magnitude:b}"),
        Base::Octal => format!("{sign}0o{magnitude:o}"),
        Base::Hexadecimal => format!("{sign}0x{magnitude:X}"),
        Base::Decimal => return None,
    })
}

/// Degrees, minutes and seconds, e.g. `12°30′15″`.
fn dms(x: f64) -> String {
    let sign = if x < 0.0 { "-" } else { "" };
    let abs = x.abs();
    let mut degrees = abs.trunc();
    let minutes_exact = (abs - degrees) * 60.0;
    let mut minutes = minutes_exact.trunc();
    let mut seconds = ((minutes_exact - minutes) * 60.0 * 100.0).round() / 100.0;
    if seconds >= 60.0 {
        seconds -= 60.0;
        minutes += 1.0;
    }
    if minutes >= 60.0 {
        minutes -= 60.0;
        degrees += 1.0;
    }
    format!(
        "{sign}{}°{}′{}″",
        format_number(degrees),
        minutes as i64,
        format_number(seconds)
    )
}

/// The simplest fraction equal to `x` (to 1E-10), with a denominator up to
/// 10 000, found with continued fractions. `None` for whole numbers and
/// values without a small fraction.
fn fraction(x: f64) -> Option<String> {
    let abs = x.abs();
    if x.fract() == 0.0 || abs > 1e9 {
        return None;
    }
    let (mut h_prev, mut h) = (0_i64, 1_i64);
    let (mut k_prev, mut k) = (1_i64, 0_i64);
    let mut rest = abs;
    for _ in 0..40 {
        let a = rest.floor();
        let a_int = a as i64;
        let h_next = a_int.checked_mul(h)?.checked_add(h_prev)?;
        let k_next = a_int.checked_mul(k)?.checked_add(k_prev)?;
        if k_next > 10_000 {
            return None;
        }
        (h_prev, h, k_prev, k) = (h, h_next, k, k_next);
        if ((h as f64 / k as f64) - abs).abs() <= 1e-10 * abs.max(1.0) {
            let sign = if x < 0.0 { "-" } else { "" };
            return (k > 1).then(|| format!("{sign}{h}/{k}"));
        }
        let fractional = rest - a;
        if fractional < 1e-15 {
            return None;
        }
        rest = 1.0 / fractional;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(apply: impl FnOnce(&mut DisplaySettings)) -> Style {
        let mut d = DisplaySettings::default();
        apply(&mut d);
        Style::plain(&d)
    }

    #[test]
    fn normal_notation() {
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(15.0), "15");
        assert_eq!(format_number(-2.5), "-2.5");
        assert_eq!(format_number(1.0 / 3.0), "0.3333333333");
        assert_eq!(format_number(2.0 / 3.0 * 1000.0), "666.6666667");
        assert_eq!(format_number(1.633124e16), "1.633124E16");
        assert_eq!(format_number(1e-13), "1E-13");
        assert_eq!(format_number(-2.5e20), "-2.5E20");
        assert_eq!(format_number(9999999999.0), "9999999999");
        assert_eq!(format_number(12345678901.0), "1.23456789E10");
    }

    #[test]
    fn other_notations() {
        let fix = with(|d| d.notation = Notation::Fixed(2));
        assert_eq!(format_real(1.23456, &fix), "1.23");
        assert_eq!(format_real(-0.001, &fix), "0.00");
        let sci = with(|d| d.notation = Notation::Scientific(3));
        assert_eq!(format_real(1234.5, &sci), "1.23E3");
        let eng = with(|d| d.notation = Notation::Engineering);
        assert_eq!(format_real(12345.0, &eng), "12.345E3");
        assert_eq!(format_real(0.00042, &eng), "420E-6");
    }

    #[test]
    fn bases_fractions_dms() {
        let hex = with(|d| d.base = Base::Hexadecimal);
        assert_eq!(format_real(255.0, &hex), "0xFF");
        assert_eq!(format_real(-31.0, &hex), "-0x1F");
        assert_eq!(format_real(2.5, &hex), "2.5");
        let mut words = with(|d| d.base = Base::Hexadecimal);
        words.word_bits = Some(8);
        assert_eq!(format_real(-1.0, &words), "0xFF");
        assert_eq!(format_real(255.0, &words), "0xFF");
        assert_eq!(format_real(256.0, &words), "256");
        let bin = with(|d| d.base = Base::Binary);
        assert_eq!(format_real(5.0, &bin), "0b101");

        let frac = with(|d| d.fractions = true);
        assert_eq!(format_real(0.75, &frac), "3/4");
        assert_eq!(format_real(-7.0 / 3.0, &frac), "-7/3");
        assert_eq!(format_real(2.0, &frac), "2");
        assert_eq!(format_real(std::f64::consts::PI, &frac), "3.141592654");

        let dms_on = with(|d| d.dms = true);
        assert_eq!(format_real(12.5041666666667, &dms_on), "12°30′15″");
        assert_eq!(format_real(-0.5, &dms_on), "-0°30′0″");
    }

    #[test]
    fn complex_matrix_list() {
        let d = DisplaySettings::default();
        assert_eq!(format_value(&Complex::new(3.0, 4.0).into(), &d), "3+4i");
        let polar = with(|d| d.polar = true);
        assert_eq!(
            format_styled(&Complex::new(0.0, 2.0).into(), &polar),
            "2∠90"
        );
        assert_eq!(format_value(&Complex::new(1.0, -1.0).into(), &d), "1-i");
        assert_eq!(format_value(&Complex::new(0.0, -2.5).into(), &d), "-2.5i");
        let frac = with(|d| d.fractions = true);
        assert_eq!(
            format_styled(&Complex::new(0.0, 0.5).into(), &frac),
            "(1/2)i"
        );
        let list = Value::list_of_numbers([1.0, 2.5]).unwrap();
        assert_eq!(format_value(&list, &d), "{1, 2.5}");
    }
}
