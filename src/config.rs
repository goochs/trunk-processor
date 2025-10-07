use serde::Deserialize;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ProcessorConfig {
    pub s3_client: object_store::aws::AmazonS3,
    pub http_client: reqwest::Client,
    pub env: EnvConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EnvConfig {
    pub transcription_endpoint: String,
    pub bucket_name: String,
    pub discord_webhook: String,
    pub model_name: String,
}

use crate::error::{Error, Result};

fn init_env() -> Result<EnvConfig> {
    envy::from_env::<EnvConfig>()
        .map_err(|e| Error::Configuration(format!("Environment configuration error: {}", e)))
}

fn init_s3_client(b: &str) -> Result<object_store::aws::AmazonS3> {
    object_store::aws::AmazonS3Builder::from_env()
        .with_bucket_name(b)
        .build()
        .map_err(|e| Error::Configuration(format!("S3 client configuration error: {}", e)))
}

fn init_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client")
}

pub fn initialize() -> Result<ProcessorConfig> {
    let env = init_env()?;
    let s3_client = init_s3_client(&env.bucket_name)?;
    let http_client = init_http_client();

    Ok(ProcessorConfig {
        s3_client,
        http_client,
        env,
    })
}
