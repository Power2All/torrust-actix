# Builds the checkout this file belongs to, so build from the repository root:
#   docker build -f docker/Dockerfile -t power2all/torrust-actix .
# (docker/build.bat and docker-compose.yml already do.)

FROM rust:1.98-alpine3.24 AS builder
# curl: utoipa-swagger-ui downloads the Swagger UI bundle at build time. No OpenSSL: TLS is rustls.
RUN apk add --no-cache musl-dev curl
WORKDIR /src
COPY . .
RUN cargo build --release --locked

# The runtime image carries only the binary: no toolchain, sources or build tree.
FROM alpine:3.24
# python3 runs the healthcheck (it reads the binds from config.toml with tomllib).
RUN apk add --no-cache python3 && adduser -D -h /app/torrust-actix tracker
WORKDIR /app/torrust-actix
COPY --from=builder /src/target/release/torrust-actix target/release/torrust-actix
COPY docker/init.sh target/release/init.sh
COPY docker/healthcheck.py healthcheck
RUN chmod +x target/release/init.sh healthcheck && chown -R tracker:tracker target/release
USER tracker
EXPOSE 8080/tcp
EXPOSE 6969/tcp
EXPOSE 6969/udp
CMD ["/app/torrust-actix/target/release/init.sh"]
HEALTHCHECK CMD /app/torrust-actix/healthcheck || exit 1