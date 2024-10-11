FROM rust:1.81.0 as builder
WORKDIR /usr/src/veloquent-core
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/veloquent-core /usr/local/bin/veloquent-core
EXPOSE 80
CMD ["veloquent-core"]
