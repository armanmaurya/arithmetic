//! Units for `convert(value, _from, _to)`. To add one, add a line to
//! `UNITS`; the lexer picks it up automatically.
//!
//! Every unit is `scale × value + offset` in the SI unit of its quantity.
//! The offset is only non-zero for temperatures.

use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    Length,
    Area,
    Volume,
    Mass,
    Time,
    Speed,
    Force,
    Energy,
    Power,
    Pressure,
    Temperature,
    Angle,
    Data,
}

pub struct UnitDef {
    pub name: &'static str,
    pub quantity: Quantity,
    pub scale: f64,
    pub offset: f64,
    pub summary: &'static str,
}

const fn unit(
    name: &'static str,
    quantity: Quantity,
    scale: f64,
    summary: &'static str,
) -> UnitDef {
    UnitDef {
        name,
        quantity,
        scale,
        offset: 0.0,
        summary,
    }
}

use Quantity::*;

pub const UNITS: &[UnitDef] = &[
    // length, in metres
    unit("_m", Length, 1.0, "metre"),
    unit("_km", Length, 1e3, "kilometre"),
    unit("_cm", Length, 1e-2, "centimetre"),
    unit("_mm", Length, 1e-3, "millimetre"),
    unit("_um", Length, 1e-6, "micrometre"),
    unit("_nm", Length, 1e-9, "nanometre"),
    unit("_in", Length, 0.0254, "inch"),
    unit("_ft", Length, 0.3048, "foot"),
    unit("_yd", Length, 0.9144, "yard"),
    unit("_mi", Length, 1609.344, "mile"),
    unit("_nmi", Length, 1852.0, "nautical mile"),
    unit("_au", Length, 1.495_978_707e11, "astronomical unit"),
    unit("_ly", Length, 9.460_730_472_580_8e15, "light year"),
    unit("_pc", Length, 3.085_677_581_491_367e16, "parsec"),
    // area, in square metres
    unit("_m2", Area, 1.0, "square metre"),
    unit("_km2", Area, 1e6, "square kilometre"),
    unit("_cm2", Area, 1e-4, "square centimetre"),
    unit("_ha", Area, 1e4, "hectare"),
    unit("_acre", Area, 4_046.856_422_4, "acre"),
    unit("_ft2", Area, 0.092_903_04, "square foot"),
    unit("_in2", Area, 6.4516e-4, "square inch"),
    unit("_mi2", Area, 2_589_988.110_336, "square mile"),
    // volume, in cubic metres
    unit("_m3", Volume, 1.0, "cubic metre"),
    unit("_cm3", Volume, 1e-6, "cubic centimetre"),
    unit("_L", Volume, 1e-3, "litre"),
    unit("_mL", Volume, 1e-6, "millilitre"),
    unit("_galUS", Volume, 3.785_411_784e-3, "US gallon"),
    unit("_galUK", Volume, 4.546_09e-3, "imperial gallon"),
    unit("_ptUS", Volume, 4.731_764_73e-4, "US pint"),
    unit("_flozUS", Volume, 2.957_352_956_25e-5, "US fluid ounce"),
    // mass, in kilograms
    unit("_kg", Mass, 1.0, "kilogram"),
    unit("_g", Mass, 1e-3, "gram"),
    unit("_mg", Mass, 1e-6, "milligram"),
    unit("_t", Mass, 1e3, "tonne"),
    unit("_lb", Mass, 0.453_592_37, "pound"),
    unit("_oz", Mass, 0.028_349_523_125, "ounce"),
    unit("_st", Mass, 6.350_293_18, "stone"),
    // time, in seconds
    unit("_s", Time, 1.0, "second"),
    unit("_ms", Time, 1e-3, "millisecond"),
    unit("_min", Time, 60.0, "minute"),
    unit("_hr", Time, 3600.0, "hour"),
    unit("_day", Time, 86_400.0, "day"),
    unit("_week", Time, 604_800.0, "week"),
    unit("_yr", Time, 31_557_600.0, "Julian year (365.25 days)"),
    // speed, in metres per second
    unit("_mps", Speed, 1.0, "metre per second"),
    unit("_kmh", Speed, 1.0 / 3.6, "kilometre per hour"),
    unit("_mph", Speed, 0.447_04, "mile per hour"),
    unit("_knot", Speed, 1852.0 / 3600.0, "knot"),
    // force, in newtons
    unit("_N", Force, 1.0, "newton"),
    unit("_kN", Force, 1e3, "kilonewton"),
    unit("_kgf", Force, 9.806_65, "kilogram-force"),
    unit("_lbf", Force, 4.448_221_615_260_5, "pound-force"),
    unit("_dyn", Force, 1e-5, "dyne"),
    // energy, in joules
    unit("_J", Energy, 1.0, "joule"),
    unit("_kJ", Energy, 1e3, "kilojoule"),
    unit("_cal", Energy, 4.184, "calorie"),
    unit("_kcal", Energy, 4184.0, "kilocalorie"),
    unit("_eV", Energy, 1.602_176_634e-19, "electronvolt"),
    unit("_Wh", Energy, 3600.0, "watt-hour"),
    unit("_kWh", Energy, 3.6e6, "kilowatt-hour"),
    unit("_Btu", Energy, 1_055.055_852_62, "British thermal unit"),
    unit("_kgfm", Energy, 9.806_65, "kilogram-force metre"),
    // power, in watts
    unit("_W", Power, 1.0, "watt"),
    unit("_kW", Power, 1e3, "kilowatt"),
    unit("_hp", Power, 745.699_871_582_270_2, "mechanical horsepower"),
    // pressure, in pascals
    unit("_Pa", Pressure, 1.0, "pascal"),
    unit("_kPa", Pressure, 1e3, "kilopascal"),
    unit("_bar", Pressure, 1e5, "bar"),
    unit("_atm", Pressure, 101_325.0, "standard atmosphere"),
    unit("_mmHg", Pressure, 133.322_387_415, "millimetre of mercury"),
    unit("_psi", Pressure, 6_894.757_293_168, "pound per square inch"),
    unit(
        "_kgfcm2",
        Pressure,
        98_066.5,
        "kilogram-force per square centimetre",
    ),
    // temperature, in kelvin
    unit("_K", Temperature, 1.0, "kelvin"),
    UnitDef {
        name: "_degC",
        quantity: Temperature,
        scale: 1.0,
        offset: 273.15,
        summary: "degree Celsius",
    },
    UnitDef {
        name: "_degF",
        quantity: Temperature,
        scale: 5.0 / 9.0,
        offset: 273.15 - 32.0 * 5.0 / 9.0,
        summary: "degree Fahrenheit",
    },
    // angle, in radians
    unit("_rad", Angle, 1.0, "radian"),
    unit("_deg", Angle, PI / 180.0, "degree"),
    unit("_grad", Angle, PI / 200.0, "gradian"),
    unit("_arcmin", Angle, PI / 10_800.0, "minute of arc"),
    unit("_arcsec", Angle, PI / 648_000.0, "second of arc"),
    unit("_rev", Angle, 2.0 * PI, "revolution"),
    // data, in bytes
    unit("_B", Data, 1.0, "byte"),
    unit("_bit", Data, 0.125, "bit"),
    unit("_KB", Data, 1e3, "kilobyte (1000 bytes)"),
    unit("_MB", Data, 1e6, "megabyte (10⁶ bytes)"),
    unit("_GB", Data, 1e9, "gigabyte (10⁹ bytes)"),
    unit("_KiB", Data, 1024.0, "kibibyte"),
    unit("_MiB", Data, 1_048_576.0, "mebibyte"),
    unit("_GiB", Data, 1_073_741_824.0, "gibibyte"),
];

