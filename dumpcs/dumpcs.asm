org 0x100

section .text

    ; create file
    mov  ah, 3ch
    mov  cx, 0
    mov  dx, filename
    int  21h
    mov  [handle], ax

    ; write data
    mov  ah, 40h
    mov  bx, [handle]
    mov  cx, 0xFFFF   ; length
    mov  dx, 0        ; start
    int  21h

    ; close file
    mov  ah, 3eh
    mov  bx, [handle]
    int  21h

    ; exit
    mov  ax,4c00h
    int  21h


section .data
    filename db   "cs.bin",0

section .bss
    handle   resw 1
