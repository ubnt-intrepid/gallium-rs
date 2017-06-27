alter table oauth_apps add column redirect_uri text not null default 'urn:ietf:wg:oauth:2.0:oob';
