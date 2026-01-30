//! # piptable-parser
//!
//! Parser for the piptable DSL combining VBA-like syntax with SQL.
//!
//! This crate uses [pest](https://pest.rs) for parsing.

mod builder;

use pest::Parser;
use pest_derive::Parser as PestParser;
use piptable_core::{PipError, PipResult, Program, SqlQuery};

pub use builder::BuildError;

#[derive(PestParser)]
#[grammar = "grammar.pest"]
struct PiptableParser;

/// Main parser interface.
pub struct PipParser;

impl PipParser {
    /// Parse a piptable script string into an AST.
    ///
    /// # Errors
    ///
    /// Returns a `PipError::Parse` if the input is invalid.
    pub fn parse_str(input: &str) -> PipResult<Program> {
        let pairs = PiptableParser::parse(Rule::program, input).map_err(|e| {
            let (line, col) = e.line_col.to_pos().unwrap_or((1, 1));
            PipError::parse(line, col, e.to_string())
        })?;

        builder::build_program(pairs).map_err(|e| PipError::parse(e.line, e.column, e.message))
    }

    /// Parse a SQL query string into AST.
    ///
    /// # Errors
    ///
    /// Returns a `PipError::Parse` if the SQL is invalid.
    pub fn parse_sql(input: &str) -> PipResult<SqlQuery> {
        // Wrap in query() to match grammar
        const WRAPPER_PREFIX_LEN: usize = 6; // "query("
        let wrapped = format!("query({input})");
        let pairs = PiptableParser::parse(Rule::query_expr, &wrapped).map_err(|e| {
            let (line, col) = e.line_col.to_pos().unwrap_or((1, 1));
            // Adjust column for the wrapper prefix on line 1
            let adjusted_col = if line == 1 && col > WRAPPER_PREFIX_LEN {
                col - WRAPPER_PREFIX_LEN
            } else {
                col.max(1)
            };
            PipError::parse(line, adjusted_col, e.to_string())
        })?;

        let query_pair = pairs
            .into_iter()
            .next()
            .expect("query_expr should produce at least one pair");
        let sql_query_pair = query_pair
            .into_inner()
            .next()
            .expect("query_expr should contain sql_query");

        builder::build_sql_query(sql_query_pair).map_err(|e| {
            // Adjust column for builder errors on line 1
            let adjusted_col = if e.line == 1 && e.column > WRAPPER_PREFIX_LEN {
                e.column - WRAPPER_PREFIX_LEN
            } else {
                e.column.max(1)
            };
            PipError::parse(e.line, adjusted_col, e.message)
        })
    }
}

/// Extension trait for pest error line/column extraction.
trait LineColExt {
    fn to_pos(&self) -> Option<(usize, usize)>;
}

impl LineColExt for pest::error::LineColLocation {
    fn to_pos(&self) -> Option<(usize, usize)> {
        match self {
            pest::error::LineColLocation::Pos((line, col)) => Some((*line, *col)),
            pest::error::LineColLocation::Span((line, col), _) => Some((*line, *col)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piptable_core::{
        BinaryOp, Expr, JoinCondition, JoinType, Literal, ParamMode, SortDirection, Statement,
        TableRef,
    };

    // ========================================================================
    // Basic parsing tests
    // ========================================================================

    #[test]
    fn test_parse_empty() {
        let result = PipParser::parse_str("");
        assert!(result.is_ok());
        assert!(result.unwrap().statements.is_empty());
    }

    // ========================================================================
    // Lambda parsing tests (Issue #160)
    // ========================================================================

    #[test]
    fn test_parse_lambda_single_param() {
        let result = PipParser::parse_str("dim double = x => x * 2");
        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);

        if let Statement::Dim { value, .. } = &program.statements[0] {
            assert!(matches!(value, Expr::Lambda { params, .. } if params.len() == 1));
        } else {
            panic!("Expected Dim statement with lambda");
        }
    }

    #[test]
    fn test_parse_lambda_multiple_params() {
        let result = PipParser::parse_str("dim add = (x, y) => x + y");
        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);

        if let Statement::Dim { value, .. } = &program.statements[0] {
            assert!(matches!(value, Expr::Lambda { params, .. } if params.len() == 2));
        } else {
            panic!("Expected Dim statement with lambda");
        }
    }

    #[test]
    fn test_parse_lambda_no_params() {
        let result = PipParser::parse_str("dim constant = () => 42");
        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);

        if let Statement::Dim { value, .. } = &program.statements[0] {
            assert!(matches!(value, Expr::Lambda { params, .. } if params.is_empty()));
        } else {
            panic!("Expected Dim statement with lambda");
        }
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let result = PipParser::parse_str("!@#$%^ invalid");
        assert!(result.is_err());
    }

