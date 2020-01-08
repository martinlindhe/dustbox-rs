use crate::machine::Machine;
use crate::debug::ProgramTracer;

use std::fmt;

// workaround for lack of pretty multiline diffs: https://github.com/colin-kiegel/rust-pretty-assertions/issues/24
/// Wrapper around string slice that makes debug output `{:?}` to print string same way as `{}`.
/// Used in different `assert*!` macros in combination with `pretty_assertions` crate to make
/// test failures to show nice diffs.
#[derive(PartialEq, Eq)]
#[doc(hidden)]
struct PrettyString<'a>(pub &'a str);

/// Make diff to display string as multi-line string
impl<'a> fmt::Debug for PrettyString<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(self.0)
  }
}

fn ass_eq(left: &str, right: &str) {
    assert_eq!(PrettyString(left), PrettyString(right));
}

#[test]
fn trace_simple() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBA, 0x04, 0x00,   // mov dx,0x4
        0x89, 0xD1,         // mov cx,dx
        0x8E, 0xC2,         // mov es,dx
        0xEB, 0x00,         // jmp short 0x107
        0xC3,               // ret
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] BA0400           Mov16    dx, 0x0004                    ; dx = 0x0004
[085F:0103] 89D1             Mov16    cx, dx                        ; cx = 0x0004
[085F:0105] 8EC2             Mov16    es, dx                        ; es = 0x0004
[085F:0107] EB00             JmpShort 0x0109

[085F:0109] C3               Retn                                   ; xref: jump@085F:0107

", &res);
}

#[test]
fn trace_unknown_byte() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xC3,               // ret
        0x00,
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] C3               Retn

[085F:0101] 00               db       0x00
", &res);
}

#[test]
fn trace_unknown_bytes() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xC3,               // ret
        0x00, 0x01, 0x02, 0x03,
        0x04, 0x05,
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] C3               Retn

[085F:0101] 00010203         db       0x00, 0x01, 0x02, 0x03
[085F:0105] 0405             db       0x04, 0x05
", &res);
}


#[test]
fn trace_unknown_bytes_fragmented() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x72, 0x06, // jc s_0108
        0xEB, 0x01, // jmp short s_0105
        0x03,       // db 3
        0xC3,       // s_0105: ret
        0x04, 0x05, // db 4, 5
        0xEB, 0xFB, // s_0108: jmp s_0105
        0x06,       // db 6
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] 7206             Jc       0x0108
[085F:0102] EB01             JmpShort 0x0105

[085F:0104] 03               db       0x03
[085F:0105] C3               Retn                                   ; xref: jump@085F:0102, jump@085F:0108

[085F:0106] 0405             db       0x04, 0x05
[085F:0108] EBFB             JmpShort 0x0105                        ; xref: branch@085F:0100

[085F:010A] 06               db       0x06
", &res);
}


#[test]
fn trace_unreferenced_data() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBA, 0x04, 0x00,   // mov dx,0x4
        0x89, 0xD1,         // mov cx,dx
        0xEB, 0x01,         // jmp short 0x108
        0x90,               // nop (unreferenced)
        0xC3,               // ret
        0x40,               // inc ax (unreferenceed)
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);

    ass_eq("[085F:0100] BA0400           Mov16    dx, 0x0004                    ; dx = 0x0004
[085F:0103] 89D1             Mov16    cx, dx                        ; cx = 0x0004
[085F:0105] EB01             JmpShort 0x0108

[085F:0107] 90               db       0x90
[085F:0108] C3               Retn                                   ; xref: jump@085F:0105

[085F:0109] 40               db       0x40
", &res);
}

#[test]
fn trace_annotates_stosw() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xAB,           // stosw
        0xF3, 0xAB,     // rep stosw
        0xE4, 0x60,     // in al, 0x60
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] AB               Stosw                                  ; [es:di] = ax
[085F:0101] F3AB             Rep      Stosw                         ; while cx-- > 0 { [es:di] = ax }
[085F:0103] E460             In8      al, 0x60                      ; keyboard: input buffer (0x0060)
", &res);
}

#[test]
fn trace_sepatate_call_destination_separators() {
    // this test makes sure newlines separate code blocks
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xE8, 0x05, 0x00,   // call 0x10b
        0xB8, 0x02, 0x00,   // mov ax,0x2
        0xEB, 0x04,         // jmp short 0x10f
        0xB8, 0x03, 0x00,   // mov ax,0x3
        0xC3,               // ret
        0xCD, 0x20,         // int 0x20
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] B80100           Mov16    ax, 0x0001                    ; ax = 0x0001
[085F:0103] E80500           CallNear 0x010B
[085F:0106] B80200           Mov16    ax, 0x0002                    ; ax = 0x0002
[085F:0109] EB04             JmpShort 0x010F

