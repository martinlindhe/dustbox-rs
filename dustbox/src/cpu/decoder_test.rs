use pretty_assertions::assert_eq;

use crate::machine::Machine;

#[test]
fn can_disassemble_addressing_mod0_noprefix() {
    // tests all forms of mod=0 addressing
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x89, 0x08,                     // mov [bx+si],cx
        0x89, 0x09,                     // mov [bx+di],cx
        0x89, 0x0A,                     // mov [bp+si],cx           XXX BUG default segment is ss
        0x89, 0x0B,                     // mov [bp+di],cx           XXX BUG default segment is ss
        0x89, 0x0C,                     // mov [si],cx
        0x89, 0x0D,                     // mov [di],cx
        0x89, 0x0E, 0x60, 0x80,         // mov [0x8060],cx                     XXX BUG default segment is ss
        0x89, 0x0F,                     // mov [bx],cx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] 8908             Mov16    word [ds:bx+si], cx
[085F:0102] 8909             Mov16    word [ds:bx+di], cx
[085F:0104] 890A             Mov16    word [ds:bp+si], cx
[085F:0106] 890B             Mov16    word [ds:bp+di], cx
[085F:0108] 890C             Mov16    word [ds:si], cx
[085F:010A] 890D             Mov16    word [ds:di], cx
[085F:010C] 890E6080         Mov16    word [ds:0x8060], cx
[085F:0110] 890F             Mov16    word [ds:bx], cx", res);
}

#[test]
fn can_disassemble_addressing_mod0_opsize() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // operand-size override
        0x66, 0x89, 0x08,               // mov [bx+si],ecx
        0x66, 0x89, 0x09,               // mov [bx+di],ecx
        0x66, 0x89, 0x0A,               // mov [bp+si],ecx
        0x66, 0x89, 0x0B,               // mov [bp+di],ecx
        0x66, 0x89, 0x0C,               // mov [si],ecx
        0x66, 0x89, 0x0D,               // mov [di],ecx
        0x66, 0x89, 0x0E, 0x60, 0x80,   // mov [0x8060],ecx
        0x66, 0x89, 0x0F,               // mov [bx],ecx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] 668908           Mov32    dword [ds:bx+si], ecx
[085F:0103] 668909           Mov32    dword [ds:bx+di], ecx
[085F:0106] 66890A           Mov32    dword [ds:bp+si], ecx
[085F:0109] 66890B           Mov32    dword [ds:bp+di], ecx
[085F:010C] 66890C           Mov32    dword [ds:si], ecx
[085F:010F] 66890D           Mov32    dword [ds:di], ecx
[085F:0112] 66890E6080       Mov32    dword [ds:0x8060], ecx
[085F:0117] 66890F           Mov32    dword [ds:bx], ecx", res);
}

#[test]
fn can_disassemble_addressing_mod0_adrsize() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // adress-size override
        0x67, 0x89, 0x08,                           // mov [eax],cx
        0x67, 0x89, 0x09,                           // mov [ecx],cx
        0x67, 0x89, 0x0A,                           // mov [edx],cx
        0x67, 0x89, 0x0B,                           // mov [ebx],cx
        0x67, 0x89, 0x0C, 0x00,                     // mov [dword eax+eax],cx     SIB encoding
        0x67, 0x89, 0x0D, 0x04, 0x03, 0x02, 0x01,   // mov [dword 0x1020304],cx
        0x67, 0x89, 0x0E,                           // mov [esi],cx
        0x67, 0x89, 0x0F,                           // mov [edi],cx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] 678908           Mov16    word [ds:eax], cx
[085F:0103] 678909           Mov16    word [ds:ecx], cx
[085F:0106] 67890A           Mov16    word [ds:edx], cx
[085F:0109] 67890B           Mov16    word [ds:ebx], cx
[085F:010C] 67890C00         Mov16    word [ds:eax+eax], cx
[085F:0110] 67890D04030201   Mov16    dword [ds:0x1020304], cx
[085F:0117] 67890E           Mov16    word [ds:esi], cx
[085F:011A] 67890F           Mov16    word [ds:edi], cx", res);
}

