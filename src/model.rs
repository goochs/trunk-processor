use chrono::{DateTime, TimeDelta, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::{
    NoneAsEmptyString, chrono_0_4::datetime_utc_ts_seconds_from_any, serde_as,
    skip_serializing_none,
};
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::common::{map_float_sec_to_timedelta, map_int_to_bool};
use crate::schema::{calls, freqlist, sources, srclist, talkgroups};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::Audiotype"]
#[serde(rename_all = "snake_case")]
pub enum AudioType {
    Analog,
    Digital,
    #[serde(rename = "digital_tdma")]
    DigitalTdma,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadataRaw {
    #[serde(flatten)]
    pub call: Call,
    #[serde(flatten)]
    pub talkgroup: Talkgroups,
    #[serde(alias = "freqList")]
    pub freq_list: Vec<FreqList>,
    #[serde(alias = "srcList")]
    src_list_raw: Vec<SrcListRaw>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    #[serde(flatten)]
    pub call: Call,
    #[serde(flatten)]
    pub talkgroup: Talkgroups,
    #[serde(alias = "freqList")]
    pub freq_list: Vec<FreqList>,
    #[serde(alias = "srcList")]
    pub src_list: Vec<SrcList>,
    pub sources: Vec<Source>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SrcListRaw {
    src: i32,
    #[serde(deserialize_with = "datetime_utc_ts_seconds_from_any::deserialize")]
    time: DateTime<Utc>,
    #[serde(deserialize_with = "map_float_sec_to_timedelta")]
    pos: TimeDelta,
    #[serde(deserialize_with = "map_int_to_bool")]
    emergency: bool,
    #[serde_as(as = "NoneAsEmptyString")]
    signal_system: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    tag: Option<String>,
}

impl AudioMetadataRaw {
    pub fn split_src_list(&self) -> (Vec<SrcList>, Vec<Source>) {
        let mut src_list = Vec::new();
        let mut sources = Vec::new();

        for item in &self.src_list_raw {
            src_list.push(SrcList {
                call_id: String::new(),
                hashed: 0,
                src: item.src,
                time: item.time,
                pos: item.pos,
                emergency: item.emergency,
                signal_system: item.signal_system.clone(),
            });

            sources.push(Source {
                src: item.src,
                tag: item.tag.clone(),
            });
        }

        (src_list, sources)
    }
}

#[derive(
    AsChangeset,
    Insertable,
    Queryable,
    Identifiable,
    Selectable,
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[diesel(primary_key(talkgroup))]
#[diesel(table_name = talkgroups)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Talkgroups {
    pub talkgroup: i32,
    pub talkgroup_tag: String,
    pub talkgroup_description: String,
    pub talkgroup_group_tag: String,
    pub talkgroup_group: String,
}

#[skip_serializing_none]
#[derive(
    AsChangeset,
    Insertable,
    Queryable,
    Identifiable,
    Selectable,
    Associations,
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[diesel(belongs_to(Talkgroups, foreign_key = talkgroup))]
#[diesel(table_name = calls)]
#[diesel(primary_key(filename))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Call {
    pub freq: i32,
    pub freq_error: i16,
    pub signal: i16,
    pub noise: i16,
    pub source_num: i16,
    pub recorder_num: i16,
    pub tdma_slot: i16,
    pub phase2_tdma: i16,
    #[serde(deserialize_with = "datetime_utc_ts_seconds_from_any::deserialize")]
    pub start_time: DateTime<Utc>,
    #[serde(deserialize_with = "datetime_utc_ts_seconds_from_any::deserialize")]
    pub stop_time: DateTime<Utc>,
    #[serde(deserialize_with = "map_int_to_bool")]
    pub emergency: bool,
    pub priority: i16,
    pub mode: i16,
    pub duplex: i16,
    #[serde(deserialize_with = "map_int_to_bool")]
    pub encrypted: bool,
    pub call_length: i16,
    #[serde(skip)]
    pub talkgroup: i32,
    #[diesel(sql_type = Varchar)]
    pub audio_type: AudioType,
    pub short_name: String,
    #[serde(skip)]
    pub transcription: Option<String>,
    #[serde(skip)]
    pub filename: String,
}

#[skip_serializing_none]
#[derive(
    AsChangeset,
    Insertable,
    Queryable,
    Identifiable,
    Selectable,
    Associations,
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Hash,
)]
#[diesel(belongs_to(Call))]
#[diesel(table_name = freqlist)]
#[diesel(primary_key(hashed))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FreqList {
    #[serde(skip)]
    pub call_id: String,
    #[serde(skip)]
    pub hashed: i64,
    pub freq: i32,
    #[serde(deserialize_with = "datetime_utc_ts_seconds_from_any::deserialize")]
    pub time: DateTime<Utc>,
    #[serde(deserialize_with = "map_float_sec_to_timedelta")]
    pub pos: TimeDelta,
    #[serde(deserialize_with = "map_float_sec_to_timedelta")]
    pub len: TimeDelta,
    pub error_count: i16,
    pub spike_count: i16,
}

#[serde_as]
#[derive(
    AsChangeset,
    Insertable,
    Queryable,
    Identifiable,
    Selectable,
    Associations,
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Hash,
)]
#[diesel(belongs_to(Call))]
#[diesel(table_name = srclist)]
#[diesel(primary_key(hashed))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SrcList {
    #[serde(skip)]
    pub call_id: String,
    #[serde(skip)]
    pub hashed: i64,
    pub src: i32,
    #[serde(deserialize_with = "datetime_utc_ts_seconds_from_any::deserialize")]
    pub time: DateTime<Utc>,
    #[serde(deserialize_with = "map_float_sec_to_timedelta")]
    pub pos: TimeDelta,
    #[serde(deserialize_with = "map_int_to_bool")]
    pub emergency: bool,
    #[serde_as(as = "NoneAsEmptyString")]
    pub signal_system: Option<String>,
}

#[serde_as]
#[derive(
    AsChangeset,
    Insertable,
    Queryable,
    Identifiable,
    Selectable,
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Hash,
)]
#[diesel(table_name = sources)]
#[diesel(primary_key(src))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Source {
    #[serde(skip)]
    pub src: i32,
    #[serde_as(as = "NoneAsEmptyString")]
    pub tag: Option<String>,
}

pub trait IsList {
    fn set_call_id(&mut self, id: String);
    fn calculate_hash(&mut self);
}

impl IsList for SrcList {
    fn set_call_id(&mut self, id: String) {
        self.call_id = id;
    }
    fn calculate_hash(&mut self) {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        self.hashed = s.finish() as i64
    }
}

impl IsList for FreqList {
    fn set_call_id(&mut self, id: String) {
        self.call_id = id;
    }
    fn calculate_hash(&mut self) {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        self.hashed = s.finish() as i64
    }
}
