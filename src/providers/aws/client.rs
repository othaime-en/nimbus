use aws_sdk_costexplorer::Client as CostExplorerClient;
use aws_sdk_ec2::Client as Ec2Client;
use aws_config::SdkConfig;

pub struct AwsClient {
    pub ec2: Ec2Client,
    pub cost_explorer: CostExplorerClient,
}

impl AwsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            ec2: Ec2Client::new(config),
            cost_explorer: CostExplorerClient::new(config),
        }
    }
}