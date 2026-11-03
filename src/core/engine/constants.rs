//! Named constants. To add one, add a line to `CONSTANTS`; the lexer picks
//! it up automatically.
//!
//! Physical constants start with `_` (e.g. `_c`), so they never clash with
//! function or variable names. Values are CODATA 2018, in SI units.

use crate::core::engine::value::Complex;

pub struct ConstantDef {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub value: Complex,
    pub summary: &'static str,
}

const fn real(
    name: &'static str,
    aliases: &'static [&'static str],
    value: f64,
    summary: &'static str,
) -> ConstantDef {
    ConstantDef {
        name,
        aliases,
        value: Complex::new(value, 0.0),
        summary,
    }
}

pub const CONSTANTS: &[ConstantDef] = &[
    real(
        "π",
        &["pi"],
        std::f64::consts::PI,
        "ratio of a circle's circumference to its diameter",
    ),
    real(
        "e",
        &[],
        std::f64::consts::E,
        "base of the natural logarithm",
    ),
    ConstantDef {
        name: "i",
        aliases: &[],
        value: Complex::I,
        summary: "imaginary unit, i² = −1",
    },
    real("_c", &[], 299_792_458.0, "speed of light in vacuum, m/s"),
    real("_h", &[], 6.626_070_15e-34, "Planck constant, J·s"),
    real(
        "_hbar",
        &["_ħ"],
        1.054_571_817e-34,
        "reduced Planck constant, J·s",
    ),
    real(
        "_G",
        &[],
        6.674_30e-11,
        "gravitational constant, m³/(kg·s²)",
    ),
    real("_gn", &["_g0"], 9.806_65, "standard gravity, m/s²"),
    real("_NA", &[], 6.022_140_76e23, "Avogadro constant, 1/mol"),
    real("_kB", &["_k"], 1.380_649e-23, "Boltzmann constant, J/K"),
    real("_R", &[], 8.314_462_618, "molar gas constant, J/(mol·K)"),
    real("_F", &[], 96_485.332_12, "Faraday constant, C/mol"),
    real("_qe", &[], 1.602_176_634e-19, "elementary charge, C"),
    real("_me", &[], 9.109_383_701_5e-31, "electron mass, kg"),
    real("_mp", &[], 1.672_621_923_69e-27, "proton mass, kg"),
    real("_mn", &[], 1.674_927_498_04e-27, "neutron mass, kg"),
    real("_u", &[], 1.660_539_066_60e-27, "atomic mass constant, kg"),
    real(
        "_eps0",
        &["_ε0"],
        8.854_187_812_8e-12,
        "vacuum permittivity, F/m",
    ),
    real(
        "_mu0",
        &["_μ0"],
        1.256_637_062_12e-6,
        "vacuum permeability, N/A²",
    ),
    real("_ke", &[], 8.987_551_792_3e9, "Coulomb constant, N·m²/C²"),
    real(
        "_sigma",
        &["_σ"],
        5.670_374_419e-8,
        "Stefan–Boltzmann constant, W/(m²·K⁴)",
    ),
    real("_a0", &[], 5.291_772_109_03e-11, "Bohr radius, m"),
    real("_Rinf", &[], 10_973_731.568_160, "Rydberg constant, 1/m"),
    real(
        "_alpha",
        &["_α"],
        7.297_352_569_3e-3,
        "fine-structure constant",
    ),
    real(
        "_Vm",
        &[],
        22.413_969_54e-3,
        "molar volume of an ideal gas at 0 °C and 1 atm, m³/mol",
    ),
    real("_mmu", &["_mμ"], 1.883_531_627e-28, "muon mass, kg"),
    real(
        "_muN",
        &["_μN"],
        5.050_783_746_1e-27,
        "nuclear magneton, J/T",
    ),
    real("_muB", &["_μB"], 9.274_010_078_3e-24, "Bohr magneton, J/T"),
    real(
        "_re",
        &[],
        2.817_940_326_2e-15,
        "classical electron radius, m",
    ),
    real(
        "_lambdac",
        &["_λc"],
        2.426_310_238_67e-12,
        "Compton wavelength, m",
    ),
    real(
        "_gammap",
        &["_γp"],
        2.675_221_874_4e8,
        "proton gyromagnetic ratio, 1/(s·T)",
    ),
    real(
        "_lambdacp",
        &["_λcp"],
        1.321_409_855_39e-15,
        "proton Compton wavelength, m",
    ),
    real(
        "_lambdacn",
        &["_λcn"],
        1.319_590_905_81e-15,
        "neutron Compton wavelength, m",
    ),
    real(
        "_mup",
        &["_μp"],
        1.410_606_797_36e-26,
        "proton magnetic moment, J/T",
    ),
    real(
        "_mue",
        &["_μe"],
        -9.284_764_704_3e-24,
        "electron magnetic moment, J/T",
    ),
    real(
        "_mun",
        &["_μn"],
        -9.662_365_1e-27,
        "neutron magnetic moment, J/T",
    ),
    real(
        "_mumu",
        &["_μμ"],
        -4.490_448_30e-26,
        "muon magnetic moment, J/T",
    ),
    real(
        "_c1",
        &[],
        3.741_771_852e-16,
        "first radiation constant, W·m²",
    ),
    real(
        "_c2",
        &[],
        1.438_776_877e-2,
        "second radiation constant, m·K",
    ),
    real(
        "_phi0",
        &["_φ0"],
        2.067_833_848e-15,
        "magnetic flux quantum, Wb",
    ),
    real("_G0", &[], 7.748_091_729e-5, "conductance quantum, S"),
    real(
        "_Z0",
        &[],
        376.730_313_668,
        "characteristic impedance of vacuum, Ω",
    ),
    real("_t0", &[], 273.15, "0 °C in kelvin"),
    real("_Patm", &[], 101_325.0, "standard atmosphere, Pa"),
];

/// A constant, as the lexer hands it to the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConstId(u16);

impl ConstId {
    pub fn def(self) -> &'static ConstantDef {
        &CONSTANTS[self.0 as usize]
    }

    pub fn name(self) -> &'static str {
        self.def().name
    }

    /// Every constant with every spelling.
    pub(crate) fn all_names() -> impl Iterator<Item = (&'static str, Self)> {
        CONSTANTS.iter().enumerate().flat_map(|(i, def)| {
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