    // ========================================================================
    // SELECT tests (Issue #15)
    // ========================================================================

    #[test]
    fn parse_select_star() {
        let sql = "SELECT * FROM users";
        let query = PipParser::parse_sql(sql).unwrap();
        assert_eq!(query.select.items.len(), 1);
        assert!(matches!(
            &query.select.items[0].expr,
            Expr::Variable(name) if name == "*"
        ));
    }

    #[test]
    fn parse_select_columns() {
        let sql = "SELECT a, b, c FROM users";
        let query = PipParser::parse_sql(sql).unwrap();
        assert_eq!(query.select.items.len(), 3);
    }

    #[test]
    fn parse_select_with_alias() {
        let sql = "SELECT a AS col_a, b AS col_b FROM t";
        let query = PipParser::parse_sql(sql).unwrap();
        assert_eq!(query.select.items[0].alias, Some("col_a".to_string()));
        assert_eq!(query.select.items[1].alias, Some("col_b".to_string()));
    }

    #[test]
    fn parse_select_expressions() {
        let sql = "SELECT a + b, c * 2 FROM t";
        let query = PipParser::parse_sql(sql).unwrap();
        assert_eq!(query.select.items.len(), 2);
        // First item should be a binary expression
        assert!(matches!(&query.select.items[0].expr, Expr::Binary { .. }));
    }

    #[test]
    fn parse_select_distinct() {
        let sql = "SELECT DISTINCT a, b FROM t";
        let query = PipParser::parse_sql(sql).unwrap();
        assert!(query.select.distinct);
    }

    // ========================================================================
    // FROM tests (Issue #15)
    // ========================================================================

    #[test]
    fn parse_from_identifier() {
        let sql = "SELECT * FROM users";
        let query = PipParser::parse_sql(sql).unwrap();
        let from = query.from.unwrap();
        assert!(matches!(from.source, TableRef::Table(name) if name == "users"));
    }

    #[test]
    fn parse_from_file_path() {
        let sql = r#"SELECT * FROM "data.csv""#;
        let query = PipParser::parse_sql(sql).unwrap();
        let from = query.from.unwrap();
        assert!(matches!(from.source, TableRef::File(path) if path == "data.csv"));
    }

    #[test]
    fn parse_from_with_alias() {
        let sql = "SELECT * FROM users u";
        let query = PipParser::parse_sql(sql).unwrap();
        let from = query.from.unwrap();
        assert_eq!(from.alias, Some("u".to_string()));
    }

    // ========================================================================
    // WHERE tests (Issue #15)
    // ========================================================================

    #[test]
    fn parse_where_simple() {
        let sql = "SELECT * FROM t WHERE x = 10";
        let query = PipParser::parse_sql(sql).unwrap();
        assert!(query.where_clause.is_some());
    }

    #[test]
    fn parse_where_and() {
        let sql = "SELECT * FROM t WHERE x > 10 AND y < 20";
        let query = PipParser::parse_sql(sql).unwrap();
        let where_expr = query.where_clause.unwrap();
        assert!(matches!(
            *where_expr,
            Expr::Binary {
                op: BinaryOp::And,
                ..
            }
        ));
    }

