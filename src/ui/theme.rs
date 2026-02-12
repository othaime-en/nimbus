use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub fn tab_active() -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn tab_inactive() -> Style {
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
    }

    pub fn status_bar() -> Style {
        Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
    }

    pub fn border() -> Style {
        Style::default().fg(Color::Gray)
    }

    pub fn title() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn help_key() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    pub fn help_text() -> Style {
        Style::default().fg(Color::White)
    }
}