all:
	nasm -f bin -o prober.com prober.asm
	cp prober.com ../../../dos-software-decoding
	cp prober.com ~/vm-share
	cp prober.com ~/dosbox-x
	ndisasm -o 0x100 prober.com | head -n 10
