//! Standard spreadsheet functions implementation

use piptable_primitives::Value;

/// Sum function - adds all numeric values
pub fn sum(values: &[Value]) -> Value {
    let mut total = 0.0;
    for value in values {
        match value {
            Value::Int(n) => total += *n as f64,
            Value::Float(f) => total += f,
            _ => {} // Skip non-numeric values
        }
    }
    Value::Float(total)
}

/// Average function - calculates mean of numeric values
pub fn average(values: &[Value]) -> Value {
    let mut total = 0.0;
    let mut count = 0;
    
    for value in values {
        match value {
            Value::Int(n) => {
                total += *n as f64;
                count += 1;
            }
            Value::Float(f) => {
                total += f;
                count += 1;
            }
            _ => {} // Skip non-numeric values
        }
    }
    
    if count == 0 {
        Value::Error(piptable_primitives::ErrorValue::Div0)
    } else {
        Value::Float(total / count as f64)
    }
}

/// Count function - counts non-empty cells
pub fn count(values: &[Value]) -> Value {
    let count = values.iter().filter(|v| !matches!(v, Value::Empty)).count();
    Value::Int(count as i64)
}

/// Max function - finds maximum value
pub fn max(values: &[Value]) -> Value {
    let mut max_val: Option<f64> = None;
    
    for value in values {
        let num = match value {
            Value::Int(n) => *n as f64,
            Value::Float(f) => *f,
            _ => continue,
        };
        
        max_val = Some(match max_val {
            None => num,
            Some(m) => m.max(num),
        });
    }
    
    match max_val {
        Some(v) => Value::Float(v),
        None => Value::Error(piptable_primitives::ErrorValue::Value),
    }
}

/// Min function - finds minimum value  
pub fn min(values: &[Value]) -> Value {
    let mut min_val: Option<f64> = None;
    
    for value in values {
        let num = match value {
            Value::Int(n) => *n as f64,
            Value::Float(f) => *f,
            _ => continue,
        };
        
        min_val = Some(match min_val {
            None => num,
            Some(m) => m.min(num),
        });
    }
    
    match min_val {
        Some(v) => Value::Float(v),
        None => Value::Error(piptable_primitives::ErrorValue::Value),
    }
}