use crate::error::{Error, Result};
use crate::model::{AudioMetadata, AudioMetadataRaw};

use axum::body::Bytes;
use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use serde::{Deserialize, Deserializer, Serialize, de};
use tracing::info;

#[derive(Clone)]
pub struct UploadedFile {
    pub name: String,
    pub data: Bytes,
}

pub struct UploadData {
    pub json: UploadedFile,
    pub audio: UploadedFile,
}

#[derive(Debug, Serialize)]
pub struct Webhook {
    pub username: String,
    pub avatar_url: String,
    pub embeds: Vec<WebhookEmbed>,
}

#[derive(Debug, Serialize)]
pub struct WebhookEmbed {
    pub color: String,
    pub timestamp: String,
    pub title: String,
    pub fields: Vec<EmbedField>,
}

#[derive(Debug, Serialize)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
}

#[derive(Debug)]
pub enum EmbedFieldType {
    Timestamp(String),
    RadioIds(Vec<i32>),
    Transcription(String),
}

impl EmbedFieldType {
    pub fn into_embed_field(self) -> EmbedField {
        match self {
            EmbedFieldType::Timestamp(timestamp) => EmbedField {
                name: "Start timestamp:".to_string(),
                value: timestamp,
            },
            EmbedFieldType::RadioIds(ids) => EmbedField {
                name: "Radio IDs:".to_string(),
                value: ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            },
            EmbedFieldType::Transcription(text) => EmbedField {
                name: "Transcription:".to_string(),
                value: text,
            },
        }
    }
}

impl UploadData {
    pub fn deserialize_json(&self) -> Result<AudioMetadata> {
        let raw: AudioMetadataRaw =
            serde_json::from_slice(&self.json.data).map_err(Error::JsonParsing)?;
        let (src_list, sources) = raw.split_src_list();
        Ok(AudioMetadata {
            call: raw.call,
            talkgroup: raw.talkgroup,
            freq_list: raw.freq_list,
            src_list,
            sources,
        })
    }
}

pub fn format_timestamp_from_datetime(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn map_int_to_bool<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match u8::deserialize(deserializer)? {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(de::Error::custom(format!("Expected 0 or 1, got {}", other))),
    }
}

pub fn map_float_sec_to_timedelta<'de, D>(
    deserializer: D,
) -> std::result::Result<TimeDelta, D::Error>
where
    D: Deserializer<'de>,
{
    let float_val = f64::deserialize(deserializer)?;
    let nanoseconds_per_second: f64 = 1_000_000_000.0;
    let nanoseconds: i64 = (float_val * nanoseconds_per_second) as i64;

    Ok(TimeDelta::nanoseconds(nanoseconds))
}

pub fn run_migrations(
    migrations: EmbeddedMigrations,
    connection: &mut impl MigrationHarness<diesel::pg::Pg>,
) -> Result<()> {
    let applied = connection.run_pending_migrations(migrations)?;

    if applied.is_empty() {
        info!("No pending migrations to run");
    } else {
        info!(count = applied.len(), "Applied migrations");
        for migration in &applied {
            info!(migration = %migration, "Applied");
        }
    }

    Ok(())
}
