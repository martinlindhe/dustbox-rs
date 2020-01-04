use crate::machine::Machine;

#[test]
fn can_disassemble_basic() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xE8, 0x05, 0x00, // call l_0x108   ; call a later offset
        0xBA, 0x0B, 0x01, // mov dx,0x10b
        0xB4, 0x09,       // mov ah,0x9
        0xCD, 0x21,       // l_0x108: int 0x21
        0xE8, 0xFB, 0xFF, // call l_0x108   ; call an earlier offset
        0xFF, 0x18,       // call far [bx+si]
    ];
    machine.load_executable(&code);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 6);
    assert_eq!("[085F:0100] E80500           CallNear 0x0108
[085F:0103] BA0B01           Mov16    dx, 0x010B
[085F:0106] B409             Mov8     ah, 0x09
[085F:0108] CD21             Int      0x21
[085F:010A] E8FBFF           CallNear 0x0108
[085F:010D] FF18             CallFar  word [ds:bx+si]",
               res);
}

#[test]
fn can_disassemble_lea() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x8D, 0x47, 0x80, // lea ax,[bx-0x80]
 ];
    machine.load_executable(&code);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 1);
    assert_eq!("[085F:0100] 8D4780           Lea16    ax, word [ds:bx-0x80]",
               res);
}

#[test]
fn can_disassemble_segment_prefixed() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x26, 0x88, 0x25, // mov [es:di],ah
        0x26, 0x8A, 0x25, // mov ah,[es:di]
    ];
    machine.load_executable(&code);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] 268825           Mov8     byte [es:di], ah
[085F:0103] 268A25           Mov8     ah, byte [es:di]",
               res);
}

#[test]
fn can_disassemble_values() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x81, 0xC7, 0xC0, 0x00,       // add di,0xc0
        0x83, 0xC7, 0x3A,             // add di,byte +0x3a
        0x83, 0xC7, 0xC6,             // add di,byte -0x3a
    ];
    machine.load_executable(&code);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 4);
    assert_eq!("[085F:0100] 803E311000       Cmp8     byte [ds:0x1031], 0x00
[085F:0105] 81C7C000         Add16    di, 0x00C0
[085F:0109] 83C73A           Add16    di, byte +0x3A
[085F:010C] 83C7C6           Add16    di, byte -0x3A",
               res);
}

#[test]
fn can_disassemble_relative_short_jumps() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x74, 0x04, // jz 0x106
        0x74, 0xFE, // jz 0x102
        0x74, 0x00, // jz 0x106
        0x74, 0xFA, // jz 0x102
    ];
    machine.load_executable(&code);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 4);
    assert_eq!("[085F:0100] 7404             Jz       0x0106
[085F:0102] 74FE             Jz       0x0102
[085F:0104] 7400             Jz       0x0106
[085F:0106] 74FA             Jz       0x0102",
               res);
}
