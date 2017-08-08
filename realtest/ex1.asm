; for testing instruction decoding

org  0x100        ; .com files always start 256 bytes into the segment

    mov ax, 0x1313
    ret

