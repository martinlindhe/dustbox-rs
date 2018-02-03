test:
	cargo test --all -- --color always --nocapture

expensive-test:
	RUST_TEST_THREADS=1 cargo test --release -- --color always --nocapture --ignored

expensive-256:
	cargo test demo_256 --release -- --color always --nocapture --ignored

expensive-512:
	cargo test demo_512 --release -- --color always --nocapture --ignored

expensive-fuzz:
	cargo test fuzz --release -- --color always --nocapture --ignored

bench:
	cargo bench --all

run:
	cargo run --package dustbox_gtk

run-release:
	cargo run --release --package dustbox_gtk

lint:
	# rm -rf target
	rustup run nightly cargo clippy --all

typos:
	speller . > spell

glade:
	glade debugger/src/interface.glade

bindiff:
	vbindiff ~/dosbox-x/MEMDUMP.BIN emu_mem.bin
