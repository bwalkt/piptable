//! AST builder - converts pest parse tree to AST nodes.

use pest::iterators::{Pair, Pairs};
use piptable_core::{
    BinaryOp, Expr, FromClause, ImportOptions, JoinCondition, JoinType, Literal, OrderByItem,
    Param, ParamMode, Program, SelectClause, SelectItem, SortDirection, SqlQuery, Statement,
    TableRef, UnaryOp,
};

use crate::Rule;

/// Error during AST building with location info.
#[derive(Debug)]
pub struct BuildError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl BuildError {
    pub fn new(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            column,
            message: message.into(),
        }
    }

    pub fn from_pair(pair: &Pair<Rule>, message: impl Into<String>) -> Self {
        let (line, column) = pair.line_col();
        Self::new(line, column, message)
    }
}

type BuildResult<T> = Result<T, BuildError>;

/// Build a Program AST from pest pairs.
pub fn build_program(pairs: Pairs<Rule>) -> BuildResult<Program> {
    let mut statements = Vec::new();

    for pair in pairs {
        match pair.as_rule() {
            Rule::program => {
                // Descend into the program rule to get statements
                for inner in pair.into_inner() {
                    if inner.as_rule() == Rule::statement {
                        let stmt = build_statement(inner)?;
                        statements.push(stmt);
                    }
                    // EOI and other rules are ignored
                }
            }
            Rule::statement => {
                let stmt = build_statement(pair)?;
                statements.push(stmt);
            }
            _ => {} // EOI and other rules are ignored
        }
    }

    Ok(Program::from_statements(statements))
}

/// Build a Statement from a pest pair.
fn build_statement(pair: Pair<Rule>) -> BuildResult<Statement> {
    let inner = pair.into_inner().next().unwrap();
    let (line, _) = inner.line_col();

    match inner.as_rule() {
        Rule::expr_stmt => {
            let expr_pair = inner.into_inner().next().unwrap();
            let expr = build_expr(expr_pair)?;
            Ok(Statement::Expr { expr, line })
        }
        Rule::dim_stmt => build_dim_stmt(inner, line),
        Rule::assignment_stmt => build_assignment_stmt(inner, line),
        Rule::if_stmt => build_if_stmt(inner, line),
        Rule::for_each_stmt => build_for_each_stmt(inner, line),
        Rule::for_stmt => build_for_stmt(inner, line),
        Rule::while_stmt => build_while_stmt(inner, line),
        Rule::function_def => build_function_def(inner, line),
        Rule::sub_def => build_sub_def(inner, line),
        Rule::return_stmt => build_return_stmt(inner, line),
        Rule::exit_function_stmt => Ok(Statement::ExitFunction { line }),
        Rule::exit_sub_stmt => Ok(Statement::ExitSub { line }),
        Rule::exit_for_stmt => Ok(Statement::ExitFor { line }),
        Rule::exit_while_stmt => Ok(Statement::ExitWhile { line }),
        Rule::call_stmt => build_call_stmt(inner, line),
        Rule::export_stmt => build_export_stmt(inner, line),
        Rule::import_stmt => build_import_stmt(inner, line),
        Rule::append_stmt => build_append_stmt(inner, line),
        Rule::upsert_stmt => build_upsert_stmt(inner, line),
        _ => Err(BuildError::from_pair(
            &inner,
            format!("Unexpected statement rule: {:?}", inner.as_rule()),
        )),
    }
}

fn build_dim_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    // Check for type hint
    let mut type_hint = None;
    let mut value_pair = inner.next().unwrap();

    if value_pair.as_rule() == Rule::type_hint {
        let type_inner = value_pair.into_inner().next().unwrap();
        type_hint = Some(build_type_name(type_inner)?);
        value_pair = inner.next().unwrap();
    }

    let value = build_expr(value_pair)?;

    Ok(Statement::Dim {
        name,
        type_hint,
        value,
        line,
    })
}

fn build_assignment_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();
    let lvalue_pair = inner.next().unwrap();
    let value_pair = inner.next().unwrap();

    let target = build_lvalue(lvalue_pair)?;
    let value = build_expr(value_pair)?;

    Ok(Statement::Assignment {
        target,
        value,
        line,
    })
}

fn build_if_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();

    let condition = build_expr(inner.next().unwrap())?;
    let mut then_body = Vec::new();
    let mut elseif_clauses = Vec::new();
    let mut else_body = None;

    for item in inner {
        match item.as_rule() {
            Rule::statement => {
                then_body.push(build_statement(item)?);
            }
            Rule::elseif_clause => {
                let mut elseif_inner = item.into_inner();
                let cond = build_expr(elseif_inner.next().unwrap())?;
                let mut body = Vec::new();
                for stmt in elseif_inner {
                    if stmt.as_rule() == Rule::statement {
                        body.push(build_statement(stmt)?);
                    }
                }
                elseif_clauses.push(piptable_core::ElseIfClause {
                    condition: cond,
                    body,
                });
            }
            Rule::else_clause => {
                let mut body = Vec::new();
                for stmt in item.into_inner() {
                    if stmt.as_rule() == Rule::statement {
                        body.push(build_statement(stmt)?);
                    }
                }
                else_body = Some(body);
            }
            _ => {}
        }
    }

    Ok(Statement::If {
        condition,
        then_body,
        elseif_clauses,
        else_body,
        line,
    })
}

