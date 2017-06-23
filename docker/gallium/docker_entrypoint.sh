#!/bin/bash

# change permission
chown -R git:git /data
chmod -R 0755 /data

/usr/bin/supervisord -c /etc/supervisord.conf
