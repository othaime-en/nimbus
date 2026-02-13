use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::Theme;

pub async fn render_resource_list(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    if state.loading {
        render_loading(frame, area);
        return;
    }

    if let Some(ref error) = state.error_message {
        render_error(frame, area, error);
        return;
    }

    let resources = state.resources.try_read();
    
    if resources.is_err() {
        render_message(frame, area, "Loading resources...");
        return;
    }

    let resources = resources.unwrap();

    if resources.is_empty() {
        render_empty_state(frame, area);
        return;
    }

    let header_cells = ["Type", "Name", "ID", "State", "Region", "Cost/Month"]
        .iter()
        .map(|h| Cell::from(*h).style(Theme::table_header()));
    let header = Row::new(header_cells).height(1).style(Theme::table_header());

    let rows: Vec<Row> = resources
        .iter()
        .enumerate()
        .map(|(idx, resource)| {
            let cost = resource
                .cost_per_month()
                .map(|c| format!("${:.2}", c))
                .unwrap_or_else(|| "-".to_string());

            let cells = vec![
                Cell::from(resource.resource_type().as_str()),
                Cell::from(resource.name()),
                Cell::from(resource.id()),
                Cell::from(resource.state().as_str()).style(state_style(resource.state())),
                Cell::from(resource.region()),
                Cell::from(cost),
            ];

            let mut row = Row::new(cells).height(1);
            if state.filtered_resources.get(state.selected_index) == Some(&idx) {
                row = row.style(Theme::selected_row());
            }
            row
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Min(20),
        Constraint::Min(18),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Resources ({})", resources.len()))
                .style(Theme::border()),
        )
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn render_loading(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("Loading resources...", Theme::title())),
        Line::from(""),
        Line::from("Please wait..."),
    ];

    let paragraph = ratatui::widgets::Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).style(Theme::border()))
        .style(Theme::help_text());

    frame.render_widget(paragraph, area);
}

fn render_error(frame: &mut Frame, area: Rect, error: &str) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("Error", Theme::error())),
        Line::from(""),
        Line::from(error),
        Line::from(""),
        Line::from("Press 'r' to retry"),
    ];

    let paragraph = ratatui::widgets::Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Error")
                .style(Theme::border()),
        )
        .style(Theme::help_text());

    frame.render_widget(paragraph, area);
}

fn render_empty_state(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("No Resources Found", Theme::title())),
        Line::from(""),
        Line::from("No cloud resources are currently available."),
        Line::from(""),
        Line::from("Press 'r' to refresh"),
    ];

    let paragraph = ratatui::widgets::Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).style(Theme::border()))
        .style(Theme::help_text());

    frame.render_widget(paragraph, area);
}

fn render_message(frame: &mut Frame, area: Rect, message: &str) {
    let text = vec![
        Line::from(""),
        Line::from(message),
    ];

    let paragraph = ratatui::widgets::Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).style(Theme::border()));

    frame.render_widget(paragraph, area);
}

fn state_style(state: crate::core::ResourceState) -> Style {
    use crate::core::ResourceState;
    use ratatui::style::Color;

    match state {
        ResourceState::Running => Style::default().fg(Color::Green),
        ResourceState::Stopped => Style::default().fg(Color::Yellow),
        ResourceState::Terminated => Style::default().fg(Color::Red),
        ResourceState::Pending | ResourceState::Starting => Style::default().fg(Color::Cyan),
        ResourceState::Stopping => Style::default().fg(Color::Yellow),
        ResourceState::Error => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ResourceState::Unknown => Style::default().fg(Color::Gray),
    }
}