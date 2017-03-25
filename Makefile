.PHONY: samples

samples:
	cd samples/adrmode && make
	ndisasm samples/adrmode/adrmode.com -o 0x100