#[test]
fn can_disassemble_addressing_mod0_sib_scale1() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // adress-size override
        0x67, 0x89, 0x0C, 0x00, // mov [dword eax+eax],cx
        0x67, 0x89, 0x0C, 0x08, // mov [dword eax+ecx],cx
        0x67, 0x89, 0x0C, 0x10, // mov [dword eax+edx],cx
        0x67, 0x89, 0x0C, 0x18, // mov [dword eax+ebx],cx
        // invalid
        0x67, 0x89, 0x0C, 0x28, // mov [dword eax+ebp],cx
        0x67, 0x89, 0x0C, 0x30, // mov [dword eax+esi],cx
        0x67, 0x89, 0x0C, 0x38, // mov [dword eax+edi],cx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 7);
    assert_eq!("[085F:0100] 67890C00         Mov16    word [ds:eax+eax], cx
[085F:0104] 67890C08         Mov16    word [ds:eax+ecx], cx
[085F:0108] 67890C10         Mov16    word [ds:eax+edx], cx
[085F:010C] 67890C18         Mov16    word [ds:eax+ebx], cx
[085F:0110] 67890C28         Mov16    word [ds:eax+ebp], cx
[085F:0114] 67890C30         Mov16    word [ds:eax+esi], cx
[085F:0118] 67890C38         Mov16    word [ds:eax+edi], cx", res);
}

#[test]
fn can_disassemble_addressing_mod0_sib_scale2() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // adress-size override
        0x67, 0x89, 0x0C, 0x40, // mov [dword eax+eax*2],cx
        0x67, 0x89, 0x0C, 0x48, // mov [dword eax+ecx*2],cx
        0x67, 0x89, 0x0C, 0x50, // mov [dword eax+edx*2],cx
        0x67, 0x89, 0x0C, 0x58, // mov [dword eax+ebx*2],cx
        // invalid
        0x67, 0x89, 0x0C, 0x68, // mov [dword eax+ebp*2],cx
        0x67, 0x89, 0x0C, 0x70, // mov [dword eax+esi*2],cx
        0x67, 0x89, 0x0C, 0x78, // mov [dword eax+edi*2],cx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 7);
    assert_eq!("[085F:0100] 67890C40         Mov16    word [ds:eax+eax*2], cx
[085F:0104] 67890C48         Mov16    word [ds:eax+ecx*2], cx
[085F:0108] 67890C50         Mov16    word [ds:eax+edx*2], cx
[085F:010C] 67890C58         Mov16    word [ds:eax+ebx*2], cx
[085F:0110] 67890C68         Mov16    word [ds:eax+ebp*2], cx
[085F:0114] 67890C70         Mov16    word [ds:eax+esi*2], cx
[085F:0118] 67890C78         Mov16    word [ds:eax+edi*2], cx", res);
}

#[test]
fn can_disassemble_addressing_mod0_sib_scale4() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // adress-size override
        0x67, 0x89, 0x0C, 0x80, // mov [dword eax+eax*4],cx
        0x67, 0x89, 0x0C, 0x88, // mov [dword eax+ecx*4],cx
        0x67, 0x89, 0x0C, 0x90, // mov [dword eax+edx*4],cx
        0x67, 0x89, 0x0C, 0x98, // mov [dword eax+ebx*4],cx
        // invalid
        0x67, 0x89, 0x0C, 0xA8, // mov [dword eax+ebp*4],cx
        0x67, 0x89, 0x0C, 0xB0, // mov [dword eax+esi*4],cx
        0x67, 0x89, 0x0C, 0xB8, // mov [dword eax+edi*4],cx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 7);
    assert_eq!("[085F:0100] 67890C80         Mov16    word [ds:eax+eax*4], cx
[085F:0104] 67890C88         Mov16    word [ds:eax+ecx*4], cx
[085F:0108] 67890C90         Mov16    word [ds:eax+edx*4], cx
[085F:010C] 67890C98         Mov16    word [ds:eax+ebx*4], cx
[085F:0110] 67890CA8         Mov16    word [ds:eax+ebp*4], cx
[085F:0114] 67890CB0         Mov16    word [ds:eax+esi*4], cx
[085F:0118] 67890CB8         Mov16    word [ds:eax+edi*4], cx", res);
}

