use crate::error::{Error, Result};

use axum::body::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct UploadedFile {
    pub name: String,
    pub data: Bytes,
}

pub struct UploadData {
    pub json: UploadedFile,
    pub audio: UploadedFile,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub freq: i64,
    pub freq_error: i64,
    pub signal: i64,
    pub noise: i64,
    pub source_num: i64,
    pub recorder_num: i64,
    pub tdma_slot: i64,
    pub phase2_tdma: i64,
    pub start_time: i64,
    pub stop_time: i64,
    pub emergency: i64,
    pub priority: i64,
    pub mode: i64,
    pub duplex: i64,
    pub encrypted: i64,
    pub call_length: i64,
    pub talkgroup: i64,
    pub talkgroup_tag: String,
    pub talkgroup_description: String,
    pub talkgroup_group_tag: String,
    pub talkgroup_group: String,
    pub audio_type: String,
    pub short_name: String,
    #[serde(alias = "freqList")]
    pub freq_list: Vec<FreqList>,
    #[serde(alias = "srcList")]
    pub src_list: Vec<SrcList>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreqList {
    pub freq: i64,
    pub time: i64,
    pub pos: f64,
    pub len: f64,
    pub error_count: i64,
    pub spike_count: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SrcList {
    pub src: i64,
    pub time: i64,
    pub pos: f64,
    pub emergency: i64,
    pub signal_system: String,
    pub tag: String,
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
    RadioIds(Vec<i64>),
    Transcription(String),
    // Easy to extend with new field types
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
    pub fn serialize_json(&self) -> Result<AudioMetadata> {
        serde_json::from_slice(&self.json.data).map_err(Error::JsonParsing)
    }
}
