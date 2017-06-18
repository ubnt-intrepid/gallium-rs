create table public_keys (
    id          serial    primary key
  , created_at  timestamp not null default CURRENT_TIMESTAMP
  , key         text      not null
);
