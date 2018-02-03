section .text

; in: dx
print_dollar_dx:
    mov  ah, 0x09           ; DOS 1+ WRITE STRING TO STANDARD OUTPUT
    int  0x21
    ret

; in: ax
print_hex_at_ax:
;-----------------------
; convert the value in AX to hexadecimal ASCIIs
;-----------------------
    mov di, hexTemp         ; get the offset address
    mov cl, 4               ; number of ASCII
P1: rol ax, 4               ; 1 Nibble (start with highest byte)
    mov bl, al
    and bl, 0Fh             ; only low-Nibble
    add bl, 30h             ; convert to ASCII
    cmp bl, 39h             ; above 9?
    jna short P2
    add bl, 7               ; "A" to "F"
P2: mov [di], bl            ; store ASCII in buffer
    inc di                  ; increase target address
    dec cl                  ; decrease loop counter
    jnz P1                  ; jump if cl is not equal 0 (zeroflag is not set)
;-----------------------
; Print string
;-----------------------
    mov dx, hexTemp
    call print_dollar_dx
    ret

section .data
    hexTemp  db '0000',0xA,'$' ; buffer for ASCII string
