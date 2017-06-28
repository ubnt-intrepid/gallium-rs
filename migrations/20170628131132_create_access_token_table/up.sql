create table access_tokens (
    id            serial      primary key
  , created_at    timestamp   not null default CURRENT_TIMESTAMP
  , user_id       integer     not null
  , oauth_app_id  integer     not null
  , hash          varchar(40) not null unique
  , description   text        not null
  , foreign key (user_id) references users(id)
  , foreign key (oauth_app_id) references oauth_apps(id)
);
