use crate::core::{CostBreakdown, CostPeriod};
use crate::error::{NimbusError, Result};
use aws_sdk_costexplorer::types::{DateInterval, Granularity, GroupDefinition};
use aws_sdk_costexplorer::Client as CostExplorerClient;
use chrono::{Duration, Utc};

pub struct AwsCostExplorer {
    client: CostExplorerClient,
}

impl AwsCostExplorer {
    pub fn new(client: CostExplorerClient) -> Self {
        Self { client }
    }

    pub async fn get_total_cost(&self, period: CostPeriod) -> Result<f64> {
        let (start, end) = Self::get_date_range(period);

        let response = self
            .client
            .get_cost_and_usage()
            .time_period(
                DateInterval::builder()
                    .start(start)
                    .end(end)
                    .build()
                    .map_err(|e| NimbusError::provider("AWS", format!("Invalid date range: {}", e)))?,
            )
            .granularity(Granularity::Monthly)
            .metrics("UnblendedCost")
            .send()
            .await
            .map_err(|e| {
                NimbusError::provider("AWS", format!("Failed to fetch cost data: {}", e))
            })?;

        let total: f64 = response
            .results_by_time()
            .iter()
            .filter_map(|result| {
                result.total()
                    .and_then(|total_map| total_map.get("UnblendedCost"))
                    .and_then(|metric| metric.amount())
                    .and_then(|amount| amount.parse::<f64>().ok())
            })
            .sum();

        Ok(total)
    }

    pub async fn get_cost_breakdown(&self) -> Result<CostBreakdown> {
        let period = CostPeriod::ThisMonth;
        let (start, end) = Self::get_date_range(period);

        let by_service = self
            .client
            .get_cost_and_usage()
            .time_period(
                DateInterval::builder()
                    .start(&start)
                    .end(&end)
                    .build()
                    .map_err(|e| NimbusError::provider("AWS", format!("Invalid date range: {}", e)))?,
            )
            .granularity(Granularity::Monthly)
            .metrics("UnblendedCost")
            .group_by(
                GroupDefinition::builder()
                    .r#type(aws_sdk_costexplorer::types::GroupDefinitionType::Dimension)
                    .key("SERVICE")
                    .build(),
            )
            .send()
            .await
            .map_err(|e| {
                NimbusError::provider("AWS", format!("Failed to fetch service breakdown: {}", e))
            })?;

        let mut breakdown = CostBreakdown::new();

        for result in by_service.results_by_time() {
            for group in result.groups() {
                let keys = group.keys();
                if let Some(service_name) = keys.first() {
                    if let Some(metrics) = group.metrics() {
                        if let Some(cost_metric) = metrics.get("UnblendedCost") {
                            if let Some(amount_str) = cost_metric.amount() {
                                if let Ok(amount) = amount_str.parse::<f64>() {
                                    breakdown.add_service_cost(service_name.to_string(), amount);
                                    breakdown.total += amount;
                                }
                            }
                        }
                    }
                }
            }
        }

        let by_region = self
            .client
            .get_cost_and_usage()
            .time_period(
                DateInterval::builder()
                    .start(start)
                    .end(end)
                    .build()
                    .map_err(|e| NimbusError::provider("AWS", format!("Invalid date range: {}", e)))?,
            )
            .granularity(Granularity::Monthly)
            .metrics("UnblendedCost")
            .group_by(
                GroupDefinition::builder()
                    .r#type(aws_sdk_costexplorer::types::GroupDefinitionType::Dimension)
                    .key("REGION")
                    .build(),
            )
            .send()
            .await
            .map_err(|e| {
                NimbusError::provider("AWS", format!("Failed to fetch region breakdown: {}", e))
            })?;

        for result in by_region.results_by_time() {
            for group in result.groups() {
                let keys = group.keys();
                if let Some(region_name) = keys.first() {
                    if let Some(metrics) = group.metrics() {
                        if let Some(cost_metric) = metrics.get("UnblendedCost") {
                            if let Some(amount_str) = cost_metric.amount() {
                                if let Ok(amount) = amount_str.parse::<f64>() {
                                    breakdown.add_region_cost(region_name.to_string(), amount);
                                }
                            }
                        }
                    }
                }
            }
        }

        let prev_period_start = Utc::now() - Duration::days(60);
        let prev_period_end = Utc::now() - Duration::days(30);

        let prev_total = self
            .get_total_cost_for_range(
                prev_period_start.format("%Y-%m-%d").to_string(),
                prev_period_end.format("%Y-%m-%d").to_string(),
            )
            .await
            .unwrap_or(0.0);

        if prev_total > 0.0 {
            breakdown.trend_percentage = ((breakdown.total - prev_total) / prev_total) * 100.0;
        }

        Ok(breakdown)
    }

    async fn get_total_cost_for_range(&self, start: String, end: String) -> Result<f64> {
        let response = self
            .client
            .get_cost_and_usage()
            .time_period(
                DateInterval::builder()
                    .start(start)
                    .end(end)
                    .build()
                    .map_err(|e| NimbusError::provider("AWS", format!("Invalid date range: {}", e)))?,
            )
            .granularity(Granularity::Monthly)
            .metrics("UnblendedCost")
            .send()
            .await
            .map_err(|e| {
                NimbusError::provider("AWS", format!("Failed to fetch cost data: {}", e))
            })?;

        let total: f64 = response
            .results_by_time()
            .iter()
            .filter_map(|result| {
                result.total()
                    .and_then(|total_map| total_map.get("UnblendedCost"))
                    .and_then(|metric| metric.amount())
                    .and_then(|amount| amount.parse::<f64>().ok())
            })
            .sum();

        Ok(total)
    }

    fn get_date_range(period: CostPeriod) -> (String, String) {
        let end = Utc::now();
        let start = match period {
            CostPeriod::Today => end - Duration::days(1),
            CostPeriod::ThisWeek => end - Duration::days(7),
            CostPeriod::ThisMonth => end - Duration::days(30),
            CostPeriod::Last30Days => end - Duration::days(30),
        };

        (
            start.format("%Y-%m-%d").to_string(),
            end.format("%Y-%m-%d").to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_date_range() {
        let (start, end) = AwsCostExplorer::get_date_range(CostPeriod::Today);
        assert!(start < end);
        assert_eq!(start.len(), 10);
        assert_eq!(end.len(), 10);
    }
}