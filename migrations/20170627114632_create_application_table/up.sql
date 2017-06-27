create table applications (
    id          serial      primary key
  , name        text        not null unique
  , created_at  timestamp   not null default CURRENT_TIMESTAMP
  , client_id   varchar(40) not null unique
);
