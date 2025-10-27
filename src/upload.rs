use crate::common::*;
use crate::config::{FilterConfig, ProcessorConfig};
use crate::error::{Error, Result};
use crate::model::{self, AudioMetadata};
use crate::schema;

use axum::{
    extract::{Multipart, State},
    http::header::HeaderMap,
};
use chrono::{DateTime, Utc};
use diesel::{insert_into, prelude::*};
use object_store::{self, ObjectStore, PutPayload, aws::AmazonS3, path::Path};
use reqwest::{
    Client,
    multipart::{Form, Part},
};
use std::{collections::HashMap, time::Instant};
use tracing::info;

const MAX_FILE_SIZE: usize = 50 * 1024 * 1024; // 50MB

async fn multipart_to_struct(mut m: Multipart) -> Result<UploadData> {
    let mut files_map: HashMap<String, UploadedFile> = HashMap::new();

    while let Some(field) = m
        .next_field()
        .await
        .map_err(|e| Error::Multipart(e.to_string()))?
    {
        let name = field
            .name()
            .ok_or_else(|| Error::Multipart("Field missing name".to_string()))?
            .to_string();

        let file_name = field
            .file_name()
            .ok_or_else(|| Error::MissingField(format!("Missing filename for field: {}", name)))?
            .to_string();

        let file_data = field
            .bytes()
            .await
            .map_err(|e| Error::Multipart(e.to_string()))?;

        if file_data.len() > MAX_FILE_SIZE {
            return Err(Error::FileTooLarge {
                size: file_data.len(),
                max_size: MAX_FILE_SIZE,
            });
        }

        match name.as_str() {
            "json" => {
                if !file_name.ends_with(".json") {
                    return Err(Error::InvalidFileType(
                        "JSON file must have .json extension".to_string(),
                    ));
                }
            }
            "audio" => {
                if !file_name.ends_with(".m4a") {
                    return Err(Error::InvalidFileType(
                        "Audio file must have .m4a extension".to_string(),
                    ));
                }
            }
            _ => {
                return Err(Error::InvalidFileType(
                    "Filename must match 'Audio' or 'json'".to_string(),
                ));
            }
        }

        files_map.insert(
            name,
            UploadedFile {
                name: file_name,
                data: file_data,
            },
        );
    }

    validate_and_build(files_map)
}

