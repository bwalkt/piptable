//! Cell data access helpers with short/long key fallback.

use serde_json::Value as JsonValue;

/// Retrieves a field using a short or long key fallback.
fn get_field<'a>(value: Option<&'a JsonValue>, short: &str, long: &str) -> Option<&'a JsonValue> {
    let JsonValue::Object(map) = value? else {
        return None;
    };
    map.get(short).or_else(|| map.get(long))
}

/// Returns the user-entered value, preferring short key `ue` over `userEnteredValue`.
pub fn get_cell_user_entered_value(cell: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(cell, "ue", "userEnteredValue")
}

/// Returns the effective value, preferring short key `ev` over `effectiveValue`.
pub fn get_cell_effective_value(cell: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(cell, "ev", "effectiveValue")
}

/// Returns the user-entered format, preferring short key `uf` over `userEnteredFormat`.
pub fn get_cell_user_entered_format(cell: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(cell, "uf", "userEnteredFormat")
}

/// Returns the effective format, preferring short key `ef` over `effectiveFormat`.
pub fn get_cell_effective_format(cell: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(cell, "ef", "effectiveFormat")
}

/// Returns the formatted value, preferring short key `fv` over `formattedValue`.
pub fn get_cell_formatted_value(cell: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(cell, "fv", "formattedValue")
}

/// Returns the style id, preferring short key `sid` over `styleId`.
pub fn get_style_id(style_ref: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(style_ref, "sid", "styleId")
}

/// Returns the number extended value, preferring short key `nv` over `numberValue`.
pub fn get_extended_value_number(ext_value: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(ext_value, "nv", "numberValue")
}

/// Returns the string extended value, preferring short key `sv` over `stringValue`.
pub fn get_extended_value_string(ext_value: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(ext_value, "sv", "stringValue")
}

/// Returns the boolean extended value, preferring short key `bv` over `boolValue`.
pub fn get_extended_value_bool(ext_value: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(ext_value, "bv", "boolValue")
}

/// Returns the formula extended value, preferring short key `fv` over `formulaValue`.
pub fn get_extended_value_formula(ext_value: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(ext_value, "fv", "formulaValue")
}

/// Returns the error extended value, preferring short key `ev` over `errorValue`.
pub fn get_extended_value_error(ext_value: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(ext_value, "ev", "errorValue")
}

/// Cell data helper tests.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Verifies cell value fallback keys.
    #[test]
    fn test_cell_value_fallbacks() {
        let cell = json!({"ue": 1, "userEnteredValue": 2, "fv": "x"});
        assert_eq!(get_cell_user_entered_value(Some(&cell)).unwrap(), &json!(1));
        assert_eq!(get_cell_formatted_value(Some(&cell)).unwrap(), &json!("x"));
    }

    /// Verifies style id fallback keys.
    #[test]
    fn test_style_id_fallback() {
        let style = json!({"styleId": "abc"});
        assert_eq!(get_style_id(Some(&style)).unwrap(), &json!("abc"));
    }
}
