# SamarDB v1.2.0 Official Linux Container
FROM ubuntu:22.04 AS builder

RUN apt-get update && apt-get install -y --no-install-recommends     build-essential     curl     git     ca-certificates     && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain to build L++ v1.2.0
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build/lpp
RUN git clone https://github.com/samarnever-droid/lplusplus.git . && cargo build --release
ENV PATH="/build/lpp/target/release:${PATH}"

WORKDIR /build/samardb
COPY . .
RUN lpp src/main.lpp -o /build/samardb/samardb-server

# Runtime Stage
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /var/lib/samardb
COPY --from=builder /build/samardb/samardb-server /usr/local/bin/samardb-server

EXPOSE 5432
VOLUME ["/var/lib/samardb/data"]

ENTRYPOINT ["samardb-server"]
CMD ["--port", "5432", "--data-dir", "/var/lib/samardb/data"]
