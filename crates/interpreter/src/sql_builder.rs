//! SQL query building and translation utilities.

use crate::Interpreter;
use async_recursion::async_recursion;
use piptable_core::{
    BinaryOp, Expr, FromClause, JoinClause, JoinType, Literal, OrderByItem, PipResult,
    SelectClause, SelectItem, SortDirection, SqlQuery, TableRef, UnaryOp, Value,
};
use std::sync::Arc;

impl Interpreter {
    /// Evaluate a SQL query by converting it to string and executing.
    pub async fn eval_query(&mut self, query: &SqlQuery) -> PipResult<Value> {
        let sql = self.sql_query_to_string(query).await?;
        let batches = self.sql.query(&sql).await?;
        Ok(Value::Table(batches.into_iter().map(Arc::new).collect()))
    }

    /// Convert a SQL query AST to a SQL string.
    #[async_recursion]
    pub async fn sql_query_to_string(&mut self, query: &SqlQuery) -> PipResult<String> {
        let mut sql = String::new();

        // WITH clause
        if let Some(_with) = &query.with_clause {
            return Err(piptable_core::PipError::runtime(
                0,
                "WITH clause (Common Table Expressions) is not yet supported",
            ));
        }

        // SELECT clause
        sql.push_str("SELECT ");
        if query.select.distinct {
            sql.push_str("DISTINCT ");
        }
        sql.push_str(&self.select_clause_to_string(&query.select).await?);

        // FROM clause
        if let Some(from) = &query.from {
            sql.push_str(" FROM ");
            sql.push_str(&self.from_clause_to_string(from).await?);
        }

        // JOIN clauses
        for join in &query.joins {
            sql.push_str(&self.join_clause_to_string(join).await?);
        }

        // WHERE clause
        if let Some(where_expr) = &query.where_clause {
            sql.push_str(" WHERE ");
            sql.push_str(&self.expr_to_sql(where_expr).await?);
        }

        // GROUP BY
        if let Some(group_by) = &query.group_by {
            sql.push_str(" GROUP BY ");
            let mut exprs = Vec::new();
            for e in group_by {
                exprs.push(self.expr_to_sql(e).await?);
            }
            sql.push_str(&exprs.join(", "));
        }

        // HAVING
        if let Some(having) = &query.having {
            sql.push_str(" HAVING ");
            sql.push_str(&self.expr_to_sql(having).await?);
        }

        // ORDER BY
        if let Some(order_by) = &query.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(&self.order_by_to_string(order_by).await?);
        }

        // LIMIT
        if let Some(limit) = &query.limit {
            sql.push_str(" LIMIT ");
            sql.push_str(&self.expr_to_sql(limit).await?);
        }

        // OFFSET
        if let Some(offset) = &query.offset {
            sql.push_str(" OFFSET ");
            sql.push_str(&self.expr_to_sql(offset).await?);
        }

