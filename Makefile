all: release docker

debug:
	cross build --target=x86_64-unknown-linux-musl
	# cargo build

release:
	cross build --release --target=x86_64-unknown-linux-musl
	# cargo build --release

docker:
	docker build --no-cache -t registry.cn-hangzhou.aliyuncs.com/bis28/rmqx .
	docker push registry.cn-hangzhou.aliyuncs.com/bis28/rmqx

clean:
	cargo clean

