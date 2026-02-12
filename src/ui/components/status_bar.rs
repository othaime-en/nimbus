use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use crate::app::{AppState, TabIndex};
use crate::ui::theme::Theme;

pub fn render_tabs(frame: &mut Frame, area: Rect, state: &AppState) {
    let tab_titles: Vec<Line> = TabIndex::all()
        .iter()
        .map(|tab| Line::from(tab.as_str()))
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title("Nimbus"))
        .select(state.active_tab.index())
        .style(Theme::tab_inactive())
        .highlight_style(Theme::tab_active());

    frame.render_widget(tabs, area);
}

pub fn render_tab_content(frame: &mut Frame, area: Rect, state: &AppState) {
    let content = match state.active_tab {
        TabIndex::AWS => create_tab_placeholder("AWS", "AWS resources will appear here"),
        TabIndex::GCP => create_tab_placeholder("GCP", "GCP resources will appear here"),
        TabIndex::Azure => create_tab_placeholder("Azure", "Azure resources will appear here"),
        TabIndex::AllClouds => {
            create_tab_placeholder("All Clouds", "Combined view of all cloud resources")
        }
    };

    frame.render_widget(content, area);
}

fn create_tab_placeholder<'a>(title: &'a str, message: &'a str) -> Paragraph<'a> {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(title, Theme::title())),
        Line::from(""),
        Line::from(message),
        Line::from(""),
        Line::from("No resources loaded yet."),
        Line::from(""),
        Line::from("Phase 1.3: TUI Shell Complete ✓"),
    ];

    Paragraph::new(text).block(Block::default().borders(Borders::ALL).style(Theme::border()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tab_placeholder() {
        let widget = create_tab_placeholder("Test", "Test message");
        // Basic smoke test to ensure widget creation doesn't panic
        assert!(true);
    }
}