test:
	cargo test --all -- --color always --nocapture

test-harness:
	cargo run --release --package dustbox_harness

expensive-encode:
	RUST_TEST_THREADS=1 cargo test encode -- --color always --nocapture --ignored

bench:
	cargo bench --all

mips:
	cargo test --release mips -- --nocapture

run:
	cargo run --package dustbox_debugger

run-release:
	cargo run --release --package dustbox_debugger

disasm:
	cargo run --release --package dustbox_disassembler

install-disasm:
	cargo install --path disassembler --force

fuzz:
	cargo run --package dustbox_fuzzer -- --mutations 50 --host 172.16.72.129

lint:
	cargo clippy --all

prober:
	cd utils/prober && make

glade:
	glade debugger/src/interface.glade
