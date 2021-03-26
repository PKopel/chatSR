FROM rust:1.51 as builder

WORKDIR /usr/src/server
COPY . .

RUN cargo install --bin server --path .

FROM debian:buster-slim
RUN rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/server /usr/local/bin/server
CMD ["server"]