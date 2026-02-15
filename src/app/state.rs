use crate::core::{CloudProvider, CloudResource};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabIndex {
    AWS,
    GCP,
    Azure,
    AllClouds,
}

impl TabIndex {
    pub fn all() -> Vec<TabIndex> {
        vec![
            TabIndex::AWS,
            TabIndex::GCP,
            TabIndex::Azure,
            TabIndex::AllClouds,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TabIndex::AWS => "AWS",
            TabIndex::GCP => "GCP",
            TabIndex::Azure => "Azure",
            TabIndex::AllClouds => "All Clouds",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            TabIndex::AWS => 0,
            TabIndex::GCP => 1,
            TabIndex::Azure => 2,
            TabIndex::AllClouds => 3,
        }
    }

    pub fn from_index(index: usize) -> Option<TabIndex> {
        match index {
            0 => Some(TabIndex::AWS),
            1 => Some(TabIndex::GCP),
            2 => Some(TabIndex::Azure),
            3 => Some(TabIndex::AllClouds),
            _ => None,
        }
    }

    pub fn next(&self) -> TabIndex {
        let all = Self::all();
        let current_index = self.index();
        let next_index = (current_index + 1) % all.len();
        all[next_index]
    }

    pub fn prev(&self) -> TabIndex {
        let all = Self::all();
        let current_index = self.index();
        let prev_index = if current_index == 0 {
            all.len() - 1
        } else {
            current_index - 1
        };
        all[prev_index]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Dashboard,
    ResourceList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
}

pub struct AppState {
    pub providers: Vec<Arc<RwLock<Box<dyn CloudProvider>>>>,
    pub active_tab: TabIndex,
    pub resources: Arc<RwLock<Vec<Box<dyn CloudResource>>>>,
    pub filtered_resources: Vec<usize>,
    pub selected_index: usize,
    pub filter_text: String,
    pub view_mode: ViewMode,
    pub input_mode: InputMode,
    pub loading: bool,
    pub last_refresh: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub should_quit: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            active_tab: TabIndex::AWS,
            resources: Arc::new(RwLock::new(Vec::new())),
            filtered_resources: Vec::new(),
            selected_index: 0,
            filter_text: String::new(),
            view_mode: ViewMode::ResourceList,
            input_mode: InputMode::Normal,
            loading: false,
            last_refresh: None,
            error_message: None,
            should_quit: false,
        }
    }

    pub fn with_providers(mut self, providers: Vec<Arc<RwLock<Box<dyn CloudProvider>>>>) -> Self {
        self.providers = providers;
        self
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
    }

    pub fn set_tab(&mut self, tab: TabIndex) {
        self.active_tab = tab;
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn start_loading(&mut self) {
        self.loading = true;
        self.error_message = None;
    }

    pub fn stop_loading(&mut self) {
        self.loading = false;
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.loading = false;
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn enter_filter_mode(&mut self) {
        self.input_mode = InputMode::Filter;
    }

    pub fn exit_filter_mode(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn is_filtering(&self) -> bool {
        self.input_mode == InputMode::Filter
    }

    pub fn push_filter_char(&mut self, c: char) {
        self.filter_text.push(c);
        self.apply_filter();
    }

    pub fn pop_filter_char(&mut self) {
        self.filter_text.pop();
        self.apply_filter();
    }

    pub fn clear_filter(&mut self) {
        self.filter_text.clear();
        self.apply_filter();
    }

    pub async fn refresh_resources(&mut self) -> crate::error::Result<()> {
        self.start_loading();

        let mut all_resources = Vec::new();
        let mut had_error = None;

        for provider in &self.providers {
            let provider = provider.read().await;
            match provider.list_all_resources().await {
                Ok(resources) => {
                    all_resources.extend(resources);
                }
                Err(e) => {
                    had_error = Some(format!("Failed to load resources: {}", e));
                    break;
                }
            }
        }

        if let Some(error) = had_error {
            self.set_error(error.clone());
            return Err(crate::error::NimbusError::Other(error));
        }

        let mut resources = self.resources.write().await;
        *resources = all_resources;
        drop(resources);

        self.apply_filter();
        self.last_refresh = Some(chrono::Utc::now());
        self.stop_loading();

        Ok(())
    }

    pub fn apply_filter(&mut self) {
        let filter_lower = self.filter_text.to_lowercase();
        
        if filter_lower.is_empty() {
            self.filtered_resources = (0..tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    self.resources.read().await.len()
                })
            })).collect();
        } else {
            self.filtered_resources = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let resources = self.resources.read().await;
                    resources
                        .iter()
                        .enumerate()
                        .filter(|(_, resource)| {
                            resource.name().to_lowercase().contains(&filter_lower)
                                || resource.id().to_lowercase().contains(&filter_lower)
                                || resource.resource_type().as_str().to_lowercase().contains(&filter_lower)
                                || resource.state().as_str().to_lowercase().contains(&filter_lower)
                                || resource.region().to_lowercase().contains(&filter_lower)
                        })
                        .map(|(idx, _)| idx)
                        .collect()
                })
            });
        }

        if self.selected_index >= self.filtered_resources.len() && !self.filtered_resources.is_empty() {
            self.selected_index = self.filtered_resources.len() - 1;
        }
    }

    pub fn next_resource(&mut self) {
        if !self.filtered_resources.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_resources.len();
        }
    }

    pub fn prev_resource(&mut self) {
        if !self.filtered_resources.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.filtered_resources.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn resource_count(&self) -> usize {
        self.filtered_resources.len()
    }

    pub fn total_resource_count(&self) -> usize {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.resources.read().await.len()
            })
        })
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_index_as_str() {
        assert_eq!(TabIndex::AWS.as_str(), "AWS");
        assert_eq!(TabIndex::GCP.as_str(), "GCP");
        assert_eq!(TabIndex::Azure.as_str(), "Azure");
        assert_eq!(TabIndex::AllClouds.as_str(), "All Clouds");
    }

    #[test]
    fn test_tab_index() {
        assert_eq!(TabIndex::AWS.index(), 0);
        assert_eq!(TabIndex::GCP.index(), 1);
        assert_eq!(TabIndex::Azure.index(), 2);
        assert_eq!(TabIndex::AllClouds.index(), 3);
    }

    #[test]
    fn test_tab_from_index() {
        assert_eq!(TabIndex::from_index(0), Some(TabIndex::AWS));
        assert_eq!(TabIndex::from_index(1), Some(TabIndex::GCP));
        assert_eq!(TabIndex::from_index(2), Some(TabIndex::Azure));
        assert_eq!(TabIndex::from_index(3), Some(TabIndex::AllClouds));
        assert_eq!(TabIndex::from_index(4), None);
    }

    #[test]
    fn test_tab_next() {
        assert_eq!(TabIndex::AWS.next(), TabIndex::GCP);
        assert_eq!(TabIndex::GCP.next(), TabIndex::Azure);
        assert_eq!(TabIndex::Azure.next(), TabIndex::AllClouds);
        assert_eq!(TabIndex::AllClouds.next(), TabIndex::AWS);
    }

    #[test]
    fn test_tab_prev() {
        assert_eq!(TabIndex::AWS.prev(), TabIndex::AllClouds);
        assert_eq!(TabIndex::AllClouds.prev(), TabIndex::Azure);
        assert_eq!(TabIndex::Azure.prev(), TabIndex::GCP);
        assert_eq!(TabIndex::GCP.prev(), TabIndex::AWS);
    }

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        assert_eq!(state.active_tab, TabIndex::AWS);
        assert!(!state.should_quit);
        assert!(!state.loading);
        assert_eq!(state.view_mode, ViewMode::ResourceList);
        assert_eq!(state.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_app_state_next_tab() {
        let mut state = AppState::new();
        state.next_tab();
        assert_eq!(state.active_tab, TabIndex::GCP);
    }

    #[test]
    fn test_app_state_prev_tab() {
        let mut state = AppState::new();
        state.prev_tab();
        assert_eq!(state.active_tab, TabIndex::AllClouds);
    }

    #[test]
    fn test_app_state_set_tab() {
        let mut state = AppState::new();
        state.set_tab(TabIndex::Azure);
        assert_eq!(state.active_tab, TabIndex::Azure);
    }

    #[test]
    fn test_app_state_quit() {
        let mut state = AppState::new();
        state.quit();
        assert!(state.should_quit);
    }

    #[test]
    fn test_app_state_loading() {
        let mut state = AppState::new();
        state.start_loading();
        assert!(state.loading);
        assert!(state.error_message.is_none());

        state.stop_loading();
        assert!(!state.loading);
    }

    #[test]
    fn test_app_state_error() {
        let mut state = AppState::new();
        state.set_error("Test error".to_string());
        assert_eq!(state.error_message, Some("Test error".to_string()));
        assert!(!state.loading);

        state.clear_error();
        assert!(state.error_message.is_none());
    }

    #[test]
    fn test_filter_mode() {
        let mut state = AppState::new();
        assert_eq!(state.input_mode, InputMode::Normal);
        assert!(!state.is_filtering());

        state.enter_filter_mode();
        assert_eq!(state.input_mode, InputMode::Filter);
        assert!(state.is_filtering());

        state.exit_filter_mode();
        assert_eq!(state.input_mode, InputMode::Normal);
        assert!(!state.is_filtering());
    }

    #[test]
    fn test_filter_text_manipulation() {
        let mut state = AppState::new();
        
        state.push_filter_char('t');
        state.push_filter_char('e');
        state.push_filter_char('s');
        state.push_filter_char('t');
        assert_eq!(state.filter_text, "test");

        state.pop_filter_char();
        assert_eq!(state.filter_text, "tes");

        state.clear_filter();
        assert_eq!(state.filter_text, "");
    }
}