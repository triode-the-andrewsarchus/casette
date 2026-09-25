//! Physical unit registry and dimensional classification.
//!
//! Provides lookup and classification for standard SI and Imperial units,
//! enabling the friendly compiler to distinguish physical units from
//! mathematical unknown variables.

/// Checks if an identifier is a recognized physical unit symbol or alias.
#[must_use]
pub fn is_known_unit(name: &str) -> bool {
    matches!(
        name,
        // Length
        "m" | "meter"
            | "meters"
            | "km"
            | "kilometer"
            | "kilometers"
            | "cm"
            | "centimeter"
            | "mm"
            | "millimeter"
            | "mi"
            | "mile"
            | "miles"
            | "ft"
            | "foot"
            | "feet"
            | "inch"
            | "inches"
            | "yd"
            | "yard"
            | "yards"
            // Time
            | "s"
            | "sec"
            | "second"
            | "seconds"
            | "ms"
            | "min"
            | "minute"
            | "minutes"
            | "hr"
            | "hour"
            | "hours"
            | "day"
            | "days"
            | "yr"
            | "year"
            // Mass
            | "kg"
            | "kilogram"
            | "kilograms"
            | "g"
            | "gram"
            | "grams"
            | "mg"
            | "lb"
            | "lbs"
            | "pound"
            | "pounds"
            | "oz"
            | "ounce"
            // Force, Work, Power, Pressure
            | "N"
            | "newton"
            | "J"
            | "joule"
            | "W"
            | "watt"
            | "kW"
            | "Pa"
            | "pascal"
            | "bar"
            | "atm"
            | "psi"
            // Speed & Angles
            | "mph"
            | "kph"
            | "deg"
            | "degree"
            | "degrees"
            | "rad"
            | "radian"
            | "radians"
    )
}

/// Checks if an identifier is an unambiguous multi-character unit (e.g. `mi`, `km`, `ft`, `mph`).
///
/// Unambiguous units are never treated as algebraic variables unless explicitly bound
/// (e.g. `solve ... for mi`).
#[must_use]
pub fn is_unambiguous_unit(name: &str) -> bool {
    is_known_unit(name) && name != "m" && name != "s" && name != "g" && name != "min"
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
    fn test_unit_recognition() {
        assert!(is_known_unit("mi"));
        assert!(is_known_unit("km"));
        assert!(is_known_unit("mph"));
        assert!(is_known_unit("kg"));
        assert!(is_known_unit("hr"));
        assert!(!is_known_unit("x"));
        assert!(!is_known_unit("foo"));

        assert!(is_unambiguous_unit("mi"));
        assert!(is_unambiguous_unit("km"));
        assert!(!is_unambiguous_unit("m"));
    }
}