fn build_for_each_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();
    let variable = inner.next().unwrap().as_str().to_string();
    let iterable = build_expr(inner.next().unwrap())?;

    let mut body = Vec::new();
    for item in inner {
        if item.as_rule() == Rule::statement {
            body.push(build_statement(item)?);
        }
    }

    Ok(Statement::ForEach {
        variable,
        iterable,
        body,
        line,
    })
}

fn build_for_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();
    let variable = inner.next().unwrap().as_str().to_string();
    let start = build_expr(inner.next().unwrap())?;
    let end = build_expr(inner.next().unwrap())?;

    let mut step = None;
    let mut body = Vec::new();

    for item in inner {
        match item.as_rule() {
            Rule::step_clause => {
                let step_expr = item.into_inner().next().unwrap();
                step = Some(build_expr(step_expr)?);
            }
            Rule::statement => {
                body.push(build_statement(item)?);
            }
            _ => {}
        }
    }

    Ok(Statement::For {
        variable,
        start,
        end,
        step,
        body,
        line,
    })
}

fn build_while_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();
    let condition = build_expr(inner.next().unwrap())?;

    let mut body = Vec::new();
    for item in inner {
        if item.as_rule() == Rule::statement {
            body.push(build_statement(item)?);
        }
    }

    Ok(Statement::While {
        condition,
        body,
        line,
    })
}

fn build_function_def(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();
    let mut is_async = false;

    let mut next = inner.next().unwrap();
    if next.as_str().eq_ignore_ascii_case("async") {
        is_async = true;
        next = inner.next().unwrap();
    }

    let name = next.as_str().to_string();
    let mut params = Vec::new();
    let mut body = Vec::new();

    for item in inner {
        match item.as_rule() {
            Rule::param_list => {
                for param in item.into_inner() {
                    params.push(build_param(param)?);
                }
            }
            Rule::statement => {
                body.push(build_statement(item)?);
            }
            _ => {}
        }
    }

    Ok(Statement::Function {
        name,
        params,
        body,
        is_async,
        line,
    })
}

fn build_sub_def(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();
    let mut is_async = false;

    let mut next = inner.next().unwrap();
    if next.as_str().eq_ignore_ascii_case("async") {
        is_async = true;
        next = inner.next().unwrap();
    }

    let name = next.as_str().to_string();
    let mut params = Vec::new();
    let mut body = Vec::new();

    for item in inner {
        match item.as_rule() {
            Rule::param_list => {
                for param in item.into_inner() {
                    params.push(build_param(param)?);
                }
            }
            Rule::statement => {
                body.push(build_statement(item)?);
            }
            _ => {}
        }
    }

    Ok(Statement::Sub {
        name,
        params,
        body,
        is_async,
        line,
    })
}

fn build_param(pair: Pair<Rule>) -> BuildResult<Param> {
    let mut mode = ParamMode::ByVal;
    let mut name: Option<String> = None;

    for item in pair.clone().into_inner() {
        match item.as_rule() {
            Rule::param_modifier => {
                mode = match item.as_str().to_lowercase().as_str() {
                    "byref" => ParamMode::ByRef,
                    _ => ParamMode::ByVal,
                };
            }
            Rule::ident => {
                name = Some(item.as_str().to_string());
            }
            _ => {}
        }
    }

    let Some(name) = name else {
        return Err(BuildError::from_pair(&pair, "Expected parameter name"));
    };

    Ok(Param { name, mode })
}

fn build_return_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let value = pair.into_inner().next().map(build_expr).transpose()?;
    Ok(Statement::Return { value, line })
}

fn build_call_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();
    let function = inner.next().unwrap().as_str().to_string();

    let mut args = Vec::new();
    if let Some(arg_list) = inner.next() {
        for arg in arg_list.into_inner() {
            args.push(build_expr(arg)?);
        }
    }

    Ok(Statement::Call {
        function,
        args,
        line,
    })
}

fn build_export_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();
    let source = build_expr(inner.next().unwrap())?;
    let destination = build_expr(inner.next().unwrap())?;

    // Check for append mode and with_clause
    let mut append = false;
    let mut options = None;

    for next_pair in inner {
        match next_pair.as_rule() {
            Rule::export_mode => {
                append = true;
            }
            Rule::with_clause => {
                options = Some(build_expr(next_pair.into_inner().next().unwrap())?);
            }
            _ => {
                // Handle any other future options
                options = Some(build_expr(next_pair)?);
            }
        }
    }

    Ok(Statement::Export {
        source,
        destination,
        append,
        options,
        line,
    })
}

fn build_import_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();

    // Parse file_list (comma-separated expressions)
    let file_list = inner.next().unwrap();
    let sources: Vec<Expr> = file_list
        .into_inner()
        .map(build_expr)
        .collect::<BuildResult<Vec<_>>>()?;

    // Parse optional sheet clause, target identifier, and options
    let mut sheet_name = None;
    let mut target = String::new();
    let mut options = ImportOptions::default();

    for p in inner {
        match p.as_rule() {
            Rule::sheet_clause => {
                let sheet_expr = p.into_inner().next().unwrap();
                sheet_name = Some(build_expr(sheet_expr)?);
            }
            Rule::ident => {
                target = p.as_str().to_string();
            }
            Rule::import_options => {
                options = build_import_options(p)?;
            }
            Rule::with_clause => {
                // For backward compatibility, ignore with clause but don't error
                // This allows old scripts using "with {...}" to continue working
            }
            _ => {}
        }
    }

    Ok(Statement::Import {
        sources,
        target,
        sheet_name,
        options,
        line,
    })
}

