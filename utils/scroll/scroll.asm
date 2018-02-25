; scrolls screen in gfx mode

org 0x100

section .text
start:
    mov ax,13h
    int 10h

    mov ah, 0xc     ; draw a pixel
    mov bh, 0       ; page 0
    mov al, 13      ; pixel color
    mov cx, 50      ; x
    mov dx, 50      ; y
    int 0x10


    mov ah, 0x06 ; scroll up
    mov ch, 10 ; upper_y
    mov cl, 10 ; upper_x
    mov dh, 100 ; lower_y
    mov dl, 100 ; lower_x
    mov al, 5 ; lines
    mov bh, 1 ; attr
    int 0x10

    ; wait for any key and exit
    xor ah,ah
    int 16h
    ret
