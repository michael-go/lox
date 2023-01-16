build:
	cargo build

build-release:
	cargo build --release
	
gen-fixture-outs: build install-cargo-script
	cargo script tests/create-fixture-outs.rs
	
test:
	cargo test

fmt:
	cargo fmt

install-cargo-script:
	cargo install cargo-script
