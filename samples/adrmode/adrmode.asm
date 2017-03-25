; for testing instruction decoding

    org  0x100        ; .com files always start 256 bytes into the segment


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
