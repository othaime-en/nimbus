use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::Theme;

pub fn render_status_bar(frame: &mut Frame, area: Rect) {
    let shortcuts = vec![
        ("q", "Quit"),
        ("Tab", "Next Tab"),
        ("1-4", "Jump to Tab"),
        ("?", "Help"),
    ];

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