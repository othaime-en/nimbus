use std::fmt;

/// Actions that can be performed on cloud resources.
/// 
/// Not all actions are supported by all resource types. Use
/// `CloudResource::supported_actions()` to determine which actions
/// are available for a specific resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Start a stopped resource (e.g., EC2 instance, RDS database)
    Start,
    /// Stop a running resource without terminating it
    Stop,
    /// Restart a resource (stop then start)
    Restart,
    /// Permanently delete/terminate a resource
    Terminate,
    /// View detailed information about the resource
    ViewDetails,
    /// View logs for the resource (if applicable)
    ViewLogs,
    /// Modify resource configuration
    Modify,
}

impl Action {
    /// Returns the human-readable name of the action.
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Start => "Start",
            Action::Stop => "Stop",
            Action::Restart => "Restart",
            Action::Terminate => "Terminate",
            Action::ViewDetails => "View Details",
            Action::ViewLogs => "View Logs",
            Action::Modify => "Modify",
        }
    }

    /// Returns true if this action is destructive and should require confirmation.
    pub fn is_destructive(&self) -> bool {
        matches!(self, Action::Terminate)
    }

    /// Returns true if this action modifies resource state.
    pub fn is_mutating(&self) -> bool {
        matches!(
            self,
            Action::Start | Action::Stop | Action::Restart | Action::Terminate | Action::Modify
        )
    }

    /// Returns true if this action is read-only (viewing information).
    pub fn is_readonly(&self) -> bool {
        matches!(self, Action::ViewDetails | Action::ViewLogs)
    }

    /// Returns all available actions.
    pub fn all() -> Vec<Action> {
        vec![
            Action::Start,
            Action::Stop,
            Action::Restart,
            Action::Terminate,
            Action::ViewDetails,
            Action::ViewLogs,
            Action::Modify,
        ]
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_as_str() {
        assert_eq!(Action::Start.as_str(), "Start");
        assert_eq!(Action::Stop.as_str(), "Stop");
        assert_eq!(Action::Terminate.as_str(), "Terminate");
    }

    #[test]
    fn test_action_display() {
        assert_eq!(format!("{}", Action::Start), "Start");
        assert_eq!(format!("{}", Action::ViewDetails), "View Details");
    }

    #[test]
    fn test_action_is_destructive() {
        assert!(Action::Terminate.is_destructive());
        assert!(!Action::Start.is_destructive());
        assert!(!Action::Stop.is_destructive());
        assert!(!Action::Restart.is_destructive());
        assert!(!Action::ViewDetails.is_destructive());
    }

    #[test]
    fn test_action_is_mutating() {
        assert!(Action::Start.is_mutating());
        assert!(Action::Stop.is_mutating());
        assert!(Action::Restart.is_mutating());
        assert!(Action::Terminate.is_mutating());
        assert!(Action::Modify.is_mutating());
        assert!(!Action::ViewDetails.is_mutating());
        assert!(!Action::ViewLogs.is_mutating());
    }

    #[test]
    fn test_action_is_readonly() {
        assert!(Action::ViewDetails.is_readonly());
        assert!(Action::ViewLogs.is_readonly());
        assert!(!Action::Start.is_readonly());
        assert!(!Action::Stop.is_readonly());
        assert!(!Action::Terminate.is_readonly());
    }

    #[test]
    fn test_action_all() {
        let all = Action::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&Action::Start));
        assert!(all.contains(&Action::Terminate));
    }

    #[test]
    fn test_action_equality() {
        assert_eq!(Action::Start, Action::Start);
        assert_ne!(Action::Start, Action::Stop);
    }

    #[test]
    fn test_action_clone() {
        let action = Action::Start;
        let cloned = action;
        assert_eq!(action, cloned);
    }
}