#[test]
fn can_disassemble_addressing_mod0_sib_scale8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // adress-size override
        0x67, 0x89, 0x0C, 0xC0, // mov [dword eax+eax*8],cx
        0x67, 0x89, 0x0C, 0xC8, // mov [dword eax+ecx*8],cx
        0x67, 0x89, 0x0C, 0xD0, // mov [dword eax+edx*8],cx
        0x67, 0x89, 0x0C, 0xD8, // mov [dword eax+ebx*8],cx
        // invalid
        0x67, 0x89, 0x0C, 0xE8, // mov [dword eax+ebp*8],cx
        0x67, 0x89, 0x0C, 0xF0, // mov [dword eax+esi*8],cx
        0x67, 0x89, 0x0C, 0xF8, // mov [dword eax+edi*8],cx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 7);
    assert_eq!("[085F:0100] 67890CC0         Mov16    word [ds:eax+eax*8], cx
[085F:0104] 67890CC8         Mov16    word [ds:eax+ecx*8], cx
[085F:0108] 67890CD0         Mov16    word [ds:eax+edx*8], cx
[085F:010C] 67890CD8         Mov16    word [ds:eax+ebx*8], cx
[085F:0110] 67890CE8         Mov16    word [ds:eax+ebp*8], cx
[085F:0114] 67890CF0         Mov16    word [ds:eax+esi*8], cx
[085F:0118] 67890CF8         Mov16    word [ds:eax+edi*8], cx", res);
}

#[test]
fn can_disassemble_addressing_mod0_sib_xxx() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // adress-size override
        // sib displacement encoding
        0x67, 0xC7, 0x04, 0x85, 0x00, 0x00, 0x00, 0x00, 0x4B, 0xD5, // mov word [dword eax*4+0x0],0xd54b
        0x67, 0xC7, 0x04, 0x85, 0xC0, 0xFF, 0xFF, 0xFF, 0x4B, 0xD5, // mov word [dword eax*4-0x40],0xd54b
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] 67C70485000000004BD5 Mov16    word [ds:+eax*4+0x00000000], 0xD54B
[085F:010A] 67C70485C0FFFFFF4BD5 Mov16    word [ds:+eax*4-0x00000040], 0xD54B", res);
}

#[test]
fn can_disassemble_addressing_mod1_noprefix() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x89, 0x48, 0xF6,       // mov [bx+si-0xa],cx
        0x89, 0x49, 0xF6,       // mov [bx+di-0xa],cx
        0x89, 0x4A, 0xF6,       // mov [bp+si-0xa],cx
        0x89, 0x4B, 0xF6,       // mov [bp+di-0xa],cx
        0x89, 0x4C, 0xF6,       // mov [si-0xa],cx
        0x89, 0x4D, 0xF6,       // mov [di-0xa],cx
        0x89, 0x4E, 0xF6,       // mov [bp-0xa],cx
        0x89, 0x4F, 0xF6,       // mov [bx-0xa],cx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] 8948F6           Mov16    word [ds:bx+si-0x0A], cx
[085F:0103] 8949F6           Mov16    word [ds:bx+di-0x0A], cx
[085F:0106] 894AF6           Mov16    word [ds:bp+si-0x0A], cx
[085F:0109] 894BF6           Mov16    word [ds:bp+di-0x0A], cx
[085F:010C] 894CF6           Mov16    word [ds:si-0x0A], cx
[085F:010F] 894DF6           Mov16    word [ds:di-0x0A], cx
[085F:0112] 894EF6           Mov16    word [ds:bp-0x0A], cx
[085F:0115] 894FF6           Mov16    word [ds:bx-0x0A], cx", res);
}

#[test]
fn can_disassemble_addressing_mod1_opsize() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // operand-size override
        0x66, 0x89, 0x48, 0xF6, // mov [bx+si-0xa],ecx
        0x66, 0x89, 0x49, 0xF6, // mov [bx+di-0xa],ecx
        0x66, 0x89, 0x4A, 0xF6, // mov [bp+si-0xa],ecx
        0x66, 0x89, 0x4B, 0xF6, // mov [bp+di-0xa],ecx
        0x66, 0x89, 0x4C, 0xF6, // mov [si-0xa],ecx
        0x66, 0x89, 0x4D, 0xF6, // mov [di-0xa],ecx
        0x66, 0x89, 0x4E, 0xF6, // mov [bp-0xa],ecx
        0x66, 0x89, 0x4F, 0xF6, // mov [bx-0xa],ecx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] 668948F6         Mov32    dword [ds:bx+si-0x0A], ecx
