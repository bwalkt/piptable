# Abstract Syntax Tree (AST) Reference

The PipTable Abstract Syntax Tree (AST) represents the parsed structure of PipTable programs. This reference documents the AST nodes related to join operations and their relationships.

## Overview

The PipTable AST is defined in Rust and uses strongly-typed enums and structs to represent different language constructs. Join operations are represented as expression nodes in the AST tree.

## Core AST Types

### Expression Enum

Join operations are represented as variants of the main `Expr` enum:

```rust,ignore
pub enum Expr {
    // ... other variants ...
    
    /// Join expression combining two sheets/tables
    Join {
        /// Left side of the join (sheet or expression)
        left: Box<Expr>,
        /// Right side of the join (sheet or expression)  
        right: Box<Expr>,
        /// Type of join operation
        join_type: JoinType,
        /// Join condition specification
        condition: JoinCondition,
    },
    
    // ... other variants ...
}
```

### JoinType Enum

Specifies the type of join operation to perform:

```rust,ignore
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum JoinType {
    /// Inner join - only matching rows from both sides
    Inner,
    /// Left outer join - all rows from left, matching from right
    Left,
    /// Right outer join - all rows from right, matching from left
    Right,
    /// Full outer join - all rows from both sides
    Full,
    /// Cross join - Cartesian product (SQL only, not DSL syntax)
    Cross,
}
```

#### JoinType Usage

- **Inner**: Returns only rows where join condition matches in both tables
- **Left**: Returns all rows from left table, with nulls for unmatched right rows
- **Right**: Returns all rows from right table, with nulls for unmatched left rows  
- **Full**: Returns all rows from both tables, with nulls for unmatched rows
- **Cross**: Only available in SQL queries, not in DSL join syntax

### JoinCondition Enum

Specifies how tables should be joined:

```rust,ignore
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JoinCondition {
    /// Join on same column name in both tables: `on "column_name"`
    On(String),
    /// Join on different column names: `on "left_col" = "right_col"`
    OnColumns { 
        /// Column name in left table
        left: String, 
        /// Column name in right table
        right: String 
    },
}
```

#### JoinCondition Examples

**Same Column Join**:
```vba
employees join departments on "dept_id"
```
```rust,ignore
JoinCondition::On("dept_id".to_string())
```

**Different Column Join**:
```vba
employees join departments on "department_id" = "id"
```
```rust,ignore
JoinCondition::OnColumns {
    left: "department_id".to_string(),
    right: "id".to_string(),
}
```

## AST Construction

### Parser Integration

The parser constructs join AST nodes during the parsing phase:

```rust,ignore
// Simplified parser logic
fn build_join_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let mut pairs = pair.into_inner();
    let mut left = build_or_expr(pairs.next().unwrap())?;
    
    while let Some(join_op) = pairs.next() {
        let join_type = match join_op.as_str() {
            "join" => JoinType::Inner,
            "left" => JoinType::Left,
            "right" => JoinType::Right,  
            "full" => JoinType::Full,
            _ => return Err(BuildError::InvalidJoinType),
        };
        
        let right = build_or_expr(pairs.next().unwrap())?;
        let condition = build_join_condition(pairs.next().unwrap())?;
        
        left = Expr::Join {
            left: Box::new(left),
            right: Box::new(right),
            join_type,
            condition,
        };
    }
    
    Ok(left)
}
```

### Grammar Rules

The grammar rules that generate join AST nodes:

```pest
// Join expression with left-associative chaining
join_expr = { or_expr ~ (join_op ~ or_expr ~ join_condition)* }

// Join operators
join_op = { left_join | right_join | full_join | inner_join }
left_join = { "left" ~ "join" }
right_join = { "right" ~ "join" }  
full_join = { "full" ~ "join" }
inner_join = { "join" }

// Join conditions
join_condition = { "on" ~ (join_key_pair | string) }
join_key_pair = { string ~ "=" ~ string }
```

## AST Traversal

### Visitor Pattern

Join AST nodes can be processed using the visitor pattern:

```rust,ignore
impl<T> ExprVisitor<T> for MyVisitor {
    fn visit_join(
        &mut self, 
        left: &Expr, 
        right: &Expr, 
        join_type: &JoinType, 
        condition: &JoinCondition
    ) -> Result<T> {
        // Process left expression
        let left_result = self.visit_expr(left)?;
        
        // Process right expression  
        let right_result = self.visit_expr(right)?;
        
        // Handle join-specific logic
        match join_type {
            JoinType::Inner => self.handle_inner_join(left_result, right_result, condition),
            JoinType::Left => self.handle_left_join(left_result, right_result, condition),
            JoinType::Right => self.handle_right_join(left_result, right_result, condition),
            JoinType::Full => self.handle_full_join(left_result, right_result, condition),
            JoinType::Cross => self.handle_cross_join(left_result, right_result),
        }
    }
}
```