/// A unit, as the lexer hands it to the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitId(u16);

impl UnitId {
    pub fn def(self) -> &'static UnitDef {
        &UNITS[self.0 as usize]
    }

    pub fn name(self) -> &'static str {
        self.def().name
    }

    pub(crate) fn all_names() -> impl Iterator<Item = (&'static str, Self)> {
        UNITS
            .iter()
            .enumerate()
            .map(|(i, def)| (def.name, Self(i as u16)))
    }

    pub fn lookup(name: &str) -> Option<Self> {
        Self::all_names()
            .find(|(n, _)| *n == name)
            .map(|(_, id)| id)
    }
}

/// Converts `value` from one unit to another of the same quantity.
pub fn convert(value: f64, from: UnitId, to: UnitId) -> Option<f64> {
    let (from, to) = (from.def(), to.def());
    if from.quantity != to.quantity {
        return None;
    }
    let si = value * from.scale + from.offset;
    Some((si - to.offset) / to.scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conv(value: f64, from: &str, to: &str) -> Option<f64> {
        convert(value, UnitId::lookup(from)?, UnitId::lookup(to)?)
    }

    #[test]
    fn conversions() {
        let close = |a: Option<f64>, b: f64| (a.unwrap() - b).abs() < 1e-9 * b.abs().max(1.0);
        assert!(close(conv(1.0, "_in", "_cm"), 2.54));
        assert!(close(conv(100.0, "_degC", "_degF"), 212.0));
        assert!(close(conv(-40.0, "_degF", "_degC"), -40.0));
        assert!(close(conv(0.0, "_degC", "_K"), 273.15));
        assert!(close(conv(1.0, "_hr", "_s"), 3600.0));
        assert!(close(conv(180.0, "_deg", "_rad"), PI));
        assert_eq!(conv(1.0, "_m", "_s"), None);
    }

    #[test]
    fn names_are_unique() {
        let mut names: Vec<_> = UNITS.iter().map(|u| u.name).collect();
        names.sort();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len);
        assert!(UNITS.iter().all(|u| u.name.starts_with('_')));
    }
}
