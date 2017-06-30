create table users (
    id            serial    primary key
  , name          text      not null unique
  , email_address text      not null
  , bcrypt_hash   text      not null
  , created_at    timestamp not null default CURRENT_TIMESTAMP
  , screen_name   text      not null default ''
  , is_admin      boolean   not null default 'false'
);

create table public_keys (
    id          serial    primary key
  , created_at  timestamp not null default CURRENT_TIMESTAMP
  , key         text      not null
  , user_id     integer   not null
  , title       text      not null
  , foreign key (user_id) references users(id)
);

create table projects (
    id          serial    primary key
  , created_at  timestamp not null default CURRENT_TIMESTAMP
  , user_id     integer   not null
  , name        text      not null
  , description text      not null default ''
  , foreign key (user_id) references users(id)
);

create table access_tokens (
    id            serial      primary key
  , created_at    timestamp   not null default CURRENT_TIMESTAMP
  , user_id       integer     not null
  , hash          varchar(40) not null unique
  , foreign key (user_id) references users(id)
);
