//! Physical unit registry and dimensional classification.
//!
//! Provides lookup and classification for all 267+ base and derived SageMath units,
//! SI prefixes, and common abbreviations/plurals, enabling the friendly compiler to
//! distinguish physical units from mathematical unknown variables.

/// Checks if an identifier is a recognized physical unit symbol or alias.
#[must_use]
pub fn is_known_unit(name: &str) -> bool {
    matches!(
        name,
        "A" | "Amp" | "Amps" | "C" | "F" | "G" | "GHz" | "GJ" | "GOhm" | "GOhms" | "GPa" |
        "GW" | "Gallon" | "Gallons" | "GeV" | "Gohm" | "H" | "Hz" | "J" | "Joule" | "Joules" |
        "K" | "Kelvin" | "L" | "Liter" | "Liters" | "MHz" | "MJ" | "MN" | "MOhm" | "MOhms" |
        "MPa" | "MV" | "MW" | "MWh" | "MeV" | "Mohm" | "N" | "Ohm" | "Ohms" | "Pa" | "S" |
        "T" | "THz" | "TW" | "V" | "Volt" | "Volts" | "W" | "Watt" | "Watts" | "Wb" | "Wh" |
        "abampere" | "abamperes" | "abcoulomb" | "abcoulombs" | "abfarad" | "abfarads" |
        "abhenry" | "abhenrys" | "abmho" | "abmhos" | "abohm" | "abohms" | "abvolt" |
        "abvolts" | "acre" | "acres" | "amp" | "ampere" | "ampere_turn" | "ampere_turns" |
        "amperes" | "amps" | "amu" | "amus" | "angstrom" | "angstroms" | "apostilb" |
        "apostilbs" | "arc_minute" | "arc_minutes" | "arc_second" | "arc_seconds" | "are" |
        "ares" | "assay_ton" | "assay_tons" | "astronomical_unit" | "astronomical_units" |
        "atmosphere" | "atmospheres" | "atomic_mass_unit" | "atomic_mass_units" | "au" |
        "avoirdupois_ounce" | "avoirdupois_ounces" | "avoirdupois_pound" |
        "avoirdupois_pounds" | "bag" | "bags" | "bale" | "bales" | "bar" | "barn" | "barns" |
        "barrel" | "barrels" | "bars" | "barye" | "baryes" | "baud" | "bauds" | "becquerel" |
        "becquerels" | "biot" | "biots" | "bit" | "bits" | "board_foot" | "board_foots" |
        "bohr_magneton" | "bohr_magnetons" | "bolt" | "bolts" | "british_thermal_unit" |
        "british_thermal_units" | "btu" | "btus" | "bucket" | "buckets" | "bushel" |
        "bushels" | "butt" | "butts" | "byte" | "bytes" | "cL" | "cable_international" |
        "cable_internationals" | "cable_us" | "cal" | "caliber" | "calibers" | "calorie" |
        "calories" | "candela" | "candelas" | "candle" | "candles" | "carat" | "carats" |
        "cd" | "celsius" | "cental" | "centals" | "centigrade" | "centigrades" |
        "centiliter" | "centiliters" | "centimeter" | "centimeters" | "century" | "centurys" |
        "chain" | "chains" | "cheval_vapeur" | "cheval_vapeurs" | "cicero" | "ciceros" |
        "cl" | "cm" | "cord" | "cords" | "coulomb" | "coulombs" | "cubic_meter" |
        "cubic_meters" | "cubit" | "cubits" | "cup" | "cups" | "curie" | "curies" | "dL" |
        "dalton" | "daltons" | "day" | "days" | "decade" | "decades" | "deciliter" |
        "deciliters" | "deg" | "degC" | "degF" | "degree" | "degrees" | "denier" | "deniers" |
        "didot" | "didots" | "diopter" | "diopters" | "dl" | "drachma" | "drachmas" |
        "dtp_point" | "dtp_points" | "dyne" | "dynes" | "eV" | "electron_volt" |
        "electron_volts" | "elementary_charge" | "elementary_charges" | "elementary_entity" |
        "elementary_entitys" | "ell" | "ells" | "ephah" | "ephahs" | "erg" | "ergs" | "ev" |
        "evs" | "fahrenheit" | "fahrenheits" | "farad" | "faraday" | "faradays" | "farads" |
        "fathom" | "fathoms" | "feet" | "feets" | "fermi" | "fermis" | "fifth" | "fifths" |
        "firkin" | "firkins" | "floz" | "fluid_dram" | "fluid_drams" | "fluid_ounce" |
        "fluid_ounces" | "foot" | "foot_candle" | "foot_candles" | "foots" | "fortnight" |
        "fortnights" | "franklin" | "franklins" | "ft" | "furlong" | "furlongs" | "g" |
        "gal" | "galileo" | "galileos" | "gallon" | "gallons" | "gals" | "gauss" |
        "geepound" | "geepounds" | "gigahertz" | "gigajoule" | "gigajoules" | "gigaohm" |
        "gigaohms" | "gigapascal" | "gigawatt" | "gigawatts" | "gilbert" | "gilberts" |
        "gill" | "gills" | "grade" | "grades" | "grain" | "grains" | "gram" | "gram_weight" |
        "gram_weights" | "grams" | "gravity" | "gravitys" | "gray" | "grays" |
        "gross_hundredweight" | "gross_hundredweights" | "hPa" | "hand" | "hands" |
        "hectare" | "hectares" | "hectopascal" | "hefnerkerze" | "hefnerkerzes" | "henries" |
        "henry" | "henrys" | "hertz" | "hertzs" | "hogshead" | "hogsheads" | "horsepower" |
        "horsepowers" | "hour" | "hours" | "hp" | "hr" | "hundredweight" | "hundredweights" |
        "imperial_gallon" | "imperial_gallons" | "imperial_pint" | "imperial_pints" | "in" |
        "inch" | "inch_mercury" | "inch_mercurys" | "inches" | "inchs" | "jeroboam" |
        "jeroboams" | "jigger" | "jiggers" | "joule" | "joules" | "kA" | "kHz" | "kJ" | "kN" |
        "kOhm" | "kOhms" | "kPa" | "kV" | "kW" | "kWh" | "kayser" | "kaysers" | "kcal" |
        "keV" | "kelvin" | "kelvins" | "kg" | "kiloamp" | "kiloamps" | "kilocalorie" |
        "kilocalories" | "kilogram" | "kilogram_force" | "kilogram_forces" |
        "kilogram_weight" | "kilogram_weights" | "kilograms" | "kilohertz" | "kilojoule" |
        "kilojoules" | "kilometer" | "kilometers" | "kilonewton" | "kilonewtons" | "kiloohm" |
        "kiloohms" | "kilopascal" | "kilopascals" | "kilovolt" | "kilovolts" | "kilowatt" |
        "kilowatt_hour" | "kilowatt_hours" | "kilowatts" | "km" | "knot" | "knots" | "kohm" |
        "kph" | "l" | "lambert" | "lamberts" | "lb" | "lbs" | "league" | "leagues" | "libra" |
        "libras" | "light_year" | "light_years" | "link" | "links" | "liter" | "liters" |
        "litre" | "litres" | "lm" | "long_ton" | "long_tons" | "lumen" | "lumens" | "lumerg" |
        "lumergs" | "lux" | "luxs" | "lx" | "ly" | "m" | "mA" | "mC" | "mF" | "mH" | "mJ" |
        "mL" | "mN" | "mT" | "mV" | "mW" | "magnum" | "magnums" | "maxwell" | "maxwells" |
        "mbar" | "megahertz" | "megajoule" | "megajoules" | "megaohm" | "megaohms" |
        "megapascal" | "megapascals" | "megavolt" | "megavolts" | "megawatt" |
        "megawatt_hour" | "megawatts" | "meter" | "meters" | "metre" | "metres" |
        "metric_ton" | "metric_tons" | "mg" | "mho" | "mhos" | "mi" | "microamp" |
        "microamps" | "microcoulomb" | "microfarad" | "microfarads" | "microgram" |
        "micrograms" | "microhenries" | "microhenry" | "microliter" | "microliters" |
        "micron" | "microns" | "microsecond" | "microseconds" | "microvolt" | "microvolts" |
        "microwatt" | "microwatts" | "mil" | "mile" | "miles" | "millenium" | "milleniums" |
        "milliamp" | "milliampere" | "milliamps" | "millibar" | "millibars" | "millicoulomb" |
        "millifarad" | "millifarads" | "milligram" | "milligrams" | "millihenries" |
        "millihenry" | "millijoule" | "millijoules" | "milliliter" | "milliliters" |
        "millimeter" | "millimeter_mercury" | "millimeter_mercurys" | "millimeters" |
        "milliohm" | "milliohms" | "millisecond" | "milliseconds" | "millivolt" |
        "millivolts" | "milliwatt" | "milliwatts" | "mils" | "min" | "mina" | "minas" |
        "minim" | "minims" | "minute" | "minutes" | "ml" | "mm" | "mmhg" | "mmhgs" | "mmol" |
        "mohm" | "mol" | "mole" | "moles" | "month" | "months" | "mph" | "ms" | "nA" | "nC" |
        "nF" | "nH" | "nV" | "nanocoulomb" | "nanofarad" | "nanofarads" | "nanohenries" |
        "nanohenry" | "nanometer" | "nanometers" | "nanosecond" | "nanoseconds" |
        "nautical_mile" | "nautical_miles" | "net_hundredweight" | "net_hundredweights" |
        "newton" | "newtons" | "nibble" | "nibbles" | "nit" | "nits" | "nm" | "nmi" |
        "noggin" | "noggins" | "ns" | "nuclear_magneton" | "nuclear_magnetons" | "obol" |
        "obols" | "oersted" | "oersteds" | "ohm" | "ohms" | "omer" | "omers" | "ounce" |
        "ounce_troy" | "ounce_troys" | "ounces" | "oz" | "pA" | "pC" | "pF" | "pa" |
        "parsec" | "parsecs" | "pas" | "pascal" | "pascals" | "peck" | "pecks" |
        "pennyweight" | "pennyweights" | "perch" | "perchs" | "phot" | "phots" | "pica" |
        "picas" | "picocoulomb" | "picofarad" | "picofarads" | "pint" | "pints" | "pm" |
        "poise" | "poises" | "pole" | "poles" | "pondus" | "pony" | "ponys" | "pound" |
        "pound_force" | "pound_forces" | "pound_troy" | "pound_troys" | "pound_weight" |
        "pound_weights" | "poundal" | "poundals" | "pounds" | "pounds_per_square_inch" |
        "pounds_per_square_inchs" | "psi" | "psis" | "pt" | "pts" | "puncheon" | "puncheons" |
        "qt" | "qts" | "quadrant" | "quadrants" | "quart" | "quarts" | "quintal" |
        "quintals" | "rad" | "radian" | "radians" | "rads" | "rankine" | "rankines" |
        "register_ton" | "register_tons" | "reyn" | "reyns" | "rhes" | "right_angle" |
        "right_angles" | "rod" | "rods" | "roentgen" | "roentgens" | "rontgen" | "rontgens" |
        "rood" | "roods" | "rope" | "ropes" | "rutherford" | "rutherfords" | "rydberg" |
        "rydbergs" | "s" | "seam" | "seams" | "sec" | "second" | "seconds" | "section" |
        "sections" | "shekel" | "shekels" | "short_hundredweight" | "short_hundredweights" |
        "short_ton" | "short_tons" | "shot" | "shots" | "sidereal_day" | "sidereal_days" |
        "sidereal_second" | "sidereal_seconds" | "sidereal_year" | "sidereal_years" |
        "siemens" | "skein" | "skeins" | "slug" | "slugs" | "solar_mass" | "square_chain" |
        "square_chains" | "square_meter" | "square_meters" | "stadion" | "stadions" |
        "stadium" | "stadiums" | "statampere" | "statamperes" | "statcoulomb" |
        "statcoulombs" | "statfarad" | "statfarads" | "stathenry" | "stathenrys" | "statohm" |
        "statohms" | "statute_mile" | "statute_miles" | "statvolt" | "statvolts" |
        "steradian" | "steradians" | "stere" | "steres" | "stilb" | "stilbs" | "stokes" |
        "stone" | "stones" | "survey_foot" | "survey_foots" | "survey_mile" | "survey_miles" |
        "tablespoon" | "tablespoons" | "talbot" | "talbots" | "talent" | "talents" | "tbsp" |
        "teaspoon" | "teaspoons" | "terahertz" | "terawatt" | "tesla" | "teslas" | "tex" |
        "texs" | "therm" | "therms" | "ton" | "ton_force" | "ton_forces" | "tonne" |
        "tonnes" | "tons" | "torr" | "torrs" | "township" | "townships" | "tropical_year" |
        "tropical_years" | "tsp" | "tun" | "tuns" | "uA" | "uC" | "uF" | "uH" | "uL" | "uT" |
        "uV" | "uW" | "ug" | "uk_gallon" | "uk_gallons" | "uk_pint" | "uk_pints" | "ul" |
        "um" | "us" | "volt" | "volts" | "watt" | "watt_hour" | "watt_hours" | "watts" |
        "weber" | "webers" | "week" | "weeks" | "wey" | "weys" | "wine_bottle" |
        "wine_bottles" | "wk" | "x_unit" | "x_units" | "yard" | "yards" | "yd" | "year" |
        "years" | "yr"
    )
}

