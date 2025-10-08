#![deny(unused_crate_dependencies)]
mod common;
mod config;
mod error;

use crate::common::*;
use crate::config::{FilterConfig, ProcessorConfig};
use crate::error::{Error, Result};

use axum::{
    Router,
    extract::{Multipart, State},
    http::HeaderMap,
    routing::{get, post},
};
use chrono::{DateTime, SecondsFormat, Utc};
use object_store::{self, ObjectStore, PutPayload, aws::AmazonS3, path::Path};
use reqwest::multipart::{Form, Part};
use std::{collections::HashMap, time::Instant};
use tokio::net::TcpListener;
use tracing::info;

// Constants
const MAX_FILE_SIZE: usize = 50 * 1024 * 1024; // 50MB
const ALLOWED_AUDIO_EXTENSIONS: &[&str] = &[".m4a", ".wav"];

// Helper function to extract, validate, and convert the collected HashMap into UploadData.
fn validate_and_build(mut fields: HashMap<String, UploadedFile>) -> Result<UploadData> {
    // use HashMap::remove() to extract mandatory fields.
    // if the key is not present, it returns None, which ok_or_else converts to an error.
    let json_file = fields
        .remove("json")
        .ok_or_else(|| Error::MissingField(String::from("json")))?;

    let audio_file = fields
        .remove("audio")
        .ok_or_else(|| Error::MissingField(String::from("audio")))?;

    Ok(UploadData {
        json: json_file,
        audio: audio_file,
    })
}

async fn multipart_to_struct(mut m: Multipart) -> Result<UploadData> {
    // 1. Use a HashMap to collect all files dynamically
    let mut files_map: HashMap<String, UploadedFile> = HashMap::new();

    // 2. Process all fields in a unified loop
    while let Some(field) = m
        .next_field()
        .await
        .map_err(|e| Error::Multipart(e.to_string()))?
    {
        let name = field
            .name()
            .ok_or_else(|| Error::Multipart("Field missing name".to_string()))?
            .to_string();

        // Filename is mandatory check
        let file_name = field
            .file_name()
            .ok_or_else(|| Error::MissingField(format!("Missing filename for field: {}", name)))?
            .to_string();

        // Get the file data
        let file_data = field
            .bytes()
            .await
            .map_err(|e| Error::Multipart(e.to_string()))?;

        // Validate file size
        if file_data.len() > MAX_FILE_SIZE {
            return Err(Error::FileTooLarge {
                size: file_data.len(),
                max_size: MAX_FILE_SIZE,
            });
        }

        // Validate file extensions
        match name.as_str() {
            "json" => {
                if !file_name.ends_with(".json") {
                    return Err(Error::InvalidFileType(
                        "JSON file must have .json extension".to_string(),
                    ));
                }
            }
            "audio" => {
                if !ALLOWED_AUDIO_EXTENSIONS
                    .iter()
                    .any(|ext| file_name.ends_with(ext))
                {
                    return Err(Error::InvalidFileType(format!(
                        "Audio file must have one of these extensions: {}",
                        ALLOWED_AUDIO_EXTENSIONS.join(", ")
                    )));
                }
            }
            _ => {} // Allow other field types
        }

        // 3. Insert the constructed struct into the HashMap
        files_map.insert(
            name,
            UploadedFile {
                name: file_name,
                data: file_data,
            },
        );
    }

    // 4. Pass the collected map to a separate validation function
    validate_and_build(files_map)
}

fn json_from_bytes(b: &axum::body::Bytes) -> Result<AudioMetadata> {
    serde_json::from_slice(b).map_err(Error::JsonParsing)
}

fn dt_from_epoch(e: i64) -> Result<DateTime<Utc>> {
    DateTime::from_timestamp_secs(e)
        .ok_or_else(|| Error::DateTime("Invalid epoch provided to create dt".to_string()))
}

