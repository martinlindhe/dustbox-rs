all:
	nasm -f bin -o mouse.com mouse.asm
	ndisasm -o 0x100 mouse.com
	cp mouse.com ~/dosbox-x
