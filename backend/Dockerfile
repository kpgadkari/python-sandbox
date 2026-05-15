FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.toml
COPY src src
RUN cargo build --release

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 sandbox
WORKDIR /app
COPY --from=build /app/target/release/python-sandbox-backend /usr/local/bin/python-sandbox-backend
RUN mkdir -p /app/data && chown -R sandbox:sandbox /app
USER sandbox
EXPOSE 8080
CMD ["python-sandbox-backend"]