fn build_append_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();

    // Get target variable name
    let target = inner.next().unwrap().as_str().to_string();

    let mut distinct = false;
    let mut key = None;
    let mut source_expr = None;

    for p in inner {
        match p.as_rule() {
            Rule::append_type => {
                distinct = true;
            }
            Rule::append_key => {
                let key_pair = p.into_inner().next().unwrap();
                let parsed = build_literal(key_pair.clone())?;
                let key_str = match parsed {
                    Literal::String(s) if !s.is_empty() => s,
                    Literal::String(_) => {
                        return Err(BuildError::from_pair(
                            &key_pair,
                            "Append key cannot be empty",
                        ))
                    }
                    _ => {
                        return Err(BuildError::from_pair(
                            &key_pair,
                            "Append key must be a string",
                        ))
                    }
                };
                key = Some(key_str);
            }
            Rule::expr => {
                source_expr = Some(build_expr(p)?);
            }
            _ => {
                source_expr = Some(build_expr(p)?);
            }
        }
    }

    let source = source_expr
        .ok_or_else(|| BuildError::new(line, 0, "Missing source expression in append statement"))?;

    // Validate: if distinct is used, key must be present
    if distinct && key.is_none() {
        return Err(BuildError::new(
            line,
            0,
            "append distinct requires 'on' clause with key column",
        ));
    }

    // Validate: if key is present, distinct must be used
    if key.is_some() && !distinct {
        return Err(BuildError::new(
            line,
            0,
            "'on' clause can only be used with 'append distinct'",
        ));
    }

    Ok(Statement::Append {
        target,
        source,
        distinct,
        key,
        line,
    })
}

fn build_upsert_stmt(pair: Pair<Rule>, line: usize) -> BuildResult<Statement> {
    let mut inner = pair.into_inner();

    // Get target variable name
    let target = inner.next().unwrap().as_str().to_string();

    // Get source expression
    let source = build_expr(inner.next().unwrap())?;

    // Skip "on" keyword (handled by grammar) and get the key
    let key_pair = inner.next().unwrap();
    let parsed = build_literal(key_pair.clone())?;
    let key = match parsed {
        Literal::String(s) if !s.is_empty() => s,
        Literal::String(_) => {
            return Err(BuildError::from_pair(
                &key_pair,
                "Upsert key cannot be empty",
            ))
        }
        _ => {
            return Err(BuildError::from_pair(
                &key_pair,
                "Upsert key must be a string",
            ))
        }
    };

    Ok(Statement::Upsert {
        target,
        source,
        key,
        line,
    })
}

fn build_import_options(pair: Pair<Rule>) -> BuildResult<ImportOptions> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::without_headers => Ok(ImportOptions::without_headers()),
        Rule::named_params => {
            let mut options = ImportOptions::default();
            for param in inner.into_inner() {
                let mut param_inner = param.into_inner();
                let key_pair = param_inner.next().unwrap();
                let key = key_pair.as_str();
                let value_pair = param_inner.next().unwrap();
                let value = build_expr(value_pair.clone())?;

                match key {
                    "headers" => {
                        if let Expr::Literal(Literal::Bool(b)) = value {
                            options.has_headers = Some(b);
                        } else {
                            return Err(BuildError::from_pair(
                                &value_pair,
                                "headers option must be a boolean (true or false)",
                            ));
                        }
                    }
                    _ => {
                        return Err(BuildError::from_pair(
                            &key_pair,
                            format!("Unknown import option: {key}"),
                        ));
                    }
                }
            }
            Ok(options)
        }
        _ => Ok(ImportOptions::default()),
    }
}

/// Build an expression from a pest pair.
pub fn build_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    match pair.as_rule() {
        Rule::expr => {
            // expr contains a single join_expr
            let inner = pair.into_inner().next().unwrap();
            build_join_expr(inner)
        }
        Rule::join_expr => build_join_expr(pair),
        Rule::or_expr => build_or_expr(pair),
        Rule::and_expr => build_and_expr(pair),
        Rule::not_expr => build_not_expr(pair),
        Rule::comparison_expr => build_comparison_expr(pair),
        Rule::additive_expr => build_additive_expr(pair),
        Rule::multiplicative_expr => build_multiplicative_expr(pair),
        Rule::unary_expr => build_unary_expr(pair),
        Rule::postfix_expr => build_postfix_expr(pair),
        Rule::primary_expr => build_primary_expr(pair),
        Rule::literal => build_literal_expr(pair),
        Rule::query_expr => build_query_expr(pair),
        Rule::fetch_expr => build_fetch_expr(pair),
        Rule::lambda_expr => build_lambda_expr(pair),
        Rule::array_literal => build_array_literal(pair),
        Rule::object_literal => build_object_literal(pair),
        Rule::ident => Ok(Expr::Variable(pair.as_str().to_string())),
        _ => {
            // Try to descend into inner
            if let Some(inner) = pair.into_inner().next() {
                build_expr(inner)
            } else {
                Err(BuildError::new(0, 0, "Empty expression"))
            }
        }
    }
}

