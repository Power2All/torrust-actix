FROM rust:alpine

RUN apk add git musl-dev curl
RUN git clone https://github.com/Power2All/torrust-actix.git /tmp/torrust-actix
RUN cd /tmp/torrust-actix && git checkout tags/v4.0.2
WORKDIR /tmp/torrust-actix
RUN cd /tmp/torrust-actix
RUN cargo build --release && rm -Rf target/release/.fingerprint target/release/build target/release/deps target/release/examples target/release/incremental
COPY init.sh /tmp/torrust-actix/target/release/init.sh
RUN chmod +x /tmp/torrust-actix/target/release/init.sh
EXPOSE 8080/tcp
EXPOSE 6969/tcp
EXPOSE 6969/udp
CMD cd /tmp/torrust-actix/target/release/ && ./init.sh && ./torrust-actix



