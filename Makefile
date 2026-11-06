default: build

test: build
	cargo test --all --tests

build:
	mkdir -p target/wasm32v1-none/optimized

	cargo rustc --manifest-path=Cargo.toml --crate-type=cdylib --target=wasm32v1-none --release
	stellar contract optimize \
		--wasm target/wasm32v1-none/release/backstop_manager.wasm \
		--wasm-out target/wasm32v1-none/optimized/backstop_manager.wasm

	cd target/wasm32v1-none/optimized/ && \
		for i in *.wasm ; do \
			ls -l "$$i"; \
		done

fmt:
	cargo fmt --all

clean:
	cargo clean