fn path_from_json(j: &AudioMetadata) -> Result<String> {
    let dt: DateTime<Utc> = dt_from_epoch(j.start_time)?;

    let date_path = format!("{}", dt.format("%Y/%m/%d"));

    let system_path = j
        .short_name
        .split('-')
        .next_back()
        .ok_or_else(|| Error::Multipart("short name must be populated".to_string()))?;

    Ok(format!("{}/{}", system_path, date_path))
}

async fn upload_file_to_s3(s3: &AmazonS3, path: &str, file: &UploadedFile) -> Result<()> {
    let object_path = format!("{}/{}", path, file.name);
    let location = Path::parse(object_path)?;

    // Retry logic with exponential backoff
    let max_retries = 3;
    for attempt in 0..max_retries {
        let payload = PutPayload::from_bytes(file.data.clone());

        match s3.put(&location, payload).await {
            Ok(_) => return Ok(()),
            Err(e) if attempt == max_retries - 1 => return Err(Error::S3Upload(e)),
            Err(_) => {
                let delay = std::time::Duration::from_millis(100 * 2_u64.pow(attempt));
                tokio::time::sleep(delay).await;
            }
        }
    }

    Ok(())
}

async fn upload_files(s3: &AmazonS3, path: &str, files: &UploadData) -> Result<()> {
    let json_fut = upload_file_to_s3(s3, path, &files.json);
    let audio_fut = upload_file_to_s3(s3, path, &files.audio);

    tokio::try_join!(json_fut, audio_fut)?;
    Ok(())
}

async fn transcribe_audio(f: &UploadedFile, c: &ProcessorConfig) -> Result<String> {
    let file = reqwest::multipart::Part::bytes(f.data.to_vec())
        .file_name(f.name.clone())
        .mime_str("application/octet-stream")
        .map_err(|e| Error::Multipart(e.to_string()))?;

    let form = reqwest::multipart::Form::new()
        .part("file", file)
        .text("model", c.env.model_name.clone())
        .text("language", "en")
        .text("response_format", "text");

    let res = c
        .http_client
        .post(&c.env.transcription_endpoint)
        .multipart(form)
        .send()
        .await?
        .text()
        .await
        .map_err(|e| Error::Multipart(e.to_string()))?;

    Ok(res)
}

