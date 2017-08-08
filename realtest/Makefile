.PHONY: test ex1 cc

ex1:
	nasm -f elf64 -o ex1 ex1.asm && gobjdump -d ./ex1

test:
	gcc test.c -g -o test
	./test

cc:
	gcc cc.c -o cc
	gobjdump -d ./cc
