all:
	cargo build --release --target=x86_64-unknown-linux-musl
	docker build --no-cache -t  registry.cn-hangzhou.aliyuncs.com/bis28/rmqx .
	docker push registry.cn-hangzhou.aliyuncs.com/bis28/rmqx