fn validate_and_build(mut fields: HashMap<String, UploadedFile>) -> Result<UploadData> {
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

fn path_from_json(j: &AudioMetadata) -> Result<String> {
    let dt: DateTime<Utc> = j.call.start_time;

    let date_path = format!("{}", dt.format("%Y/%m/%d"));

    let system_path = j
        .call
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
    let file = Part::bytes(f.data.to_vec()).file_name(f.name.clone());

    let form = Form::new()
        .part("file", file)
        .text("model", c.env.model_name.clone())
        .text("language", "en")
        .text("response_format", "text");

    let res = c
        .http_client
        .post(&c.env.transcription_endpoint)
        .multipart(form)
        .send()
        .await
        .map_err(Error::WebhookSend)?
        .text()
        .await
        .map_err(Error::WebhookSend)?;

    Ok(res)
}

async fn create_webhook(m: &AudioMetadata, tr: String) -> Result<String> {
    let timestamp = format_timestamp_from_datetime(m.call.start_time);

    let field_types = vec![
        EmbedFieldType::Timestamp(timestamp.clone()),
        EmbedFieldType::RadioIds(m.src_list.iter().map(|x| x.src).collect()),
        EmbedFieldType::Transcription(tr),
    ];

    let fields: Vec<EmbedField> = field_types
        .into_iter()
        .map(|field_type| field_type.into_embed_field())
        .collect();

    let embeds = vec![WebhookEmbed {
        color: "12110930".to_string(),
        timestamp,
        title: format!(
            "{} - {}",
            m.talkgroup.talkgroup_group, m.talkgroup.talkgroup_description
        ),
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
    client: &Client,
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
    let tgid_as_string = &m.talkgroup.talkgroup.to_string();
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
        && c.group().contains(&m.talkgroup.talkgroup_group)
    {
        info!(group = %m.talkgroup.talkgroup_group, "Matched group, transcribing");
        return true;
    };

    // return false if not negated by previous tgid include, or group include
    info!(group = %m.talkgroup.talkgroup_group, tgid = %tgid_as_string, "Filter values unmatched");
    false
}

fn set_call_ids<T: model::IsList>(v: &mut [T], id: String) {
    for item in v.iter_mut() {
        item.set_call_id(id.clone());
        item.calculate_hash();
    }
}

async fn write_to_database(m: &AudioMetadata, c: &ProcessorConfig) -> Result<()> {
    use schema::calls::dsl::*;
    use schema::freqlist::dsl::*;
    use schema::sources::dsl::*;
    use schema::srclist::dsl::*;
    use schema::talkgroups::dsl::*;

    let mut connection = c
        .clone()
        .db_pool
        .get()
        .map_err(|e| Error::Database(e.to_string()))?;

    for item in &m.sources {
        insert_into(sources)
            .values(item)
            .on_conflict(schema::sources::src)
            .do_update()
            .set(item)
            .execute(&mut connection)
            .map_err(|e| Error::Database(e.to_string()))?;
    }

    insert_into(talkgroups)
        .values(&m.talkgroup)
        .on_conflict(schema::talkgroups::talkgroup)
        .do_update()
        .set(&m.talkgroup)
        .execute(&mut connection)
        .map_err(|e| Error::Database(e.to_string()))?;

    connection
        .transaction(|conn| {
            let _call_id: String = insert_into(calls)
                .values(&m.call)
                .on_conflict(schema::calls::filename)
                .do_update()
                .set(&m.call)
                .returning(schema::calls::filename)
                .get_result(conn)?;

            let mut src_list = m.src_list.clone();
            let mut freq_list = m.freq_list.clone();

            set_call_ids(&mut src_list, _call_id.clone());
            set_call_ids(&mut freq_list, _call_id);

            insert_into(srclist)
                .values(src_list)
                .on_conflict(schema::srclist::hashed)
                .do_nothing()
                .execute(conn)?;

            insert_into(freqlist)
                .values(freq_list)
                .on_conflict(schema::freqlist::hashed)
                .do_nothing()
                .execute(conn)?;

            diesel::result::QueryResult::Ok(())
        })
        .map_err(|e| Error::Database(e.to_string()))
}

// ---------------------------------------------------------------------
// --- HANDLER AND MAIN ---
// ---------------------------------------------------------------------

pub async fn upload(
    State(config): State<ProcessorConfig>,
    headers: HeaderMap,
    m: Multipart,
) -> Result<String> {
    let upload_start = Instant::now();
    info!("Starting upload processing");

    let files: UploadData = multipart_to_struct(m).await?;

    let meta = &mut files.deserialize_json()?;
    let path: String = path_from_json(meta)?;

    meta.call.filename = path.clone() + "/" + &files.audio.name;
    meta.call.talkgroup = meta.talkgroup.talkgroup;

    info!(talkgroup = meta.talkgroup.talkgroup, path = %path, "Processed audio metadata");

    let do_transcription = if headers.contains_key("archive") {
        info!(file = %meta.call.filename, "Set to archive:");
        false
    } else if config.filter.enabled() {
        filter_on_metadata(meta, &config.filter).await
    } else {
        false
    };

    if !do_transcription {
        let upload_fut = upload_files(&config.s3_client, &path, &files);

        meta.call.transcription = None;
        let db_fut = write_to_database(meta, &config);

        tokio::try_join!(upload_fut, db_fut)?;
    } else if do_transcription {
        let upload_fut = upload_files(&config.s3_client, &path, &files);
        let transcription_fut = transcribe_audio(&files.audio, &config);

        let (_, transcription) = tokio::try_join!(upload_fut, transcription_fut)?;

        meta.call.transcription = Some(transcription.clone());

        let db_fut = write_to_database(meta, &config);
        let webhook_fut = send_webhook(
            &config.http_client,
            &config.env.discord_webhook,
            meta,
            transcription,
            files.audio,
        );

        tokio::try_join!(db_fut, webhook_fut)?;
    }

    let duration = Instant::now().duration_since(upload_start);
    info!(
        duration_ms = duration.as_millis(),
        "Upload processing completed successfully"
    );

    Ok("Upload successful".to_string())
}
