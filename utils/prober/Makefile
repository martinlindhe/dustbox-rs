all:
	nasm -f bin -o prober.com prober.asm
	ndisasm -o 0x100 prober.com | more
	cp prober.com ../../../dos-software-decoding
	cp prober.com ~/vm-share
	cp prober.com ~/dosbox-x
