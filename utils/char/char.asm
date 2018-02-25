; writes a char on screen in graphics mode

org 0x100

section .text
start:
    mov ax,13h
    int 10h

    mov ah, 0xa         ; draw a character
    mov al, 'S'
    mov bh, 0           ; page
    mov bl, 1           ; attrib
    mov cx, 1           ; count
    int 0x10


    ; wait for any key and exit
    xor ah,ah
    int 16h
    ret
