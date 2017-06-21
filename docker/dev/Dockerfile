FROM ubuntu:xenial

RUN apt-get update && \
    apt-get install -y \
      build-essential \
      curl \
      git \
      libpq-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Rust toolchain (for build executable)
RUN curl -sSf https://sh.rustup.rs | sh -s -- --no-modify-path -y && \
    /root/.cargo/bin/cargo install diesel_cli --no-default-features --features postgres
ENV PATH=/root/.cargo/bin:$PATH

VOLUME [ "/source", "/opt/gallium" ]

WORKDIR /source

CMD [ "/bin/bash" ]