### AST Analysis

Common patterns for analyzing join AST nodes:

```rust,ignore
// Count join operations in an expression
fn count_joins(expr: &Expr) -> usize {
    match expr {
        Expr::Join { left, right, .. } => {
            1 + count_joins(left) + count_joins(right)
        },
        _ => 0,
    }
}

// Extract all join conditions
fn extract_join_conditions(expr: &Expr) -> Vec<&JoinCondition> {
    let mut conditions = Vec::new();
    
    fn collect_conditions(expr: &Expr, conditions: &mut Vec<&JoinCondition>) {
        match expr {
            Expr::Join { left, right, condition, .. } => {
                conditions.push(condition);
                collect_conditions(left, conditions);
                collect_conditions(right, conditions);
            },
            _ => {}
        }
    }
    
    collect_conditions(expr, &mut conditions);
    conditions
}

// Validate join column existence
fn validate_join_columns(expr: &Expr, available_columns: &[String]) -> Result<()> {
    match expr {
        Expr::Join { left, right, condition, .. } => {
            // Recursively validate sub-expressions
            validate_join_columns(left, available_columns)?;
            validate_join_columns(right, available_columns)?;
            
            // Validate join condition columns
            match condition {
                JoinCondition::On(col) => {
                    if !available_columns.contains(col) {
                        return Err(Error::ColumnNotFound(col.clone()));
                    }
                },
                JoinCondition::OnColumns { left: l, right: r } => {
                    if !available_columns.contains(l) {
                        return Err(Error::ColumnNotFound(l.clone()));
                    }
                    if !available_columns.contains(r) {
                        return Err(Error::ColumnNotFound(r.clone()));
                    }
                },
            }
            
            Ok(())
        },
        _ => Ok(()),
    }
}
```

## Serialization

### JSON Representation

Join AST nodes can be serialized to JSON for debugging or external processing:

```json
{
  "type": "Join",
  "left": {
    "type": "Identifier", 
    "name": "employees"
  },
  "right": {
    "type": "Identifier",
    "name": "departments"  
  },
  "join_type": "Inner",
  "condition": {
    "type": "OnColumns",
    "left": "dept_id",
    "right": "id"
  }
}
```

### Debug Display

Join AST nodes implement Debug for readable output:

```rust,ignore
impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Join { left, right, join_type, condition } => {
                write!(f, "Join({:?} {:?} {:?} on {:?})", left, join_type, right, condition)
            },
            // ... other variants ...
        }
    }
}
```

## Type System Integration

### Type Checking

Join operations are type-checked during analysis:

```rust,ignore
impl TypeChecker {
    fn check_join(&mut self, 
                  left: &Expr, 
                  right: &Expr, 
                  condition: &JoinCondition) -> Result<Type> {
        
        // Check left and right expressions are sheets/tables
        let left_type = self.check_expr(left)?;
        let right_type = self.check_expr(right)?;
        
        match (&left_type, &right_type) {
            (Type::Sheet(left_schema), Type::Sheet(right_schema)) => {
                // Validate join condition against schemas
                self.validate_join_condition(condition, left_schema, right_schema)?;
                
                // Compute result schema
                let result_schema = self.merge_schemas(left_schema, right_schema, condition)?;
                Ok(Type::Sheet(result_schema))
            },
            _ => Err(TypeError::InvalidJoinOperands(left_type, right_type)),
        }
    }
}
```

### Schema Evolution

Join operations affect the schema of the result:

```rust,ignore
fn merge_schemas(
    left: &Schema, 
    right: &Schema, 
    condition: &JoinCondition
) -> Result<Schema> {
    let mut columns = left.columns.clone();
    
    // Add right columns, handling name conflicts
    for right_col in &right.columns {
        let col_name = if columns.iter().any(|c| c.name == right_col.name) {
            format!("{}_right", right_col.name)
        } else {
            right_col.name.clone()
        };
        
        columns.push(Column {
            name: col_name,
            data_type: right_col.data_type.clone(),
            nullable: true, // Join results can introduce nulls
        });
    }
    
    Ok(Schema { columns })
}
```

## Error Handling

### Join-Specific Errors

Common errors related to join AST processing:

```rust,ignore
#[derive(Debug, Error)]
pub enum JoinError {
    #[error("Join column '{0}' not found in left table")]
    LeftColumnNotFound(String),
    
    #[error("Join column '{0}' not found in right table")]  
    RightColumnNotFound(String),
    
    #[error("Join condition column types incompatible: {0:?} vs {1:?}")]
    IncompatibleColumnTypes(DataType, DataType),
    
    #[error("Invalid join operands: expected sheets, found {0:?} and {1:?}")]
    InvalidOperands(Type, Type),
    
    #[error("Empty join condition")]
    EmptyJoinCondition,
}
```

