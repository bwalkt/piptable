use comrak::nodes::{AstNode, NodeValue};
use comrak::{parse_document, Arena, ComrakOptions};

use crate::error::{MarkdownError, Result};

#[derive(Debug, Clone)]
pub struct MarkdownTable {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct MarkdownTables {
    pub tables: Vec<MarkdownTable>,
}

#[derive(Debug, Clone, Copy)]
pub struct MarkdownOptions {
    pub min_table_rows: usize,
    pub min_table_cols: usize,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            min_table_rows: 2,
            min_table_cols: 2,
        }
    }
}

impl MarkdownTables {
    pub fn from_markdown(markdown: &str) -> Result<Self> {
        Self::from_markdown_with_options(markdown, MarkdownOptions::default())
    }

    pub fn from_markdown_with_options(markdown: &str, options: MarkdownOptions) -> Result<Self> {
        let arena = Arena::new();
        let mut options = ComrakOptions::default();
        options.extension.table = true;

        let root = parse_document(&arena, markdown, &options);
        let mut tables = Vec::new();
        collect_tables(root, &mut tables)?;

        let filtered = tables
            .into_iter()
            .filter(|table| {
                let row_count = table.rows.len();
                let col_count = table
                    .rows
                    .first()
                    .map(|row| row.len())
                    .or_else(|| table.headers.as_ref().map(|h| h.len()))
                    .unwrap_or(0);
                row_count >= options.min_table_rows && col_count >= options.min_table_cols
            })
            .collect();

        Ok(Self { tables: filtered })
    }
}

fn collect_tables<'a>(node: &'a AstNode<'a>, tables: &mut Vec<MarkdownTable>) -> Result<()> {
    for child in node.children() {
        match &child.data.borrow().value {
            NodeValue::Table(_) => {
                tables.push(extract_table(child)?);
            }
            _ => {
                collect_tables(child, tables)?;
            }
        }
    }
    Ok(())
}

fn extract_table<'a>(table_node: &'a AstNode<'a>) -> Result<MarkdownTable> {
    let mut headers: Option<Vec<String>> = None;
    let mut rows = Vec::new();

    for child in table_node.children() {
        if let NodeValue::TableRow(is_header) = &child.data.borrow().value {
            if *is_header {
                headers = Some(extract_table_row(child));
            } else {
                rows.push(extract_table_row(child));
            }
        }
    }

    if headers.is_none() && rows.is_empty() {
        return Err(MarkdownError::InvalidTable);
    }

    Ok(MarkdownTable { headers, rows })
}

fn extract_table_row<'a>(row_node: &'a AstNode<'a>) -> Vec<String> {
    let mut cells = Vec::new();
    for cell in row_node.children() {
        if let NodeValue::TableCell = &cell.data.borrow().value {
            cells.push(extract_text(cell));
        }
    }
    cells
}

fn extract_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();
    collect_text(node, &mut text);
    text
}

fn collect_text<'a>(node: &'a AstNode<'a>, output: &mut String) {
    match &node.data.borrow().value {
        NodeValue::Text(text) => {
            output.push_str(text);
        }
        NodeValue::Code(code) => {
            output.push_str(&code.literal);
        }
        NodeValue::SoftBreak | NodeValue::LineBreak => {
            output.push(' ');
        }
        _ => {
            for child in node.children() {
                collect_text(child, output);
            }
        }
    }
}
