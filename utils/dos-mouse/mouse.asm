org 0x100

; TODO: change pixel color based on mouse LEFT/RIGHT/MIDDLE pressed

section .text

    mov ax, 0x13
    int 0x10            ; 320x200x256 colors

    ; set up mouse resolution (default is 640x200)
    mov ax, 7
    mov cx, 0           ; min pos
    mov dx, 320         ; max pos
    int 33h             ; mouse width 0-320

    mov ax, 8
    mov cx, 0
    mov dx, 200
    int 33h             ; mouse height 0-200



    push 0xA000
    pop es              ; video segment

draw_mouse:
    mov ax, 0x03
    int 0x33            ; get mouse status, CX=X, DX=Y, BX=buttons

    mov word [xVal], cx
    mov word [yVal], dx
    mov word [buttons], bx

    mov ax, 320
    mul word [yVal]     ; ax = y base offset
    add ax, [xVal]      ; exact offset
    mov di, ax          ; es:di = video offset

    ; draw pixel
    mov dx, [buttons]
    inc dl
    ; with default vga palette, blue = no mouse button, green is left and turqouse is right
    mov byte [es:di], dl


    ; idle for a while
    mov bp, 10000
    delay:
    dec bp
    nop
    jnz delay


    jmp draw_mouse


    mov ax, 0x03
    int 0x10        ; text mode

    mov ah, 0x4c
    int 0x21        ; exit to dos

section .data

section .bss
    xVal    resw 1
    yVal    resw 1
    buttons resw 1
