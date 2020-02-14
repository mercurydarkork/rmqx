FROM scratch
LABEL maintainer="gao.qingfeng@gmail.com"
COPY target/x86_64-unknown-linux-musl/release/client /mqttclient
ENTRYPOINT ["/mqttclient"]