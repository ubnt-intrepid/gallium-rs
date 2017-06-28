alter table oauth_apps add column user_id integer not null;
alter table oauth_apps add constraint oauth_apps_user_id
    foreign key (user_id) references users(id);