        Ok(sql)
    }

    pub(crate) async fn select_clause_to_string(
        &mut self,
        select: &SelectClause,
    ) -> PipResult<String> {
        let mut items = Vec::new();
        for item in &select.items {
            items.push(self.select_item_to_string(item).await?);
        }
        Ok(items.join(", "))
    }

    pub(crate) async fn select_item_to_string(&mut self, item: &SelectItem) -> PipResult<String> {
        let expr_str = self.expr_to_sql(&item.expr).await?;
        Ok(match &item.alias {
            Some(alias) => format!("{expr_str} AS {alias}"),
            None => expr_str,
        })
    }

    pub(crate) async fn from_clause_to_string(&mut self, from: &FromClause) -> PipResult<String> {
        // Pass the alias info to table_ref_to_string to avoid double aliasing
        let source = self
            .table_ref_to_string_with_context(&from.source, from.alias.is_some())
            .await?;
        Ok(match &from.alias {
            Some(alias) => format!("{source} AS {alias}"),
            None => source,
        })
    }

    // Helper that knows if an external alias will be applied
    async fn table_ref_to_string_with_context(
        &mut self,
        table_ref: &TableRef,
        has_external_alias: bool,
    ) -> PipResult<String> {
        match table_ref {
            TableRef::Table(name) => {
                // Check if this refers to a variable containing a Sheet or Table
                if let Some(value) = self.get_var(name).await {
                    // Check if we've already registered this variable
                    let sheet_tables = self.sheet_tables.read().await;
                    if let Some(existing_table) = sheet_tables.get(name) {
                        // Only auto-alias if no external alias is provided
                        if has_external_alias {
                            return Ok(existing_table.clone());
                        }
                        return Ok(format!("{} AS {}", existing_table, name));
                    }
                    drop(sheet_tables);

                    // Handle Sheet variables
                    if let Some(sheet) = value.as_sheet() {
                        // Convert Sheet to Table and register it
                        let table_name = self.register_sheet_as_table(name, sheet).await?;

                        // Remember that we registered this sheet
                        let mut sheet_tables = self.sheet_tables.write().await;
                        sheet_tables.insert(name.to_string(), table_name.clone());

                        // Only auto-alias if no external alias is provided
                        if has_external_alias {
                            return Ok(table_name);
                        }
                        return Ok(format!("{} AS {}", table_name, name));
                    }

                    // Handle Table variables (RecordBatch vectors)
                    if let Some(batches) = value.as_table() {
                        // Register the table directly
                        let table_name = self.register_table_variable(name, batches).await?;

                        // Remember that we registered this table
                        let mut sheet_tables = self.sheet_tables.write().await;
                        sheet_tables.insert(name.to_string(), table_name.clone());

                        // Only auto-alias if no external alias is provided
                        if has_external_alias {
                            return Ok(table_name);
                        }
                        return Ok(format!("{} AS {}", table_name, name));
                    }
                }
                // Otherwise, treat as regular table name
                Ok(name.clone())
            }
            // For other TableRef variants, delegate to the regular method
            _ => self.table_ref_to_string(table_ref).await,
        }
    }

    #[async_recursion]
    pub(crate) async fn table_ref_to_string(&mut self, table_ref: &TableRef) -> PipResult<String> {
        // Default to no external alias context
        match table_ref {
            TableRef::Table(_) => {
                self.table_ref_to_string_with_context(table_ref, false)
                    .await
            }
            TableRef::Qualified {
                database,
                schema,
                table,
            } => Ok(match schema {
                Some(s) => format!("{database}.{s}.{table}"),
                None => format!("{database}.{table}"),
            }),
            TableRef::File(path) => {
                // Register the file and return table name
                let table_name = self.register_file(path).await?;
                Ok(table_name)
            }
            TableRef::Function { name, args } => {
                let mut arg_strs = Vec::new();
                for a in args {
                    arg_strs.push(self.func_arg_to_string(a).await?);
                }
                Ok(format!("{}({})", name, arg_strs.join(", ")))
            }
            TableRef::Stdin => Ok("stdin".to_string()),
            TableRef::Subquery(query) => {
                let sql = self.sql_query_to_string(query).await?;
                Ok(format!("({sql})"))
            }
        }
    }

    pub(crate) async fn func_arg_to_string(
        &mut self,
        arg: &piptable_core::FunctionArg,
    ) -> PipResult<String> {
        match arg {
            piptable_core::FunctionArg::Positional(expr) => self.expr_to_sql(expr).await,
            piptable_core::FunctionArg::Named { name, value } => {
                let val_str = self.expr_to_sql(value).await?;
                Ok(format!("{name} => {val_str}"))
            }
        }
    }

    pub(crate) async fn join_clause_to_string(&mut self, join: &JoinClause) -> PipResult<String> {
        let join_type = match join.join_type {
            JoinType::Inner => " INNER JOIN ",
            JoinType::Left => " LEFT JOIN ",
            JoinType::Right => " RIGHT JOIN ",
            JoinType::Full => " FULL OUTER JOIN ",
            JoinType::Cross => " CROSS JOIN ",
        };

        // Pass alias context to avoid double aliasing
        let table = self
            .table_ref_to_string_with_context(&join.table, join.alias.is_some())
            .await?;
        let mut result = format!("{join_type}{table}");

        if let Some(alias) = &join.alias {
            result.push_str(&format!(" AS {alias}"));
        }

        if let Some(on) = &join.on_clause {
            result.push_str(" ON ");
            result.push_str(&self.expr_to_sql(on).await?);
        }

        Ok(result)
    }

    pub(crate) async fn order_by_to_string(
        &mut self,
        order_by: &[OrderByItem],
    ) -> PipResult<String> {
        let mut items = Vec::new();
        for item in order_by {
            items.push(self.order_item_to_string(item).await?);
        }
        Ok(items.join(", "))
    }

    pub(crate) async fn order_item_to_string(&mut self, item: &OrderByItem) -> PipResult<String> {
        let expr = self.expr_to_sql(&item.expr).await?;
        let dir = match item.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };
        Ok(format!("{expr} {dir}"))
    }

    /// Convert an expression to SQL string.
    #[async_recursion]
    pub async fn expr_to_sql(&mut self, expr: &Expr) -> PipResult<String> {
        match expr {
            Expr::Literal(lit) => Ok(self.literal_to_sql(lit)),
            Expr::Variable(name) => {
                if name == "*" {
                    Ok("*".to_string())
                } else {
                    // Quote identifier to avoid conflicts with SQL keywords
                    Ok(format!("\"{}\"", name))
                }
            }
            Expr::Binary { left, op, right } => {
                let l = self.expr_to_sql(left).await?;
                let r = self.expr_to_sql(right).await?;
                let op_str = self.binary_op_to_sql(*op);
                Ok(format!("({l} {op_str} {r})"))
            }
            Expr::Unary { op, operand } => {
                let val = self.expr_to_sql(operand).await?;
                match op {
                    UnaryOp::Neg => Ok(format!("-{val}")),
                    UnaryOp::Not => Ok(format!("NOT {val}")),
                }
            }
            Expr::FieldAccess { object, field } => {
                let obj = self.expr_to_sql(object).await?;
                Ok(format!("{obj}.{field}"))
            }
            Expr::Call { function, args } => {
                let mut arg_strs = Vec::new();
                for a in args {
                    arg_strs.push(self.expr_to_sql(a).await?);
                }
                Ok(format!("{}({})", function, arg_strs.join(", ")))
            }
            Expr::CallExpr { .. } => {
                // Fallback to evaluation for non-identifier callees (e.g., lambdas)
                let val = self.eval_expr(expr).await?;
                Ok(self.value_to_sql(&val))
            }
            _ => {
                // For complex expressions, evaluate and inline the result
                let val = self.eval_expr(expr).await?;
                Ok(self.value_to_sql(&val))
            }
        }
    }

    /// Renders a literal value to SQL syntax.
    pub(crate) fn literal_to_sql(&self, lit: &Literal) -> String {
        match lit {
            Literal::Null => "NULL".to_string(),
            Literal::Bool(b) => b.to_string().to_uppercase(),
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::String(s) => format!("'{}'", s.replace('\'', "''")),
            Literal::Interval { value, unit } => {
                use piptable_core::IntervalUnit;
                let unit_str = match unit {
                    IntervalUnit::Millisecond => "MILLISECOND",
                    IntervalUnit::Second => "SECOND",
                    IntervalUnit::Minute => "MINUTE",
                    IntervalUnit::Hour => "HOUR",
                    IntervalUnit::Day => "DAY",
                    IntervalUnit::Week => "WEEK",
                    IntervalUnit::Month => "MONTH",
                    IntervalUnit::Year => "YEAR",
                };
                format!("INTERVAL '{}' {}", value, unit_str)
            }
        }
    }

    /// Maps a binary operator to its SQL representation.
    pub(crate) fn binary_op_to_sql(&self, op: BinaryOp) -> &'static str {
        match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "=",
            BinaryOp::Ne => "<>",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "AND",
            BinaryOp::Or => "OR",
            BinaryOp::Concat => "||",
            BinaryOp::Like => "LIKE",
            BinaryOp::In => "IN",
        }
    }

    /// Renders a runtime value to a SQL literal.
    pub(crate) fn value_to_sql(&self, val: &Value) -> String {
        match val {
            Value::Null => "NULL".to_string(),
            Value::Bool(b) => b.to_string().to_uppercase(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => format!("'{}'", s.replace('\'', "''")),
            _ => "NULL".to_string(),
        }
    }
}
