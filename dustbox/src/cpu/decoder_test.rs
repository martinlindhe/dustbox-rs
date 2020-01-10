use pretty_assertions::assert_eq;

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
    machine.load_executable(&code, 0x085F);

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
    machine.load_executable(&code, 0x085F);

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
    machine.load_executable(&code, 0x085F);

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
    machine.load_executable(&code, 0x085F);

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
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 4);
    assert_eq!("[085F:0100] 7404             Jz       0x0106
[085F:0102] 74FE             Jz       0x0102
[085F:0104] 7400             Jz       0x0106
[085F:0106] 74FA             Jz       0x0102",
               res);
}

#[test]
fn can_disassemble_xor16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x31, 0xCB,                 // xor bx,cx
        0x81, 0xF3, 0x55, 0x44,     // xor bx,0x4455
        0x35, 0x22, 0x11,           // xor ax,0x1122
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 31CB             Xor16    bx, cx
[085F:0102] 81F35544         Xor16    bx, 0x4455
[085F:0106] 352211           Xor16    ax, 0x1122", res);
}

#[test]
fn can_disassemble_xor32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0x31, 0xCB,                           // xor ebx,ecx
        0x66, 0x81, 0xF3, 0x11, 0x22, 0x55, 0x44,   // xor ebx,0x44552211
        0x66, 0x35, 0xAA, 0xDD, 0xEE, 0xFF,         // xor eax,0xffeeddaa
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 6631CB           Xor32    ebx, ecx
[085F:0103] 6681F311225544   Xor32    ebx, 0x44552211
[085F:010A] 6635AADDEEFF     Xor32    eax, 0xFFEEDDAA", res);
}

#[test]
fn can_disassemble_or32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0x09, 0xCB,                           // or ebx,ecx
        0x66, 0x81, 0xCB, 0x11, 0x22, 0x55, 0x44,   // or ebx,0x44552211
        0x66, 0x0D, 0xAA, 0xDD, 0xEE, 0xFF,         // or eax,0xffeeddaa
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 6609CB           Or32     ebx, ecx
[085F:0103] 6681CB11225544   Or32     ebx, 0x44552211
[085F:010A] 660DAADDEEFF     Or32     eax, 0xFFEEDDAA", res);
}

#[test]
fn can_disassemble_adc32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0x11, 0xCB,                           // adc ebx,ecx
        0x66, 0x81, 0xD3, 0x11, 0x22, 0x55, 0x44,   // adc ebx,0x44552211
        0x66, 0x15, 0xAA, 0xDD, 0xEE, 0xFF,         // adc eax,0xffeeddaa
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 6611CB           Adc32    ebx, ecx
[085F:0103] 6681D311225544   Adc32    ebx, 0x44552211
[085F:010A] 6615AADDEEFF     Adc32    eax, 0xFFEEDDAA", res);
}

#[test]
fn can_disassemble_add32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0x01, 0xCB,                           // add ebx,ecx
        0x66, 0x81, 0xC3, 0x11, 0x22, 0x55, 0x44,   // add ebx,0x44552211
        0x66, 0x05, 0xAA, 0xDD, 0xEE, 0xFF,         // add eax,0xffeeddaa
    ];  

    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 6601CB           Add32    ebx, ecx
[085F:0103] 6681C311225544   Add32    ebx, 0x44552211
[085F:010A] 6605AADDEEFF     Add32    eax, 0xFFEEDDAA", res);
}

#[test]
fn can_disassemble_sub32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0x29, 0xCB,                           // sub ebx,ecx
        0x66, 0x81, 0xEB, 0x11, 0x22, 0x55, 0x44,   // sub ebx,0x44552211
        0x66, 0x2D, 0xAA, 0xDD, 0xEE, 0xFF,         // sub eax,0xffeeddaa
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 6629CB           Sub32    ebx, ecx
[085F:0103] 6681EB11225544   Sub32    ebx, 0x44552211
[085F:010A] 662DAADDEEFF     Sub32    eax, 0xFFEEDDAA", res);
}

#[test]
fn can_disassemble_sbb32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0x19, 0xCB,                           // sbb ebx,ecx
        0x66, 0x81, 0xDB, 0x11, 0x22, 0x55, 0x44,   // sbb ebx,0x44552211
        0x66, 0x1D, 0xAA, 0xDD, 0xEE, 0xFF,         // sbb eax,0xffeeddaa
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 6619CB           Sbb32    ebx, ecx
[085F:0103] 6681DB11225544   Sbb32    ebx, 0x44552211
[085F:010A] 661DAADDEEFF     Sbb32    eax, 0xFFEEDDAA", res);
}

