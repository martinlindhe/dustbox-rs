; for testing instruction decoding

    org  0x100        ; .com files always start 256 bytes into the segment



    ; es segment prefix stuff
    mov [es:di], ah  ; 268825
    mov ah, [es:di]  ; 268A25


;---
    mov bx, 0x1234
    mov es, bx
    mov [data0], es
    data0: dw 0x0000

; ---
mov bx, data1
mov ah, [bx]  ; should  give ah = 0x99

data1: db 0x99


; ---
mov dl, 0x13
mov al, dl

xor cx, ax
xor ax, cx

mov ax, 0x8888
mov ds, ax
push ds
pop es

        call l1
        db 0xBA, 0x0B, 0x01
        db 0xB4, 0x09
        l1:
        db 0xCD, 0x21
        call l1


; --
    mov cl, 0x99 ;

    mov ax, 0x444; 
    mov es, ax ; 

    mov bx, 0x123;
    mov ss, bx ;

    ; mov [0x8080], bx ; [u16]
    ; mov [ax+0x4], cx
