use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::Theme;

pub async fn render_resource_list(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let (filter_area, table_area) = if state.is_filtering() || !state.filter_text.is_empty() {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, area)
    };

    if let Some(filter_area) = filter_area {
        render_filter_input(frame, filter_area, state);
    }

    if state.loading {
        render_loading(frame, table_area);
        return;
    }

    if let Some(ref error) = state.error_message {
        render_error(frame, table_area, error);
        return;
    }

    let resources = state.resources.try_read();
    
    if resources.is_err() {
        render_message(frame, table_area, "Loading resources...");
        return;
    }

    let resources = resources.unwrap();

    if resources.is_empty() {
        render_empty_state(frame, table_area);
        return;
    }

    if state.filtered_resources.is_empty() && !state.filter_text.is_empty() {
        render_no_matches(frame, table_area, &state.filter_text);
        return;
    }

    let header_cells = ["Type", "Name", "ID", "State", "Region", "Cost/Month"]
        .iter()
        .map(|h| Cell::from(*h).style(Theme::table_header()));
    let header = Row::new(header_cells).height(1).style(Theme::table_header());

    let rows: Vec<Row> = state
        .filtered_resources
        .iter()
        .enumerate()
        .filter_map(|(display_idx, &resource_idx)| {
            resources.get(resource_idx).map(|resource| {
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
                if display_idx == state.selected_index {
                    row = row.style(Theme::selected_row());
                }
                row
            })
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

    let title = if state.filtered_resources.len() != resources.len() {
        format!(
            "Resources ({} of {} shown)",
            state.filtered_resources.len(),
            resources.len()
        )
    } else {
        format!("Resources ({})", resources.len())
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(Theme::border()),
        )
        .column_spacing(1);

    frame.render_widget(table, table_area);
}

fn render_filter_input(frame: &mut Frame, area: Rect, state: &AppState) {
    let filter_text = if state.is_filtering() {
        format!("Filter: {}█", state.filter_text)
    } else {
        format!("Filter: {}", state.filter_text)
    };

    let style = if state.is_filtering() {
        Theme::filter_active()
    } else {
        Theme::filter_inactive()
    };

    let input = Paragraph::new(filter_text)
        .style(style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if state.is_filtering() {
                    Theme::filter_active()
                } else {
                    Theme::border()
                }),
        );

    frame.render_widget(input, area);
}

fn render_loading(frame: &mut Frame, area: Rect) {
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner_idx = (chrono::Utc::now().timestamp_millis() / 100) as usize % frames.len();
    let spinner = frames[spinner_idx];

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(spinner, Theme::spinner()),
            Span::raw(" "),
            Span::styled("Loading resources...", Theme::title()),
        ]),
        Line::from(""),
        Line::from("Please wait..."),
    ];

    let paragraph = ratatui::widgets::Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).style(Theme::border()))
        .style(Theme::help_text())
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_error(frame: &mut Frame, area: Rect, error: &str) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("⚠ Error", Theme::error())),
        Line::from(""),
        Line::from(error),
        Line::from(""),
        Line::from(Span::styled("Press 'r' to retry", Theme::help_key())),
    ];

    let paragraph = ratatui::widgets::Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Error")
                .style(Theme::border()),
        )
        .style(Theme::help_text())
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_empty_state(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("No Resources Found", Theme::title())),
        Line::from(""),
        Line::from("No cloud resources are currently available."),
        Line::from("This could mean:"),
        Line::from("  • No resources exist in this account"),
        Line::from("  • The selected region has no resources"),
        Line::from("  • You don't have permission to view resources"),
        Line::from(""),
        Line::from(vec![
            Span::styled("r", Theme::help_key()),
            Span::raw(": Refresh  "),
            Span::styled("q", Theme::help_key()),
            Span::raw(": Quit"),
        ]),
    ];

    let paragraph = ratatui::widgets::Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).style(Theme::border()))
        .style(Theme::help_text())
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_no_matches(frame: &mut Frame, area: Rect, filter: &str) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("No Matches Found", Theme::title())),
        Line::from(""),
        Line::from(vec![
            Span::raw("No resources match filter: "),
            Span::styled(format!("\"{}\"", filter), Theme::help_key()),
        ]),
        Line::from(""),
        Line::from("Try:"),
        Line::from("  • Using different search terms"),
        Line::from("  • Clearing the filter (ESC)"),
        Line::from("  • Checking the spelling"),
        Line::from(""),
        Line::from(vec![
            Span::styled("ESC", Theme::help_key()),
            Span::raw(": Clear filter  "),
            Span::styled("Backspace", Theme::help_key()),
            Span::raw(": Edit filter"),
        ]),
    ];

    let paragraph = ratatui::widgets::Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("No Matches")
                .style(Theme::border()),
        )
        .style(Theme::help_text())
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_message(frame: &mut Frame, area: Rect, message: &str) {
    let text = vec![Line::from(""), Line::from(message)];

    let paragraph = ratatui::widgets::Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).style(Theme::border()))
        .alignment(ratatui::layout::Alignment::Center);

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