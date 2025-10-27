// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "audiotype"))]
    pub struct Audiotype;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Audiotype;

    calls (filename) {
        filename -> Varchar,
        freq -> Int4,
        freq_error -> Int2,
        signal -> Int2,
        noise -> Int2,
        source_num -> Int2,
        recorder_num -> Int2,
        tdma_slot -> Int2,
        phase2_tdma -> Int2,
        start_time -> Timestamptz,
        stop_time -> Timestamptz,
        emergency -> Bool,
        priority -> Int2,
        mode -> Int2,
        duplex -> Int2,
        encrypted -> Bool,
        call_length -> Int2,
        talkgroup -> Int4,
        audio_type -> Audiotype,
        short_name -> Varchar,
        transcription -> Nullable<Varchar>,
    }
}

diesel::table! {
    freqlist (hashed) {
        call_id -> Varchar,
        hashed -> Int8,
        freq -> Int4,
        time -> Timestamptz,
        pos -> Interval,
        len -> Interval,
        error_count -> Int2,
        spike_count -> Int2,
    }
}

diesel::table! {
    sources (src) {
        src -> Int4,
        tag -> Nullable<Varchar>,
    }
}

diesel::table! {
    srclist (hashed) {
        call_id -> Varchar,
        hashed -> Int8,
        src -> Int4,
        time -> Timestamptz,
        pos -> Interval,
        emergency -> Bool,
        signal_system -> Nullable<Varchar>,
    }
}

diesel::table! {
    talkgroups (talkgroup) {
        talkgroup -> Int4,
        talkgroup_tag -> Varchar,
        talkgroup_description -> Varchar,
        talkgroup_group_tag -> Varchar,
        talkgroup_group -> Varchar,
    }
}

diesel::joinable!(calls -> talkgroups (talkgroup));
diesel::joinable!(freqlist -> calls (call_id));
diesel::joinable!(srclist -> calls (call_id));
diesel::joinable!(srclist -> sources (src));

diesel::allow_tables_to_appear_in_same_query!(calls, freqlist, sources, srclist, talkgroups,);
