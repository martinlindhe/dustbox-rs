; for testing instruction decoding

    org  0x100        ; .com files always start 256 bytes into the segment


  ;  int 3 ; // breakpoint for dosbox debugger. run "BPINT 3" in dosbox debugger before running program


; rep movsw test
    soffs: lea si,[soffs]
    lea di,[0x200]
    mov cx, 5
    cld
    rep movsw  ; F3A4


; addressing mode
    mov bx, 0x200
    mov byte [bx+0x2c],0xff
    mov ax,[bx+0x2c]

    mov [bx+0xff2c],ax




; lea test
    lea sp,[0x1bf0]  ; 8D26F01B



; cmp flags and overflow stuff
    mov bx, 0
    mov di, bx
    cmp di, 0x2000  ; 81FF0020




    ; flags
    mov ah, 0xfe
    add ah, 0x2  ; overflow and zero should be set
    ; XXX The OF, SF, ZF, AF, CF, and PF flags are set according to the result.


;---
    mov di, 0x100
    add di,byte +0x3a
    add di,byte -0x3a

;---
    mov byte [0x1031],0x38  ; C606311038


    ; es segment prefix stuff
    mov bx, 0x1234
    mov es, bx
    mov ah, 0x88
    mov [es:di], ah
    mov al, [es:di]


;---
    mov bx, 0x1234
    mov es, bx
    mov [data0], es
    data0: dw 0x0000

; ---
mov bx, data1
mov ah, [bx]  ; should  give ah = 0x99

data1: db 0x99


; ---
mov dl, 0x13
mov al, dl

xor cx, ax
xor ax, cx

mov ax, 0x8888
mov ds, ax
push ds
pop es

        call l1
        db 0xBA, 0x0B, 0x01
        db 0xB4, 0x09
        l1:
        db 0xCD, 0x21
        call l1


; --
    mov cl, 0x99 ;

    mov ax, 0x444; 
    mov es, ax ; 

    mov bx, 0x123;
    mov ss, bx ;

    ; mov [0x8080], bx ; [u16]
    ; mov [ax+0x4], cx
