FROM rust:1.81-slim-bookworm as builder
WORKDIR /usr/src/veloquent-core
COPY . .
RUN export CARGO_HOME=.cargo \
    && mkdir -p .cargo \
    && cp bfsu.toml $CARGO_HOME/config.toml \
    && cargo build --release

FROM debian:bookworm-slim as runner
COPY --from=builder /usr/local/cargo/target/release/veloquent-core /usr/local/bin/veloquent-core
COPY --from=builder /usr/src/veloquent-core/veloquent.toml /usr/local/etc/veloquent.toml
EXPOSE 8080
CMD ["veloquent-core"]
