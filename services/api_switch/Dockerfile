FROM debian:bookworm-slim

# container setup basics
ENV HOME=/root
RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y curl build-essential pkg-config libssl-dev vim tree net-tools

# env variables and credentials
ENV PATH="/root/.cargo/bin:${PATH}"
RUN echo "cd $HOME" > /root/bashrc 
RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

WORKDIR /root/codebase

# setup rust nightly
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN . ~/.bashrc
RUN rustup toolchain install nightly
RUN rustup default nightly
RUN cargo install cargo-watch
RUN cargo install sqlx-cli

# Bring the service source into the image.
#
# Required by the self-host compose, which mounts no volumes: without this the
# container starts on an empty /root/codebase, `cargo sqlx migrate run` fails
# with "error canonicalizing path migrations", and the service crash-loops. The
# dev compose bind-mounts over this same path, so day-to-day development is
# unaffected by it.
COPY . .

# Compile at image build time rather than on every container start. sqlx builds
# against the checked-in .sqlx query cache, so no database is needed here.
ENV SQLX_OFFLINE=true
RUN cargo build --release
