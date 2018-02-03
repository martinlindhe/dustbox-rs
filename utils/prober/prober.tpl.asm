    org  0x100

section .text
start:
    ; call clear_regs
    ; call clear_mem

    ; -------------------------
    ; run a snippet to analyse:
    ; -------------------------
    {{ snippet }}

    call save_regs
    call print_regs

    mov  ax, 0x4c00       ; exit to dos
    int  0x21

%include "regs.inc.asm"
%include "print.inc.asm"
