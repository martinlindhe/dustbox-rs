all:
	nasm -f bin -o readfile.com readfile.asm
	ndisasm -o 0x100 readfile.com
	cp readfile.com ~/dosbox-x
