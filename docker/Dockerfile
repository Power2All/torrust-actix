FROM rust:alpine3.18

RUN apk add git musl-dev
RUN git clone https://github.com/Power2All/torrust-actix.git /root/torrust-actix
RUN cd /tmp/torrust-actix && git checkout tags/v4.0.0
WORKDIR /tmp/torrust-actix
RUN cd /tmp/torrust-actix
RUN cargo build --release && rm -Rf target/release/.fingerprint target/release/build target/release/deps target/release/examples target/release/incremental
COPY init.sh /root/torrust-actix/target/release/init.sh
RUN chmod +x /root/torrust-actix/target/release/init.sh
EXPOSE 8080/tcp
EXPOSE 6969/tcp
EXPOSE 6969/udp
CMD cd /tmp/torrust-actix/target/release/ && ./init.sh && ./torrust-actix
