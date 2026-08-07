FROM node:bookworm-slim

# container setup basics
RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y curl build-essential pkg-config libssl-dev vim tree net-tools git

# copy service source files
WORKDIR /root/codebase

# Required by the self-host compose, which mounts no volumes: without this the
# container starts on an empty /root/codebase and `npm i && npm run dev` exits
# immediately. The dev compose bind-mounts over this same path, so day-to-day
# development is unaffected by it.
COPY . .

# Install at image build time so the first container start is not a cold install.
RUN npm install