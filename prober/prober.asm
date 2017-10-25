    org  0x100        ; .com files always start 256 bytes into the segment


section .data
    ; program data
    axIs     db 'ax = $'
    bxIs     db 'bx = $'
    cxIs     db 'cx = $'
    dxIs     db 'dx = $'
    bpIs     db 'bp = $'
    spIs     db 'sp = $'
    siIs     db 'si = $'
    diIs     db 'di = $'
    esIs     db 'es = $'
    csIs     db 'cs = $'
    ssIs     db 'ss = $'
    dsIs     db 'ds = $'
    fsIs     db 'fs = $'
    gsIs     db 'gs = $'
    flagsIs  db 'flg= $'
    hexTable db '0123456789ABCDEF', 0
    hexTemp  db "0000",0Dh,0Ah,"$" ; buffer for ASCII string
    _ax dw 0
    _bx dw 0
    _cx dw 0
    _dx dw 0
    _sp dw 0
    _bp dw 0
    _si dw 0
    _di dw 0
    _es dw 0
    _cs dw 0
    _ss dw 0
    _ds dw 0
    _fs dw 0
    _gs dw 0
    _flags dw 0

section .bss
    ; uninitialized data


section .text 
    ; program code
start:
    ; init registers
    mov ax, 0
    mov bx, 0
    mov cx, 0
    mov dx, 0
    mov bp, 0
    mov si, 0
    mov di, 0
    push word 0
    popf            ; clear flags


    ; ------------------
    ; run a instruction
    mov ah, 0xff
    sahf

    ; save reg states after instruction executes
    mov [_ax], ax
    mov [_bx], bx
    mov [_cx], cx
    mov [_dx], dx
    mov [_sp], sp
    mov [_bp], bp
    mov [_si], si
    mov [_di], di

    mov [_es], es
    mov [_cs], cs
    mov [_ss], ss
    mov [_ds], ds
    mov [_fs], fs
    mov [_gs], gs

    ; read FLAGS 16bit reg
    pushf
    pop ax
    mov [_flags], ax

    ; ax
    mov  dx, axIs
    call print_dollar_dx
    mov ax, [_ax]
    call print_hex_ax

    ; bx
    mov  dx, bxIs
    call print_dollar_dx
    mov ax, [_bx]
    call print_hex_ax

    ; cx
    mov  dx, cxIs
    call print_dollar_dx
    mov ax, [_cx]
    call print_hex_ax

    ; dx
    mov  dx, dxIs
    call print_dollar_dx
    mov ax, [_dx]
    call print_hex_ax

    ; bp
    mov  dx, bpIs
    call print_dollar_dx
    mov ax, [_bp]
    call print_hex_ax

    ; sp
    mov  dx, spIs
    call print_dollar_dx
    mov ax, [_sp]
    call print_hex_ax

    ; si
    mov  dx, siIs
    call print_dollar_dx
    mov ax, [_si]
    call print_hex_ax

    ; di
    mov  dx, diIs
    call print_dollar_dx
    mov ax, [_di]
    call print_hex_ax

    ; es
    mov  dx, esIs
    call print_dollar_dx
    mov ax, [_es]
    call print_hex_ax

    ; cs
    mov  dx, csIs
    call print_dollar_dx
    mov ax, [_cs]
    call print_hex_ax

    ; ss
    mov  dx, ssIs
    call print_dollar_dx
    mov ax, [_ss]
    call print_hex_ax

    ; ds
    mov  dx, dsIs
    call print_dollar_dx
    mov ax, [_ds]
    call print_hex_ax

    ; fs
    mov  dx, fsIs
    call print_dollar_dx
    mov ax, [_fs]
    call print_hex_ax

    ; gs
    mov  dx, gsIs
    call print_dollar_dx
    mov ax, [_gs]
    call print_hex_ax

    ; flags
    mov  dx, flagsIs
    call print_dollar_dx
    mov ax, [_flags]
    call print_hex_ax

    mov  ah, 0x4c       ; exit to dos
    int  0x21

; in: dx
print_dollar_dx:
    mov  ah, 0x09       ; call "print string" function
    int  0x21
    ret

; in: ax
print_hex_ax:
;-----------------------
; convert the value in EAX to hexadecimal ASCIIs
;-----------------------
    mov di, hexTemp ; get the offset address
    mov cl,4            ; number of ASCII
P1: rol ax,4           ; 1 Nibble (start with highest byte)
    mov bl,al
    and bl,0Fh          ; only low-Nibble
    add bl,30h          ; convert to ASCII
    cmp bl,39h          ; above 9?
    jna short P2
    add bl,7            ; "A" to "F"
P2: mov [di],bl         ; store ASCII in buffer
    inc di              ; increase target address
    dec cl              ; decrease loop counter
    jnz P1              ; jump if cl is not equal 0 (zeroflag is not set)
;-----------------------
; Print string
;-----------------------
    mov dx, hexTemp ; DOS 1+ WRITE STRING TO STANDARD OUTPUT
    mov ah,9            ; DS:DX->'$'-terminated string
    int 21h
    ret
