#FROM scratch
FROM pingcap/alpine-glibc
LABEL maintainer="gao.qingfeng@gmail.com"
COPY target/release/rmqxd /
EXPOSE 1883 8883 5555 80 443 5683
ENTRYPOINT ["/rmqxd"]
