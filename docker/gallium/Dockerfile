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

EXPOSE 22 80

RUN useradd -ms /bin/bash -d /data git
RUN echo 'git:git' | chpasswd

COPY bin/* /usr/local/bin/

COPY conf/nginx.conf        /etc/nginx/nginx.conf
COPY conf/sshd_config       /etc/ssh/sshd_config
COPY conf/supervisord.conf  /etc/supervisord.conf

CMD [ "/usr/bin/supervisord", "-c", "/etc/supervisord.conf" ]