[085F:0104] 668949F6         Mov32    dword [ds:bx+di-0x0A], ecx
[085F:0108] 66894AF6         Mov32    dword [ds:bp+si-0x0A], ecx
[085F:010C] 66894BF6         Mov32    dword [ds:bp+di-0x0A], ecx
[085F:0110] 66894CF6         Mov32    dword [ds:si-0x0A], ecx
[085F:0114] 66894DF6         Mov32    dword [ds:di-0x0A], ecx
[085F:0118] 66894EF6         Mov32    dword [ds:bp-0x0A], ecx
[085F:011C] 66894FF6         Mov32    dword [ds:bx-0x0A], ecx", res);
}

#[test]
fn can_disassemble_addressing_mod1_adrsize() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // address-size override
        0x67, 0x89, 0x48, 0xF6,         // mov [eax-0xa],cx
        0x67, 0x89, 0x49, 0xF6,         // mov [ecx-0xa],cx
        0x67, 0x89, 0x4A, 0xF6,         // mov [edx-0xa],cx
        0x67, 0x89, 0x4B, 0xF6,         // mov [ebx-0xa],cx
        0x67, 0x89, 0x4C, 0x00, 0x0A,   // mov [dword eax+eax+0xa],cx       SIB encoding
        0x67, 0x89, 0x4D, 0xF6,         // mov [ebp-0xa],cx
        0x67, 0x89, 0x4E, 0xF6,         // mov [esi-0xa],cx
        0x67, 0x89, 0x4F, 0xF6,         // mov [edi-0xa],cx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] 678948F6         Mov16    word [ds:eax-0x0A], cx
[085F:0104] 678949F6         Mov16    word [ds:ecx-0x0A], cx
[085F:0108] 67894AF6         Mov16    word [ds:edx-0x0A], cx
[085F:010C] 67894BF6         Mov16    word [ds:ebx-0x0A], cx
[085F:0110] 67894C000A       Mov16    word [ds:eax+eax+0x0A], cx
[085F:0115] 67894DF6         Mov16    word [ds:ebp-0x0A], cx
[085F:0119] 67894EF6         Mov16    word [ds:esi-0x0A], cx
[085F:011D] 67894FF6         Mov16    word [ds:edi-0x0A], cx", res);
}

#[test]
fn can_disassemble_addressing_mod2_noprefix() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x89, 0x88, 0x48, 0xF4, // mov [bx+si-0xbb8],cx
        0x89, 0x89, 0x48, 0xF4, // mov [bx+di-0xbb8],cx
        0x89, 0x8A, 0x48, 0xF4, // mov [bp+si-0xbb8],cx
        0x89, 0x8B, 0x48, 0xF4, // mov [bp+di-0xbb8],cx
        0x89, 0x8C, 0x48, 0xF4, // mov [si-0xbb8],cx
        0x89, 0x8D, 0x48, 0xF4, // mov [di-0xbb8],cx
        0x89, 0x8E, 0x48, 0xF4, // mov [bp-0xbb8],cx
        0x89, 0x8F, 0x48, 0xF4, // mov [bx-0xbb8],cx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] 898848F4         Mov16    word [ds:bx+si-0x0BB8], cx
[085F:0104] 898948F4         Mov16    word [ds:bx+di-0x0BB8], cx
[085F:0108] 898A48F4         Mov16    word [ds:bp+si-0x0BB8], cx
[085F:010C] 898B48F4         Mov16    word [ds:bp+di-0x0BB8], cx
[085F:0110] 898C48F4         Mov16    word [ds:si-0x0BB8], cx
[085F:0114] 898D48F4         Mov16    word [ds:di-0x0BB8], cx
[085F:0118] 898E48F4         Mov16    word [ds:bp-0x0BB8], cx
[085F:011C] 898F48F4         Mov16    word [ds:bx-0x0BB8], cx", res);
}

