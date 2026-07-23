FROM node:bookworm-slim

# container setup basics
RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y curl build-essential pkg-config libssl-dev vim tree net-tools git

# copy service source files
WORKDIR /root/codebase