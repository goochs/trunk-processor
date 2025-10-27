CREATE TYPE AudioType AS ENUM (
  'analog',
  'digital',
  'digital_tdma'
);

CREATE TABLE calls (
    filename varchar primary key,
    freq integer not null,
    freq_error smallint not null,
    signal smallint not null,
    noise smallint not null,
    source_num smallint not null,
    recorder_num smallint not null,
    tdma_slot smallint not null,
    phase2_tdma smallint not null,
    start_time timestamptz not null,
    stop_time timestamptz not null,
    emergency boolean not null,
    priority smallint not null,
    mode smallint not null,
    duplex smallint not null,
    encrypted boolean not null,
    call_length smallint not null,
    talkgroup integer not null references talkgroups(talkgroup),
    audio_type AudioType not null,
    short_name varchar not null,
    transcription varchar
);