fn build_join_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    // join_expr = { or_expr ~ (join_op ~ or_expr ~ join_condition?)* }
    let mut inner = pair.into_inner();
    let mut left = build_or_expr(inner.next().unwrap())?;

    while let Some(pair) = inner.next() {
        if let Rule::join_op = pair.as_rule() {
            let join_inner_pair = pair.clone();
            let join_inner = pair.into_inner().next().unwrap();
            let join_type = match join_inner.as_rule() {
                Rule::inner_join => JoinType::Inner,
                Rule::left_join => JoinType::Left,
                Rule::right_join => JoinType::Right,
                Rule::full_join => JoinType::Full,
                _ => return Err(BuildError::from_pair(&join_inner_pair, "Unknown join type")),
            };

            // Get the right side expression
            let right = build_or_expr(inner.next().unwrap())?;

            // Check for join condition
            let condition = if let Some(cond_pair) = inner.next() {
                if cond_pair.as_rule() == Rule::join_condition {
                    let cond_inner = cond_pair.into_inner().next().unwrap();
                    if cond_inner.as_rule() == Rule::join_key_pair {
                        // Handle "col1" = "col2" syntax
                        let mut key_inner = cond_inner.into_inner();
                        let left_pair = key_inner.next().unwrap();
                        let right_pair = key_inner.next().unwrap();

                        // Parse join keys using build_literal for consistency
                        let left_col = match build_literal(left_pair.clone())? {
                            Literal::String(s) if !s.is_empty() => s,
                            Literal::String(_) => {
                                return Err(BuildError::from_pair(
                                    &left_pair,
                                    "Left join key cannot be empty",
                                ))
                            }
                            _ => {
                                return Err(BuildError::from_pair(
                                    &left_pair,
                                    "Left join key must be a string",
                                ))
                            }
                        };

                        let right_col = match build_literal(right_pair.clone())? {
                            Literal::String(s) if !s.is_empty() => s,
                            Literal::String(_) => {
                                return Err(BuildError::from_pair(
                                    &right_pair,
                                    "Right join key cannot be empty",
                                ))
                            }
                            _ => {
                                return Err(BuildError::from_pair(
                                    &right_pair,
                                    "Right join key must be a string",
                                ))
                            }
                        };

                        JoinCondition::OnColumns {
                            left: left_col,
                            right: right_col,
                        }
                    } else {
                        // Handle simple "id" syntax (cond_inner is a string rule)
                        let key_pair = cond_inner.clone();
                        let key = match build_literal(cond_inner)? {
                            Literal::String(s) if !s.is_empty() => s,
                            Literal::String(_) => {
                                return Err(BuildError::from_pair(
                                    &key_pair,
                                    "Join key cannot be empty",
                                ))
                            }
                            _ => {
                                return Err(BuildError::from_pair(
                                    &key_pair,
                                    "Join key must be a string",
                                ))
                            }
                        };
                        JoinCondition::On(key)
                    }
                } else {
                    // Not a join condition, should not happen with correct grammar
                    return Err(BuildError::from_pair(
                        &cond_pair,
                        "Expected join condition after join expression",
                    ));
                }
            } else {
                // No join condition provided - grammar requires it
                return Err(BuildError::from_pair(
                    &join_inner_pair,
                    "Join requires an 'on' condition",
                ));
            };

            left = Expr::Join {
                left: Box::new(left),
                right: Box::new(right),
                join_type,
                condition,
            };
        }
    }

    Ok(left)
}

fn build_or_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let mut inner = pair.into_inner();
    let mut left = build_and_expr(inner.next().unwrap())?;

    while let Some(next_pair) = inner.next() {
        // Skip or_kw, get the next and_expr
        if next_pair.as_rule() == Rule::or_kw {
            let right_pair = inner.next().unwrap();
            let right = build_and_expr(right_pair)?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        } else {
            let right = build_and_expr(next_pair)?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
    }

    Ok(left)
}

fn build_and_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let mut inner = pair.into_inner();
    let mut left = build_not_expr(inner.next().unwrap())?;

    while let Some(next_pair) = inner.next() {
        // Skip and_kw, get the next not_expr
        if next_pair.as_rule() == Rule::and_kw {
            let right_pair = inner.next().unwrap();
            let right = build_not_expr(right_pair)?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        } else {
            let right = build_not_expr(next_pair)?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
    }

    Ok(left)
}

fn build_not_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let pair_for_error = pair.clone();
    let mut inner = pair.into_inner();

    // Check if we have any inner pairs
    if let Some(first) = inner.next() {
        // Check if it's not_kw rule or starts with "not"
        if first.as_rule() == Rule::not_kw {
            // We have a NOT operator, get the comparison expression
            let operand = build_comparison_expr(inner.next().unwrap())?;
            Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
            })
        } else {
            // No NOT operator, just build the comparison expression
            build_comparison_expr(first)
        }
    } else {
        // Empty not_expr - shouldn't happen with our grammar
        Err(BuildError::from_pair(&pair_for_error, "Empty not_expr"))
    }
}

fn build_comparison_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let mut inner = pair.into_inner();
    let left = build_additive_expr(inner.next().unwrap())?;

    if let Some(next) = inner.next() {
        match next.as_rule() {
            Rule::is_null_check => {
                let is_not = next.as_str().to_lowercase().contains("not");
                let null_check = Expr::Binary {
                    left: Box::new(left),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal(Literal::Null)),
                };
                if is_not {
                    Ok(Expr::Unary {
                        op: UnaryOp::Not,
                        operand: Box::new(null_check),
                    })
                } else {
                    Ok(null_check)
                }
            }
            Rule::comparison_op => {
                let op = build_comparison_op(&next)?;
                let right = build_additive_expr(inner.next().unwrap())?;
                Ok(Expr::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                })
            }
            _ => Ok(left),
        }
    } else {
        Ok(left)
    }
}

