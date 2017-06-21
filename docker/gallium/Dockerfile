FROM ubuntu:xenial

RUN apt-get update && \
    apt-get install -y \
      git \
      nginx \
      openssh-server \
      postgresql \
      rsyslog \
      supervisor && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
RUN mkdir -p /var/run/sshd

RUN useradd -ms /usr/bin/git-shell -d /data git
RUN echo 'git:git' | chpasswd

COPY conf/nginx.conf  /etc/nginx/nginx.conf
COPY conf/sshd_config /etc/ssh/sshd_config
COPY conf/supervisord.conf /etc/supervisor/conf.d/supervisord.conf
COPY conf/show_authorized_keys.sh /usr/local/bin/show_authorized_keys.sh

EXPOSE 22 80
VOLUME [ "/data", "/opt/gallium" ]

CMD [ "/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf" ]