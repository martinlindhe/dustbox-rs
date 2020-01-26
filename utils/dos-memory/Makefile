all:
	nasm -f bin -o memory.com memory.asm
	ndisasm -o 0x100 memory.com
	cp memory.com ~/dosbox-x
