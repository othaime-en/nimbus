use std::collections::HashMap;

/// Time period for cost queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostPeriod {
    /// Current day's costs
    Today,
    /// Current week's costs (Sunday to Saturday)
    ThisWeek,
    /// Current month's costs
    ThisMonth,
    /// Last 30 days of costs
    Last30Days,
}

impl CostPeriod {
    pub fn as_str(&self) -> &'static str {
        match self {
            CostPeriod::Today => "Today",
            CostPeriod::ThisWeek => "This Week",
            CostPeriod::ThisMonth => "This Month",
            CostPeriod::Last30Days => "Last 30 Days",
        }
    }
}

/// Breakdown of cloud costs by service and region.
/// 
/// Provides detailed cost information including totals and categorizations.
/// Used by providers to return comprehensive cost data.
#[derive(Debug, Clone)]
pub struct CostBreakdown {
    /// Total cost in USD
    pub total: f64,
    /// Cost broken down by service/resource type
    pub by_service: HashMap<String, f64>,
    /// Cost broken down by region/zone
    pub by_region: HashMap<String, f64>,
    /// Percentage change from previous period (positive = increase)
    pub trend_percentage: f64,
}

impl CostBreakdown {
    /// Creates a new empty cost breakdown.
    pub fn new() -> Self {
        Self {
            total: 0.0,
            by_service: HashMap::new(),
            by_region: HashMap::new(),
            trend_percentage: 0.0,
        }
    }

    /// Creates a cost breakdown with a specific total.
    pub fn with_total(total: f64) -> Self {
        Self {
            total,
            by_service: HashMap::new(),
            by_region: HashMap::new(),
            trend_percentage: 0.0,
        }
    }

    /// Adds a service cost to the breakdown.
    pub fn add_service_cost(&mut self, service: String, cost: f64) {
        *self.by_service.entry(service).or_insert(0.0) += cost;
    }

    /// Adds a region cost to the breakdown.
    pub fn add_region_cost(&mut self, region: String, cost: f64) {
        *self.by_region.entry(region).or_insert(0.0) += cost;
    }

    /// Returns the most expensive service.
    pub fn top_service(&self) -> Option<(&String, &f64)> {
        self.by_service
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
    }

    /// Returns the most expensive region.
    pub fn top_region(&self) -> Option<(&String, &f64)> {
        self.by_region
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
    }

    /// Returns true if costs are trending up.
    pub fn is_trending_up(&self) -> bool {
        self.trend_percentage > 0.0
    }
}

impl Default for CostBreakdown {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_period_as_str() {
        assert_eq!(CostPeriod::Today.as_str(), "Today");
        assert_eq!(CostPeriod::ThisWeek.as_str(), "This Week");
        assert_eq!(CostPeriod::ThisMonth.as_str(), "This Month");
        assert_eq!(CostPeriod::Last30Days.as_str(), "Last 30 Days");
    }

    #[test]
    fn test_cost_breakdown_new() {
        let breakdown = CostBreakdown::new();
        assert_eq!(breakdown.total, 0.0);
        assert!(breakdown.by_service.is_empty());
        assert!(breakdown.by_region.is_empty());
        assert_eq!(breakdown.trend_percentage, 0.0);
    }

    #[test]
    fn test_cost_breakdown_with_total() {
        let breakdown = CostBreakdown::with_total(100.0);
        assert_eq!(breakdown.total, 100.0);
    }

    #[test]
    fn test_cost_breakdown_add_service_cost() {
        let mut breakdown = CostBreakdown::new();
        breakdown.add_service_cost("EC2".to_string(), 50.0);
        breakdown.add_service_cost("RDS".to_string(), 30.0);
        breakdown.add_service_cost("EC2".to_string(), 20.0);

        assert_eq!(breakdown.by_service.len(), 2);
        assert_eq!(*breakdown.by_service.get("EC2").unwrap(), 70.0);
        assert_eq!(*breakdown.by_service.get("RDS").unwrap(), 30.0);
    }

    #[test]
    fn test_cost_breakdown_add_region_cost() {
        let mut breakdown = CostBreakdown::new();
        breakdown.add_region_cost("us-east-1".to_string(), 100.0);
        breakdown.add_region_cost("us-west-2".to_string(), 50.0);

        assert_eq!(breakdown.by_region.len(), 2);
        assert_eq!(*breakdown.by_region.get("us-east-1").unwrap(), 100.0);
    }

    #[test]
    fn test_cost_breakdown_top_service() {
        let mut breakdown = CostBreakdown::new();
        breakdown.add_service_cost("EC2".to_string(), 50.0);
        breakdown.add_service_cost("RDS".to_string(), 100.0);
        breakdown.add_service_cost("S3".to_string(), 30.0);

        let (service, cost) = breakdown.top_service().unwrap();
        assert_eq!(service, "RDS");
        assert_eq!(*cost, 100.0);
    }

    #[test]
    fn test_cost_breakdown_top_region() {
        let mut breakdown = CostBreakdown::new();
        breakdown.add_region_cost("us-east-1".to_string(), 200.0);
        breakdown.add_region_cost("us-west-2".to_string(), 100.0);

        let (region, cost) = breakdown.top_region().unwrap();
        assert_eq!(region, "us-east-1");
        assert_eq!(*cost, 200.0);
    }

    #[test]
    fn test_cost_breakdown_is_trending_up() {
        let mut breakdown = CostBreakdown::new();
        breakdown.trend_percentage = 5.0;
        assert!(breakdown.is_trending_up());

        breakdown.trend_percentage = -5.0;
        assert!(!breakdown.is_trending_up());

        breakdown.trend_percentage = 0.0;
        assert!(!breakdown.is_trending_up());
    }

    #[test]
    fn test_cost_breakdown_default() {
        let breakdown = CostBreakdown::default();
        assert_eq!(breakdown.total, 0.0);
        assert!(breakdown.by_service.is_empty());
    }

    #[test]
    fn test_cost_period_equality() {
        assert_eq!(CostPeriod::Today, CostPeriod::Today);
        assert_ne!(CostPeriod::Today, CostPeriod::ThisWeek);
    }
}