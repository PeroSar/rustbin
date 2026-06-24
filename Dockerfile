FROM rust:trixie AS builder

RUN apt-get update && apt-get install -y golang build-essential git && rm -rf /var/lib/apt/lists/*

WORKDIR /src

RUN git clone --depth 1 https://github.com/PeroSar/rustbin.git .

RUN cargo build --release --locked

FROM debian:trixie-slim
WORKDIR /app

RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/rustbin /usr/local/bin/rustbin

ENV HOST=0.0.0.0
ENV PORT=3000
ENV DATABASE_URL=sqlite:///data/rustbin.db
ENV MAX_PASTE_SIZE=2MB
ENV CLASSIFIER_MAX_BYTES=64KB
ENV HIGHLIGHT_MAX_BYTES=256KB
ENV RENDER_CACHE_CAPACITY=128
ENV CLEANUP_INTERVAL=3600
ENV DB_MIN_CONNECTIONS=1
ENV DB_MAX_CONNECTIONS=5
ENV RUST_LOG=rustbin=info,sqlx=warn

RUN mkdir -p /data
VOLUME ["/data"]
EXPOSE 3000

CMD ["rustbin"]
