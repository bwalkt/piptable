//! Cell data access helpers with short/long key fallback.

use serde_json::Value as JsonValue;

fn get_field<'a>(value: Option<&'a JsonValue>, short: &str, long: &str) -> Option<&'a JsonValue> {
    let JsonValue::Object(map) = value? else {
        return None;
    };
    map.get(short).or_else(|| map.get(long))
}

pub fn get_cell_user_entered_value(cell: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(cell, "ue", "userEnteredValue")
}

pub fn get_cell_effective_value(cell: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(cell, "ev", "effectiveValue")
}

pub fn get_cell_user_entered_format(cell: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(cell, "uf", "userEnteredFormat")
}

pub fn get_cell_effective_format(cell: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(cell, "ef", "effectiveFormat")
}

pub fn get_cell_formatted_value(cell: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(cell, "fv", "formattedValue")
}

pub fn get_style_id(style_ref: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(style_ref, "sid", "styleId")
}

pub fn get_extended_value_number(ext_value: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(ext_value, "nv", "numberValue")
}

pub fn get_extended_value_string(ext_value: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(ext_value, "sv", "stringValue")
}

pub fn get_extended_value_bool(ext_value: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(ext_value, "bv", "boolValue")
}

pub fn get_extended_value_formula(ext_value: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(ext_value, "fv", "formulaValue")
}

pub fn get_extended_value_error(ext_value: Option<&JsonValue>) -> Option<&JsonValue> {
    get_field(ext_value, "ev", "errorValue")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cell_value_fallbacks() {
        let cell = json!({"ue": 1, "userEnteredValue": 2, "fv": "x"});
        assert_eq!(get_cell_user_entered_value(Some(&cell)).unwrap(), &json!(1));
        assert_eq!(get_cell_formatted_value(Some(&cell)).unwrap(), &json!("x"));
    }

    #[test]
    fn test_style_id_fallback() {
        let style = json!({"styleId": "abc"});
        assert_eq!(get_style_id(Some(&style)).unwrap(), &json!("abc"));
    }
}