#[test]
fn can_disassemble_addressing_mod2_opsize() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // operand-size override
        0x66, 0x89, 0x88, 0x48, 0xF4,   // mov [bx+si-0xbb8],ecx
        0x66, 0x89, 0x89, 0x48, 0xF4,   // mov [bx+di-0xbb8],ecx
        0x66, 0x89, 0x8A, 0x48, 0xF4,   // mov [bp+si-0xbb8],ecx
        0x66, 0x89, 0x8B, 0x48, 0xF4,   // mov [bp+di-0xbb8],ecx
        0x66, 0x89, 0x8C, 0x48, 0xF4,   // mov [si-0xbb8],ecx
        0x66, 0x89, 0x8D, 0x48, 0xF4,   // mov [di-0xbb8],ecx
        0x66, 0x89, 0x8E, 0x48, 0xF4,   // mov [bp-0xbb8],ecx
        0x66, 0x89, 0x8F, 0x48, 0xF4,   // mov [bx-0xbb8],ecx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] 66898848F4       Mov32    dword [ds:bx+si-0x0BB8], ecx
[085F:0105] 66898948F4       Mov32    dword [ds:bx+di-0x0BB8], ecx
[085F:010A] 66898A48F4       Mov32    dword [ds:bp+si-0x0BB8], ecx
[085F:010F] 66898B48F4       Mov32    dword [ds:bp+di-0x0BB8], ecx
[085F:0114] 66898C48F4       Mov32    dword [ds:si-0x0BB8], ecx
[085F:0119] 66898D48F4       Mov32    dword [ds:di-0x0BB8], ecx
[085F:011E] 66898E48F4       Mov32    dword [ds:bp-0x0BB8], ecx
[085F:0123] 66898F48F4       Mov32    dword [ds:bx-0x0BB8], ecx", res);
}

#[test]
fn can_disassemble_addressing_mod2_adrsize() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // address-size override
        0x67, 0x89, 0x88, 0x88, 0x13, 0x00, 0x00,       // mov [eax+0x1388],cx
        0x67, 0x89, 0x89, 0x88, 0x13, 0x00, 0x00,       // mov [ecx+0x1388],cx
        0x67, 0x89, 0x8A, 0x88, 0x13, 0x00, 0x00,       // mov [edx+0x1388],cx
        0x67, 0x89, 0x8B, 0x88, 0x13, 0x00, 0x00,       // mov [ebx+0x1388],cx
        0x67, 0x89, 0x8C, 0x00, 0x88, 0x13, 0x00, 0x00, // mov [dword eax+eax+0x1388],cx    SIB encoding
        0x67, 0x89, 0x8D, 0x88, 0x13, 0x00, 0x00,       // mov [ebp+0x1388],cx
        0x67, 0x89, 0x8E, 0x88, 0x13, 0x00, 0x00,       // mov [esi+0x1388],cx
        0x67, 0x89, 0x8F, 0x88, 0x13, 0x00, 0x00,       // mov [edi+0x1388],cx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] 67898888130000   Mov16    word [ds:eax+0x00001388], cx
[085F:0107] 67898988130000   Mov16    word [ds:ecx+0x00001388], cx
[085F:010E] 67898A88130000   Mov16    word [ds:edx+0x00001388], cx
[085F:0115] 67898B88130000   Mov16    word [ds:ebx+0x00001388], cx
[085F:011C] 67898C0088130000 Mov16    word [ds:eax+eax+0x1388], cx
[085F:0124] 67898D88130000   Mov16    word [ds:ebp+0x00001388], cx
[085F:012B] 67898E88130000   Mov16    word [ds:esi+0x00001388], cx
[085F:0132] 67898F88130000   Mov16    word [ds:edi+0x00001388], cx", res);
}

#[test]
fn can_disassemble_addressing_mod3_noprefix() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x89, 0xC8,             // mov ax,cx
        0x89, 0xC9,             // mov cx,cx
        0x89, 0xCA,             // mov dx,cx
        0x89, 0xCB,             // mov bx,cx
        0x89, 0xCC,             // mov sp,cx
        0x89, 0xCD,             // mov bp,cx
        0x89, 0xCE,             // mov si,cx
        0x89, 0xCF,             // mov di,cx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] 89C8             Mov16    ax, cx
[085F:0102] 89C9             Mov16    cx, cx
[085F:0104] 89CA             Mov16    dx, cx
[085F:0106] 89CB             Mov16    bx, cx
[085F:0108] 89CC             Mov16    sp, cx
[085F:010A] 89CD             Mov16    bp, cx
[085F:010C] 89CE             Mov16    si, cx
[085F:010E] 89CF             Mov16    di, cx", res);
}

