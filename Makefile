default: build

test: build
	cargo test --all --tests

build:
	mkdir -p target/wasm32v1-none/optimized
	stellar contract build --optimize --out-dir target/wasm32v1-none/optimized
	cd target/wasm32v1-none/optimized/ && \
		for i in *.wasm ; do \
			ls -l "$$i"; \
		done

fmt:
	cargo fmt --all

clean:
	cargo clean

