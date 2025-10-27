CREATE TABLE freqlist (
  call_id varchar not null references calls(filename),
  hashed bigint primary key,
  freq integer not null,
  time timestamptz not null,
  pos interval not null,
  len interval not null,
  error_count smallint not null,
  spike_count smallint not null
);