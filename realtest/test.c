// execute a instruction, record resulting register values
// tested on macOS, Apple LLVM version 8.1.0 (clang-802.0.41)

#include <sys/mman.h>
#include <string.h>
#include <stdio.h>
#include <stdint.h>

char code[] = {
    // llvm, osx void func begin (4 bytes)
    0x55,                   // push   %rbp
    0x48, 0x89, 0xE5,       // mov    %rsp,%rbp

    // instruction to test
    // 0xB8, 0x13, 0x13, 0x00, 0x00,      // mov eax, 0x1313
    0x66, 0xB8, 0x13, 0x13, // mov ax, 0x1313    NOTE: in 64-bit mode we must use 0x66 prefix for 16bit op

    // llvm, osx void func end (2 bytes)
    0x5D,                   // pop    %rbp
    0xC3,                   // retq

    // pad to even 16-byte length
    0x90, 0x90, 0x90, 0x90, 0x90, 0x90, // nop
};

int main(int argc, char **argv) {
    // copy code to executable buffer
    void *buf = mmap(0, sizeof(code), PROT_READ|PROT_WRITE|PROT_EXEC,
              MAP_PRIVATE|MAP_ANON, -1, 0);
    memcpy(buf, code, sizeof(code));

    uint32_t eax, ebx, ecx, edx;
    uint64_t flags;

    // clear registers and flags
    asm("movl $0, %eax");
    asm("movl $0, %ebx");
    asm("movl $0, %ecx");
    asm("movl $0, %edx");
    //asm("pushq $0");
    //asm("popfq"); // XXX flags returned as 0000202 so seems to fail???

    // run code
//    ((void (*) (void))buf)();

    // read registers and flag
    asm("movl %%eax, %0" : "=r"(eax));
    asm("movl %%ebx, %0" : "=r"(ebx));
    asm("movl %%ecx, %0" : "=r"(ecx));
    asm("movl %%edx, %0" : "=r"(edx));
    asm("pushfq"); // push flags (32 bits)
    asm("pop %rax");
    asm("movq %%rax, %0" : "=r"(flags));  // XXX bug?.. generates "mov rax,rcx"

    //printf("eax %08x  ebx %08x  ecx %08x  edx %08x\n", eax, ebx, ecx, edx);
    //printf("flag %08llx\n", flags);
}
