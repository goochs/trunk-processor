CREATE TABLE srclist (
  call_id varchar not null references calls(filename),
  hashed bigint primary key,
  src integer not null references sources(src),
  time timestamptz not null,
  pos interval not null,
  emergency boolean not null,
  signal_system varchar default null
);