    #[test]
    fn parse_where_or() {
        let sql = "SELECT * FROM t WHERE x = 1 OR x = 2";
        let query = PipParser::parse_sql(sql).unwrap();
        let where_expr = query.where_clause.unwrap();
        assert!(matches!(
            *where_expr,
            Expr::Binary {
                op: BinaryOp::Or,
                ..
            }
        ));
    }

    #[test]
    fn parse_where_comparison_ops() {
        let cases = [
            ("SELECT * FROM t WHERE x = 1", BinaryOp::Eq),
            ("SELECT * FROM t WHERE x != 1", BinaryOp::Ne),
            ("SELECT * FROM t WHERE x <> 1", BinaryOp::Ne),
            ("SELECT * FROM t WHERE x < 1", BinaryOp::Lt),
            ("SELECT * FROM t WHERE x <= 1", BinaryOp::Le),
            ("SELECT * FROM t WHERE x > 1", BinaryOp::Gt),
            ("SELECT * FROM t WHERE x >= 1", BinaryOp::Ge),
        ];

        for (sql, expected_op) in cases {
            let query = PipParser::parse_sql(sql).unwrap();
            let where_expr = query.where_clause.unwrap();
            match *where_expr {
                Expr::Binary { op, .. } => assert_eq!(op, expected_op, "Failed for: {sql}"),
                _ => panic!("Expected binary expression for: {sql}"),
            }
        }
    }

    #[test]
    fn parse_where_string_literal() {
        let sql = r#"SELECT * FROM t WHERE name = "John""#;
        let query = PipParser::parse_sql(sql).unwrap();
        assert!(query.where_clause.is_some());
    }

    // ========================================================================
    // ORDER BY tests (Issue #15)
    // ========================================================================

    #[test]
    fn parse_order_by_single() {
        let sql = "SELECT * FROM t ORDER BY a";
        let query = PipParser::parse_sql(sql).unwrap();
        let order_by = query.order_by.unwrap();
        assert_eq!(order_by.len(), 1);
        assert_eq!(order_by[0].direction, SortDirection::Asc);
    }

    #[test]
    fn parse_order_by_desc() {
        let sql = "SELECT * FROM t ORDER BY a DESC";
        let query = PipParser::parse_sql(sql).unwrap();
        let order_by = query.order_by.unwrap();
        assert_eq!(order_by[0].direction, SortDirection::Desc);
    }

    #[test]
    fn parse_order_by_multiple() {
        let sql = "SELECT * FROM t ORDER BY a DESC, b ASC, c";
        let query = PipParser::parse_sql(sql).unwrap();
        let order_by = query.order_by.unwrap();
        assert_eq!(order_by.len(), 3);
        assert_eq!(order_by[0].direction, SortDirection::Desc);
        assert_eq!(order_by[1].direction, SortDirection::Asc);
        assert_eq!(order_by[2].direction, SortDirection::Asc);
    }

    // ========================================================================
    // LIMIT tests (Issue #15)
    // ========================================================================

    #[test]
    fn parse_limit() {
        let sql = "SELECT * FROM t LIMIT 10";
        let query = PipParser::parse_sql(sql).unwrap();
        let limit = query.limit.unwrap();
        assert!(matches!(*limit, Expr::Literal(Literal::Int(10))));
    }

    #[test]
    fn parse_limit_offset() {
        let sql = "SELECT * FROM t LIMIT 10 OFFSET 20";
        let query = PipParser::parse_sql(sql).unwrap();

        let limit = query.limit.unwrap();
        assert!(matches!(*limit, Expr::Literal(Literal::Int(10))));

        let offset = query.offset.unwrap();
        assert!(matches!(*offset, Expr::Literal(Literal::Int(20))));
    }

    // ========================================================================
    // Error tests (Issue #15)
    // ========================================================================