/// Checks if an identifier is an unambiguous physical unit (e.g. `mi`, `km`, `ft`, `mph`, `kohm`, `gal`, `mA`, `mV`, `V`, `A`, `K`, `W`).
///
/// Lowercase single characters like `m` (meters), `s` (seconds), `g` (grams), `l` (liters), and `min` (minutes)
/// are considered ambiguous to avoid shadowing algebraic variables or functions in expressions.
#[must_use]
pub fn is_unambiguous_unit(name: &str) -> bool {
    is_known_unit(name) && name != "m" && name != "s" && name != "g" && name != "l" && name != "min"
}

/// Returns true if an expression contains any recognized physical unit identifier.
#[must_use]
pub fn contains_any_unit(expression: &str) -> bool {
    let chars: Vec<char> = expression.chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        if chars[idx] == '_' || chars[idx].is_alphabetic() {
            let mut j = idx + 1;
            while j < chars.len() && (chars[j] == '_' || chars[j].is_alphanumeric()) {
                j += 1;
            }
            let name: String = chars[idx..j].iter().collect();
            if is_known_unit(&name) {
                return true;
            }
            idx = j;
        } else {
            idx += 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_lookup() {
        assert!(is_known_unit("mi"));
        assert!(is_known_unit("km"));
        assert!(is_known_unit("mph"));
        assert!(is_known_unit("kg"));
        assert!(is_known_unit("hr"));
        assert!(is_known_unit("A"));
        assert!(is_known_unit("Amp"));
        assert!(is_known_unit("mA"));
        assert!(is_known_unit("V"));
        assert!(is_known_unit("Volt"));
        assert!(is_known_unit("mV"));
        assert!(is_known_unit("kV"));
        assert!(is_known_unit("ohm"));
        assert!(is_known_unit("kohm"));
        assert!(is_known_unit("W"));
        assert!(is_known_unit("kW"));
        assert!(is_known_unit("L"));
        assert!(is_known_unit("Liter"));
        assert!(is_known_unit("gal"));
        assert!(is_known_unit("gallon"));
        assert!(is_known_unit("K"));
        assert!(is_known_unit("Kelvin"));
        assert!(is_known_unit("degC"));
        assert!(is_known_unit("degF"));
        assert!(!is_known_unit("x"));
        assert!(!is_known_unit("foo"));
    }

    #[test]
    fn test_unambiguous_units() {
        assert!(is_unambiguous_unit("mi"));
        assert!(is_unambiguous_unit("km"));
        assert!(is_unambiguous_unit("Volt"));
        assert!(is_unambiguous_unit("Amp"));
        assert!(is_unambiguous_unit("kohm"));
        assert!(is_unambiguous_unit("gal"));
        assert!(is_unambiguous_unit("V"));
        assert!(is_unambiguous_unit("A"));
        assert!(!is_unambiguous_unit("m"));
        assert!(!is_unambiguous_unit("s"));
        assert!(!is_unambiguous_unit("g"));
        assert!(!is_unambiguous_unit("l"));
        assert!(!is_unambiguous_unit("min"));
    }
}
