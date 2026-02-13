use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Tabs},
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

pub async fn render_tab_content(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    crate::ui::resource_list::render_resource_list(frame, area, state).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_tabs() {
        // Smoke test to ensure function compiles
        assert!(true);
    }
}