fn format_timestamp_from_datetime(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn format_embed_ids(m: &[SrcList]) -> EmbedField {
    EmbedField {
        name: "Radio IDs:".to_string(),
        value: m
            .iter()
            .map(|x| x.src.to_string())
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn format_embed_transcription(t: String) -> EmbedField {
    EmbedField {
        name: "Transcription:".to_string(),
        value: t,
    }
}

fn format_embed_timestamp(t: &String) -> EmbedField {
    EmbedField {
        name: "Start timestamp:".to_string(),
        value: t.to_string(),
    }
}

async fn create_webhook(m: &AudioMetadata, tr: String) -> Result<String> {
    let timestamp = format_timestamp_from_datetime(dt_from_epoch(m.start_time)?);
    let fields = vec![
        format_embed_timestamp(&timestamp),
        format_embed_ids(&m.src_list),
        format_embed_transcription(tr),
    ];

    let embeds = vec![WebhookEmbed {
        color: "12110930".to_string(),
        timestamp,
        title: format!("{} - {}", m.talkgroup_group, m.talkgroup_description),
        fields,
    }];

    let webhook = Webhook {
        username: "Trunk Recorder".to_owned(),
        avatar_url: "https://raw.githubusercontent.com/TrunkRecorder/trunkrecorder.github.io/refs/heads/main/static/img/radio.png".to_owned(),
        embeds,
    };

    Ok(serde_json::to_string(&webhook)?)
}

async fn send_webhook(
    client: &reqwest::Client,
    url: &str,
    m: &AudioMetadata,
    t: String,
    f: UploadedFile,
) -> Result<()> {
    let webhook = create_webhook(m, t).await?;

    let file = Part::bytes(f.data.to_vec()).file_name(f.name.clone());
    let form = Form::new()
        .part("file1", file)
        .text("payload_json", webhook);

    client
        .post(url)
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

async fn filter_on_metadata(m: &AudioMetadata, c: &FilterConfig) -> bool {
    let tgid_as_string = &m.talkgroup.to_string();
    let deny_tgid = format!("!{}", tgid_as_string);

    if !c.tgid().is_empty() {
        // if tgid filter contains negated tgid, !do_transcription early
        if c.tgid().contains(&deny_tgid) {
            info!(tgid = %tgid_as_string, "Matched denied talkgroup, no transcribe");
            return false;
        }
        // if tgid filter contains tgid, do_transcription
        else if c.tgid().contains(tgid_as_string) {
            info!(tgid = %tgid_as_string, "Matched talkgroup, transcribing");
            return true;
        }
    };

    if !c.group().is_empty()
        // if group in include list, do_transcription
        && c.group().contains(&m.talkgroup_group)
    {
        info!(group = %m.talkgroup_group, "Matched group, transcribing");
        return true;
    };

    // return false if not negated by previous tgid include, or group include
    info!(%m.talkgroup_group, %tgid_as_string, "Group and tgid unmatched");
    false
}

// ---------------------------------------------------------------------
// --- HANDLER AND MAIN ---
// ---------------------------------------------------------------------

async fn upload(State(config): State<ProcessorConfig>, m: Multipart) -> Result<String> {
    let upload_start = Instant::now();
    info!("Starting upload processing");

    let files: UploadData = multipart_to_struct(m).await?;
    info!(
        json_file = %files.json.name,
        json_bytes = files.json.data.len(),
        audio_file = %files.audio.name,
        audio_bytes = files.audio.data.len(),
        "Files received:"
    );

    let meta: AudioMetadata = json_from_bytes(&files.json.data)?;
    let path: String = path_from_json(&meta)?;

    info!(talkgroup = meta.talkgroup, path = %path, "Processed audio metadata");

    let do_transcription = if config.filter.enabled() {
        filter_on_metadata(&meta, &config.filter).await
    } else {
        true
    };

    if !do_transcription {
        upload_files(&config.s3_client, &path, &files).await?;
    } else if do_transcription {
        let upload_fut = upload_files(&config.s3_client, &path, &files);
        let transcription_fut = transcribe_audio(&files.audio, &config);

        let (_, transcription) = tokio::try_join!(upload_fut, transcription_fut)?;

        send_webhook(
            &config.http_client,
            &config.env.discord_webhook,
            &meta,
            transcription,
            files.audio,
        )
        .await?;
    }

    let duration = Instant::now().duration_since(upload_start);
    info!(
        duration_ms = duration.as_millis(),
        "Upload processing completed successfully"
    );

    Ok("Upload successful".to_string())
}

async fn healthz(headers: HeaderMap) -> Result<String> {
    let timestamp = format_timestamp_from_datetime(Utc::now().to_utc());

    info!(
        timestamp = %timestamp,
        user_agent = %headers.get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown"),
        "Health check requested"
    );

    Ok(format!(
        "{{\"status\":\"healthy\",\"timestamp\":\"{}\",\"service\":\"trunk-processor\"}}",
        timestamp
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "trunk_processor=info,tower_http=debug".into()),
        )
        .init();

    info!("Initializing trunk-processor");

    let config = config::initialize()?;
    if config.filter.enabled() {
        info!(
            group = config.filter.group().join(", "),
            tgid = config.filter.tgid().join(", "),
            "Filter values provided"
        );
    } else {
        info!("Filtering disabled");
    }

    let app = Router::new()
        .route("/upload", post(upload).with_state(config))
        .route("/healthz", get(healthz));

    let bind_addr = "0.0.0.0:3000";
    info!(addr = %bind_addr, "Starting HTTP server");

    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(Error::ServerInit)?;
    axum::serve(listener, app)
        .await
        .map_err(Error::ServerInit)?;

    Ok(())
}
