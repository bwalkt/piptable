//! # piptable-viz
//!
//! Visualization and chart generation for piptable.
//!
//! This crate generates chart specifications that can be rendered by:
//! - HTML/Chart.js output
//! - Tauri/React frontend (via IPC messages)
//! - Static image export

use piptable_core::{ChartType, PipResult};
use serde::{Deserialize, Serialize};

/// Chart specification for rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartSpec {
    pub chart_type: ChartKind,
    pub title: String,
    pub data: ChartData,
    pub options: ChartOptions,
}

/// Chart type for visualization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChartKind {
    Bar,
    Line,
    Pie,
    Scatter,
    Area,
}

/// Chart data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub labels: Vec<String>,
    pub datasets: Vec<Dataset>,
}

/// A dataset in a chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub label: String,
    pub data: Vec<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<String>,
}

/// Chart rendering options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_axis_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y_axis_label: Option<String>,
    pub show_legend: bool,
    pub stacked: bool,
    pub horizontal: bool,
}

/// Escape HTML special characters to prevent XSS.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

impl ChartSpec {
    /// Create a new chart specification.
    #[must_use]
    pub fn new(chart_type: ChartKind, title: impl Into<String>) -> Self {
        Self {
            chart_type,
            title: title.into(),
            data: ChartData {
                labels: Vec::new(),
                datasets: Vec::new(),
            },
            options: ChartOptions::default(),
        }
    }

    /// Convert to JSON string for IPC/frontend rendering.
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails.
    pub fn to_json(&self) -> PipResult<String> {
        serde_json::to_string(self).map_err(|e| piptable_core::PipError::Internal(e.to_string()))
    }

    /// Generate HTML with embedded Chart.js.
    #[must_use]
    pub fn to_html(&self) -> String {
        // Escape title for HTML context and JSON for script context
        let title = escape_html(&self.title);
        let json = serde_json::to_string(&self)
            .unwrap_or_default()
            .replace("</", "<\\/"); // Prevent script tag breakout

        let chart_type = match self.chart_type {
            ChartKind::Bar => "bar",
            ChartKind::Line => "line",
            ChartKind::Pie => "pie",
            ChartKind::Scatter => "scatter",
            ChartKind::Area => "line", // Area is line with fill
        };

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>{title}</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
</head>
<body>
    <canvas id="chart"></canvas>
    <script>
        const spec = {json};
        const ctx = document.getElementById('chart').getContext('2d');
        new Chart(ctx, {{
            type: '{chart_type}',
            data: spec.data,
            options: {{
                responsive: true,
                plugins: {{
                    title: {{
                        display: true,
                        text: spec.title
                    }},
                    legend: {{
                        display: spec.options.show_legend
                    }}
                }}
            }}
        }});
    </script>
</body>
</html>"#,
            title = title,
            json = json,
            chart_type = chart_type,
        )
    }
}

impl From<ChartType> for ChartKind {
    fn from(ct: ChartType) -> Self {
        match ct {
            ChartType::Bar => Self::Bar,
            ChartType::Line => Self::Line,
            ChartType::Pie => Self::Pie,
            ChartType::Scatter => Self::Scatter,
            ChartType::Area => Self::Area,
        }
    }
}

/// Message types for IPC with Tauri/React frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum VizMessage {
    /// Render a chart
    RenderChart(ChartSpec),
    /// Update chart data
    UpdateData { chart_id: String, data: ChartData },
    /// Clear a chart
    ClearChart { chart_id: String },
    /// Export chart to file
    ExportChart {
        chart_id: String,
        path: String,
        format: ExportFormat,
    },
}

/// Export formats for charts.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Png,
    Svg,
    Pdf,
    Html,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_spec_new() {
        let chart = ChartSpec::new(ChartKind::Bar, "Test Chart");
        assert_eq!(chart.title, "Test Chart");
        assert!(matches!(chart.chart_type, ChartKind::Bar));
    }

    #[test]
    fn test_chart_to_json() {
        let chart = ChartSpec::new(ChartKind::Line, "Test");
        let json = chart.to_json().unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("line"));
    }

    #[test]
    fn test_chart_to_html() {
        let chart = ChartSpec::new(ChartKind::Pie, "Pie Chart");
        let html = chart.to_html();
        assert!(html.contains("Chart.js"));
        assert!(html.contains("Pie Chart"));
    }
}
