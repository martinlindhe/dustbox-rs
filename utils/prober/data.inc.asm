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
    test1fail db 'test1 FAIL',0xD,0xA,'$'
    test2fail db 'test2 FAIL',0xD,0xA,'$'
    test3fail db 'test3 FAIL',0xD,0xA,'$'
    test4fail db 'test4 FAIL',0xD,0xA,'$'
    test5fail db 'test5 FAIL',0xD,0xA,'$'
    test6fail db 'test6 FAIL',0xD,0xA,'$'
    hexTable db '0123456789ABCDEF', 0
    hexTemp  db '0000',0xD,0xA,'$' ; buffer for ASCII string
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