#[test]
fn can_disassemble_addressing_mod3_opsize() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        // operand-size override
        0x66, 0x89, 0xC8,       // mov eax,ecx
        0x66, 0x89, 0xC9,       // mov ecx,ecx
        0x66, 0x89, 0xCA,       // mov edx,ecx
        0x66, 0x89, 0xCB,       // mov ebx,ecx
        0x66, 0x89, 0xCC,       // mov esp,ecx
        0x66, 0x89, 0xCD,       // mov ebp,ecx
        0x66, 0x89, 0xCE,       // mov esi,ecx
        0x66, 0x89, 0xCF,       // mov edi,ecx
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] 6689C8           Mov32    eax, ecx
[085F:0103] 6689C9           Mov32    ecx, ecx
[085F:0106] 6689CA           Mov32    edx, ecx
[085F:0109] 6689CB           Mov32    ebx, ecx
[085F:010C] 6689CC           Mov32    esp, ecx
[085F:010F] 6689CD           Mov32    ebp, ecx
[085F:0112] 6689CE           Mov32    esi, ecx
[085F:0115] 6689CF           Mov32    edi, ecx", res);
}

// XXX test mod3_adrsize

#[test]
fn can_disassemble_call() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xE8, 0x05, 0x00,                   // call 0x108
        0xE8, 0xFB, 0xFF,                   // call 0x101
        0x66, 0xE8, 0x11, 0x00, 0x00, 0x00, // call dword 0x11d
        0xFF, 0x18,                         // call far [bx+si]
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 4);
    assert_eq!("[085F:0100] E80500           CallNear 0x0108
[085F:0103] E8FBFF           CallNear 0x0101
[085F:0106] 66E811000000     CallNear 0x0000011D
[085F:010C] FF18             CallFar  word [ds:bx+si]",
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
fn can_disassemble_mov() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBA, 0x0B, 0x01,                               // mov dx,0x10b
        0xB4, 0x09,                                     // mov ah,0x9
        0x26, 0x88, 0x25,                               // mov [es:di],ah
        0x26, 0x8A, 0x25,                               // mov ah,[es:di]
        0x67, 0x88, 0x03,                               // mov [ebx],al
        0x67, 0x89, 0x90, 0x00, 0x00, 0x00, 0x00,       // mov [eax+0x0],dx
        0x66, 0x89, 0x85, 0xC0, 0xFE,                   // mov [di-0x140],eax
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 7);
    assert_eq!("[085F:0100] BA0B01           Mov16    dx, 0x010B
[085F:0103] B409             Mov8     ah, 0x09
[085F:0105] 268825           Mov8     byte [es:di], ah
[085F:0108] 268A25           Mov8     ah, byte [es:di]
[085F:010B] 678803           Mov8     byte [ds:ebx], al
[085F:010E] 67899000000000   Mov16    word [ds:eax+0x00000000], dx
[085F:0115] 668985C0FE       Mov32    dword [ds:di-0x0140], eax", res);
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
fn can_disassemble_loop() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xE2, 0x0E,         // loop 0x110
        0x67, 0xE2, 0x0B,   // loop 0x110,ecx
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] E20E             Loop16   0x0110
[085F:0102] 67E20B           Loop32   0x0110", res);
}

#[test]
fn can_disassemble_jcxz() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xE3, 0x1F,         // jcxz 0x121
        0x67, 0xE3, 0x1C,   // jecxz 0x121
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] E31F             Jcxz     0x0121
[085F:0102] 67E31C           Jecxz    0x0121", res);
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

#[test]
fn can_disassemble_movzx() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x0F, 0xB6, 0xC3,           // movzx ax,bl
        0x66, 0x0F, 0xB6, 0xC3,     // movzx eax,bl
        0x66, 0x0F, 0xB7, 0xC3,     // movzx eax,bx
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 0FB6C3           Movzx16  ax, bl
[085F:0103] 660FB6C3         Movzx32  eax, bl
[085F:0107] 660FB7C3         Movzx32  eax, bx", res);
}

