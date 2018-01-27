org 0x100

section .text
start:
    mov ax,13h
    int 10h

    ; draw palette in 32x8 squares, each square 5x5 pixels big (so 160x40px)
    push 0a000h
    pop es
    xor di,di
    xor ax,ax   ; color
    mov cx,8    ; big rows (each having 32 5x5 squares)
bigRowLoop:
    mov bx,5    ; pixel height of single row
rowLoop:
    mov dx,32   ; squares per row
    push ax
    push di
squareLoop:
    ; draw 5 pixels with "ah:al" color, ++color, di += 5
    mov [es:di],ax
    mov [es:di+2],ax
    mov [es:di+4],al
    add ax,0101h
    add di,5
    dec dx
    jnz squareLoop
    pop di
    pop ax      ; restore color for first square
    add di,320  ; move di to start of next line
    dec bx      ; do next single pixel line
    jnz rowLoop

    ; one row of color squares is drawn, now next 32 colors
    add ax,02020h ; color += 32
    dec cx
    jnz bigRowLoop

    ; wait for any key and exit
    xor ah,ah
    int 16h
    ret
