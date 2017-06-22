create table projects (
    id          serial    primary key
  , created_at  timestamp not null default CURRENT_TIMESTAMP
  , user_id     integer   not null
  , name        text      not null
  , description text      not null
  , foreign key (user_id) references users(id)
);