#[test]
fn can_disassemble_movsx() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x0F, 0xBE, 0xD9,                   // movsx bx,cl
        0x66, 0x0F, 0xBE, 0xD9,             // movsx ebx,cl
        0x66, 0x0F, 0xBE, 0x86, 0xF1, 0x01, // movsx eax, byte [ds:bp+0x01F1]
        0x66, 0x0F, 0xBF, 0xD9,             // movsx ebx,cx
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 4);
    assert_eq!("[085F:0100] 0FBED9           Movsx16  bx, cl
[085F:0103] 660FBED9         Movsx32  ebx, cl
[085F:0107] 660FBE86F101     Movsx32  eax, byte [ds:bp+0x01F1]
[085F:010D] 660FBFD9         Movsx32  ebx, cx", res);
}

#[test]
fn can_disassemble_scas() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xAE,               // scasb
        0xAF,               // scasw
        0x66, 0xAF,         // scasd
        0xF3, 0x66, 0xAF,   // repe scasd
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 4);
    assert_eq!("[085F:0100] AE               Scasb
[085F:0101] AF               Scasw
[085F:0102] 66AF             Scasd
[085F:0104] F366AF           Repe     Scasd", res);
}

#[test]
fn can_disassemble_cmps() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0xA7,                 // cmpsd
        0xF3, 0x66, 0xA7,           // repe cmpsd
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] 66A7             Cmpsd16
[085F:0102] F366A7           Repe     Cmpsd16", res);
}

#[test]
fn can_disassemble_xchg() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x86, 0xCA,         // xchg cl,dl
        0x87, 0xCA,         // xchg cx,dx
        0x66, 0x87, 0xCA,   // xchg ecx,edx

        0x93,               // xchg ax,bx
        0x66, 0x93,         // xchg eax,ebx
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 5);
    assert_eq!("[085F:0100] 86CA             Xchg8    cl, dl
[085F:0102] 87CA             Xchg16   cx, dx
[085F:0104] 6687CA           Xchg32   ecx, edx
[085F:0107] 93               Xchg16   ax, bx
[085F:0108] 6693             Xchg32   eax, ebx", res);
}

#[test]
fn can_disassemble_fild() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xDF, 0x06, 0x58, 0x80, // fild word [0x8058]
        0xDB, 0x05,             // fild dword [di]
        //0xDF, 0x28,             // fild qword [bx+si]         XXX handle qword
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] DF065880         Fild     word [ds:0x8058]
[085F:0104] DB05             Fild     dword [ds:di]", res);
}

#[test]
fn can_disassemble_fmul() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xD8, 0x0E, 0xA4, 0x10, // fmul dword [0x10a4]
        0xDE, 0x0E, 0xA8, 0x10, // fimul word [0x10a8]
        0xDE, 0x36, 0x60, 0x80, // fidiv word [0x8060]
        0xDE, 0xF9,             // fdivp st1
        0xD8, 0x36, 0xF6, 0x01, // fdiv dword [0x1f6]
        0xD8, 0x7C, 0x04,       // fdivr dword [si+0x4]
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 6);
    assert_eq!("[085F:0100] D80EA410         Fmul     dword [ds:0x10A4]
[085F:0104] DE0EA810         Fimul    word [ds:0x10A8]
[085F:0108] DE366080         Fidiv    word [ds:0x8060]
[085F:010C] DEF9             Fdivp    st1
[085F:010E] D836F601         Fdiv     dword [ds:0x01F6]
[085F:0112] D87C04           Fdivr    dword [ds:si+0x04]", res);
}

#[test]
fn can_disassemble_fsin() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xD9, 0xFE,             // fsin
        0xD9, 0xFF,             // fcos
        0xD9, 0xFB,             // fsincos
        0xD9, 0xFC,             // frndint
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 4);
    assert_eq!("[085F:0100] D9FE             Fsin
[085F:0102] D9FF             Fcos
[085F:0104] D9FB             Fsincos
[085F:0106] D9FC             Frndint", res);
}

