alter table public_keys drop foreign key public_keys_user_id;
alter table public_keys drop column user_id;
drop table users;
