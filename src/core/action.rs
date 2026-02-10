#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Start,
    Stop,
    Restart,
    Terminate,
    ViewDetails,
    ViewLogs,
    Modify,
}

impl Action {
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

    pub fn is_destructive(&self) -> bool {
        matches!(self, Action::Terminate)
    }
}