    #[test]
    fn error_invalid_sql() {
        let sql = "SELEC * FROM t"; // typo
        let err = PipParser::parse_sql(sql);
        assert!(err.is_err());
    }

    #[test]
    fn error_reports_location() {
        let result = PipParser::parse_str("dim x =");
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Error should have location info
        assert!(!err.to_string().is_empty());
    }

    // ========================================================================
    // VBA statement tests
    // ========================================================================

    #[test]
    fn parse_dim_statement() {
        let result = PipParser::parse_str("dim x = 42");
        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);
        assert!(matches!(
            &program.statements[0],
            Statement::Dim { name, .. } if name == "x"
        ));
    }

    #[test]
    fn parse_dim_with_type_hint() {
        let result = PipParser::parse_str("dim x: int = 42");
        assert!(result.is_ok());
        let program = result.unwrap();
        match &program.statements[0] {
            Statement::Dim { type_hint, .. } => {
                assert!(type_hint.is_some());
            }
            _ => panic!("Expected Dim statement"),
        }
    }

    #[test]
    fn parse_function_definition() {
        let code = "function add(a, b) return a + b end function";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Function { name, params, .. } if name == "add" && params.len() == 2
        ));
    }

    #[test]
    fn parse_function_byval_byref_params() {
        let code = "function add(byval a, byref b) return a + b end function";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok());
        let program = result.unwrap();
        match &program.statements[0] {
            Statement::Function { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "a");
                assert_eq!(params[0].mode, ParamMode::ByVal);
                assert!(params[0].default.is_none());
                assert!(!params[0].is_param_array);
                assert_eq!(params[1].name, "b");
                assert_eq!(params[1].mode, ParamMode::ByRef);
                assert!(params[1].default.is_none());
                assert!(!params[1].is_param_array);
            }
            _ => panic!("Expected Function statement"),
        }
    }

    #[test]
    fn parse_function_optional_param() {
        let code = "function add(a, optional b = 1) return a + b end function";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok());
        let program = result.unwrap();
        match &program.statements[0] {
            Statement::Function { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[1].name, "b");
                assert!(params[1].default.is_some());
                assert!(!params[1].is_param_array);
            }
            _ => panic!("Expected Function statement"),
        }
    }

    #[test]
    fn parse_function_paramarray() {
        let code = "function sum_all(paramarray nums) return sum(nums) end function";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok());
        let program = result.unwrap();
        match &program.statements[0] {
            Statement::Function { name, params, .. } => {
                assert_eq!(name, "sum_all");
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "nums");
                assert!(params[0].is_param_array);
            }
            _ => panic!("Expected Function statement"),
        }
    }

    #[test]
    #[ignore = "planned optional/paramarray coverage"]
    fn parse_function_optional_param() {
        let code = "function add(a, optional b = 1) return a + b end function";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok());
        let program = result.unwrap();
        match &program.statements[0] {
            Statement::Function { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[1].name, "b");
                assert!(matches!(
                    &params[1].default,
                    Some(Expr::Literal(Literal::Int(1)))
                ));
                assert_eq!(params[1].mode, ParamMode::ByVal);
                assert!(!params[1].is_param_array);
            }
            _ => panic!("Expected Function statement"),
        }
    }

    #[test]
    fn parse_if_statement() {
        let code = "if x > 10 then dim y = 1 else dim y = 2 end if";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok());
        let program = result.unwrap();
        assert!(matches!(&program.statements[0], Statement::If { .. }));
    }

    #[test]
    fn parse_for_each_statement() {
        let code = "for each item in items print(item) next";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::ForEach { variable, .. } if variable == "item"
        ));
    }

    // ========================================================================
    // Import statement tests (Issue #90)
    // ========================================================================

    #[test]
    fn parse_import_basic() {
        let code = r#"import "data.csv" into myData"#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);
        assert!(matches!(
            &program.statements[0],
            Statement::Import { sources, target, sheet_name, .. }
            if sources.len() == 1 && target == "myData" && sheet_name.is_none()
        ));
    }

    #[test]
    fn parse_import_with_sheet() {
        let code = r#"import "workbook.xlsx" sheet "Sales" into salesData"#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Import { sources, target, sheet_name, .. }
            if sources.len() == 1 && target == "salesData" && sheet_name.is_some()
        ));
    }

    #[test]
    fn parse_import_multi_file() {
        let code = r#"import "q1.csv", "q2.csv", "q3.csv" into quarterly"#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Import { sources, target, .. }
            if sources.len() == 3 && target == "quarterly"
        ));
    }

    #[test]
    fn parse_import_without_headers() {
        let code = r#"import "data.csv" into raw without headers"#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Import { sources, target, options, .. }
            if sources.len() == 1 && target == "raw" && options.has_headers == Some(false)
        ));
    }

    #[test]
    fn parse_import_named_params() {
        let code = r#"import "data.csv" into raw (headers = false)"#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Import { sources, target, options, .. }
            if sources.len() == 1 && target == "raw" && options.has_headers == Some(false)
        ));
    }

    // ========================================================================
    // Join expression tests
    // ========================================================================

    #[test]
    fn parse_join_inner() {
        let code = r#"result = users join orders on "id""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Assignment { value, .. }
            if matches!(value, Expr::Join { join_type: JoinType::Inner, .. })
        ));
    }

    #[test]
    fn parse_join_left() {
        let code = r#"result = users left join orders on "user_id""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Assignment { value, .. }
            if matches!(value, Expr::Join { join_type: JoinType::Left, .. })
        ));
    }

    #[test]
    fn parse_join_right() {
        let code = r#"result = users right join orders on "id""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Assignment { value, .. }
            if matches!(value, Expr::Join { join_type: JoinType::Right, .. })
        ));
    }

    #[test]
    fn parse_join_full() {
        let code = r#"result = users full join orders on "id""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Assignment { value, .. }
            if matches!(value, Expr::Join { join_type: JoinType::Full, .. })
        ));
    }

    #[test]
    fn parse_append_basic() {
        let code = r"users append new_users";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Append { target, distinct, key, .. }
            if target == "users" && !distinct && key.is_none()
        ));
    }

    #[test]
    fn parse_append_distinct() {
        let code = r#"users append distinct new_users on "id""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Append { target, distinct, key, .. }
            if target == "users" && *distinct && key.as_deref() == Some("id")
        ));
    }

    #[test]
    fn parse_upsert() {
        let code = r#"users upsert updates on "id""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Upsert { target, key, .. }
            if target == "users" && key == "id"
        ));
    }

    #[test]
    fn parse_append_with_escaped_key() {
        // Test with escaped quotes in the key name
        let code = r#"users append distinct new_users on "col\"name""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Append { distinct, key, .. }
            if *distinct && key.as_deref() == Some(r#"col"name"#)
        ));
    }

    #[test]
    fn parse_upsert_with_escaped_key() {
        // Test with escaped characters in the key name
        let code = r#"users upsert updates on "id\nline""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Upsert { key, .. }
            if key == "id\nline"
        ));
    }

    #[test]
    fn parse_append_empty_key_error() {
        // Test that empty keys are rejected
        let code = r#"users append distinct new_users on """#;
        let result = PipParser::parse_str(code);
        assert!(result.is_err(), "Should error on empty key");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Append key cannot be empty"));
    }

    #[test]
    fn parse_upsert_empty_key_error() {
        // Test that empty keys are rejected
        let code = r#"users upsert updates on """#;
        let result = PipParser::parse_str(code);
        assert!(result.is_err(), "Should error on empty key");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Upsert key cannot be empty"));
    }

    #[test]
    fn parse_append_with_key_no_distinct_error() {
        // Test that append with "on" but no distinct fails at build time
        let code = r#"users append new_users on "id""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_err(), "Should error on key without distinct");
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("'on' clause can only be used with 'append distinct'"));
    }

    #[test]
    fn parse_append_distinct_requires_on() {
        // Test that "distinct" without "on" fails at build time
        let code = r"users append distinct new_users";
        let result = PipParser::parse_str(code);
        assert!(result.is_err(), "Should error on distinct without key");
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("append distinct requires 'on' clause"));
    }

    #[test]
    fn parse_join_with_different_columns() {
        let code = r#"result = users join orders on "id" = "user_id""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();

        if let Statement::Assignment { value, .. } = &program.statements[0] {
            if let Expr::Join { condition, .. } = value {
                assert!(matches!(
                    condition,
                    JoinCondition::OnColumns { left, right }
                    if left == "id" && right == "user_id"
                ));
            } else {
                panic!("Expected Join expression");
            }
        } else {
            panic!("Expected Assignment statement");
        }
    }

    #[test]
    fn parse_join_in_dim_statement() {
        let code = r#"dim joined_data = customers left join orders on "customer_id""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();

        assert!(matches!(
            &program.statements[0],
            Statement::Dim { value, name, .. }
            if name == "joined_data" && matches!(value, Expr::Join { join_type: JoinType::Left, .. })
        ));
    }

    #[test]
    fn parse_chained_joins() {
        // Chained joins need parentheses for proper precedence
        let code = r#"result = (users join orders on "user_id") join products on "product_id""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();

        if let Statement::Assignment { value, .. } = &program.statements[0] {
            // The outer join should have a nested join as its left operand
            if let Expr::Join { left, .. } = value {
                assert!(
                    matches!(&**left, Expr::Join { .. }),
                    "Left side should be a join"
                );
            } else {
                panic!("Expected Join expression");
            }
        } else {
            panic!("Expected Assignment statement");
        }
    }

    #[test]
    fn parse_join_with_variables() {
        let code = r#"result = sheet1 join sheet2 on "key""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();

        if let Statement::Assignment { value, .. } = &program.statements[0] {
            if let Expr::Join {
                left,
                right,
                condition,
                ..
            } = value
            {
                assert!(matches!(&**left, Expr::Variable(name) if name == "sheet1"));
                assert!(matches!(&**right, Expr::Variable(name) if name == "sheet2"));
                assert!(matches!(condition, JoinCondition::On(key) if key == "key"));
            } else {
                panic!("Expected Join expression");
            }
        } else {
            panic!("Expected Assignment statement");
        }
    }

    #[test]
    fn parse_join_with_function_calls() {
        let code = r#"result = load_users() join load_orders() on "user_id" = "customer_id""#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();

        if let Statement::Assignment { value, .. } = &program.statements[0] {
            if let Expr::Join {
                left,
                right,
                condition,
                ..
            } = value
            {
                assert!(matches!(&**left, Expr::Call { function, .. } if function == "load_users"));
                assert!(
                    matches!(&**right, Expr::Call { function, .. } if function == "load_orders")
                );
                assert!(matches!(
                    condition,
                    JoinCondition::OnColumns { left, right }
                    if left == "user_id" && right == "customer_id"
                ));
            } else {
                panic!("Expected Join expression");
            }
        } else {
            panic!("Expected Assignment statement");
        }
    }

    // ========================================================================
    // Complex query tests
    // ========================================================================

    #[test]
    fn parse_complex_query() {
        let sql = "SELECT u.name, COUNT(o.id) AS order_count FROM users u LEFT JOIN orders o ON u.id = o.user_id WHERE u.active = true GROUP BY u.name HAVING COUNT(o.id) > 5 ORDER BY order_count DESC LIMIT 10";
        let query = PipParser::parse_sql(sql).unwrap();

        assert_eq!(query.select.items.len(), 2);
        assert!(query.from.is_some());
        assert_eq!(query.joins.len(), 1);
        assert!(query.where_clause.is_some());
        assert!(query.group_by.is_some());
        assert!(query.having.is_some());
        assert!(query.order_by.is_some());
        assert!(query.limit.is_some());
    }

    // ========================================================================
    // Lambda expression tests
    // ========================================================================

    #[test]
    fn parse_lambda_no_params() {
        let code = "dim f = () => 42";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Dim { value, .. }
            if matches!(value, Expr::Lambda { params, .. } if params.is_empty())
        ));
    }

    #[test]
    fn parse_lambda_one_param() {
        let code = "dim add_one = x => x + 1";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Dim { value, .. }
            if matches!(value, Expr::Lambda { params, .. } if params.len() == 1 && params[0] == "x")
        ));
    }

    #[test]
    fn parse_lambda_multiple_params() {
        let code = "dim multiply = (a, b) => a * b";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Dim { value, .. }
            if matches!(value, Expr::Lambda { params, .. }
                if params.len() == 2 && params[0] == "a" && params[1] == "b")
        ));
    }

    #[test]
    fn parse_lambda_complex_body() {
        let code = "dim complex = x => x > 5 and x < 10";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Dim { value, .. }
            if matches!(value, Expr::Lambda { .. })
        ));
    }

    #[test]
    fn parse_method_call_with_lambda() {
        let code = "result = data.map(x => x * 2)";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Assignment { value, .. }
            if matches!(value, Expr::MethodCall { method, args, .. }
                if method == "map" && args.len() == 1)
        ));
    }

    #[test]
    fn parse_lambda_nested() {
        // Test nested lambdas (currying)
        let code = "dim curry = x => (y => x + y)";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());

        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);

        // Verify it's a lambda that returns another lambda
        assert!(matches!(
            &program.statements[0],
            Statement::Dim { value: Expr::Lambda { params, body }, .. }
            if params.len() == 1 && matches!(**body, Expr::Lambda { .. })
        ));
    }

    #[test]
    fn parse_lambda_in_array() {
        // Test lambdas as array elements
        let code = "dim funcs = [x => x + 1, x => x * 2]";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());

        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);

        // Verify it's an array containing lambda expressions
        assert!(matches!(
            &program.statements[0],
            Statement::Dim { value: Expr::Array(items), .. }
            if items.len() == 2 &&
               items.iter().all(|item| matches!(item, Expr::Lambda { .. }))
        ));
    }

    #[test]
    fn parse_lambda_in_object() {
        // Test lambdas as object values
        let code = r#"dim ops = {"add": (x, y) => x + y, "mul": (x, y) => x * y}"#;
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());

        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);

        // Verify it's an object with lambda values
        if let Statement::Dim {
            value: Expr::Object(fields),
            ..
        } = &program.statements[0]
        {
            assert_eq!(fields.len(), 2);
            // Object is a Vec of (String, Expr) pairs
            assert!(fields.iter().all(|(_, v)| matches!(v, Expr::Lambda { .. })));
        } else {
            panic!("Expected object with lambda values");
        }
    }

    #[test]
    fn parse_lambda_with_comparison() {
        // Test lambda with comparison expression
        let code = "dim is_positive = x => x > 0";
        let result = PipParser::parse_str(code);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());

        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);

        // Verify it's a lambda with binary comparison
        assert!(matches!(
            &program.statements[0],
            Statement::Dim { value: Expr::Lambda { params, body }, .. }
            if params.len() == 1 && matches!(**body, Expr::Binary { op: BinaryOp::Gt, .. })
        ));
    }

    #[test]
    fn parse_lambda_immediate_call() {
        // Test lambda immediate invocation
        let code = "dim result = ((x, y) => x + y)(5, 3)";
        let result = PipParser::parse_str(code);

        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let program = result.unwrap();
        assert_eq!(program.statements.len(), 1);

        if let Statement::Dim {
            value: Expr::CallExpr { callee, args },
            ..
        } = &program.statements[0]
        {
            assert!(matches!(**callee, Expr::Lambda { .. }));
            assert_eq!(args.len(), 2);
        } else {
            panic!("Expected immediate lambda call");
        }
    }
}
