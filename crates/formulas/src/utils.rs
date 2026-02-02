use crate::parser::parse_formula;
use crate::FormulaError;

const INVERSE_PAREN_MAP: [(&str, &str); 2] = [("(", ")"), ("[", "]")];

fn is_paren(ch: char) -> bool {
    matches!(ch, '(' | ')' | '[' | ']')
}

fn is_open_paren(ch: char) -> bool {
    matches!(ch, '(' | '[')
}

/// Add/remove parenthesis in formulas.
pub fn balance_parentheses(input: &str) -> String {
    let mut stack: Vec<char> = Vec::new();
    for ch in input.chars() {
        if is_paren(ch) {
            if is_open_paren(ch) {
                stack.push(ch);
            } else {
                stack.pop();
            }
        }
    }

    let mut out = input.to_string();
    for open in stack {
        let close = INVERSE_PAREN_MAP
            .iter()
            .find(|(k, _)| k.starts_with(open))
            .map(|(_, v)| *v)
            .unwrap_or(")");
        out.push_str(close);
    }
    out
}

/// Add closing quotes in formulas when a string is unterminated.
/// Handles escaped quotes ("") inside strings.
pub fn balance_quotes(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut in_string = false;
    let mut open_scope_count: usize = 0;
    let mut string_start_scope_count: usize = 0;

    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' {
            if in_string && i + 1 < chars.len() && chars[i + 1] == '"' {
                i += 2;
                continue;
            }
            in_string = !in_string;
            if in_string {
                string_start_scope_count = open_scope_count;
            }
            i += 1;
            continue;
        }

        if !in_string {
            if is_open_paren(ch) {
                open_scope_count += 1;
            } else if (ch == ')' || ch == ']') && open_scope_count > 0 {
                open_scope_count -= 1;
            }
        }

        i += 1;
    }

    if !in_string {
        return input.to_string();
    }

    let mut insert_index = input.len();
    while insert_index > 0 && input.chars().nth(insert_index - 1).unwrap().is_whitespace() {
        insert_index -= 1;
    }

    let mut close_start = insert_index;
    while close_start > 0 {
        let ch = input.chars().nth(close_start - 1).unwrap();
        if ch != ')' && ch != ']' {
            break;
        }
        close_start -= 1;
    }

    let trailing_parens = insert_index - close_start;
    let parens_to_keep_outside = string_start_scope_count.min(trailing_parens);
    let insertion_pos = insert_index - parens_to_keep_outside;

    let mut out = String::new();
    out.push_str(&input[..insertion_pos]);
    out.push('"');
    out.push_str(&input[insertion_pos..]);
    out
}

/// Add/remove parenthesis and add closing quotes in formulas.
pub fn balance_formula(input: &str) -> String {
    balance_parentheses(&balance_quotes(input))
}

/// Check if parenthesis is balanced.
pub fn is_balanced_parenthesis(input: &str) -> bool {
    let mut depth: i32 = 0;
    for ch in input.chars() {
        if is_open_paren(ch) {
            depth += 1;
        } else if matches!(ch, ')' | ']') {
            if depth == 0 {
                return false;
            }
            depth -= 1;
        }
    }
    depth == 0
}

/// Check if text is a formula.
pub fn is_a_formula(text: &str) -> bool {
    let text_str = text.to_string();
    text_str.starts_with('=') || is_alternate_formula(&text_str)
}

pub fn is_alternate_formula(text: &str) -> bool {
    if text.starts_with('@') {
        return true;
    }
    if !text.starts_with('+') && !text.starts_with('-') {
        return false;
    }
    let after_sign = text[1..].trim();
    if after_sign.is_empty() || after_sign.parse::<f64>().is_ok() {
        return false;
    }
    let has_letters = after_sign.chars().any(|c| c.is_ascii_alphabetic());
    if after_sign.chars().any(|c| c.is_whitespace()) && !has_letters {
        return after_sign
            .chars()
            .any(|c| matches!(c, '+' | '*' | '/' | '^' | '=' | '<' | '>'));
    }
    true
}

/// Validate formula syntax (requires leading '=').
pub fn is_valid_formula(text: &str) -> bool {
    let trimmed = text.trim();
    if !trimmed.starts_with('=') || trimmed.len() < 2 {
        return false;
    }
    parse_formula(trimmed).is_ok()
}

/// Parse formula string, returning Ok(()) on success.
pub fn validate_formula(text: &str) -> Result<(), FormulaError> {
    parse_formula(text).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_parentheses() {
        assert_eq!(balance_parentheses("=SUM(1"), "=SUM(1)");
    }

    #[test]
    fn test_balance_quotes() {
        assert_eq!(balance_quotes("=\"foo"), "=\"foo\"");
    }

    #[test]
    fn test_is_a_formula() {
        assert!(is_a_formula("=A1"));
        assert!(is_a_formula("+A1"));
        assert!(!is_a_formula("+1"));
    }

    #[test]
    fn test_is_valid_formula() {
        assert!(is_valid_formula("=A1+1"));
        assert!(!is_valid_formula("A1+1"));
    }
}
