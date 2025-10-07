use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct UploadedFile {
    pub name: String,
    pub data: axum::body::Bytes,
}

pub struct UploadData {
    pub json: UploadedFile,
    pub audio: UploadedFile,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioMetadata {
    pub freq: i64,
    #[serde(rename = "freq_error")]
    pub freq_error: i64,
    pub signal: i64,
    pub noise: i64,
    #[serde(rename = "source_num")]
    pub source_num: i64,
    #[serde(rename = "recorder_num")]
    pub recorder_num: i64,
    #[serde(rename = "tdma_slot")]
    pub tdma_slot: i64,
    #[serde(rename = "phase2_tdma")]
    pub phase2_tdma: i64,
    #[serde(rename = "start_time")]
    pub start_time: i64,
    #[serde(rename = "stop_time")]
    pub stop_time: i64,
    pub emergency: i64,
    pub priority: i64,
    pub mode: i64,
    pub duplex: i64,
    pub encrypted: i64,
    #[serde(rename = "call_length")]
    pub call_length: i64,
    pub talkgroup: i64,
    #[serde(rename = "talkgroup_tag")]
    pub talkgroup_tag: String,
    #[serde(rename = "talkgroup_description")]
    pub talkgroup_description: String,
    #[serde(rename = "talkgroup_group_tag")]
    pub talkgroup_group_tag: String,
    #[serde(rename = "talkgroup_group")]
    pub talkgroup_group: String,
    #[serde(rename = "audio_type")]
    pub audio_type: String,
    #[serde(rename = "short_name")]
    pub short_name: String,
    pub freq_list: Vec<FreqList>,
    pub src_list: Vec<SrcList>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreqList {
    pub freq: i64,
    pub time: i64,
    pub pos: f64,
    pub len: f64,
    #[serde(rename = "error_count")]
    pub error_count: i64,
    #[serde(rename = "spike_count")]
    pub spike_count: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SrcList {
    pub src: i64,
    pub time: i64,
    pub pos: f64,
    pub emergency: i64,
    #[serde(rename = "signal_system")]
    pub signal_system: String,
    pub tag: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Webhook {
    pub username: String,
    #[serde(rename = "avatar_url")]
    pub avatar_url: String,
    pub embeds: Vec<WebhookEmbed>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookEmbed {
    pub color: String,
    pub timestamp: String,
    pub title: String,
    pub fields: Vec<EmbedField>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedField {
    pub name: String,
    pub value: String,
}