[085F:010B] B80300           Mov16    ax, 0x0003                    ; xref: call@085F:0103; ax = 0x0003
[085F:010E] C3               Retn

[085F:010F] CD20             Int      0x20                          ; xref: jump@085F:0109; dos: terminate program with return code 0 | dirty all regs
", &res);
}

#[test]
fn trace_virtual_memory() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x2E, 0xA3, 0x02, 0x02, // mov [cs:0x202],ax
        0x2E, 0xA1, 0x02, 0x02, // mov ax,[cs:0x202]
        0x2E, 0xA2, 0x05, 0x02, // mov [cs:0x205],al
        0x2E, 0xA0, 0x05, 0x02, // mov al,[cs:0x205]
        0xC3,                   // ret
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] 2EA30202         Mov16    word [cs:0x0202], ax
[085F:0104] 2EA10202         Mov16    ax, word [cs:0x0202]
[085F:0108] 2EA20502         Mov8     byte [cs:0x0205], al
[085F:010C] 2EA00502         Mov8     al, byte [cs:0x0205]
[085F:0110] C3               Retn

[085F:0202] ????             dw       ????                          ; xref: word@085F:0100, word@085F:0104
[085F:0205] ??               db       ??                            ; xref: byte@085F:0108, byte@085F:010C
", &res);
}

#[test]
fn trace_break_after_dos_int20() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xCD, 0x20, // int 0x20
        0x90,       // db 0x90
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] CD20             Int      0x20                          ; dos: terminate program with return code 0 | dirty all regs
[085F:0102] 90               db       0x90
", &res);
}

#[test]
fn trace_break_after_dos_int21_4c() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB4, 0x4C, // mov ah,0x4C
        0xCD, 0x21, // int 0x21
        0x90,       // db 0x90
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] B44C             Mov8     ah, 0x4C                      ; ah = 0x4C
[085F:0102] CD21             Int      0x21                          ; dos: terminate program with return code in AL | dirty all regs
[085F:0104] 90               db       0x90
", &res);
}

#[test]
fn trace_dont_annotate_dirty_regs() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x13, 0x00,   // mov ax,0x13
        0xCD, 0x10,         // int 0x10       ; makes all reg dirty
        0x89, 0xC3,         // mov bx,ax      ; ax is dirty
        0xB8, 0x12, 0x00,   // mov ax,0x12
        0x89, 0xC3,         // mov bx,ax      ; ax is is clean == 0x12
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] B81300           Mov16    ax, 0x0013                    ; ax = 0x0013
[085F:0103] CD10             Int      0x10                          ; video: set 320x200x256 mode (0x13) | dirty all regs
[085F:0105] 89C3             Mov16    bx, ax                        ; bx is dirty
[085F:0107] B81200           Mov16    ax, 0x0012                    ; ax = 0x0012
[085F:010A] 89C3             Mov16    bx, ax                        ; bx = 0x0012
", &res);
}

#[test]
fn trace_annotate_int() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x03, 0x00,       // mov ax,0x0013
        0xCD, 0x10,             // int 0x10
        0xB4, 0x4C,             // mov ah,0x4C
        0xCD, 0x21,             // int 0x21
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] B80300           Mov16    ax, 0x0003                    ; ax = 0x0003
[085F:0103] CD10             Int      0x10                          ; video: set 80x25x16 text mode (0x03) | dirty all regs
[085F:0105] B44C             Mov8     ah, 0x4C                      ; ah = 0x4C
[085F:0107] CD21             Int      0x21                          ; dos: terminate program with return code in AL | dirty all regs
", &res);
}

#[test]
fn trace_annotate_out() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x34, 0x12,   // mov ax,0x1234
        0xBA, 0xC8, 0x03,   // mov dx,0x03C8
        0xE6, 0x40,         // out 0x40,al
        0xE7, 0x40,         // out 0x40,ax
        0xEE,               // out dx,al
        0xEF,               // out dx,ax
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] B83412           Mov16    ax, 0x1234                    ; ax = 0x1234
[085F:0103] BAC803           Mov16    dx, 0x03C8                    ; dx = 0x03C8
[085F:0106] E640             Out8     0x40, al                      ; pit: counter 0, counter divisor (0x0040) = 34
[085F:0108] E740             Out16    0x40, ax                      ; pit: counter 0, counter divisor (0x0040) = 1234
[085F:010A] EE               Out8     dx, al                        ; vga: PEL address write mode (0x03C8) = 34
[085F:010B] EF               Out16    dx, ax                        ; vga: PEL address write mode (0x03C8) = 1234
", &res);
}

