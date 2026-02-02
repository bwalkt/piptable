use chrono::{DateTime, Utc};
use piptable_primitives::Value;

#[derive(Debug, Clone, Default)]
pub struct SsfFormatOptions<'a> {
    /// Locale support is not implemented yet.
    pub locale: Option<&'a str>,
}

#[derive(Debug, Clone)]
struct FormatSection {
    pattern: String,
    color: Option<String>,
}

const KNOWN_COLORS: [&str; 8] = [
    "BLACK", "WHITE", "RED", "GREEN", "BLUE", "YELLOW", "MAGENTA", "CYAN",
];

pub fn ssf_format(pattern: &str, value: &Value, _opts: Option<SsfFormatOptions<'_>>) -> String {
    let section = select_section(pattern, value);
    let mut section = section.unwrap_or(FormatSection {
        pattern: pattern.to_string(),
        color: None,
    });
    section.pattern = strip_color_tokens(&section.pattern);

    match value {
        Value::Empty => String::new(),
        Value::Bool(b) => format_text(&section.pattern, if *b { "TRUE" } else { "FALSE" }),
        Value::String(s) => format_text(&section.pattern, s),
        Value::Error(err) => err.label().to_string(),
        Value::Array(_) => "#VALUE!".to_string(),
        Value::Int(n) => format_number_or_date(&section.pattern, *n as f64),
        Value::Float(f) => format_number_or_date(&section.pattern, *f),
    }
}

pub fn ssf_format_color(pattern: &str, value: &Value) -> Option<String> {
    let section = select_section(pattern, value)?;
    section.color
}

fn select_section(pattern: &str, value: &Value) -> Option<FormatSection> {
    let sections: Vec<&str> = pattern.split(';').collect();

    let section = match value {
        Value::String(_) => sections.get(3).copied().unwrap_or(sections[0]),
        Value::Int(n) => choose_numeric_section(*n as f64, &sections),
        Value::Float(f) => choose_numeric_section(*f, &sections),
        Value::Bool(_) | Value::Empty | Value::Error(_) | Value::Array(_) => sections[0],
    };

    let color = extract_color(section);
    Some(FormatSection {
        pattern: section.to_string(),
        color,
    })
}

fn choose_numeric_section<'a>(value: f64, sections: &'a [&'a str]) -> &'a str {
    if value.is_nan() {
        return sections[0];
    }
    if value < 0.0 {
        sections.get(1).copied().unwrap_or(sections[0])
    } else if value == 0.0 {
        sections.get(2).copied().unwrap_or(sections[0])
    } else {
        sections[0]
    }
}

fn strip_color_tokens(section: &str) -> String {
    let mut out = String::with_capacity(section.len());
    let mut chars = section.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            let mut token = String::new();
            for next in chars.by_ref() {
                if next == ']' {
                    break;
                }
                token.push(next);
            }
            let upper = token.trim().to_ascii_uppercase();
            if KNOWN_COLORS.iter().any(|c| *c == upper) {
                continue;
            }
            out.push('[');
            out.push_str(&token);
            out.push(']');
        } else {
            out.push(ch);
        }
    }
    out
}

fn extract_color(section: &str) -> Option<String> {
    let mut chars = section.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            let mut token = String::new();
            for next in chars.by_ref() {
                if next == ']' {
                    break;
                }
                token.push(next);
            }
            let trimmed = token.trim();
            if KNOWN_COLORS
                .iter()
                .any(|color| color.eq_ignore_ascii_case(trimmed))
            {
                return Some(trimmed.to_ascii_uppercase());
            }
        }
    }
    None
}

fn format_text(pattern: &str, text: &str) -> String {
    if pattern.contains('@') {
        pattern.replace('@', text)
    } else {
        text.to_string()
    }
}

fn format_number_or_date(pattern: &str, value: f64) -> String {
    if is_date_pattern(pattern) {
        return format_date_pattern(pattern, value);
    }
    format_number_pattern(pattern, value)
}

fn format_number_pattern(pattern: &str, value: f64) -> String {
    let mut working = pattern.to_string();
    let mut prefix = String::new();
    let mut suffix = String::new();

    let mut first_placeholder = None;
    let mut last_placeholder = None;
    for (idx, ch) in working.char_indices() {
        if matches!(ch, '0' | '#' | '?') {
            if first_placeholder.is_none() {
                first_placeholder = Some(idx);
            }
            last_placeholder = Some(idx + ch.len_utf8());
        }
    }

    if let (Some(first), Some(last)) = (first_placeholder, last_placeholder) {
        prefix = working[..first].to_string();
        suffix = working[last..].to_string();
        working = working[first..last].to_string();
    }

    let use_percent = suffix.contains('%') || prefix.contains('%') || working.contains('%');
    let mut number = value;
    if use_percent {
        number *= 100.0;
    }

    // When pattern contains minus sign (e.g., "-0.0"), we should:
    // 1. Use absolute value for formatting
    // 2. Let the pattern's minus sign handle the negative display
    let has_minus_in_pattern =
        prefix.contains('-') && !prefix.chars().any(|ch| ch.is_ascii_alphabetic());
    if value < 0.0 && has_minus_in_pattern {
        number = number.abs();
    }

    let (int_pattern, frac_pattern) = match working.split_once('.') {
        Some((left, right)) => (left, right),
        None => (working.as_str(), ""),
    };

    let decimals = frac_pattern
        .chars()
        .filter(|c| *c == '0' || *c == '#')
        .count();
    let has_required_decimal = frac_pattern.chars().any(|c| c == '0');
    let use_separator = int_pattern.contains(',');

    let formatted = if decimals > 0 {
        format!("{:.*}", decimals, number)
    } else {
        format!("{:.0}", number)
    };

    let mut parts = formatted.split('.');
    let int_part = parts.next().unwrap_or("0");
    let frac_part = parts.next();

    let mut int_formatted = if use_separator {
        format_with_thousands(int_part)
    } else {
        int_part.to_string()
    };

    // Only add minus sign if:
    // 1. Value is negative AND
    // 2. Pattern doesn't already contain a minus sign
    if value < 0.0 && !has_minus_in_pattern && !int_formatted.starts_with('-') {
        int_formatted = format!("-{}", int_formatted);
    }

    let mut out = String::new();
    out.push_str(&prefix.replace('%', ""));
    out.push_str(&int_formatted);

    if decimals > 0 {
        if let Some(mut frac) = frac_part.map(|f| f.to_string()) {
            if !has_required_decimal {
                while frac.ends_with('0') {
                    frac.pop();
                }
            }
            if !frac.is_empty() {
                out.push('.');
                out.push_str(&frac);
            } else if has_required_decimal {
                out.push('.');
                out.push_str(&"0".repeat(decimals));
            }
        }
    }

    if suffix.contains('%') || prefix.contains('%') {
        out.push('%');
    }
    out.push_str(&suffix.replace('%', ""));

    out
}

