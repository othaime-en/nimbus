use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::app::AppState;
use crate::core::CloudResource;
use crate::ui::theme::Theme;

pub async fn render_detail_view(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    if state.show_confirmation {
        render_with_confirmation(frame, area, state).await;
    } else {
        render_detail_content(frame, area, state).await;
    }
}

async fn render_detail_content(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let resource_idx = match state.get_selected_resource_index() {
        Some(idx) => idx,
        None => {
            render_no_selection(frame, area);
            return;
        }
    };

    let resources = match state.resources.try_read() {
        Ok(r) => r,
        Err(_) => {
            render_loading(frame, area);
            return;
        }
    };

    let resource = match resources.get(resource_idx) {
        Some(r) => r,
        None => {
            render_no_selection(frame, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(10),
            Constraint::Length(12),
        ])
        .split(area);

    render_resource_header(frame, chunks[0], resource.as_ref());
    render_resource_metadata(frame, chunks[1], resource.as_ref());
    render_available_actions(frame, chunks[2], resource.as_ref(), state);
}

fn render_resource_header(frame: &mut Frame, area: Rect, resource: &dyn CloudResource) {
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Name: ", Theme::help_text()),
            Span::styled(resource.name(), Theme::title()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Type: ", Theme::help_text()),
            Span::raw(resource.resource_type().as_str()),
            Span::raw("    "),
            Span::styled("Provider: ", Theme::help_text()),
            Span::raw(resource.provider().as_str()),
            Span::raw("    "),
            Span::styled("Region: ", Theme::help_text()),
            Span::raw(resource.region()),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Resource Details")
                .style(Theme::border()),
        )
        .style(Theme::help_text());

    frame.render_widget(paragraph, area);
}

fn render_resource_metadata(frame: &mut Frame, area: Rect, resource: &dyn CloudResource) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_basic_info(frame, chunks[0], resource);
    render_tags_and_cost(frame, chunks[1], resource);
}

fn render_basic_info(frame: &mut Frame, area: Rect, resource: &dyn CloudResource) {
    let created = resource
        .created_at()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let state_style = match resource.state() {
        crate::core::ResourceState::Running => Theme::success(),
        crate::core::ResourceState::Stopped => Theme::warning(),
        crate::core::ResourceState::Error => Theme::error(),
        _ => Theme::help_text(),
    };

    let rows = vec![
        Row::new(vec!["ID", resource.id()]),
        Row::new(vec!["State", resource.state().as_str()]).style(state_style),
        Row::new(vec!["Region", resource.region()]),
        Row::new(vec!["Created", &created]),
    ];

    let widths = [Constraint::Length(12), Constraint::Min(20)];

    let table = Table::new(rows, widths)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Basic Information")
                .style(Theme::border()),
        )
        .column_spacing(2);

    frame.render_widget(table, area);
}

fn render_tags_and_cost(frame: &mut Frame, area: Rect, resource: &dyn CloudResource) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let cost = resource
        .cost_per_month()
        .map(|c| format!("${:.2}/month", c))
        .unwrap_or_else(|| "N/A".to_string());

    let cost_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Monthly Cost: ", Theme::help_text()),
            Span::styled(cost, Theme::title()),
        ]),
    ];

    let cost_widget = Paragraph::new(cost_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Cost")
                .style(Theme::border()),
        )
        .style(Theme::help_text());

    frame.render_widget(cost_widget, chunks[0]);

    let tags = resource.tags();
    let tag_items: Vec<ListItem> = if tags.is_empty() {
        vec![ListItem::new("No tags")]
    } else {
        tags.iter()
            .map(|(key, value)| ListItem::new(format!("{}: {}", key, value)))
            .collect()
    };

    let tags_widget = List::new(tag_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Tags ({})", tags.len()))
            .style(Theme::border()),
    );

    frame.render_widget(tags_widget, chunks[1]);
}

fn render_available_actions(
    frame: &mut Frame,
    area: Rect,
    resource: &dyn CloudResource,
    state: &AppState,
) {
    let actions = resource.supported_actions();

    if actions.is_empty() {
        let text = vec![Line::from("No actions available for this resource")];
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Available Actions")
                    .style(Theme::border()),
            )
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    let action_items: Vec<ListItem> = actions
        .iter()
        .enumerate()
        .map(|(idx, action)| {
            let prefix = if idx == state.selected_action {
                "> "
            } else {
                "  "
            };

            let warning = if action.is_destructive() {
                " [DESTRUCTIVE]"
            } else {
                ""
            };

            let style = if idx == state.selected_action {
                if action.is_destructive() {
                    Theme::error()
                } else {
                    Theme::selected_row()
                }
            } else if action.is_destructive() {
                Theme::warning()
            } else {
                Style::default()
            };

            ListItem::new(format!("{}{}{}", prefix, action.as_str(), warning)).style(style)
        })
        .collect();

    let help_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("↑↓", Theme::help_key()),
            Span::raw(": Select  "),
            Span::styled("Enter", Theme::help_key()),
            Span::raw(": Execute  "),
            Span::styled("ESC", Theme::help_key()),
            Span::raw(": Back"),
        ]),
    ];

    let list = List::new(action_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Available Actions")
            .style(Theme::border()),
    );

    let help = Paragraph::new(help_text).style(Theme::help_text());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    frame.render_widget(list, chunks[0]);
    frame.render_widget(help, chunks[1]);
}

async fn render_with_confirmation(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    render_detail_content(frame, area, state).await;

    let popup_area = centered_rect(60, 40, area);
    render_confirmation_dialog(frame, popup_area, state);
}

fn render_confirmation_dialog(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("⚠ Confirm Action")
        .style(Theme::warning())
        .block(Block::default().borders(Borders::ALL).style(Theme::warning()));

    let message = Paragraph::new(state.confirmation_message.as_str())
        .wrap(Wrap { trim: true })
        .style(Theme::help_text())
        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));

    let buttons = Paragraph::new(vec![Line::from(vec![
        Span::styled("Enter", Theme::help_key()),
        Span::raw(": Confirm  "),
        Span::styled("ESC", Theme::help_key()),
        Span::raw(": Cancel"),
    ])])
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(
        Block::default()
            .style(Style::default().bg(ratatui::style::Color::Black)),
        area,
    );
    frame.render_widget(title, chunks[0]);
    frame.render_widget(message, chunks[1]);
    frame.render_widget(buttons, chunks[2]);
}

fn render_loading(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("Loading...", Theme::title())),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Resource Details")
                .style(Theme::border()),
        )
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_no_selection(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from("No resource selected"),
        Line::from(""),
        Line::from("Press ESC to return to list"),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Resource Details")
                .style(Theme::border()),
        )
        .style(Theme::help_text())
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}