use crate::error::Result;

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

pub struct AppState {
    pub active_tab: TabIndex,
    pub should_quit: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            active_tab: TabIndex::AWS,
            should_quit: false,
        }
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
}