org 0x100

section .text
    ;;XXX impl DOS 2+ - ALLOCATE MEMORY. bx=FFFF
    mov ah, 0x48
    mov bx, 0x1
    int 0x21        ; DOS 2+ - ALLOCATE MEMORY

    ret

                ; Return:
                ; CF clear if successful
                ; AX = segment of allocated block
                ; CF set on error
                ; AX = error code (07h,08h) (see #01680 at AH=59h/BX=0000h)
                ; BX = size of largest available block
