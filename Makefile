all:
	cargo build --release
	docker build --no-cache -t  registry.cn-hangzhou.aliyuncs.com/bis28/rmqx .
	docker push registry.cn-hangzhou.aliyuncs.com/bis28/rmqx
