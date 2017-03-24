; for testing instruction decoding

    org  0x100        ; .com files always start 256 bytes into the segment


    mov es, ax ; 

    mov bx, 0x123;
    mov ss, bx ;

    ; mov [0x8080], bx ; [u16]
    ; mov [ax+0x4], cx
