use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{AppState, ViewMode};
use crate::ui::theme::Theme;

// CHANGES: Enhanced status bar to show success/error messages and last action
pub fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    if let Some(ref success) = state.success_message {
        let success_line = Line::from(vec![
            Span::styled("✓ ", Theme::success()),
            Span::styled(success, Theme::success()),
            Span::raw("  "),
            Span::styled("[Will auto-clear]", Theme::help_text()),
        ]);
        let status_bar = Paragraph::new(success_line).style(Theme::status_bar());
        frame.render_widget(status_bar, area);
        return;
    }
    
    if let Some(ref error) = state.error_message {
        let error_line = Line::from(vec![
            Span::styled("✗ ", Theme::error()),
            Span::styled(error, Theme::error()),
        ]);
        let status_bar = Paragraph::new(error_line).style(Theme::status_bar());
        frame.render_widget(status_bar, area);
        return;
    }

    let shortcuts = if state.is_filtering() {
        vec![
            ("Type", "to filter"),
            ("ESC", "Exit filter"),
            ("Backspace", "Delete"),
            ("Enter", "Apply"),
        ]
    } else if state.show_confirmation {
        vec![
            ("Enter", "Confirm"),
            ("ESC", "Cancel"),
        ]
    } else {
        match state.view_mode {
            ViewMode::Dashboard => {
                vec![
                    ("q", "Quit"),
                    ("Tab", "Next Tab"),
                    ("1-4", "Jump to Tab"),
                    ("r", "Refresh"),
                    ("d", "View List"),
                ]
            }
            ViewMode::ResourceList => {
                vec![
                    ("q", "Quit"),
                    ("Tab", "Next Tab"),
                    ("r", "Refresh"),
                    ("d", "Dashboard"),
                    ("/", "Filter"),
                    ("↑↓", "Navigate"),
                    ("Enter", "Details"),
                ]
            }
            ViewMode::ResourceDetail => {
                vec![
                    ("q", "Quit"),
                    ("↑↓", "Select Action"),
                    ("Enter", "Execute"),
                    ("ESC", "Back to List"),
                ]
            }
        }
    };

    let mut spans: Vec<Span> = shortcuts
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(*key, Theme::help_key()),
                Span::styled(": ", Theme::help_text()),
                Span::styled(*desc, Theme::help_text()),
                Span::styled("  ", Theme::help_text()),
            ]
        })
        .collect();

    if let Some(ref last_action) = state.last_action {
        if let Some(ref last_time) = state.last_action_time {
            let time_str = last_time.format("%H:%M:%S").to_string();
            spans.push(Span::styled(" | ", Theme::help_text()));
            spans.push(Span::styled("Last: ", Theme::help_text()));
            spans.push(Span::styled(last_action, Theme::help_key()));
            spans.push(Span::styled(" (", Theme::help_text()));
            spans.push(Span::styled(time_str, Theme::help_text()));
            spans.push(Span::styled(")", Theme::help_text()));
        }
    }

    let status_line = Line::from(spans);
    let status_bar = Paragraph::new(status_line).style(Theme::status_bar());

    frame.render_widget(status_bar, area);
}