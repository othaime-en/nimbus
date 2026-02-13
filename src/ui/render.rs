use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::app::AppState;
use crate::ui::components::render_status_bar;
use crate::ui::tabs::{render_tab_content, render_tabs};

pub async fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.size());

    render_tabs(frame, chunks[0], state);
    render_tab_content(frame, chunks[1], state).await;
    render_status_bar(frame, chunks[2]);
}