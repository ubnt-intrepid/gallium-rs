FROM ubuntu:xenial

RUN apt-get update && \
    apt-get install -y \
      build-essential \
      curl \
      git \
      libpq-dev \
      openssh-server \
      postgresql \
      rsyslog \
      supervisor && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
RUN mkdir -p /var/run/sshd

RUN useradd -ms /usr/bin/git-shell -d /data git
RUN echo 'git:git' | chpasswd

COPY conf/sshd_config /etc/ssh/sshd_config
COPY conf/supervisord.conf /etc/supervisor/conf.d/supervisord.conf
COPY conf/show_authorized_keys.sh /usr/local/bin/show_authorized_keys.sh

# Rust toolchain (for build executable)
RUN curl -sSf https://sh.rustup.rs | sh -s -- --no-modify-path -y && \
    /root/.cargo/bin/cargo install diesel_cli --no-default-features --features postgres
ENV PATH=/root/.cargo/bin:$PATH

EXPOSE 22 3000
VOLUME [ "/data" ]

# Build
ADD . /opt/gallium
WORKDIR /opt/gallium
RUN cargo install --force --root /opt/gallium

CMD [ "/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf" ]