#[test]
fn can_disassemble_fist() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xDF, 0x15,             // fist word [di]
        0xDF, 0x19,             // fistp word [bx+di]
        0xDB, 0x1E, 0x32, 0x05, // fistp dword [0x532]
        0xDB, 0x0A,             // fisttp dword [bp+si]
        //0xDF, 0x3D,             // fistp qword [di]     XXX handle qword
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 4);
    assert_eq!("[085F:0100] DF15             Fist     word [ds:di]
[085F:0102] DF19             Fistp    word [ds:bx+di]
[085F:0104] DB1E3205         Fistp    dword [ds:0x0532]
[085F:0108] DB0A             Fisttp   dword [ds:bp+si]", res);
}

#[test]
fn can_disassemble_fld() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xD9, 0xC0,             // fld st0
        0xD9, 0xC1,             // fld st1

        0xD9, 0xE8,             // fld1
        0xD9, 0xEB,             // fldpi
        0xD9, 0xE9,             // fldl2t
        0xD9, 0xEA,             // fldl2e

        0xD9, 0x06, 0xE9, 0x02, // fld dword [0x2e9]
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 7);
    assert_eq!("[085F:0100] D9C0             Fld      st0
[085F:0102] D9C1             Fld      st1
[085F:0104] D9E8             Fld1
[085F:0106] D9EB             Fldpi
[085F:0108] D9E9             Fldl2t
[085F:010A] D9EA             Fldl2e
[085F:010C] D906E902         Fld      dword [ds:0x02E9]", res);
}

#[test]
fn can_disassemble_fst() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xD9, 0x9F, 0x46, 0x62, // fstp dword [bx+0x6246]
        0xD9, 0x97, 0x46, 0x62, // fst dword [bx+0x6246]
        0xD9, 0x1E, 0xE9, 0x02, // fstp dword [0x2e9]
        0xDD, 0xD9,             // fstp st1
        0xDD, 0xD1,             // fst st1
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 5);
    assert_eq!("[085F:0100] D99F4662         Fstp     dword [ds:bx+0x6246]
[085F:0104] D9974662         Fst      dword [ds:bx+0x6246]
[085F:0108] D91EE902         Fstp     dword [ds:0x02E9]
[085F:010C] DDD9             Fstp     st1
[085F:010E] DDD1             Fst      st1", res);
}

#[test]
fn can_disassemble_fadd() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xD8, 0xC1,             // fadd st1
        0xDE, 0xC1,             // faddp st1
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] D8C1             Fadd     st1
[085F:0102] DEC1             Faddp    st1", res);
}

#[test]
fn can_disassemble_fsub() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xDE, 0xEA,             // fsubp st2
        0xDE, 0xE2,             // fsubrp st2
        0xD8, 0x28,             // fsubr dword [bx+si]
        0xD8, 0x66, 0x19,       //  fsub dword [bp+0x19]
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 4);
    assert_eq!("[085F:0100] DEEA             Fsubp    st2
[085F:0102] DEE2             Fsubrp   st2
[085F:0104] D828             Fsubr    dword [ds:bx+si]
[085F:0106] D86619           Fsub     dword [ds:bp+0x19]", res);
}

#[test]
fn can_disassemble_fpatan() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xD9, 0xF3,             // fpatan
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 1);
    assert_eq!("[085F:0100] D9F3             Fpatan", res);
}

#[test]
fn can_disassemble_ffree() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xDD, 0xC0,             // ffree st0
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 1);
    assert_eq!("[085F:0100] DDC0             Ffree    st0", res);
}

#[test]
fn can_disassemble_fxch() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xD9, 0xC9,             // fxch st1
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 1);
    assert_eq!("[085F:0100] D9C9             Fxch     st1", res);
}

#[test]
fn can_disassemble_fldcw() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xD9, 0x28,             // fldcw [bx+si]
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 1);
    assert_eq!("[085F:0100] D928             Fldcw    word [ds:bx+si]", res);
}

#[test]
fn can_disassemble_fcom() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xDE, 0x17,             // ficom word [bx]
        0xDE, 0x1F,             // ficomp word [bx]
        0xDA, 0x1F,             // ficomp dword [bx]
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] DE17             Ficom    word [ds:bx]
[085F:0102] DE1F             Ficomp   word [ds:bx]
[085F:0104] DA1F             Ficomp   dword [ds:bx]", res);
}

#[test]
fn can_disassemble_finit() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xDB, 0xE3,             // finit
        0xD9, 0xE4,             // ftst
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] DBE3             Finit
[085F:0102] D9E4             Ftst", res);
}