#[test]
fn can_disassemble_and32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0x21, 0xCB,                           // and ebx,ecx
        0x66, 0x81, 0xE3, 0x11, 0x22, 0x55, 0x44,   // and ebx,0x44552211
        0x66, 0x25, 0xAA, 0xDD, 0xEE, 0xFF,         // and eax,0xffeeddaa
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 6621CB           And32    ebx, ecx
[085F:0103] 6681E311225544   And32    ebx, 0x44552211
[085F:010A] 6625AADDEEFF     And32    eax, 0xFFEEDDAA", res);
}

#[test]
fn can_disassemble_cmp32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0x39, 0xCB,                           // cmp ebx,ecx
        0x66, 0x81, 0xFB, 0x11, 0x22, 0x55, 0x44,   // cmp ebx,0x44552211
        0x66, 0x3D, 0xAA, 0xDD, 0xEE, 0xFF,         // cmp eax,0xffeeddaa
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 6639CB           Cmp32    ebx, ecx
[085F:0103] 6681FB11225544   Cmp32    ebx, 0x44552211
[085F:010A] 663DAADDEEFF     Cmp32    eax, 0xFFEEDDAA", res);
}

#[test]
fn can_disassemble_test16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x85, 0xCB,                             // test bx,cx
        0xF7, 0xC3, 0x55, 0x44,                 // test bx,0x4455
        0xA9, 0xEE, 0xFF,                       // test ax,0xffee
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 85CB             Test16   bx, cx
[085F:0102] F7C35544         Test16   bx, 0x4455
[085F:0106] A9EEFF           Test16   ax, 0xFFEE", res);
}

#[test]
fn can_disassemble_test32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0x85, 0xCB,                           // test ebx,ecx
        0x66, 0xF7, 0xC3, 0x11, 0x22, 0x55, 0x44,   // test ebx,0x44552211
        0x66, 0xA9, 0xAA, 0xDD, 0xEE, 0xFF,         // test eax,0xffeeddaa
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 6685CB           Test32   ebx, ecx
[085F:0103] 66F7C311225544   Test32   ebx, 0x44552211
[085F:010A] 66A9AADDEEFF     Test32   eax, 0xFFEEDDAA", res);
}

#[test]
fn can_disassemble_not() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xF7, 0xD3,                             // not bx
        0x66, 0xF7, 0xD3,                       // not ebx
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] F7D3             Not16    bx
[085F:0102] 66F7D3           Not32    ebx", res);
}

#[test]
fn can_disassemble_neg() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xF7, 0xDB,                             // neg bx
        0x66, 0xF7, 0xDB,                       // neg ebx
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] F7DB             Neg16    bx
[085F:0102] 66F7DB           Neg32    ebx", res);
}

#[test]
fn can_disassemble_mul() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xF7, 0xE3,                             // mul bx
        0x66, 0xF7, 0xE3,                       // mul ebx
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] F7E3             Mul16    bx
[085F:0102] 66F7E3           Mul32    ebx", res);
}

#[test]
fn can_disassemble_imul() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xF7, 0xEB,                                 // imul bx
        0x66, 0xF7, 0xEB,                           // imul ebx
        0x0F, 0xAF, 0xDA,                           // imul bx,dx
        0x66, 0x0F, 0xAF, 0xDA,                     // imul ebx,edx
        0x6B, 0xDA, 0x20,                           // imul bx,dx,byte +0x20
        0x66, 0x6B, 0xDA, 0x20,                     // imul ebx,edx,byte +0x20
        0x69, 0xDA, 0x22, 0x44,                     // imul bx,dx,word 0x4422
        0x66, 0x69, 0xDA, 0x88, 0x66, 0x34, 0x12,   // imul ebx,edx,dword 0x12346688
    ];

    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] F7EB             Imul16   bx
[085F:0102] 66F7EB           Imul32   ebx
[085F:0105] 0FAFDA           Imul16   bx, dx
[085F:0108] 660FAFDA         Imul32   ebx, edx
[085F:010C] 6BDA20           Imul16   bx, dx, byte +0x20
[085F:010F] 666BDA20         Imul32   ebx, edx, byte +0x20
[085F:0113] 69DA2244         Imul16   bx, dx, 0x4422
[085F:0117] 6669DA88663412   Imul32   ebx, edx, 0x12346688", res);
}

#[test]
fn can_disassemble_div() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xF7, 0xF3,                             // div bx
        0x66, 0xF7, 0xF3,                       // div ebx
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] F7F3             Div16    bx
[085F:0102] 66F7F3           Div32    ebx", res);
}

#[test]
fn can_disassemble_idiv() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xF7, 0xFB,                             // idiv bx
        0x66, 0xF7, 0xFB,                       // idiv ebx
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] F7FB             Idiv16   bx
[085F:0102] 66F7FB           Idiv32   ebx", res);
}
