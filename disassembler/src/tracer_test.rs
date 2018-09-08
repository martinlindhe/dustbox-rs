use dustbox::machine::Machine;

use tracer;


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

    let mut tracer = tracer::Tracer::new();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    assert_eq!("[085F:0100] BA0400           Mov16    dx, 0x0004
[085F:0103] 89D1             Mov16    cx, dx
[085F:0105] EB00             JmpShort 0x0107
[085F:0107] C3               Retn                                   ; xref: 085F:0105
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

    let mut tracer = tracer::Tracer::new();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    println!("{}", res);
    assert_eq!("[085F:0100] BA0400           Mov16    dx, 0x0004
[085F:0103] 89D1             Mov16    cx, dx
[085F:0105] EB01             JmpShort 0x0108
[085F:0107] 90               db       0x90
[085F:0108] C3               Retn                                   ; xref: 085F:0105
[085F:0109] 40               db       0x40
", res);
}

/*
#[test]
fn trace_data_ref() {
    let mut machine = Machine::default();
    machine.cpu.deterministic = true;
    let code: Vec<u8> = vec![
        0xBA, 0x10, 0x01,       // mov dx,0x110
        0xB4, 0x09,             // mov ah,0x9
        0xCD, 0x21,             // int 0x21
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

    let mut tracer = tracer::Tracer::new();
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
