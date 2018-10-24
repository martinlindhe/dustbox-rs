test:
	cargo test --all -- --color always --nocapture

expensive-demo:
	RUST_TEST_THREADS=1 cargo test demo --release -- --color always --nocapture --ignored

expensive-256:
	RUST_TEST_THREADS=1 cargo test demo_256 --release -- --color always --nocapture --ignored

expensive-512:
	RUST_TEST_THREADS=1 cargo test demo_512 --release -- --color always --nocapture --ignored

# includes demo_256_32bit and demo_512_32bit
expensive-32bit:
	RUST_TEST_THREADS=1 cargo test 32bit --release -- --color always --nocapture --ignored

expensive-games:
	RUST_TEST_THREADS=1 cargo test games_com --release -- --color always --nocapture --ignored

expensive-encode:
	RUST_TEST_THREADS=1 cargo test encode -- --color always --nocapture --ignored

bench:
	cargo bench --all

mips:
	cargo test --release mips -- --nocapture

run:
	cargo run --package dustbox_debugger

run-release:
	cargo run --release --package dustbox_gtk

fuzz:
	cargo run --package dustbox_fuzzer

lint:
	cargo +nightly clippy --all --allow cyclomatic_complexity

prober:
	cd utils/prober && make

typos:
	speller . > spell

glade:
	glade debugger/src/interface.glade

bindiff:
	vbindiff ~/dosbox-x/MEMDUMP.BIN emu_mem.bin
