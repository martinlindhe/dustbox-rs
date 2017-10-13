test:
	cargo test -- --color always --nocapture

bench:
	cargo bench

run-debug:
	cargo run

run-release:
	cargo run --release

lint:
	rm -rf target
	rustup run nightly cargo clippy
