section .text

; prints hex
; params:
; ax = u16 value to print
; dx = pointer to prefixed $-string
prefixed_print_hex_u16:
    push ax
    call print_dollar_dx ; in = dx
    pop ax
    call print_hex_u16 ; in = ax
    ret

; prints hex
; params:
; ax = u16 value to print
; dx = pointer to prefixed $-string
prefixed_print_hex_u32:
    push ax
    call print_dollar_dx ; in = dx
    pop ax
    call print_hex_u32 ; in = ax
    ret


; in: dx: pointer to $-string
print_dollar_dx:
    mov  ah, 0x09           ; DOS 1+ WRITE STRING TO STANDARD OUTPUT
    int  0x21
    ret


; in: ax
print_hex_u16:
;-----------------------
; convert the value in AX to hexadecimal ASCIIs
;-----------------------
    mov di, hexTemp16       ; get the offset address
    mov cl, 4               ; number of ASCII
P1_16: rol ax, 4               ; 1 Nibble (start with highest byte)
    mov bl, al
    and bl, 0Fh             ; only low-Nibble
    add bl, 30h             ; convert to ASCII
    cmp bl, 39h             ; above 9?
    jna short P2_16
    add bl, 7               ; "A" to "F"
P2_16: mov [di], bl            ; store ASCII in buffer
    inc di                  ; increase target address
    dec cl                  ; decrease loop counter
    jnz P1_16                  ; jump if cl is not equal 0 (zeroflag is not set)
;-----------------------
; Print string
;-----------------------
    mov dx, hexTemp16
    call print_dollar_dx
    ret


; in: eax
print_hex_u32:
;-----------------------
; convert the value in AX to hexadecimal ASCIIs
;-----------------------
    mov di, hexTemp32       ; get the offset address
    mov cl, 8               ; number of ASCII
P1_32: rol eax, 4              ; 1 Nibble (start with highest byte)
    mov bl, al
    and bl, 0Fh             ; only low-Nibble
    add bl, 30h             ; convert to ASCII
    cmp bl, 39h             ; above 9?
    jna short P2_32
    add bl, 7               ; "A" to "F"
P2_32: mov [di], bl            ; store ASCII in buffer
    inc di                  ; increase target address
    dec cl                  ; decrease loop counter
    jnz P1_32                  ; jump if cl is not equal 0 (zeroflag is not set)
;-----------------------
; Print string
;-----------------------
    mov dx, hexTemp32
    call print_dollar_dx
    ret


section .data
    hexTemp16  db '0000',0xD,0xA,'$'     ; buffer for 16-bit hex string
    hexTemp32  db '00000000',0xD,0xA,'$' ; buffer for 32-bit hex string
