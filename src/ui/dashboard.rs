use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

use crate::app::AppState;
use crate::core::{ResourceState, ResourceType};
use crate::ui::theme::Theme;
use std::collections::HashMap;

pub async fn render_dashboard(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    if state.loading {
        render_loading_dashboard(frame, area);
        return;
    }

    if let Some(ref error) = state.error_message {
        render_error_dashboard(frame, area, error);
        return;
    }

    let resources = state.resources.try_read();
    if resources.is_err() {
        render_loading_dashboard(frame, area);
        return;
    }

    let resources = resources.unwrap();
    if resources.is_empty() {
        render_empty_dashboard(frame, area);
        return;
    }

    let stats = calculate_dashboard_stats(&resources);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(10),
            Constraint::Min(8),
        ])
        .split(area);

    render_cost_summary(frame, chunks[0], &stats);
    render_resource_breakdown(frame, chunks[1], &stats);
    render_top_resources(frame, chunks[2], &stats);
}

struct DashboardStats {
    total_cost: f64,
    trend_percentage: f64,
    total_resources: usize,
    by_type: HashMap<ResourceType, TypeStats>,
    by_region: HashMap<String, RegionStats>,
    top_expensive: Vec<(String, String, f64)>,
}

struct TypeStats {
    count: usize,
    running: usize,
    stopped: usize,
    total_cost: f64,
}

struct RegionStats {
    count: usize,
    total_cost: f64,
}

fn calculate_dashboard_stats(resources: &[Box<dyn crate::core::CloudResource>]) -> DashboardStats {
    let mut by_type: HashMap<ResourceType, TypeStats> = HashMap::new();
    let mut by_region: HashMap<String, RegionStats> = HashMap::new();
    let mut total_cost = 0.0;
    let mut expensive_resources: Vec<(String, String, f64)> = Vec::new();

    for resource in resources {
        let cost = resource.cost_per_month().unwrap_or(0.0);
        total_cost += cost;

        let type_stats = by_type.entry(resource.resource_type()).or_insert(TypeStats {
            count: 0,
            running: 0,
            stopped: 0,
            total_cost: 0.0,
        });
        type_stats.count += 1;
        type_stats.total_cost += cost;
        
        match resource.state() {
            ResourceState::Running => type_stats.running += 1,
            ResourceState::Stopped => type_stats.stopped += 1,
            _ => {}
        }

        let region_stats = by_region
            .entry(resource.region().to_string())
            .or_insert(RegionStats {
                count: 0,
                total_cost: 0.0,
            });
        region_stats.count += 1;
        region_stats.total_cost += cost;

        if cost > 0.0 {
            expensive_resources.push((
                resource.name().to_string(),
                resource.resource_type().as_str().to_string(),
                cost,
            ));
        }
    }

    expensive_resources.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    expensive_resources.truncate(5);

    DashboardStats {
        total_cost,
        trend_percentage: 0.0,
        total_resources: resources.len(),
        by_type,
        by_region,
        top_expensive: expensive_resources,
    }
}

