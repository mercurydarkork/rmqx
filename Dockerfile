FROM scratch
LABEL maintainer="gao.qingfeng@gmail.com"
COPY target/x86_64-unknown-linux-musl/release/rmqxd /
EXPOSE 1883
ENTRYPOINT ["/rmqxd"]
