.PHONY: test ex1 x64

ex1:
	nasm -f bin -o ex1.com ex1.asm && ndisasm -o 0x100 ex1.com

test:
	gcc test.c -Wall -g -o test
	./test
