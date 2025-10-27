use diesel::{
    PgConnection,
    r2d2::{self, ConnectionManager, Pool},
};
use object_store::aws::{AmazonS3, AmazonS3Builder};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ProcessorConfig {
    pub s3_client: AmazonS3,
    pub http_client: Client,
    pub env: EnvConfig,
    pub filter: FilterConfig,
    pub db_pool: Pool<ConnectionManager<PgConnection>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EnvConfig {
    pub transcription_endpoint: String,
    pub bucket_name: String,
    pub discord_webhook: String,
    pub model_name: String,
    pub database_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FilterConfig {
    tg_group: Option<Vec<String>>,
    tg_id: Option<Vec<String>>,
}

impl FilterConfig {
    pub fn enabled(&self) -> bool {
        self.tg_group.is_some() || self.tg_id.is_some()
    }
    pub fn group(&self) -> Vec<String> {
        if let Some(group) = &self.tg_group {
            group.to_vec()
        } else {
            Vec::new()
        }
    }
    pub fn tgid(&self) -> Vec<String> {
        if let Some(tgid) = &self.tg_id {
            tgid.to_vec()
        } else {
            Vec::new()
        }
    }
}

use crate::error::{Error, Result};

fn init_env() -> Result<EnvConfig> {
    envy::from_env::<EnvConfig>()
        .map_err(|e| Error::Configuration(format!("Environment configuration error: {}", e)))
}

fn init_filter() -> Result<FilterConfig> {
    envy::prefixed("FILTER_")
        .from_env::<FilterConfig>()
        .map_err(|e| Error::Configuration(format!("Environment configuration error: {}", e)))
}

fn init_s3_client(b: &str) -> Result<AmazonS3> {
    AmazonS3Builder::from_env()
        .with_bucket_name(b)
        .build()
        .map_err(|e| Error::Configuration(format!("S3 client configuration error: {}", e)))
}

fn init_http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(20))
        .build()
        .expect("Failed to create HTTP client")
}

fn init_db_pool(url: &str) -> Result<Pool<ConnectionManager<PgConnection>>> {
    let manager = ConnectionManager::new(url.to_string());
    r2d2::Pool::builder()
        .max_size(10)
        .build(manager)
        .map_err(|e| Error::Database(e.to_string()))
}

pub fn initialize() -> Result<ProcessorConfig> {
    let env = init_env()?;
    let s3_client = init_s3_client(&env.bucket_name)?;
    let db_pool = init_db_pool(&env.database_url)?;

    Ok(ProcessorConfig {
        env,
        s3_client,
        db_pool,
        http_client: init_http_client(),
        filter: init_filter()?,
    })
}
