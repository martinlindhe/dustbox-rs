section .text


; TODO: make it print the intial value of SP when run
;; XXXX how to call this, and how to use result!?

hex_to_char:
    push ax
    push bx

    lea   bx, [TABLE]
    mov   ax, dx

    mov   ah, al            ;make al and ah equal so we can isolate each half of the byte
    shr   ah, 4             ;ah now has the high nibble
    and   al, 0x0F          ;al now has the low nibble
    xlat                    ;lookup al's contents in our table
    xchg  ah, al            ;flip around the bytes so now we can get the higher nibble 
    xlat                    ;look up what we just flipped

    lea   bx, [STRING]
    xchg  ah, al
    mov   [bx], ax          ;append the new character to the string of bytes

    pop bx
    pop ax
    ret



section .data
TABLE: db "0123456789ABCDEF", 0


section .bss

STRING: resb  50                ; reserve 50 bytes for the string
