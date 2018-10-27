use machine::Machine;
use debug::ProgramTracer;

#[test]
fn trace_simple() {
    let mut machine = Machine::default();
    machine.cpu.deterministic = true;
    let code: Vec<u8> = vec![
        0xBA, 0x04, 0x00,   // mov dx,0x4
        0x89, 0xD1,         // mov cx,dx
        0xEB, 0x00,         // jmp short 0x107
        0xC3,               // ret
    ];
    machine.load_executable(&code);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    assert_eq!("[085F:0100] BA0400           Mov16    dx, 0x0004
[085F:0103] 89D1             Mov16    cx, dx
[085F:0105] EB00             JmpShort 0x0107

[085F:0107] C3               Retn                                   ; xref: branch@085F:0105

", res);
}

#[test]
fn trace_unreferenced_data() {
    let mut machine = Machine::default();
    machine.cpu.deterministic = true;
    let code: Vec<u8> = vec![
        0xBA, 0x04, 0x00,   // mov dx,0x4
        0x89, 0xD1,         // mov cx,dx
        0xEB, 0x01,         // jmp short 0x108
        0x90,               // nop (unreferenced)
        0xC3,               // ret
        0x40,               // inc ax (unreferenceed)
    ];
    machine.load_executable(&code);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    println!("{}", res);
    assert_eq!("[085F:0100] BA0400           Mov16    dx, 0x0004
[085F:0103] 89D1             Mov16    cx, dx
[085F:0105] EB01             JmpShort 0x0108

[085F:0107] 90               db       0x90
[085F:0108] C3               Retn                                   ; xref: branch@085F:0105

[085F:0109] 40               db       0x40
", res);
}

#[test]
fn trace_decorates_stosw() {
    let mut machine = Machine::default();
    machine.cpu.deterministic = true;
    let code: Vec<u8> = vec![
        0xAB,           // stosw
        0xF3, 0xAB,     // rep stosw
        0xE4, 0x60,     // in al, 0x60
    ];
    machine.load_executable(&code);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    assert_eq!("[085F:0100] AB               Stosw                                  ; store ax at es:di
[085F:0101] F3AB             Rep      Stosw                         ; store ax at es:di for cx times
[085F:0103] E460             In8      al, 0x60                      ; keyboard or kb controller data output buffer
", res);
}

#[test]
fn trace_sepatate_call_destination_separators() {
    // makes sure newlines separate code blocks
    let mut machine = Machine::default();
    machine.cpu.deterministic = true;
    let code: Vec<u8> = vec![
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xE8, 0x05, 0x00,   // call 0x10b
        0xB8, 0x02, 0x00,   // mov ax,0x2
        0xEB, 0x04,         // jmp short 0x10f
        0xB8, 0x03, 0x00,   // mov ax,0x3
        0xC3,               // ret
        0xCD, 0x20,         // int 0x20
    ];
    machine.load_executable(&code);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    assert_eq!("[085F:0100] B80100           Mov16    ax, 0x0001
[085F:0103] E80500           CallNear 0x010B
[085F:0106] B80200           Mov16    ax, 0x0002
[085F:0109] EB04             JmpShort 0x010F

[085F:010B] B80300           Mov16    ax, 0x0003                    ; xref: call@085F:0103
[085F:010E] C3               Retn

[085F:010F] CD20             Int      0x20                          ; xref: branch@085F:0109
", res);
}

/*
; a way to manipulate ES from bmatch.com, should be able to figure that 010F is "es = 0x0040"
[085F:0105] 50               Push16   ax
[085F:0106] 55               Push16   bp
[085F:0107] 8BEC             Mov16    bp, sp
[085F:0109] C746024000       Mov16    word [ds:bp+0x02], 0x0040     ; manipulates the value that will be popped in ES
[085F:010E] 5D               Pop16    bp
[085F:010F] 07               Pop16    es                            ; es = 0x0040
*/


/*
[085F:0118] B100             Mov8     cl, 0x00          ; cl = 0x00
[085F:011A] BAC803           Mov16    dx, 0x03C8
[085F:011D] 8AC1             Mov8     al, cl            ; al = 0x00
[085F:011F] EE               Out8     dx, al            ; OUT 0x03C8, 0x00 ...
*/

/*
[085F:01EB] B80300           Mov16    ax, 0x0003
[085F:01EE] CD10             Int      0x10              ; video - 80x25 text mode (0x00, 0x03)
[085F:01F0] B44C             Mov8     ah, 0x4C
[085F:01F2] CD21             Int      0x21              ; DOS 2+ - EXIT - TERMINATE WITH RETURN CODE al   - XXX also stop parsing here, as if hit a RET
*/

/*
[085F:0118] B100             Mov8     cl, 0x00
[085F:011A] BAC803           Mov16    dx, 0x03C8                    ; xref: branch@085F:012D
[085F:011D] 8AC1             Mov8     al, cl
[085F:011F] EE               Out8     dx, al        ; set DAC write index to CL with write to 3c8
[085F:0120] 8AC1             Mov8     al, cl
[085F:0122] C0E802           Shr8     al, 0x02      ; al = cl >> 2, to scale 0..256 to 0..64
[085F:0125] 42               Inc16    dx            ; dx = 0x3C9
[085F:0126] EE               Out8     dx, al        ; set R value for DAC register
[085F:0127] EE               Out8     dx, al        ; G
[085F:0128] EE               Out8     dx, al        ; B
[085F:0129] 41               Inc16    cx
[085F:012A] 80F900           Cmp8     cl, 0x00
[085F:012D] 75EB             Jnz      0x011A        ; loop until cl wraps to 0 again (256 steps)
*/

/*
[085F:012F] 8CC8             Mov16    ax, cs
[085F:0131] 80C410           Add8     ah, 0x10
[085F:0134] 8EE0             Mov16    fs, ax    ; fs = cs + 0x1000

[085F:0136] 80C410           Add8     ah, 0x10
[085F:0139] 8EE8             Mov16    gs, ax
[085F:013B] 0FA8             Push16   gs
[085F:013D] 07               Pop16    es        ; es = cs + 0x2000

[085F:013E] B000             Mov8     al, 0x00
[085F:0140] B500             Mov8     ch, 0x00
[085F:0142] 49               Dec16    cx
[085F:0143] 33FF             Xor16    di, di
[085F:0145] F3AA             Rep      Stosb     ; dst is [cs + 0x2000:0]

[085F:0147] 0FA0             Push16   fs
[085F:0149] 07               Pop16    es
[085F:014A] 49               Dec16    cx
[085F:014B] 2BFF             Sub16    di, di
[085F:014D] F3AA             Rep      Stosb     ; dst is [cs + 0x1000:0]
*/

/*
[085F:0147] 33C0             Xor16    ax, ax
[085F:0149] 8BF0             Mov16    si, ax    ; si = 0
*/

/*
#[test]
fn trace_data_ref() {
    let mut machine = Machine::default();
    machine.cpu.deterministic = true;
    let code: Vec<u8> = vec![
        0xBA, 0x10, 0x01,       // mov dx,0x110
        0xB4, 0x09,             // mov ah,0x9
        0xCD, 0x21,             // int 0x21     ; print $-string at cs:dx XXX ?
        0x8B, 0x0E, 0x1D, 0x01, // mov cx,[0x11d]
        0xE9, 0x09, 0x00,       // jmp place
        0xB8, 0x00, 0x4C,       // mov ax,0x4c00        ; label exit
        0xCD, 0x21,             // int 0x21

        0x00,   // XXX ? alignment?
        0x68, 0x69, 0x24,       // db 'hi$'

        0xE9, 0xF4, 0xFF,       // jmp exit             ; label place

        0x04, 0x04, 0x04,       // db (unused)
        0x66, 0x66,             // dw
        0x00, 0x01, 0x02, 0x03, // db (unused)
        
    ];
    machine.load_executable(&code);

    let mut tracer = ProgramTracer::new();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    println!("{}", res);

    // FIXME [085F:0113] and [085F:0116] is not code!
    // XXX [085F:0118] and forward - why is this parsed???!
    assert_eq!("[085F:0100] BA1001           Mov16    dx, 0x0110
[085F:0103] B409             Mov8     ah, 0x09
[085F:0105] CD21             Int      0x21
[085F:0107] 8B0E1D01         Mov16    cx, word [ds:0x011D]
[085F:010B] E90900           JmpNear  0x0117
[085F:010E] B8004C           Mov16    ax, 0x4C00                    ; xref: 085F:0117
[085F:0111] CD21             Int      0x21
[085F:0113] 006869           Add8     byte [ds:bx+si+0x69], ch
[085F:0116] 24E9             And8     al, 0xE9


[085F:0117] E9F4FF           JmpNear  0x010E                        ; xref: 085F:010B

[085F:0118] F4               Hlt
[085F:0119] FF04             Inc16    word [ds:si]
[085F:011B] 0404             Add8     al, 0x04
[085F:011D] 66660001         Add8     byte [ds:bx+di], al
[085F:0121] 0203             Add8     al, byte [ds:bp+di]", res);
}
*/
