use ratatui::style::{Color, Modifier, Style};
use crate::core::ResourceType;

pub struct Theme;

impl Theme {
    pub fn tab_active() -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn tab_inactive() -> Style {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    }

    pub fn status_bar() -> Style {
        Style::default().fg(Color::White).bg(Color::Blue)
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

    pub fn table_header() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_row() -> Style {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    }

    pub fn error() -> Style {
        Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD)
    }

    pub fn success() -> Style {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    }

    pub fn warning() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    pub fn filter_active() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .bg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }

    pub fn filter_inactive() -> Style {
        Style::default().fg(Color::White).bg(Color::Black)
    }

    pub fn spinner() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    // CHANGES: Added cache_age style
    pub fn cache_age() -> Style {
        Style::default()
            .fg(Color::Yellow)
    }
}

pub fn resource_icon(resource_type: ResourceType) -> &'static str {
    match resource_type {
        ResourceType::Compute => "[EC2]",
        ResourceType::Database => "[RDS]",
        ResourceType::Storage => "[S3] ",
        ResourceType::LoadBalancer => "[ELB]",
        ResourceType::DNS => "[R53]",
        ResourceType::Container => "[ECS]",
        ResourceType::Serverless => "[λ]  ",
        ResourceType::Network => "[VPC]",
        ResourceType::Cache => "[???]",
        ResourceType::Queue => "[SQS]",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_icons() {
        assert_eq!(resource_icon(ResourceType::Compute), "[EC2]");
        assert_eq!(resource_icon(ResourceType::Database), "[RDS]");
        assert_eq!(resource_icon(ResourceType::Storage), "[S3] ");
        assert_eq!(resource_icon(ResourceType::LoadBalancer), "[ELB]");
        assert_eq!(resource_icon(ResourceType::DNS), "[R53]");
    }
}