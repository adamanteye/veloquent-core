FROM rust:1.81-alpine3.20 as builder
WORKDIR /usr/src/veloquent-core
COPY Cargo.toml Cargo.lock bfsu.toml ./
RUN apk add --no-cache mold musl-dev
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && mv bfsu.toml /usr/local/cargo/config.toml \
    && sed -i '/^members = \[.*\]$/d' Cargo.toml \
    && sed -i '/^migration = {.*path =.*}$/d' Cargo.toml \
    && cargo fetch
COPY . .
RUN cargo build --release --features "dev"

FROM alpine:3.20 as runner
COPY --from=builder /usr/src/veloquent-core/target/release/veloquent-core /usr/local/bin/veloquent-core
COPY --from=builder /usr/src/veloquent-core/veloquent.toml /usr/local/etc/veloquent/veloquent.toml
EXPOSE 80
CMD ["veloquent-core"]
