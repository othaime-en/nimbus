use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum CostPeriod {
    Today,
    ThisWeek,
    ThisMonth,
    Last30Days,
}

#[derive(Debug, Clone)]
pub struct CostBreakdown {
    pub total: f64,
    pub by_service: HashMap<String, f64>,
    pub by_region: HashMap<String, f64>,
    pub trend_percentage: f64,
}

impl CostBreakdown {
    pub fn new() -> Self {
        Self {
            total: 0.0,
            by_service: HashMap::new(),
            by_region: HashMap::new(),
            trend_percentage: 0.0,
        }
    }
}

impl Default for CostBreakdown {
    fn default() -> Self {
        Self::new()
    }
}