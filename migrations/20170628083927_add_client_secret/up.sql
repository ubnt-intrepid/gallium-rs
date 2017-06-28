alter table oauth_apps add column client_secret varchar(40) not null default 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx';
alter table oauth_apps alter column client_secret drop default;
