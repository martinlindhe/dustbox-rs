org 0x100

section .text

    mov ah, 0x3D    ; DOS 2+ - OPEN - OPEN EXISTING FILE
    mov al, 0;      ; mode
    push cs
    pop ds
    mov dx, fileDAT ; DS:DX -> ASCIZ filename
    int 0x21
    ; ret: AX = file handle


    mov bx, ax      ; file handle
    mov cx, 24 ; NUMBER OF BYTES TO READ
    ; DS:DX -> buffer for data
    mov dx, 0x400 ; point to after program code
    mov ah, 0x3F ; DOS 2+ - READ - READ FROM FILE OR DEVICE
    int 0x21
    ; ret: AX = bytes read


    mov ah, 0x3E    ; CLOSE - CLOSE FILE  (BX=handle)
    int 0x21


    int 0x20 ; EXIT TO DOS

section .data
    fileDAT     db 'FILE.DAT',0
