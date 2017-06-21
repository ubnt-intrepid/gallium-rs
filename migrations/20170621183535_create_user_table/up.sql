create table users (
    id            serial    primary key
  , username      text      not null unique
  , email_address text      not null
  , bcrypt_hash   text      not null
  , created_at    timestamp not null default CURRENT_TIMESTAMP
);

alter table public_keys add column user_id integer not null;
alter table public_keys add constraint public_keys_user_id
    foreign key (user_id) references users(id);
