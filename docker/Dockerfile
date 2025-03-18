FROM rust:alpine

RUN apk add git musl-dev curl pkgconfig openssl-dev openssl-libs-static
RUN git clone https://github.com/Power2All/torrust-actix.git /app/torrust-actix
RUN cd /app/torrust-actix && git checkout tags/v4.0.9
WORKDIR /app/torrust-actix
RUN cd /app/torrust-actix
RUN cargo build --release && rm -Rf target/release/.fingerprint target/release/build target/release/deps target/release/examples target/release/incremental
COPY init.sh /app/torrust-actix/target/release/init.sh
COPY healthcheck.sh /app/torrust-actix/healthcheck.sh
RUN chmod +x /app/torrust-actix/target/release/init.sh
RUN chmod +x /app/torrust-actix/healthcheck.sh
EXPOSE 8080/tcp
EXPOSE 6969/tcp
EXPOSE 6969/udp
CMD cd /app/torrust-actix/target/release/ && ./init.sh && ./torrust-actix
HEALTHCHECK CMD /app/torrust-actix/healthcheck.sh || exit 1