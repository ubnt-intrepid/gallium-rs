create table users (
    id            serial      primary key
  , created_at    timestamp   not null default CURRENT_TIMESTAMP
  , name          text        not null unique
  , screen_name   text
  , bcrypt_hash   varchar(60) not null
);

create table ssh_keys (
    id          serial    primary key
  , created_at  timestamp not null default CURRENT_TIMESTAMP
  , key         text      not null
  , user_id     integer   not null
  , description text
  , foreign key (user_id) references users(id)
);

create table projects (
    id          serial    primary key
  , created_at  timestamp not null default CURRENT_TIMESTAMP
  , user_id     integer   not null
  , name        text      not null
  , description text
  , foreign key (user_id) references users(id)
);