#[test]
fn trace_annotate_in() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBA, 0x60, 0x00,   // mov dx,0x0060
        0xEC,               // in al,dx
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] BA6000           Mov16    dx, 0x0060                    ; dx = 0x0060
[085F:0103] EC               In8      al, dx                        ; keyboard: input buffer (0x0060)
", &res);
}

#[test]
fn trace_annotate_regset() {
    // this test makes sure that register initializations are annotated
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x13, 0x00,       // mov ax,0x13
        0x89, 0xC2,             // mov dx,ax
        0x42,                   // inc dx
        0xFE, 0xC2,             // inc dl
        0x88, 0xD3,             // mov bl,dl
        0x4B,                   // dec bx
        0xFE, 0xCB,             // dec bl
        0x00, 0xD3,             // add bl,dl
        0x80, 0xC3, 0x05,       // add bl,0x5
        0x01, 0xC3,             // add bx,ax
        0x81, 0xC3, 0xF0, 0xFF, // add bx,0xFFF0
        0x83, 0xC3, 0x04,       // add bx,byte +0x4
        0x28, 0xD3,             // sub bl,dl
        0x80, 0xEB, 0x05,       // sub bl,0x5
        0x29, 0xC3,             // sub bx,ax
        0x81, 0xEB, 0x00, 0x60, // sub bx,0x6000
        0x83, 0xEB, 0x04,       // sub bx,byte +0x4
        0x31, 0xC0,             // xor ax,ax
        0x30, 0xDB,             // xor bl,bl
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);
    ass_eq("[085F:0100] B81300           Mov16    ax, 0x0013                    ; ax = 0x0013
[085F:0103] 89C2             Mov16    dx, ax                        ; dx = 0x0013
[085F:0105] 42               Inc16    dx                            ; dx = 0x0014
[085F:0106] FEC2             Inc8     dl                            ; dl = 0x15
[085F:0108] 88D3             Mov8     bl, dl                        ; bl = 0x15
[085F:010A] 4B               Dec16    bx                            ; bx = 0x0014
[085F:010B] FECB             Dec8     bl                            ; bl = 0x13
[085F:010D] 00D3             Add8     bl, dl                        ; bl = 0x28
[085F:010F] 80C305           Add8     bl, 0x05                      ; bl = 0x2D
[085F:0112] 01C3             Add16    bx, ax                        ; bx = 0x0040
[085F:0114] 81C3F0FF         Add16    bx, 0xFFF0                    ; bx = 0x0030
[085F:0118] 83C304           Add16    bx, byte +0x04                ; bx = 0x0034
[085F:011B] 28D3             Sub8     bl, dl                        ; bl = 0x1F
[085F:011D] 80EB05           Sub8     bl, 0x05                      ; bl = 0x1A
[085F:0120] 29C3             Sub16    bx, ax                        ; bx = 0x0007
[085F:0122] 81EB0060         Sub16    bx, 0x6000                    ; bx = 0xA007
[085F:0126] 83EB04           Sub16    bx, byte +0x04                ; bx = 0xA003
[085F:0129] 31C0             Xor16    ax, ax                        ; ax = 0x0000
[085F:012B] 30DB             Xor8     bl, bl                        ; bl = 0x00
", &res);
}

/*
; TODO detect pattern
; a way to manipulate ES from bmatch.com, should be able to figure that 010F is "es = 0x0040"
[085F:0105] 50               Push16   ax
[085F:0106] 55               Push16   bp
[085F:0107] 8BEC             Mov16    bp, sp
[085F:0109] C746024000       Mov16    word [ds:bp+0x02], 0x0040     ; manipulates the value that will be popped in ES
[085F:010E] 5D               Pop16    bp
[085F:010F] 07               Pop16    es                            ; es = 0x0040
*/

