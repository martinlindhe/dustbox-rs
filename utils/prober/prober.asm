    org  0x100

section .data
    ; initialized data

section .bss
    ; uninitialized data


section .text
    ; program code
start:
    ; call clear_regs
    ; call clear_mem

    ; -------------------------
    ; run a snippet to analyse:
    ; -------------------------
call clear_flags




    mov si, 0x100;
    mov di, 0x200;
    mov cx, 4;



        mov ax,0xff0
        imul ax, ax, 0xf0


    call save_regs
    call print_regs


    mov  ah, 0x4c       ; exit to dos
    int  0x21

%include "regs.inc.asm"
%include "print.inc.asm"
