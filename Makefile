test:
	cargo test --all -- --color always --nocapture

test-harness:
	cargo run --release --package harness harness/sets/demo-com-16bit.yml
	cargo run --release --package harness harness/sets/demo-com-32bit.yml
	cargo run --release --package harness harness/sets/games-com-commercial-16bit.yml

expensive-encode:
	cargo test encode -- --color always --nocapture --ignored

bench:
	cargo bench --all

mips:
	cargo test --release mips -- --nocapture

run:
	cargo run --package debugger

run-release:
	cargo run --release --package debugger

disasm:
	cargo run --release --package disasm

install-disasm:
	cargo install --path disasm --force

fuzz:
	cargo run --package fuzzer -- supersafe --mutations 50 --host 172.16.72.129
	# cargo run --package fuzzer -- dosbox-x --mutations 20
	# cargo run --package fuzzer -- vmrun --mutations 50 --vmx "/Users/m/Documents/Virtual Machines.localized/Windows XP Professional.vmwarevm/Windows XP Professional.vmx" --username vmware --password vmware

lint:
	cargo clippy --all

prober:
	cd utils/prober && make

glade:
	glade debugger/src/interface.glade

coverage:
	# XXX requires linux -jan 2020. osx support https://github.com/xd009642/tarpaulin/issues/152
	cargo tarpaulin --out Html
