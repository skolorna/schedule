FROM ekidd/rust-musl-builder:beta as builder

RUN USER=rust cargo new --bin schedule
WORKDIR /home/rust/src/schedule
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release

RUN rm src/*.rs
ADD . ./
RUN rm ./target/x86_64-unknown-linux-musl/release/deps/schedule*

RUN cargo build --release

FROM alpine:latest

RUN addgroup -S appuser \
    && adduser -S -g appuser appuser

RUN apk update \
	&& apk add --no-cache ca-certificates \
	&& rm -rf /var/cache/apk/*

WORKDIR /usr/src/app

COPY --from=builder /home/rust/src/schedule/target/x86_64-unknown-linux-musl/release/schedule ./

RUN chown -R appuser:appuser /usr/src/app
USER appuser

EXPOSE 8000
CMD ["./schedule"]
