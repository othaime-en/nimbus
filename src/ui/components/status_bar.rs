use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{AppState, ViewMode};
use crate::ui::theme::Theme;

pub fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let shortcuts = if state.is_filtering() {
        vec![
            ("Type", "to filter"),
            ("ESC", "Exit filter"),
            ("Backspace", "Delete"),
            ("Enter", "Apply"),
        ]
    } else {
        let view_hint = match state.view_mode {
            ViewMode::Dashboard => "List",
            ViewMode::ResourceList => "Dashboard",
        };
        
        let mut base_shortcuts = vec![
            ("q", "Quit"),
            ("Tab", "Next Tab"),
            ("1-4", "Jump to Tab"),
            ("r", "Refresh"),
            ("d", &format!("View {}", view_hint)),
        ];
        
        if matches!(state.view_mode, ViewMode::ResourceList) {
            base_shortcuts.extend_from_slice(&[
                ("/", "Filter"),
                ("↑↓", "Navigate"),
            ]);
        }
        
        base_shortcuts
    };

    let spans: Vec<Span> = shortcuts
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

    let status_line = Line::from(spans);
    let status_bar = Paragraph::new(status_line).style(Theme::status_bar());

    frame.render_widget(status_bar, area);
}