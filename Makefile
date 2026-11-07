SOURCE_REPO_META := source_repo=github:BLNT-Protocol/backstop-manager
WASM := target/wasm32v1-none/optimized/backstop_manager.wasm

.PHONY: default build verify-source-metadata test check fmt clean

default: build

build:
	stellar contract build --package backstop-manager --locked \
		--meta $(SOURCE_REPO_META) \
		--out-dir target/wasm32v1-none/optimized
	$(MAKE) --no-print-directory verify-source-metadata

verify-source-metadata:
	@stellar contract info meta --wasm $(WASM) 2>/dev/null | \
		grep -Fq "source_repo: github:BLNT-Protocol/backstop-manager" || \
		{ printf 'error: missing source_repo metadata in backstop-manager WASM\n' >&2; exit 1; }

test: build
	cargo fmt --all -- --check
	cargo test --all --tests --locked

check:
	cargo check --all-targets --all-features --locked

fmt:
	cargo fmt --all

clean:
	cargo clean