fn format_with_thousands(input: &str) -> String {
    let mut chars: Vec<char> = input.chars().collect();
    let negative = chars.first() == Some(&'-');
    if negative {
        chars.remove(0);
    }

    let mut out = String::new();
    for (idx, ch) in chars.iter().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(*ch);
    }

    let mut out: String = out.chars().rev().collect();
    if negative {
        out.insert(0, '-');
    }
    out
}

fn is_date_pattern(pattern: &str) -> bool {
    let cleaned = strip_quoted_text(pattern);
    let lower = cleaned.to_ascii_lowercase();

    if lower.contains('y') || lower.contains('d') || lower.contains('h') || lower.contains('s') {
        return true;
    }

    if lower.contains('m') {
        return lower.contains(':');
    }

    false
}

fn format_date_pattern(pattern: &str, value: f64) -> String {
    let Some(dt) = excel_date_to_datetime(value) else {
        return value.to_string();
    };
    let chrono_pattern = excel_pattern_to_chrono(pattern);
    dt.format(&chrono_pattern).to_string()
}

fn excel_pattern_to_chrono(pattern: &str) -> String {
    let mut out = pattern.to_ascii_lowercase();
    let replacements = [
        ("yyyy", "{YYYY}"),
        ("yy", "{YY}"),
        ("mmmm", "{MMMM}"),
        ("mmm", "{MMM}"),
        ("mm", "{MM}"),
        ("m", "{M}"),
        ("dddd", "{DDDD}"),
        ("ddd", "{DDD}"),
        ("dd", "{DD}"),
        ("d", "{D}"),
        ("hh", "{HH}"),
        ("h", "{H}"),
        ("ss", "{SS}"),
        ("s", "{S}"),
    ];

    for (excel, placeholder) in replacements {
        out = out.replace(excel, placeholder);
    }

    let chrono_replacements = [
        ("{YYYY}", "%Y"),
        ("{YY}", "%y"),
        ("{MMMM}", "%B"),
        ("{MMM}", "%b"),
        ("{MM}", "%m"),
        ("{M}", "%-m"),
        ("{DDDD}", "%A"),
        ("{DDD}", "%a"),
        ("{DD}", "%d"),
        ("{D}", "%-d"),
        ("{HH}", "%H"),
        ("{H}", "%-H"),
        ("{SS}", "%S"),
        ("{S}", "%-S"),
    ];

    for (placeholder, chrono) in chrono_replacements {
        out = out.replace(placeholder, chrono);
    }

    out
}

fn excel_date_to_datetime(serial: f64) -> Option<DateTime<Utc>> {
    const EXCEL_EPOCH: i64 = 25569;
    let mut days = serial.floor() as i64;
    if days >= 60 {
        // Excel's 1900 leap year bug: skip the non-existent 1900-02-29
        days -= 1;
    }
    let time_fraction = serial - serial.floor();
    let unix_days = days - EXCEL_EPOCH;
    let unix_seconds = unix_days * 86400 + (time_fraction * 86400.0) as i64;
    DateTime::from_timestamp(unix_seconds, 0)
}

fn strip_quoted_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_quotes = false;
    for ch in input.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if !in_quotes {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssf_format_basic_number() {
        let value = Value::Float(1234.5);
        assert_eq!(ssf_format("#,##0.00", &value, None), "1,234.50");
    }

    #[test]
    fn test_ssf_format_negative_section() {
        let value = Value::Float(-12.3);
        assert_eq!(ssf_format("0.0;-0.0", &value, None), "-12.3");
    }

    #[test]
    fn test_ssf_format_color() {
        let value = Value::Float(-1.0);
        assert_eq!(
            ssf_format_color("[Red]-0.0", &value),
            Some("RED".to_string())
        );
    }

    #[test]
    fn test_ssf_format_date() {
        let value = Value::Float(44562.0);
        let formatted = ssf_format("mm/dd/yyyy", &value, None);
        assert_eq!(formatted, "12/31/2021");
    }

    #[test]
    fn test_ssf_format_text() {
        let value = Value::String("hello".to_string());
        assert_eq!(ssf_format("@", &value, None), "hello");
    }
}
