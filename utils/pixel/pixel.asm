org 0x100

section .text
start:
    mov ax,13h
    int 10h

    ; draw a pixel with int 10 call
    mov ah, 0xc
    mov bh, 0 ; page 0
    mov al, 13 ; pixel color
    mov cx, 50; x
    mov dx, 50 ; y
    int 0x10


    ; wait for any key and exit
    xor ah,ah
    int 16h
    ret