/// Maps a parsed comparison operator token to its corresponding `BinaryOp` variant.
///
/// Recognizes the operators: `=`, `==`, `!=`, `<>`, `<`, `<=`, `>`, `>=`, case-insensitive `like`, and case-insensitive `in`.
///
/// # Returns
///
/// `BinaryOp` matching the operator, or a `BuildError` when the token is unrecognized.
///
/// # Examples
///
/// ```
/// use pest::iterators::Pair;
/// // This example demonstrates expected mapping; constructing a real `Pair` requires the parser.
/// // Assume `pair.as_str()` yields "==", then:
/// // let op = build_comparison_op(&pair).unwrap();
/// // assert_eq!(op, BinaryOp::Eq);
/// ```
fn build_comparison_op(pair: &Pair<Rule>) -> BuildResult<BinaryOp> {
    let s = pair.as_str();
    match s {
        "=" | "==" => Ok(BinaryOp::Eq),
        "!=" | "<>" => Ok(BinaryOp::Ne),
        "<" => Ok(BinaryOp::Lt),
        "<=" => Ok(BinaryOp::Le),
        ">" => Ok(BinaryOp::Gt),
        ">=" => Ok(BinaryOp::Ge),
        _ if s.eq_ignore_ascii_case("like") => Ok(BinaryOp::Like),
        _ if s.eq_ignore_ascii_case("in") => Ok(BinaryOp::In),
        _ => Err(BuildError::from_pair(
            pair,
            format!("Unknown comparison operator: {s}"),
        )),
    }
}

/// Builds an expression AST node for a sequence of additions and subtractions.
///
/// Parses a `pair` containing an additive expression and folds left-to-right into
/// nested `Expr::Binary` nodes using `BinaryOp::Add` for `"+"` and `BinaryOp::Sub` for `"-"`.
///
/// # Returns
///
/// An `Expr` representing the parsed additive expression.
///
/// # Examples
///
/// ```ignore
/// // Given a `pair` that matches an additive expression (e.g. from the parser),
/// // build the corresponding AST expression.
/// let expr = build_additive_expr(pair).unwrap();
/// ```
fn build_additive_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let mut inner = pair.into_inner();
    let mut left = build_multiplicative_expr(inner.next().unwrap())?;

    while let Some(op_pair) = inner.next() {
        // op_pair should be add_op rule
        let op_str = op_pair.as_str();
        let op = if op_str == "+" {
            BinaryOp::Add
        } else {
            BinaryOp::Sub
        };
        let right = build_multiplicative_expr(inner.next().unwrap())?;
        left = Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        };
    }

    Ok(left)
}

/// Builds an expression tree for a sequence of multiplicative operations (`*`, `/`, `%`) from a parse `pair`.
///
/// Operators are parsed left-to-right and combined into left-associative `Expr::Binary` nodes.
///
/// # Returns
/// An `Expr` representing the parsed multiplicative expression.
///
/// # Examples
///
/// ```
/// // Equivalent AST for the expression `a * b / c`
/// use piptable_core::ast::{Expr, BinaryOp};
///
/// let expr = Expr::Binary {
///     left: Box::new(Expr::Binary {
///         left: Box::new(Expr::Variable("a".into())),
///         op: BinaryOp::Mul,
///         right: Box::new(Expr::Variable("b".into())),
///     }),
///     op: BinaryOp::Div,
///     right: Box::new(Expr::Variable("c".into())),
/// };
/// ```
fn build_multiplicative_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let mut inner = pair.into_inner();
    let mut left = build_unary_expr(inner.next().unwrap())?;

    while let Some(op_pair) = inner.next() {
        // op_pair should be mul_op rule
        let op_str = op_pair.as_str();
        let op = match op_str {
            "*" => BinaryOp::Mul,
            "/" => BinaryOp::Div,
            "%" => BinaryOp::Mod,
            _ => unreachable!("unexpected mul_op: {}", op_str),
        };
        let right = build_unary_expr(inner.next().unwrap())?;
        left = Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        };
    }

    Ok(left)
}

/// Builds an AST node for a unary expression.
///
/// Produces `Expr::Unary` with `UnaryOp::Neg` when the operator token is `"-"`;
/// for unary `"+"` the operand is returned unchanged.
///
/// # Examples
///
/// ```ignore
/// // Constructing the equivalent result directly:
/// let operand = Expr::Literal(Literal::Int(1));
/// let neg = Expr::Unary { op: UnaryOp::Neg, operand: Box::new(operand.clone()) };
/// assert!(matches!(neg, Expr::Unary { .. }));
/// ```
fn build_unary_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    match first.as_rule() {
        Rule::unary_op => {
            let op_str = first.as_str();
            let operand = build_postfix_expr(inner.next().unwrap())?;
            if op_str == "-" {
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                })
            } else {
                // Unary + is a no-op
                Ok(operand)
            }
        }
        _ => build_postfix_expr(first),
    }
}

