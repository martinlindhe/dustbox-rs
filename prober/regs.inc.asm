clear_regs:
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
    ret

print_regs:
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

    ret
