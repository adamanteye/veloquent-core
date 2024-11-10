FROM rust:1.81-slim-bookworm as builder
WORKDIR /usr/src/veloquent-core
COPY Cargo.toml Cargo.lock bfsu.toml ./
RUN apt-get update && apt-get install -y mold
RUN --mount=type=cache,target=/root/.cargo mkdir -p src .cargo && echo 'fn main() {}' > src/main.rs \
    && mv bfsu.toml .cargo/config.toml \
    && sed -i '/^members = \[.*\]$/d' Cargo.toml \
    && sed -i '/^migration = {.*path =.*}$/d' Cargo.toml \
    && cargo fetch
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim as runner
COPY --from=builder /usr/src/veloquent-core/target/release/veloquent-core /usr/local/bin/veloquent-core
COPY --from=builder /usr/src/veloquent-core/veloquent.toml /usr/local/etc/veloquent.toml
EXPOSE 80
CMD ["veloquent-core"]