fn render_cost_summary(frame: &mut Frame, area: Rect, stats: &DashboardStats) {
    let trend_indicator = if stats.trend_percentage > 0.0 {
        format!("↑ {:.1}%", stats.trend_percentage)
    } else if stats.trend_percentage < 0.0 {
        format!("↓ {:.1}%", stats.trend_percentage.abs())
    } else {
        "→ 0%".to_string()
    };

    let trend_style = if stats.trend_percentage > 0.0 {
        Theme::warning()
    } else if stats.trend_percentage < 0.0 {
        Theme::success()
    } else {
        Theme::help_text()
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Monthly Cost: ", Theme::help_text()),
            Span::styled(
                format!("${:.2}", stats.total_cost),
                Theme::title(),
            ),
            Span::raw("  "),
            Span::styled(trend_indicator, trend_style),
            Span::raw("    "),
            Span::styled("Resources: ", Theme::help_text()),
            Span::styled(
                format!("{}", stats.total_resources),
                Theme::title(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Regions: ", Theme::help_text()),
            Span::raw(format!("{}  ", stats.by_region.len())),
            Span::styled("Types: ", Theme::help_text()),
            Span::raw(format!("{}", stats.by_type.len())),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Overview")
                .style(Theme::border()),
        )
        .style(Theme::help_text());

    frame.render_widget(paragraph, area);
}

fn render_resource_breakdown(frame: &mut Frame, area: Rect, stats: &DashboardStats) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_type_breakdown(frame, chunks[0], stats);
    render_region_breakdown(frame, chunks[1], stats);
}

fn render_type_breakdown(frame: &mut Frame, area: Rect, stats: &DashboardStats) {
    let header_cells = ["Type", "Count", "Running", "Stopped", "Cost/Month"]
        .iter()
        .map(|h| ratatui::widgets::Cell::from(*h).style(Theme::table_header()));
    let header = Row::new(header_cells).height(1).style(Theme::table_header());

    let mut type_list: Vec<(&ResourceType, &TypeStats)> = stats.by_type.iter().collect();
    type_list.sort_by(|a, b| b.1.total_cost.partial_cmp(&a.1.total_cost).unwrap());

    let rows: Vec<Row> = type_list
        .iter()
        .map(|(resource_type, type_stats)| {
            let cells = vec![
                ratatui::widgets::Cell::from(resource_type.as_str()),
                ratatui::widgets::Cell::from(format!("{}", type_stats.count)),
                ratatui::widgets::Cell::from(format!("{}", type_stats.running)),
                ratatui::widgets::Cell::from(format!("{}", type_stats.stopped)),
                ratatui::widgets::Cell::from(format!("${:.2}", type_stats.total_cost)),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(15),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Min(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("By Resource Type")
                .style(Theme::border()),
        )
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn render_region_breakdown(frame: &mut Frame, area: Rect, stats: &DashboardStats) {
    let header_cells = ["Region", "Resources", "Cost/Month"]
        .iter()
        .map(|h| ratatui::widgets::Cell::from(*h).style(Theme::table_header()));
    let header = Row::new(header_cells).height(1).style(Theme::table_header());

    let mut region_list: Vec<(&String, &RegionStats)> = stats.by_region.iter().collect();
    region_list.sort_by(|a, b| b.1.total_cost.partial_cmp(&a.1.total_cost).unwrap());

    let rows: Vec<Row> = region_list
        .iter()
        .map(|(region, region_stats)| {
            let cells = vec![
                ratatui::widgets::Cell::from(region.as_str()),
                ratatui::widgets::Cell::from(format!("{}", region_stats.count)),
                ratatui::widgets::Cell::from(format!("${:.2}", region_stats.total_cost)),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let widths = [
        Constraint::Min(15),
        Constraint::Length(12),
        Constraint::Length(14),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("By Region")
                .style(Theme::border()),
        )
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn render_top_resources(frame: &mut Frame, area: Rect, stats: &DashboardStats) {
    let header_cells = ["#", "Name", "Type", "Monthly Cost"]
        .iter()
        .map(|h| ratatui::widgets::Cell::from(*h).style(Theme::table_header()));
    let header = Row::new(header_cells).height(1).style(Theme::table_header());

    let rows: Vec<Row> = stats
        .top_expensive
        .iter()
        .enumerate()
        .map(|(idx, (name, resource_type, cost))| {
            let cells = vec![
                ratatui::widgets::Cell::from(format!("{}", idx + 1)),
                ratatui::widgets::Cell::from(name.as_str()),
                ratatui::widgets::Cell::from(resource_type.as_str()),
                ratatui::widgets::Cell::from(format!("${:.2}", cost))
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(3),
        Constraint::Min(20),
        Constraint::Length(15),
        Constraint::Length(15),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Top 5 Most Expensive Resources")
                .style(Theme::border()),
        )
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn render_loading_dashboard(frame: &mut Frame, area: Rect) {
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner_idx = (chrono::Utc::now().timestamp_millis() / 100) as usize % frames.len();
    let spinner = frames[spinner_idx];

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(spinner, Theme::spinner()),
            Span::raw(" "),
            Span::styled("Loading dashboard...", Theme::title()),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Dashboard")
                .style(Theme::border()),
        )
        .style(Theme::help_text())
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_error_dashboard(frame: &mut Frame, area: Rect, error: &str) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("⚠ Error", Theme::error())),
        Line::from(""),
        Line::from(error),
        Line::from(""),
        Line::from(Span::styled("Press 'r' to retry", Theme::help_key())),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Dashboard Error")
                .style(Theme::border()),
        )
        .style(Theme::help_text())
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_empty_dashboard(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("No Resources Found", Theme::title())),
        Line::from(""),
        Line::from("No cloud resources are currently available."),
        Line::from(""),
        Line::from(vec![
            Span::styled("r", Theme::help_key()),
            Span::raw(": Refresh  "),
            Span::styled("d", Theme::help_key()),
            Span::raw(": Switch to List View"),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Dashboard")
                .style(Theme::border()),
        )
        .style(Theme::help_text())
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, area);
}