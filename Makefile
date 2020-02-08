all: release docker

debug:
	cross build --target=x86_64-unknown-linux-musl

release:
	cross build --release --target=x86_64-unknown-linux-musl

docker:
	docker build -t registry.cn-hangzhou.aliyuncs.com/bis28/rmqx .
	docker push registry.cn-hangzhou.aliyuncs.com/bis28/rmqx

clean:
	cargo clean

