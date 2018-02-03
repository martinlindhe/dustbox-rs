section .text
save_regs:
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
    ret

;clear_regs:
;    ; init registers
;    mov ax, 0
;    mov bx, 0
;    mov cx, 0
;    mov dx, 0
;    mov bp, 0
;    mov si, 0
;    mov di, 0
;    ret

;clear_flags:
;    push ax
;    popf                    ; clear flags
;    ret

;clear_mem:
;    ; writes 0xFF to CS:0400...CS:FFFF
;    mov di, 0x0400
;    mov cx, 0x8000
;    mov al, 0xff
;    rep stosb
;    ret

print_ax:
    mov  dx, axIs
    call print_dollar_dx
    mov ax, [_ax]
    call print_hex_at_ax
    ret

print_bx:
    mov  dx, bxIs
    call print_dollar_dx
    mov ax, [_bx]
    call print_hex_at_ax
    ret

print_cx:
    mov  dx, cxIs
    call print_dollar_dx
    mov ax, [_cx]
    call print_hex_at_ax
    ret

print_dx:
    mov  dx, dxIs
    call print_dollar_dx
    mov ax, [_dx]
    call print_hex_at_ax
    ret

print_bp:
    mov  dx, bpIs
    call print_dollar_dx
    mov ax, [_bp]
    call print_hex_at_ax
    ret

print_sp:
    mov  dx, spIs
    call print_dollar_dx
    mov ax, [_sp]
    call print_hex_at_ax
    ret

print_si:
    mov  dx, siIs
    call print_dollar_dx
    mov ax, [_si]
    call print_hex_at_ax
    ret

print_di:
    mov  dx, diIs
    call print_dollar_dx
    mov ax, [_di]
    call print_hex_at_ax
    ret

print_es:
    mov  dx, esIs
    call print_dollar_dx
    mov ax, [_es]
    call print_hex_at_ax
    ret

print_cs:
    mov  dx, csIs
    call print_dollar_dx
    mov ax, [_cs]
    call print_hex_at_ax
    ret

print_ss:
    mov  dx, ssIs
    call print_dollar_dx
    mov ax, [_ss]
    call print_hex_at_ax
    ret

print_ds:
    mov  dx, dsIs
    call print_dollar_dx
    mov ax, [_ds]
    call print_hex_at_ax
    ret

print_fs:
    mov  dx, fsIs
    call print_dollar_dx
    mov ax, [_fs]
    call print_hex_at_ax
    ret

print_gs:
    mov  dx, gsIs
    call print_dollar_dx
    mov ax, [_gs]
    call print_hex_at_ax
    ret

print_flags:
    mov  dx, flagsIs
    call print_dollar_dx
    mov ax, [_flags]
    call print_hex_at_ax
    ret

print_regs:
    call print_ax
    call print_bx
    call print_cx
    call print_dx
    call print_bp
    call print_sp
    call print_si
    call print_di

    call print_es
    call print_cs
    call print_ss
    call print_ds
    call print_fs
    call print_gs

    call print_flags
    ret


section .data
    axIs     db 'ax=$'
    bxIs     db 'bx=$'
    cxIs     db 'cx=$'
    dxIs     db 'dx=$'
    bpIs     db 'bp=$'
    spIs     db 'sp=$'
    siIs     db 'si=$'
    diIs     db 'di=$'
    esIs     db 'es=$'
    csIs     db 'cs=$'
    ssIs     db 'ss=$'
    dsIs     db 'ds=$'
    fsIs     db 'fs=$'
    gsIs     db 'gs=$'
    flagsIs  db 'flag=$'
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