fn build_postfix_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let mut inner = pair.into_inner();
    let mut expr = build_primary_expr(inner.next().unwrap())?;

    // Collect all postfix operations first to look ahead
    let postfixes: Vec<_> = inner.collect();
    let mut i = 0;

    while i < postfixes.len() {
        let postfix = &postfixes[i];
        match postfix.as_rule() {
            Rule::field_access => {
                let field = postfix
                    .clone()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str()
                    .to_string();

                // Look ahead to see if this is followed by call_args (method call)
                if i + 1 < postfixes.len() && postfixes[i + 1].as_rule() == Rule::call_args {
                    // This is a method call: object.method(args)
                    let mut args = Vec::new();
                    if let Some(arg_list) = postfixes[i + 1].clone().into_inner().next() {
                        for arg in arg_list.into_inner() {
                            args.push(build_expr(arg)?);
                        }
                    }
                    expr = Expr::MethodCall {
                        object: Box::new(expr),
                        method: field,
                        args,
                    };
                    i += 2; // Skip both field_access and call_args
                } else {
                    // Regular field access
                    expr = Expr::FieldAccess {
                        object: Box::new(expr),
                        field,
                    };
                    i += 1;
                }
            }
            Rule::array_index => {
                let index = build_expr(postfix.clone().into_inner().next().unwrap())?;
                expr = Expr::ArrayIndex {
                    array: Box::new(expr),
                    index: Box::new(index),
                };
                i += 1;
            }
            Rule::type_assertion => {
                let type_name = build_type_name(postfix.clone().into_inner().next().unwrap())?;
                expr = Expr::TypeAssertion {
                    expr: Box::new(expr),
                    type_name,
                };
                i += 1;
            }
            Rule::call_args => {
                let mut args = Vec::new();
                if let Some(arg_list) = postfix.clone().into_inner().next() {
                    for arg in arg_list.into_inner() {
                        args.push(build_expr(arg)?);
                    }
                }
                if let Expr::Variable(name) = expr {
                    expr = Expr::Call {
                        function: name,
                        args,
                    };
                } else {
                    expr = Expr::CallExpr {
                        callee: Box::new(expr),
                        args,
                    };
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(expr)
}

fn build_primary_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let inner = pair.into_inner().next().unwrap();
    build_expr(inner)
}

fn build_literal_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let inner = pair.into_inner().next().unwrap();
    let literal = build_literal(inner)?;
    Ok(Expr::Literal(literal))
}

fn build_literal(pair: Pair<Rule>) -> BuildResult<Literal> {
    match pair.as_rule() {
        Rule::null => Ok(Literal::Null),
        Rule::boolean => {
            let b = pair.as_str().eq_ignore_ascii_case("true");
            Ok(Literal::Bool(b))
        }
        Rule::integer => {
            let n: i64 = pair
                .as_str()
                .parse()
                .map_err(|_| BuildError::from_pair(&pair, "Invalid integer"))?;
            Ok(Literal::Int(n))
        }
        Rule::float => {
            let f: f64 = pair
                .as_str()
                .parse()
                .map_err(|_| BuildError::from_pair(&pair, "Invalid float"))?;
            Ok(Literal::Float(f))
        }
        Rule::string => {
            let s = pair.as_str();
            // Remove quotes and unescape
            let unescaped = unescape_string(&s[1..s.len() - 1]);
            Ok(Literal::String(unescaped))
        }
        Rule::interval => {
            let mut inner = pair.into_inner();
            let value: i64 = inner
                .next()
                .unwrap()
                .as_str()
                .parse()
                .map_err(|_| BuildError::new(0, 0, "Invalid interval value"))?;
            let unit = build_interval_unit(inner.next().unwrap())?;
            Ok(Literal::Interval { value, unit })
        }
        _ => Err(BuildError::from_pair(
            &pair,
            format!("Unknown literal type: {:?}", pair.as_rule()),
        )),
    }
}

fn unescape_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('/') => result.push('/'),
                Some('b') => result.push('\u{0008}'),
                Some('f') => result.push('\u{000C}'),
                Some('u') => {
                    // Unicode escape: \uXXXX
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(ch) = char::from_u32(code) {
                            result.push(ch);
                        }
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn build_interval_unit(pair: Pair<Rule>) -> BuildResult<piptable_core::IntervalUnit> {
    use piptable_core::IntervalUnit;
    let s = pair.as_str().to_lowercase();
    match s.as_str() {
        "millisecond" | "milliseconds" => Ok(IntervalUnit::Millisecond),
        "second" | "seconds" => Ok(IntervalUnit::Second),
        "minute" | "minutes" => Ok(IntervalUnit::Minute),
        "hour" | "hours" => Ok(IntervalUnit::Hour),
        "day" | "days" => Ok(IntervalUnit::Day),
        "week" | "weeks" => Ok(IntervalUnit::Week),
        "month" | "months" => Ok(IntervalUnit::Month),
        "year" | "years" => Ok(IntervalUnit::Year),
        _ => Err(BuildError::from_pair(&pair, "Unknown interval unit")),
    }
}

fn build_type_name(pair: Pair<Rule>) -> BuildResult<piptable_core::TypeName> {
    use piptable_core::TypeName;
    let s = pair.as_str().to_lowercase();
    match s.as_str() {
        "int" => Ok(TypeName::Int),
        "float" => Ok(TypeName::Float),
        "string" => Ok(TypeName::String),
        "bool" => Ok(TypeName::Bool),
        "timestamp" => Ok(TypeName::Timestamp),
        "duration" => Ok(TypeName::Duration),
        "array" => Ok(TypeName::Array),
        "object" => Ok(TypeName::Object),
        "table" => Ok(TypeName::Table),
        _ => Err(BuildError::from_pair(&pair, "Unknown type name")),
    }
}

fn build_lvalue(pair: Pair<Rule>) -> BuildResult<piptable_core::LValue> {
    use piptable_core::LValue;

    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut lvalue = LValue::Variable(name);

    for postfix in inner {
        match postfix.as_rule() {
            Rule::field_access => {
                let field = postfix.into_inner().next().unwrap().as_str().to_string();
                lvalue = LValue::Field {
                    object: Box::new(lvalue),
                    field,
                };
            }
            Rule::array_index => {
                let index = build_expr(postfix.into_inner().next().unwrap())?;
                lvalue = LValue::Index {
                    array: Box::new(lvalue),
                    index: Box::new(index),
                };
            }
            _ => {}
        }
    }

    Ok(lvalue)
}

fn build_query_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let sql_query_pair = pair.into_inner().next().unwrap();
    let query = build_sql_query(sql_query_pair)?;
    Ok(Expr::Query(Box::new(query)))
}

/// Build a SQL query from a pest pair.
pub fn build_sql_query(pair: Pair<Rule>) -> BuildResult<SqlQuery> {
    let mut with_clause = None;
    let mut select = SelectClause {
        distinct: false,
        items: Vec::new(),
    };
    let mut from = None;
    let mut joins = Vec::new();
    let mut where_clause = None;
    let mut group_by = None;
    let mut having = None;
    let mut order_by = None;
    let mut limit = None;
    let mut offset = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::with_clause_sql => {
                with_clause = Some(build_with_clause(inner)?);
            }
            Rule::select_clause => {
                select = build_select_clause(inner)?;
            }
            Rule::from_clause => {
                from = Some(build_from_clause(inner)?);
            }
            Rule::join_clause => {
                joins.push(build_join_clause(inner)?);
            }
            Rule::where_clause => {
                let expr = build_expr(inner.into_inner().next().unwrap())?;
                where_clause = Some(Box::new(expr));
            }
            Rule::group_by_clause => {
                let mut exprs = Vec::new();
                for expr_pair in inner.into_inner() {
                    exprs.push(build_expr(expr_pair)?);
                }
                group_by = Some(exprs);
            }
            Rule::having_clause => {
                let expr = build_expr(inner.into_inner().next().unwrap())?;
                having = Some(Box::new(expr));
            }
            Rule::order_by_clause => {
                let mut items = Vec::new();
                for item_pair in inner.into_inner() {
                    items.push(build_order_by_item(item_pair)?);
                }
                order_by = Some(items);
            }
            Rule::limit_clause => {
                let mut limit_inner = inner.into_inner();
                let limit_expr = build_expr(limit_inner.next().unwrap())?;
                limit = Some(Box::new(limit_expr));

                if let Some(offset_expr_pair) = limit_inner.next() {
                    let offset_expr = build_expr(offset_expr_pair)?;
                    offset = Some(Box::new(offset_expr));
                }
            }
            _ => {}
        }
    }

    Ok(SqlQuery {
        with_clause,
        select,
        from,
        joins,
        where_clause,
        group_by,
        having,
        order_by,
        limit,
        offset,
        trigger: None,
    })
}

