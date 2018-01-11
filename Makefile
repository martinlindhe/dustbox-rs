test:
	cargo test --all -- --color always --nocapture

expensive-test:
	cargo test --all -- --color always --nocapture --ignored

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
