use crate::core::{CloudResource, CostBreakdown, CostPeriod};

/// Approximates GCP cost from static per-resource pricing estimates.
///
/// Unlike AWS Cost Explorer, GCP has no simple "get my actual spend" REST
/// API -- real historical spend requires a BigQuery billing export the
/// user sets up separately in their own project. Rather than build that
/// integration now, this aggregates each listed resource's own
/// `cost_per_month()` estimate (the same static-table approach already
/// used for EC2/RDS), the same way the dashboard would show an estimate
/// for any resource type. This was chosen over a BigQuery-based estimator
/// during the Phase 3.1 handoff discussion; revisit if real spend data
/// becomes a priority.
pub struct GcpCostEstimator;

impl GcpCostEstimator {
    /// Scales the summed monthly estimate down to the requested period.
    /// This is a proportional approximation, not a real historical query --
    /// there's no notion of "yesterday's actual cost" here the way there is
    /// with AWS Cost Explorer.
    pub fn total_cost(resources: &[Box<dyn CloudResource>], period: CostPeriod) -> f64 {
        let monthly: f64 = resources.iter().filter_map(|r| r.cost_per_month()).sum();

        let fraction = match period {
            CostPeriod::Today => 1.0 / 30.0,
            CostPeriod::ThisWeek => 7.0 / 30.0,
            CostPeriod::ThisMonth | CostPeriod::Last30Days => 1.0,
        };

        monthly * fraction
    }

    /// Builds a breakdown by resource type and region from the same
    /// per-resource estimates. `trend_percentage` is left at 0.0 -- there's
    /// no historical data to compare against without a real billing source.
    pub fn cost_breakdown(resources: &[Box<dyn CloudResource>]) -> CostBreakdown {
        let mut breakdown = CostBreakdown::new();

        for resource in resources {
            if let Some(cost) = resource.cost_per_month() {
                breakdown.add_service_cost(resource.resource_type().to_string(), cost);
                breakdown.add_region_cost(resource.region().to_string(), cost);
                breakdown.total += cost;
            }
        }

        breakdown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Action, Provider, ResourceState, ResourceType};
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;

    struct FakeResource {
        cost: Option<f64>,
        resource_type: ResourceType,
        region: String,
        labels: HashMap<String, String>,
    }

    impl FakeResource {
        fn new(cost: Option<f64>, resource_type: ResourceType, region: &str) -> Self {
            Self {
                cost,
                resource_type,
                region: region.to_string(),
                labels: HashMap::new(),
            }
        }
    }

    impl CloudResource for FakeResource {
        fn id(&self) -> &str {
            "fake-id"
        }
        fn name(&self) -> &str {
            "fake"
        }
        fn resource_type(&self) -> ResourceType {
            self.resource_type
        }
        fn provider(&self) -> Provider {
            Provider::GCP
        }
        fn region(&self) -> &str {
            &self.region
        }
        fn state(&self) -> ResourceState {
            ResourceState::Running
        }
        fn cost_per_month(&self) -> Option<f64> {
            self.cost
        }
        fn tags(&self) -> &HashMap<String, String> {
            &self.labels
        }
        fn created_at(&self) -> Option<DateTime<Utc>> {
            None
        }
        fn supported_actions(&self) -> Vec<Action> {
            vec![]
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_total_cost_this_month_sums_all() {
        let resources: Vec<Box<dyn CloudResource>> = vec![
            Box::new(FakeResource::new(Some(100.0), ResourceType::Compute, "us-central1")),
            Box::new(FakeResource::new(Some(50.0), ResourceType::Database, "us-central1")),
        ];

        assert_eq!(
            GcpCostEstimator::total_cost(&resources, CostPeriod::ThisMonth),
            150.0
        );
    }

    #[test]
    fn test_total_cost_today_is_scaled_down() {
        let resources: Vec<Box<dyn CloudResource>> =
            vec![Box::new(FakeResource::new(Some(300.0), ResourceType::Compute, "us-central1"))];

        let today = GcpCostEstimator::total_cost(&resources, CostPeriod::Today);
        assert!((today - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_cost_breakdown_groups_by_service_and_region() {
        let resources: Vec<Box<dyn CloudResource>> = vec![
            Box::new(FakeResource::new(Some(100.0), ResourceType::Compute, "us-central1")),
            Box::new(FakeResource::new(Some(20.0), ResourceType::Compute, "us-east1")),
        ];

        let breakdown = GcpCostEstimator::cost_breakdown(&resources);
        assert_eq!(breakdown.total, 120.0);
        assert_eq!(*breakdown.by_service.get("Compute").unwrap(), 120.0);
        assert_eq!(*breakdown.by_region.get("us-central1").unwrap(), 100.0);
        assert_eq!(*breakdown.by_region.get("us-east1").unwrap(), 20.0);
    }
}
