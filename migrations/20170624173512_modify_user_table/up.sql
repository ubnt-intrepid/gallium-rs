alter table users rename column username to name;
alter table users add column screen_name text not null default '';
