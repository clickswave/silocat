FROM postgres:18-bookworm

# update debian and install basic tools
RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y curl build-essential pkg-config libssl-dev vim

# setup postgres
RUN apt-get install -y postgresql-contrib

WORKDIR /root/codebase