fn build_with_clause(pair: Pair<Rule>) -> BuildResult<piptable_core::WithClause> {
    let mut recursive = false;
    let mut ctes = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_str().eq_ignore_ascii_case("recursive") {
            recursive = true;
        } else if inner.as_rule() == Rule::cte {
            ctes.push(build_cte(inner)?);
        }
    }

    Ok(piptable_core::WithClause { recursive, ctes })
}

fn build_cte(pair: Pair<Rule>) -> BuildResult<piptable_core::Cte> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    let mut columns = None;
    let mut query = None;

    for item in inner {
        match item.as_rule() {
            Rule::ident => {
                let cols = columns.get_or_insert_with(Vec::new);
                cols.push(item.as_str().to_string());
            }
            Rule::sql_query => {
                query = Some(Box::new(build_sql_query(item)?));
            }
            _ => {}
        }
    }

    Ok(piptable_core::Cte {
        name,
        columns,
        query: query.expect("CTE must contain a query"),
    })
}

fn build_select_clause(pair: Pair<Rule>) -> BuildResult<SelectClause> {
    let mut distinct = false;
    let mut items = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::distinct_kw {
            distinct = true;
        } else if inner.as_rule() == Rule::select_list {
            // Check if it's SELECT * (no child pairs for literal *)
            let list_text = inner.as_str().trim();
            if list_text == "*" {
                items.push(SelectItem {
                    expr: Expr::Variable("*".to_string()),
                    alias: None,
                });
            } else {
                // Has select_item children
                for item in inner.into_inner() {
                    if item.as_rule() == Rule::select_item {
                        items.push(build_select_item(item)?);
                    }
                }
            }
        }
    }

    Ok(SelectClause { distinct, items })
}

fn build_select_item(pair: Pair<Rule>) -> BuildResult<SelectItem> {
    let mut inner = pair.into_inner();
    let expr = build_expr(inner.next().unwrap())?;
    let alias = inner.next().map(|p| p.as_str().to_string());

    Ok(SelectItem { expr, alias })
}

fn build_from_clause(pair: Pair<Rule>) -> BuildResult<FromClause> {
    let mut inner = pair.into_inner();
    let table_ref_pair = inner.next().unwrap();
    let source = build_table_ref(table_ref_pair)?;

    // Alias can be either `AS ident` or just `alias_ident`
    let alias = inner.next().map(|p| p.as_str().to_string());

    Ok(FromClause { source, alias })
}

