// XXX test. execute a instruction, record resulting register values

// XXX ret near = 0xC3, ret far = 0xCB
char code[] = {
    0xB8, 0x13, 0x13, // mov ax,0x1313
    0xC3,             // ret
};

// NOTE: requires linux

#include <stdio.h>
int main(int argc, char **argv)
{
    void *buf;

    /* copy code to executable buffer */    
    buf = mmap(0, sizeof(code), PROT_READ|PROT_WRITE|PROT_EXEC,
              MAP_PRIVATE|MAP_ANON, -1, 0);
    memcpy(buf, code, sizeof(code));

    /* run code */
    ((void (*) (void))buf)();

    // save resulting registers
    // asm("movl %%eax, %0;" : "=r" (eax) : );
    register int eax asm("eax");
    register int ebx asm("ebx");
    register int ecx asm("ecx");
    register int edx asm("edx");
    register int ebp asm("ebp");
    register int esp asm("esp");

    printf("eax %08x  ebx %08x  ecx %08x  edx %08x\n", eax, ebx, ecx, edx);


    printf("DONE\n");
}
