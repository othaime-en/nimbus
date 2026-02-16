use aws_config::SdkConfig;
use aws_sdk_costexplorer::Client as CostExplorerClient;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_elasticloadbalancingv2::Client as ElbClient;
use aws_sdk_rds::Client as RdsClient;
use aws_sdk_route53::Client as Route53Client;
use aws_sdk_s3::Client as S3Client;

pub struct AwsClient {
    pub ec2: Ec2Client,
    pub rds: RdsClient,
    pub s3: S3Client,
    pub elb: ElbClient,
    pub route53: Route53Client,
    pub cost_explorer: CostExplorerClient,
}

impl AwsClient {
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            ec2: Ec2Client::new(config),
            rds: RdsClient::new(config),
            s3: S3Client::new(config),
            elb: ElbClient::new(config),
            route53: Route53Client::new(config),
            cost_explorer: CostExplorerClient::new(config),
        }
    }
}