/*
; TODO better register tracing
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

#[test]
fn trace_data_ref() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBA, 0x14, 0x01,       // mov dx,string
        0xB4, 0x09,             // mov ah,0x9
        0xCD, 0x21,             // int 0x21     ; DS:DX -> '$'-terminated string
        0x8B, 0x0E, 0x1D, 0x01, // mov cx,[0x11d]
        0xE9, 0x09, 0x00,       // jmp place
        0xB8, 0x00, 0x4C,       // exit: mov ax,0x4c00
        0xCD, 0x21,             // int 0x21
        0x00,                   // db 0
        0x68, 0x69, 0x24,       // string: db 'hi$'
        0xE9, 0xF4, 0xFF,       // place: jmp exit
        0x04, 0x04, 0x04,       // db (unused)
        0x66, 0x66,             // dw
        0x00, 0x01, 0x02, 0x03, // db (unused)
        
    ];
    machine.load_executable(&code, 0x085F);

    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    let res = tracer.present_trace(&mut machine);

// XXX [085F:0114] 686924           Push16   0x2469                        ; xref: str$@085F:0105
// XXX strip trailing space

    ass_eq("[085F:0100] BA1401           Mov16    dx, 0x0114                    ; dx = 0x0114
[085F:0103] B409             Mov8     ah, 0x09                      ; ah = 0x09
[085F:0105] CD21             Int      0x21                          ; dos: write $-terminated string at DS:DX to stdout | dirty all regs
[085F:0107] 8B0E1D01         Mov16    cx, word [ds:0x011D]
[085F:010B] E90900           JmpNear  0x0117

[085F:010E] B8004C           Mov16    ax, 0x4C00                    ; xref: jump@085F:0117; ax = 0x4C00
[085F:0111] CD21             Int      0x21                          ; dos: terminate program with return code in AL | dirty all regs
[085F:0113] 00               db       0x00
[085F:0114] 686924           db       \'hi$\'                         ; xref: str$@085F:0105
[085F:0117] E9F4FF           JmpNear  0x010E                        ; xref: jump@085F:010B

[085F:011A] 04040466         db       0x04, 0x04, 0x04, 0x66
[085F:011E] 66000102         db       0x66, 0x00, 0x01, 0x02
[085F:0122] 03               db       0x03
", &res);
}


/*
// BUG:
games-com-commercial/Pente (1984)(Michael Leach)/pente.com

[085F:0CDD] 0BC0             Or16     ax, ax                        ; ax is dirty
[085F:0CDE] C0740653         INVALID C0byte [ds:si+0x06], 0x53      ; xref: branch@085F:0C7A				XXX since this offset is part of a instruction, do not track it separately
[085F:0CDF] 7406             Jz       0x0CE7
[085F:0CE1] 53               Push16   bx

00000CDD  0BC0              or ax,ax
00000CDF  7406              jz 0xce7
00000CE1  53                push bx

or16 takes up 1 byte in some sense but 2 bytes in op size ?+++

earlier:
[085F:0C79] 6F               Outsw
[085F:0C7A] 677261           Jc       0x0CDE
[085F:0C7D] 6D               Insw

the reason is this jump ref to middle of the or instr at [085F:0CDD]


same issue:
games-com-commercial/Snake Game (1992)(Freeware)/snake.com: ERROR: invalid/unhandled reg op INVALID C0byte [ds:si+0x06], 0x53
games-com-commercial/Gooku (1987)(Anonymous)/go-moku.com: ERROR: invalid/unhandled reg op INVALID C0byte [ds:si+0x06], 0x53
games-com-commercial/Yatzy (1984)(Jan Ivar Gundersen)/yatzy.com: ERROR: invalid/unhandled reg op INVALID C0byte [ds:si+0x06], 0x53
games-com-commercial/Gnafu (1986)(Anonymous)/gnafu.com: ERROR: invalid/unhandled reg op INVALID C0byte [ds:si+0x06], 0x53
games-com-commercial/Madball (1985)(Microtec)/madball.com: ERROR: invalid/unhandled reg op INVALID C0byte [ds:si+0x06], 0x53
games-com-commercial/Turbo Bridge (1985)(Anonymous)/tbridge.com: ERROR: invalid/unhandled reg op INVALID C0byte [ds:si+0x06], 0x53
*/

/*

BUG: demo-com-16bit/conf/org/conf.com
020E+ in listing is not in rom, so they should be marked as unknown bytes and not visited

[085F:01FF] 2400             And8     al, 0x00                      ; al is dirty
[085F:020E] 0000             Add8     byte [ds:bx+si], al           ; xref: branch@085F:01EC
[085F:0212] 0000             Add8     byte [ds:bx+si], al           ; xref: branch@085F:01F0
[085F:024A] 0000             Add8     byte [ds:bx+si], al           ; xref: branch@085F:01E3
[085F:0251] 0000             Add8     byte [ds:bx+si], al           ; xref: branch@085F:01EA
[085F:025A] 0000             Add8     byte [ds:bx+si], al           ; xref: branch@085F:01E5

leads to errors:
ERROR: breaking because we reached end of file at 085F:0201 (indicates incorrect parsing)
ERROR: breaking because we reached end of file at 085F:024C (indicates incorrect parsing)
ERROR: breaking because we reached end of file at 085F:025C (indicates incorrect parsing)
ERROR: breaking because we reached end of file at 085F:0253 (indicates incorrect parsing)
ERROR: breaking because we reached end of file at 085F:0210 (indicates incorrect parsing)
ERROR: breaking because we reached end of file at 085F:0214 (indicates incorrect parsing)
*/

