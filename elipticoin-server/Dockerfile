# Use a modern version of rust
FROM rust:latest
LABEL maintainer="peter@peterctucker.com"
RUN apt update
RUN apt install git-all -y
RUN build_deps="curl" && \
    apt-get update && \
    DEBIANe_FRONTEND=noninteractive apt-get install -y --no-install-recommends ${build_deps} ca-certificates && \
    curl -s https://packagecloud.io/install/repositories/github/git-lfs/script.deb.sh | bash && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends git-lfs && \
    git lfs install && \
    DEBIAN_FRONTEND=noninteractive apt-get purge -y --auto-remove ${build_deps} && \
    rm -r /var/lib/apt/lists/*
RUN apt update && apt install build-essential libpq-dev pkg-config libssl-dev llvm clang -y
WORKDIR root
RUN git clone https://github.com/ellipticoin/ellipticoind.git
WORKDIR ellipticoind
RUN rm -rf target
RUN rustup component add rustfmt --toolchain 1.47.0-x86_64-unknown-linux-gnu
RUN C_INCLUDE_PATH=/usr/lib/gcc/x86_64-linux-gnu/7/include cargo build --release 
WORKDIR /root/ellipticoind
RUN ./target/release/ellipticoind generate-keypair >> 'keys.txt'
COPY env_setup.py ./env_setup.py

ARG PRIVATE_KEY
ENV PRIVATE_KEY $PRIVATE_KEY

RUN python3 env_setup.py
RUN apt install curl
RUN curl https://bootstrap.pypa.io/get-pip.py -o get-pip.py
RUN python3 get-pip.py
RUN pip install wait-for-it
RUN python3 --version
RUN apt install postgresql-client-common -y
RUN apt install postgresql-client -y
RUN apt install  redis-server redis -y
RUN git lfs install
RUN git lfs pull