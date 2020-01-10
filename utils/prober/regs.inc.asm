section .text
save_regs:
    ; save reg states after instruction executes
    mov [_eax], eax
    mov [_ebx], ebx
    mov [_ecx], ecx
    mov [_edx], edx
    mov [_esp], esp
    mov [_ebp], ebp
    mov [_esi], esi
    mov [_edi], edi

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

print_flags:
    mov  dx, flagsIs
    call print_dollar_dx
    mov ax, [_flags]
    call print_hex_u16
    ret



print_regs:
    ; -----------
    ; 32 BIT REGS
    ; -----------
    mov dx, eaxIs
    mov eax, [_eax]
    call prefixed_print_hex_u32

    mov dx, ebxIs
    mov eax, [_ebx]
    call prefixed_print_hex_u32

    mov dx, ecxIs
    mov eax, [_ecx]
    call prefixed_print_hex_u32

    mov dx, edxIs
    mov eax, [_edx]
    call prefixed_print_hex_u32

    mov dx, ebpIs
    mov eax, [_ebp]
    call prefixed_print_hex_u32

    mov dx, espIs
    mov eax, [_esp]
    call prefixed_print_hex_u32

    mov dx, esiIs
    mov eax, [_esi]
    call prefixed_print_hex_u32

    mov dx, ediIs
    mov eax, [_edi]
    call prefixed_print_hex_u32

    ; -----------
    ; 16 BIT REGS
    ; -----------
    mov dx, esIs
    mov ax, [_es]
    call prefixed_print_hex_u16

    mov dx, csIs
    mov ax, [_cs]
    call prefixed_print_hex_u16

    mov dx, ssIs
    mov ax, [_ss]
    call prefixed_print_hex_u16

    mov dx, dsIs
    mov ax, [_ds]
    call prefixed_print_hex_u16

    mov dx, fsIs
    mov ax, [_fs]
    call prefixed_print_hex_u16

    mov dx, gsIs
    mov ax, [_gs]
    call prefixed_print_hex_u16

    call print_flags
    ret


section .data
    eaxIs    db 'eax=$'
    ebxIs    db 'ebx=$'
    ecxIs    db 'ecx=$'
    edxIs    db 'edx=$'
    ebpIs    db 'ebp=$'
    espIs    db 'esp=$'
    esiIs    db 'esi=$'
    ediIs    db 'edi=$'
    esIs     db 'es=$'
    csIs     db 'cs=$'
    ssIs     db 'ss=$'
    dsIs     db 'ds=$'
    fsIs     db 'fs=$'
    gsIs     db 'gs=$'
    flagsIs  db 'flag=$'
    _eax   dd 0
    _ebx   dd 0
    _ecx   dd 0
    _edx   dd 0
    _esp   dd 0
    _ebp   dd 0
    _esi   dd 0
    _edi   dd 0
    _es    dw 0
    _cs    dw 0
    _ss    dw 0
    _ds    dw 0
    _fs    dw 0
    _gs    dw 0
    _flags dw 0
