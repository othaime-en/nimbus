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
    ResourceDetail,
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
    pub success_message: Option<String>,
    pub should_quit: bool,
    pub selected_action: usize,
    pub show_confirmation: bool,
    pub confirmation_message: String,
    pub last_action: Option<String>,
    pub last_action_time: Option<DateTime<Utc>>,
    pub cache_enabled: bool, // CHANGES: Added cache awareness
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
            view_mode: ViewMode::Dashboard,
            input_mode: InputMode::Normal,
            loading: false,
            last_refresh: None,
            error_message: None,
            success_message: None,
            should_quit: false,
            selected_action: 0,
            show_confirmation: false,
            confirmation_message: String::new(),
            last_action: None,
            last_action_time: None,
            cache_enabled: false, // CHANGES: Initialize cache_enabled
        }
    }

    pub fn with_providers(mut self, providers: Vec<Arc<RwLock<Box<dyn CloudProvider>>>>) -> Self {
        self.providers = providers;
        self
    }

    // CHANGES: Added method to enable cache awareness
    pub fn with_cache_enabled(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    // CHANGES: Added method to check if using cached data
    pub fn is_using_cache(&self) -> bool {
        self.cache_enabled && self.last_refresh.is_some()
    }

    // CHANGES: Added method to get cache age display string
    pub fn cache_age_display(&self) -> Option<String> {
        if !self.cache_enabled {
            return None;
        }

        self.last_refresh.map(|refresh_time| {
            let age = Utc::now().signed_duration_since(refresh_time);
            
            if age.num_minutes() < 1 {
                "just now".to_string()
            } else if age.num_minutes() < 60 {
                format!("{}m ago", age.num_minutes())
            } else if age.num_hours() < 24 {
                format!("{}h ago", age.num_hours())
            } else {
                format!("{}d ago", age.num_days())
            }
        })
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

    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Dashboard => ViewMode::ResourceList,
            ViewMode::ResourceList => ViewMode::Dashboard,
            ViewMode::ResourceDetail => ViewMode::ResourceList,
        };
    }

    pub fn enter_detail_view(&mut self) {
        if !self.filtered_resources.is_empty() {
            self.view_mode = ViewMode::ResourceDetail;
            self.selected_action = 0;
        }
    }

    pub fn exit_detail_view(&mut self) {
        self.view_mode = ViewMode::ResourceList;
        self.selected_action = 0;
        self.show_confirmation = false;
    }

    pub fn next_action(&mut self, max_actions: usize) {
        if max_actions > 0 {
            self.selected_action = (self.selected_action + 1) % max_actions;
        }
    }

    pub fn prev_action(&mut self, max_actions: usize) {
        if max_actions > 0 {
            if self.selected_action == 0 {
                self.selected_action = max_actions - 1;
            } else {
                self.selected_action -= 1;
            }
        }
    }

    pub fn show_action_confirmation(&mut self, message: String) {
        self.confirmation_message = message;
        self.show_confirmation = true;
    }

    pub fn cancel_confirmation(&mut self) {
        self.show_confirmation = false;
        self.confirmation_message.clear();
    }

    pub fn get_selected_resource_index(&self) -> Option<usize> {
        self.filtered_resources.get(self.selected_index).copied()
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn start_loading(&mut self) {
        self.loading = true;
        self.error_message = None;
        self.success_message = None;
    }

    pub fn stop_loading(&mut self) {
        self.loading = false;
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.success_message = None;
        self.loading = false;
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn set_success(&mut self, message: String) {
        self.success_message = Some(message);
        self.error_message = None;
        self.loading = false;
    }

    pub fn clear_success(&mut self) {
        self.success_message = None;
    }

    pub fn clear_messages(&mut self) {
        self.error_message = None;
        self.success_message = None;
    }

    pub fn record_action(&mut self, action_description: String) {
        self.last_action = Some(action_description);
        self.last_action_time = Some(Utc::now());
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

// CHANGES: Added Clone implementation to support background refresh
impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            providers: self.providers.clone(),
            active_tab: self.active_tab,
            resources: Arc::clone(&self.resources),
            filtered_resources: self.filtered_resources.clone(),
            selected_index: self.selected_index,
            filter_text: self.filter_text.clone(),
            view_mode: self.view_mode,
            input_mode: self.input_mode,
            loading: self.loading,
            last_refresh: self.last_refresh,
            error_message: self.error_message.clone(),
            success_message: self.success_message.clone(),
            should_quit: self.should_quit,
            selected_action: self.selected_action,
            show_confirmation: self.show_confirmation,
            confirmation_message: self.confirmation_message.clone(),
            last_action: self.last_action.clone(),
            last_action_time: self.last_action_time,
            cache_enabled: self.cache_enabled,
        }
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
    fn test_success_message() {
        let mut state = AppState::new();
        state.set_success("Action completed".to_string());
        assert_eq!(state.success_message, Some("Action completed".to_string()));
        assert!(state.error_message.is_none());
        
        state.clear_success();
        assert!(state.success_message.is_none());
    }

    #[test]
    fn test_clear_messages() {
        let mut state = AppState::new();
        state.set_error("Error".to_string());
        state.set_success("Success".to_string());
        
        state.clear_messages();
        assert!(state.error_message.is_none());
        assert!(state.success_message.is_none());
    }

    // CHANGES: Added tests for cache awareness
    #[test]
    fn test_cache_enabled() {
        let state = AppState::new().with_cache_enabled(true);
        assert!(state.cache_enabled);
    }

    #[test]
    fn test_cache_age_display() {
        let mut state = AppState::new().with_cache_enabled(true);
        state.last_refresh = Some(Utc::now());
        
        let age = state.cache_age_display();
        assert!(age.is_some());
        assert_eq!(age.unwrap(), "just now");
    }

    #[test]
    fn test_is_using_cache() {
        let mut state = AppState::new().with_cache_enabled(true);
        assert!(!state.is_using_cache());
        
        state.last_refresh = Some(Utc::now());
        assert!(state.is_using_cache());
    }
}