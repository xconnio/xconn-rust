lint-check:
	cargo fmt -- --check
	cargo clippy --all-targets --all-features -- -D warnings

lint-format:
	cargo fmt --

run:
	cargo run

build:
	cargo build

build-release:
	cargo build --release