### Validation Rules

AST validation rules specific to joins:

1. **Column Existence**: Join columns must exist in respective tables
2. **Type Compatibility**: Join columns should have compatible types
3. **Non-empty Conditions**: Join conditions cannot be empty strings
4. **Operand Types**: Join operands must resolve to sheet/table types

## Integration Points

### SQL Translation

Join AST nodes are translated to SQL for database execution:

```rust,ignore
impl SqlGenerator {
    fn generate_join(&self, 
                     left: &Expr, 
                     right: &Expr, 
                     join_type: &JoinType, 
                     condition: &JoinCondition) -> Result<String> {
        
        let left_sql = self.generate_expr(left)?;
        let right_sql = self.generate_expr(right)?;
        
        let join_keyword = match join_type {
            JoinType::Inner => "INNER JOIN",
            JoinType::Left => "LEFT JOIN", 
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full => "FULL OUTER JOIN",
            JoinType::Cross => "CROSS JOIN",
        };
        
        let condition_sql = match condition {
            JoinCondition::On(col) => format!("ON {}.{} = {}.{}", "l", col, "r", col),
            JoinCondition::OnColumns { left: l, right: r } => {
                format!("ON {}.{} = {}.{}", "l", l, "r", r)
            },
        };
        
        Ok(format!("{} {} {} {}", left_sql, join_keyword, right_sql, condition_sql))
    }
}
```

### Interpreter Execution

Join AST nodes are executed by the interpreter:

```rust,ignore
impl Interpreter {
    fn eval_join(&mut self, 
                 left: &Expr, 
                 right: &Expr, 
                 join_type: &JoinType, 
                 condition: &JoinCondition) -> Result<Value> {
        
        // Evaluate operands to get sheets
        let left_val = self.eval_expr(left)?;
        let right_val = self.eval_expr(right)?;
        
        let left_sheet = value_to_sheet(&left_val)?;
        let right_sheet = value_to_sheet(&right_val)?;
        
        // Perform join operation
        let result = match (join_type, condition) {
            (JoinType::Inner, JoinCondition::On(key)) => {
                left_sheet.inner_join(&right_sheet, key)?
            },
            (JoinType::Inner, JoinCondition::OnColumns { left: l, right: r }) => {
                left_sheet.inner_join_on(&right_sheet, l, r)?
            },
            // ... other combinations ...
        };
        
        Ok(Value::Sheet(result))
    }
}
```

## Development Tools

### AST Debugging

Tools for debugging join AST structures:

```rust,ignore
// Pretty-print join AST with indentation
fn print_join_ast(expr: &Expr, indent: usize) {
    let prefix = "  ".repeat(indent);
    
    match expr {
        Expr::Join { left, right, join_type, condition } => {
            println!("{}Join {:?} on {:?}", prefix, join_type, condition);
            println!("{}├─ Left:", prefix);
            print_join_ast(left, indent + 1);
            println!("{}└─ Right:", prefix);  
            print_join_ast(right, indent + 1);
        },
        _ => {
            println!("{}{:?}", prefix, expr);
        }
    }
}

// Generate GraphViz dot file for AST visualization  
fn generate_ast_dot(expr: &Expr) -> String {
    let mut dot = String::from("digraph AST {\n");
    let mut counter = 0;
    
    fn add_node(expr: &Expr, dot: &mut String, counter: &mut usize) -> usize {
        let node_id = *counter;
        *counter += 1;
        
        match expr {
            Expr::Join { left, right, join_type, condition } => {
                dot.push_str(&format!("  {} [label=\"Join {:?}\\n{:?}\"];\n", 
                                    node_id, join_type, condition));
                
                let left_id = add_node(left, dot, counter);
                let right_id = add_node(right, dot, counter);
                
                dot.push_str(&format!("  {} -> {};\n", node_id, left_id));
                dot.push_str(&format!("  {} -> {};\n", node_id, right_id));
            },
            _ => {
                dot.push_str(&format!("  {} [label=\"{:?}\"];\n", node_id, expr));
            }
        }
        
        node_id
    }
    
    add_node(expr, &mut dot, &mut counter);
    dot.push_str("}\n");
    dot
}
```

This AST documentation provides the foundation for understanding how join operations are represented and processed within the PipTable language implementation.

## See Also

- [Join Operations DSL Reference](../dsl/joins.md) - User-facing join syntax
- [Sheet API Reference](sheet.md) - Sheet join method implementations  
- [SQL Translation](../dsl/query.md) - How joins map to SQL
- [Type System](../guide/types.md) - Type checking for joins