fn build_table_ref(pair: Pair<Rule>) -> BuildResult<TableRef> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::table_function => {
            let mut func_inner = inner.into_inner();
            let name = func_inner.next().unwrap().as_str().to_string();
            let mut args = Vec::new();

            if let Some(arg_list) = func_inner.next() {
                for arg in arg_list.into_inner() {
                    args.push(build_func_arg(arg)?);
                }
            }

            Ok(TableRef::Function { name, args })
        }
        Rule::qualified_name => {
            let parts: Vec<_> = inner.into_inner().map(|p| p.as_str().to_string()).collect();
            if parts.len() == 1 {
                Ok(TableRef::Table(parts.into_iter().next().unwrap()))
            } else if parts.len() == 2 {
                Ok(TableRef::Qualified {
                    database: parts[0].clone(),
                    schema: None,
                    table: parts[1].clone(),
                })
            } else {
                Ok(TableRef::Qualified {
                    database: parts[0].clone(),
                    schema: Some(parts[1].clone()),
                    table: parts[2].clone(),
                })
            }
        }
        Rule::string => {
            let s = inner.as_str();
            let path = s[1..s.len() - 1].to_string();
            Ok(TableRef::File(path))
        }
        Rule::sql_query => {
            let query = build_sql_query(inner)?;
            Ok(TableRef::Subquery(Box::new(query)))
        }
        _ if inner.as_str().eq_ignore_ascii_case("stdin") => Ok(TableRef::Stdin),
        _ => Err(BuildError::from_pair(&inner, "Unknown table reference")),
    }
}

fn build_func_arg(pair: Pair<Rule>) -> BuildResult<piptable_core::FunctionArg> {
    use piptable_core::FunctionArg;

    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    if first.as_rule() == Rule::ident {
        // Check if this is a named argument (has =>)
        if let Some(value_pair) = inner.next() {
            let name = first.as_str().to_string();
            let value = build_expr(value_pair)?;
            Ok(FunctionArg::Named { name, value })
        } else {
            // Just an identifier as expression
            Ok(FunctionArg::Positional(Expr::Variable(
                first.as_str().to_string(),
            )))
        }
    } else {
        let expr = build_expr(first)?;
        Ok(FunctionArg::Positional(expr))
    }
}

fn build_join_clause(pair: Pair<Rule>) -> BuildResult<piptable_core::JoinClause> {
    use piptable_core::{JoinClause, JoinType};

    let mut join_type = JoinType::Inner;
    let mut table = TableRef::Table(String::new());
    let mut alias = None;
    let mut on_clause = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::join_type => {
                let s = inner.as_str().to_lowercase();
                if s.contains("left") {
                    join_type = JoinType::Left;
                } else if s.contains("right") {
                    join_type = JoinType::Right;
                } else if s.contains("cross") {
                    join_type = JoinType::Cross;
                } else if s.contains("full") {
                    join_type = JoinType::Full;
                } else {
                    join_type = JoinType::Inner;
                }
            }
            Rule::table_ref => {
                table = build_table_ref(inner)?;
            }
            Rule::ident => {
                alias = Some(inner.as_str().to_string());
            }
            Rule::expr => {
                on_clause = Some(Box::new(build_expr(inner)?));
            }
            _ => {}
        }
    }

    Ok(JoinClause {
        join_type,
        table,
        alias,
        on_clause,
    })
}

fn build_order_by_item(pair: Pair<Rule>) -> BuildResult<OrderByItem> {
    let mut inner = pair.into_inner();
    let expr = build_expr(inner.next().unwrap())?;

    let direction = inner.next().map_or(SortDirection::Asc, |p| {
        // p is sort_direction rule
        if p.as_str().eq_ignore_ascii_case("desc") {
            SortDirection::Desc
        } else {
            SortDirection::Asc
        }
    });

    Ok(OrderByItem { expr, direction })
}

fn build_fetch_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let mut inner = pair.into_inner();
    let url = build_expr(inner.next().unwrap())?;
    let options = inner
        .next()
        .map(|p| build_expr(p).map(Box::new))
        .transpose()?;

    Ok(Expr::Fetch {
        url: Box::new(url),
        options,
    })
}

fn build_lambda_expr(pair: Pair<Rule>) -> BuildResult<Expr> {
    let span = pair.as_span();
    let (line, col) = span.start_pos().line_col();
    let inner = pair.into_inner();

    let mut params = Vec::new();

    // Parse lambda parameters and body
    // Grammar: (ident ~ "=>" ~ expr) | ("(" ~ lambda_params? ~ ")" ~ "=>" ~ expr)
    for part in inner {
        match part.as_rule() {
            Rule::ident => {
                // Single parameter without parentheses: x => expr
                params.push(part.as_str().to_string());
            }
            Rule::lambda_params => {
                // Multiple parameters in parentheses: (x, y) => expr
                for param in part.into_inner() {
                    if param.as_rule() == Rule::ident {
                        params.push(param.as_str().to_string());
                    }
                }
            }
            Rule::expr => {
                // This is the body expression
                let body = build_expr(part)?;
                return Ok(Expr::Lambda {
                    params,
                    body: Box::new(body),
                });
            }
            _ => {
                // Skip other tokens like "=>"
            }
        }
    }

    Err(BuildError::new(line, col, "Invalid lambda expression"))
}

fn build_array_literal(pair: Pair<Rule>) -> BuildResult<Expr> {
    let mut items = Vec::new();
    for item in pair.into_inner() {
        items.push(build_expr(item)?);
    }
    Ok(Expr::Array(items))
}

fn build_object_literal(pair: Pair<Rule>) -> BuildResult<Expr> {
    let mut fields = Vec::new();
    for field in pair.into_inner() {
        let mut field_inner = field.into_inner();
        let key_pair = field_inner.next().unwrap();
        let key = if key_pair.as_rule() == Rule::string {
            let s = key_pair.as_str();
            s[1..s.len() - 1].to_string()
        } else {
            key_pair.as_str().to_string()
        };
        let value = build_expr(field_inner.next().unwrap())?;
        fields.push((key, value));
    }
    Ok(Expr::Object(fields))
}
