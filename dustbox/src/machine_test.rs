use std::num::Wrapping;

use crate::machine::Machine;
use crate::cpu::R;

// TODO TEST retn, retf, retn imm16
// TODO lds, les - write tests and fix implementation - it is wrong?!

#[test]
fn can_execute_push_pop() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x88, 0x88, // mov ax,0x8888
        0x8E, 0xD8,       // mov ds,ax
        0x1E,             // push ds
        0x07,             // pop es
    ];
    machine.load_executable(&code, 0x085F);

    let stack_offset = machine.cpu.get_r16(R::SP);
    machine.execute_instruction(); // mov
    machine.execute_instruction(); // mov

    assert_eq!(stack_offset, machine.cpu.get_r16(R::SP));
    machine.execute_instruction(); // push
    assert_eq!(stack_offset - 2, machine.cpu.get_r16(R::SP));
    machine.execute_instruction(); // pop
    assert_eq!(stack_offset, machine.cpu.get_r16(R::SP));

    assert_eq!(0x8888, machine.cpu.get_r16(R::AX));
    assert_eq!(0x8888, machine.cpu.get_r16(R::DS));
    assert_eq!(0x8888, machine.cpu.get_r16(R::ES));
}

#[test]
fn can_execute_inc32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0xB8, 0xFF, 0xFF, 0x00, 0x80, // mov eax,0x8000ffff
        0x66, 0x40,                         // inc eax
    ];
    machine.load_executable(&code, 0x085F);
    machine.execute_instructions(2);
    assert_eq!(0x8001_0000, machine.cpu.get_r32(R::EAX));
}

#[test]
fn can_execute_dec32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0xB8, 0x00, 0x00, 0x01, 0x80, // mov eax,0x80010000
        0x66, 0x48,                         // dec eax
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0x8000_FFFF, machine.cpu.get_r32(R::EAX));
}

#[test]
fn can_execute_add8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB4, 0xFF,         // mov ah,0xff
        0x80, 0xC4, 0x01,   // add ah,0x1

        0xB4, 0x01,         // mov ah,0x1
        0x80, 0xC4, 0xFF,   // add ah,0xff

        0xB4, 0xFF,         // mov ah,0xff
        0x80, 0xC4, 0x00,   // add ah,0x0

        0xB4, 0xFF,         // mov ah,0xff
        0x80, 0xC4, 0xFF,   // add ah,0xff
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0x00, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.adjust);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0x00, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.adjust);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0xFF, machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.adjust);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0xFE, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.adjust);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
}

#[test]
fn can_execute_add16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0x83, 0xC0, 0x01,   // add ax,byte +0x1

        0xB8, 0x01, 0x00,   // mov ax,0x1
        0x83, 0xC0, 0xFF,   // add ax,byte -0x1

        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0x83, 0xC0, 0x00,   // add ax,byte +0x0

        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0x83, 0xC0, 0xFF,   // add ax,byte -0x1
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.adjust);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.adjust);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.adjust);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0xFFFE, machine.cpu.get_r16(R::AX));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.adjust);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
}

#[test]
fn can_execute_mov_r8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB2, 0x13, // mov dl,0x13
        0x88, 0xD0, // mov al,dl
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instruction();
    assert_eq!(0x13, machine.cpu.get_r8(R::DL));

    machine.execute_instruction();
    assert_eq!(0x13, machine.cpu.get_r8(R::AL));
}

#[test]
fn can_execute_mov_r32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0xB8, 0x78, 0x56, 0x34, 0x12, // mov eax,0x12345678
        0x66, 0xB8, 0x23, 0x01, 0xFF, 0x00, // mov eax,0xff0123
        0x66, 0x89, 0xC5,                   // mov ebp,eax
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 1);
    assert_eq!("[085F:0100] 66B878563412     Mov32    eax, 0x12345678", res);
    machine.execute_instruction();
    assert_eq!(0x1234_5678, machine.cpu.get_r32(R::EAX));

    machine.execute_instructions(2);
    assert_eq!(0x00FF_0123, machine.cpu.get_r32(R::EBP));
}

#[test]
fn can_execute_mov_r8_rm8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBB, 0x05, 0x01, // mov bx,0x105
        0x8A, 0x27,       // mov ah,[bx]   | r8, r/m8
        0x99,             // db 0x99
    ];

    machine.load_executable(&code, 0x085F);

    machine.execute_instruction();
    assert_eq!(0x103, machine.cpu.regs.ip);
    assert_eq!(0x105, machine.cpu.get_r16(R::BX));

    machine.execute_instruction();
    assert_eq!(0x105, machine.cpu.regs.ip);
    assert_eq!(0x99, machine.cpu.get_r8(R::AH));
}

#[test]
fn can_execute_mv_r16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x23, 0x01, // mov ax,0x123
        0x8B, 0xE0,       // mov sp,ax   | r16, r16
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instruction();
    assert_eq!(0x103, machine.cpu.regs.ip);
    assert_eq!(0x123, machine.cpu.get_r16(R::AX));

    machine.execute_instruction();
    assert_eq!(0x105, machine.cpu.regs.ip);
    assert_eq!(0x123, machine.cpu.get_r16(R::SP));
}

#[test]
fn can_execute_mov_r16_rm16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB9, 0x23, 0x01, // mov cx,0x123
        0x8E, 0xC1,       // mov es,cx   | r/m16, r16
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instruction();
    assert_eq!(0x103, machine.cpu.regs.ip);
    assert_eq!(0x123, machine.cpu.get_r16(R::CX));

    machine.execute_instruction();
    assert_eq!(0x105, machine.cpu.regs.ip);
    assert_eq!(0x123, machine.cpu.get_r16(R::ES));
}

#[test]
fn can_execute_mov_rm16_sreg() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBB, 0x34, 0x12,       // mov bx,0x1234
        0x8E, 0xC3,             // mov es,bx
        0x8C, 0x06, 0x09, 0x01, // mov [0x109],es  | r/m16, sreg
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instruction();
    assert_eq!(0x103, machine.cpu.regs.ip);
    assert_eq!(0x1234, machine.cpu.get_r16(R::BX));

    machine.execute_instruction();
    assert_eq!(0x105, machine.cpu.regs.ip);
    assert_eq!(0x1234, machine.cpu.get_r16(R::ES));

    machine.execute_instruction();
    assert_eq!(0x109, machine.cpu.regs.ip);
    let cs = machine.cpu.get_r16(R::CS);
    assert_eq!(0x1234, machine.mmu.read_u16(cs, 0x0109));
}

#[test]
fn can_execute_mov_data() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xC6, 0x06, 0x31, 0x10, 0x38,       // mov byte [0x1031],0x38
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instruction();
    assert_eq!(0x105, machine.cpu.regs.ip);
    let cs = machine.cpu.get_r16(R::CS);
    assert_eq!(0x38, machine.mmu.read_u8(cs, 0x1031));
}

#[test]
fn can_execute_mov_es_segment() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x68, 0x40, 0x40,               // push word 0x4040
        0x07,                           // pop es
        0xBF, 0x00, 0x02,               // mov di,0x200
        0xB4, 0x88,                     // mov ah,0x88

        0x26, 0x88, 0x25,               // mov [es:di],ah
        0x26, 0x8A, 0x05,               // mov al,[es:di]

        0x26, 0x8A, 0x45, 0x01,         // mov al,[es:di+0x1]
        0x26, 0x8A, 0x5D, 0xFF,         // mov bl,[es:di-0x1]

        0x26, 0x8A, 0x85, 0x40, 0x01,   // mov al,[es:di+0x140]
        0x26, 0x8A, 0x9D, 0xC0, 0xFE,   // mov bl,[es:di-0x140]
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(4);

    let es = machine.cpu.get_r16(R::ES);
    let di = machine.cpu.get_r16(R::DI);
    machine.execute_instruction(); // mov [es:di],ah
    assert_eq!(0x88, machine.mmu.read_u8(es, di));

    machine.execute_instruction(); // mov al,[es:di]
    assert_eq!(0x88, machine.cpu.get_r8(R::AL));

    machine.mmu.write_u8(es, di + 1, 0x1);
    machine.mmu.write_u8(es, di - 1, 0xFF);
    machine.execute_instruction(); // mov al,[es:di+0x1]
    assert_eq!(0x1, machine.cpu.get_r8(R::AL));
    machine.execute_instruction(); // mov bl,[es:di-0x1]
    assert_eq!(0xFF, machine.cpu.get_r8(R::BL));

    machine.mmu.write_u8(es, di + 0x140, 0x22);
    machine.mmu.write_u8(es, di - 0x140, 0x88);
    machine.execute_instruction(); // mov al,[es:di+0x140]
    assert_eq!(0x22, machine.cpu.get_r8(R::AL));
    machine.execute_instruction(); // mov bl,[es:di-0x140]
    assert_eq!(0x88, machine.cpu.get_r8(R::BL));
}

#[test]
fn can_execute_mov_fs_segment() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x68, 0x40, 0x40,   // push word 0x4040
        0x0F, 0xA1,         // pop fs
        0xBF, 0x00, 0x02,   // mov di,0x200
        0xB0, 0xFF,         // mov al,0xff

        0x64, 0x88, 0x05,   // mov [fs:di],al
    ];

    machine.load_executable(&code, 0x085F);
    machine.execute_instructions(5); // mov [fs:di],al
    assert_eq!(0xFF, machine.mmu.read_u8(machine.cpu.get_r16(R::FS), machine.cpu.get_r16(R::DI)));
}

#[test]
fn can_execute_imms8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBF, 0x00, 0x01, // mov di,0x100
        0x83, 0xC7, 0x3A, // add di,byte +0x3a
        0x83, 0xC7, 0xC6, // add di,byte -0x3a
    ];

    machine.load_executable(&code, 0x085F);

    machine.execute_instruction();
    assert_eq!(0x103, machine.cpu.regs.ip);
    assert_eq!(0x0100, machine.cpu.get_r16(R::DI));

    machine.execute_instruction();
    assert_eq!(0x106, machine.cpu.regs.ip);
    assert_eq!(0x013A, machine.cpu.get_r16(R::DI));

    machine.execute_instruction();
    assert_eq!(0x109, machine.cpu.regs.ip);
    assert_eq!(0x0100, machine.cpu.get_r16(R::DI));
}

#[test]
fn can_execute_with_flags() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB4, 0xFE,       // mov ah,0xfe
        0x80, 0xC4, 0x02, // add ah,0x2   - OF and ZF should be set
    ];

    machine.load_executable(&code, 0x085F);

    machine.execute_instruction();
    assert_eq!(0x102, machine.cpu.regs.ip);
    assert_eq!(0xFE, machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
    assert_eq!(false, machine.cpu.regs.flags.adjust);
    assert_eq!(false, machine.cpu.regs.flags.parity);

    machine.execute_instruction();
    assert_eq!(0x105, machine.cpu.regs.ip);
    assert_eq!(0x00, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
    assert_eq!(true, machine.cpu.regs.flags.adjust);
    assert_eq!(true, machine.cpu.regs.flags.parity);
}

#[test]
fn can_execute_cmp() {
    // make sure we dont overflow (0 - 0x2000 = overflow)
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x00,       // mov bx,0x0
        0x89, 0xDF,             // mov di,bx
        0x81, 0xFF, 0x00, 0x20, // cmp di,0x2000
    ];

    machine.load_executable(&code, 0x085F);

    machine.execute_instruction();
    assert_eq!(0x103, machine.cpu.regs.ip);
    assert_eq!(0, machine.cpu.get_r16(R::BX));

    machine.execute_instruction();
    assert_eq!(0x105, machine.cpu.regs.ip);
    assert_eq!(0, machine.cpu.get_r16(R::DI));

    machine.execute_instruction();
    assert_eq!(0x109, machine.cpu.regs.ip);

    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
    assert_eq!(false, machine.cpu.regs.flags.adjust);
    assert_eq!(true, machine.cpu.regs.flags.parity);
}

#[test]
fn can_execute_xchg() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x34, 0x12,   // mov ax,0x1234
        0xB9, 0xFF, 0xFF,   // mov cx,0xffff
        0x91,               // xchg ax,cx
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));
    assert_eq!(0x1234, machine.cpu.get_r16(R::CX));
}

#[test]
fn can_execute_rep_movsb() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBE, 0x00, 0x01,   // mov si,0x100
        0xBF, 0x00, 0x02,   // mov di,0x200
        0xB9, 0x04, 0x00,   // mov cx,0x4
        0xF3, 0xA4,         // rep movsb
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);

    // copy first 4 bytes from DS:0x100 to ES:0x200
    machine.execute_instructions(4);
    machine.execute_instruction(); // rep movsb
    machine.execute_instruction(); // rep movsb
    machine.execute_instruction(); // rep movsb
    assert_eq!(0x0, machine.cpu.get_r16(R::CX));
    let min = 0x100;
    let max = min + 4;
    for i in min..max {
        assert_eq!(
            machine.mmu.read_u8(machine.cpu.get_r16(R::ES), i),
            machine.mmu.read_u8(machine.cpu.get_r16(R::ES), i+0x100));
    }
}

#[test]
fn can_execute_rep_outsb() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBE, 0x00, 0x01,   // mov si,0x100
        0xBA, 0xC8, 0x03,   // mov dx,0x3c8
        0xB9, 0x02, 0x00,   // mov cx,0x2
        0xF3, 0x6E,         // rep outsb
    ];
    machine.load_executable(&code, 0x085F);

    assert_eq!(0, machine.gpu_mut().unwrap().dac.write_index);

    machine.execute_instructions(3);
    machine.execute_instruction(); // rep outsb
    assert_eq!(0xBE, machine.gpu_mut().unwrap().dac.write_index);

    machine.execute_instruction(); // rep outsb
    assert_eq!(0x00, machine.gpu_mut().unwrap().dac.write_index);

    assert_eq!(0x0, machine.cpu.get_r16(R::CX));
}

#[test]
fn can_execute_es_outsb() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x68, 0x00, 0x80,       // push word 0x8000
        0x07,                   // pop es
        0xBE, 0x00, 0x01,       // mov si,0x100
        0x26, 0xC6, 0x04, 0x09, // mov byte [es:si],0x9
        0xBA, 0xC8, 0x03,       // mov dx,0x3c8
        0x26, 0x6E,             // es outsb
    ];
    machine.load_executable(&code, 0x085F);

    assert_eq!(0, machine.gpu_mut().unwrap().dac.write_index);
    machine.execute_instructions(6);
    assert_eq!(0x09, machine.gpu_mut().unwrap().dac.write_index);
}

#[test]
fn can_execute_lea() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBB, 0x44, 0x44,           // mov bx,0x4444
        0x8D, 0x3F,                 // lea di,[bx]
        0x8D, 0x36, 0x33, 0x22,     // lea si,[0x2233]
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0x4444, machine.cpu.get_r16(R::DI));

    machine.execute_instruction();
    assert_eq!(0x2233, machine.cpu.get_r16(R::SI));
}

#[test]
fn can_execute_8bit_16bit_addressing() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x02,             // mov bx,0x200
        0xC6, 0x47, 0x2C, 0xFF,       // mov byte [bx+0x2c],0xff  ; rm8 [amode+s8]
        0x8B, 0x14,                   // mov dx,[si]              ; rm16 [reg]
        0x8B, 0x47, 0x2C,             // mov ax,[bx+0x2c]         ; rm16 [amode+s8]
        0x89, 0x87, 0x30, 0x00,       // mov [bx+0x0030],ax       ; rm16 [amode+s16]
        0x89, 0x05,                   // mov [di],ax              ; rm16 [amode]
        0xC6, 0x85, 0xAE, 0x06, 0xFE, // mov byte [di+0x6ae],0xfe ; rm8 [amode+s16]
        0x8A, 0x85, 0xAE, 0x06,       // mov al,[di+0x6ae]
    ];

    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 8);
    assert_eq!("[085F:0100] BB0002           Mov16    bx, 0x0200
[085F:0103] C6472CFF         Mov8     byte [ds:bx+0x2C], 0xFF
[085F:0107] 8B14             Mov16    dx, word [ds:si]
[085F:0109] 8B472C           Mov16    ax, word [ds:bx+0x2C]
[085F:010C] 89873000         Mov16    word [ds:bx+0x0030], ax
[085F:0110] 8905             Mov16    word [ds:di], ax
[085F:0112] C685AE06FE       Mov8     byte [ds:di+0x06AE], 0xFE
[085F:0117] 8A85AE06         Mov8     al, byte [ds:di+0x06AE]", res);

    machine.execute_instruction();
    assert_eq!(0x200, machine.cpu.get_r16(R::BX));

    machine.execute_instruction();
    let ds = machine.cpu.get_r16(R::DS);
    assert_eq!(0xFF, machine.mmu.read_u8(ds, 0x22C));

    machine.execute_instruction();
    // should have read word at [0x100]
    assert_eq!(0x00BB, machine.cpu.get_r16(R::DX));

    machine.execute_instruction();
    // should have read word at [0x22C]
    assert_eq!(0x00FF, machine.cpu.get_r16(R::AX));

    machine.execute_instruction();
    // should have written word to [0x230]
    assert_eq!(0x00FF, machine.mmu.read_u16(ds, 0x230));

    machine.execute_instruction();
    // should have written ax to [di]
    let di = machine.cpu.get_r16(R::DI);
    assert_eq!(0x00FF, machine.mmu.read_u16(ds, di));

    machine.execute_instruction();
    // should have written byte to [di+0x06AE]
    assert_eq!(0xFE, machine.mmu.read_u8(ds, (Wrapping(di) +
                                     Wrapping(0x06AE)).0));

    machine.execute_instruction();
    // should have read byte from [di+0x06AE] to al
    assert_eq!(0xFE, machine.cpu.get_r8(R::AL));
}

#[test]
fn can_execute_32bit_addressing() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0xBB, 0x00, 0x02, 0x00, 0x00,         // mov ebx,0x200
        0x66, 0x89, 0x1E, 0x50, 0x02,               // mov [0x250],ebx
        0x66, 0xC7, 0x05, 0x01, 0x01, 0x01, 0x01,   // mov dword [di],0x1010101
        0x66, 0x89, 0x5D, 0xF8,                     // mov [di-0x8],ebx
        0x66, 0x89, 0x9D, 0xC0, 0xFE,               // mov [di-0x140],ebx
    ];

    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 5);
    assert_eq!("[085F:0100] 66BB00020000     Mov32    ebx, 0x00000200
[085F:0106] 66891E5002       Mov32    dword [ds:0x0250], ebx
[085F:010B] 66C70501010101   Mov32    dword [ds:di], 0x01010101
[085F:0112] 66895DF8         Mov32    dword [ds:di-0x08], ebx
[085F:0116] 66899DC0FE       Mov32    dword [ds:di-0x0140], ebx", res);

    machine.execute_instructions(2);
    let ds = machine.cpu.get_r16(R::DS);
    assert_eq!(0x0000_0200, machine.mmu.read_u32(ds, 0x250));

    machine.execute_instruction();
    let di = machine.cpu.get_r16(R::DI);
    assert_eq!(0x0101_0101, machine.mmu.read_u32(ds, di));

    machine.execute_instruction();
    let di = machine.cpu.get_r16(R::DI);
    assert_eq!(0x0000_0200, machine.mmu.read_u32(ds, di - 8));

    machine.execute_instruction();
    let di = machine.cpu.get_r16(R::DI);
    assert_eq!(0x0000_0200, machine.mmu.read_u32(ds, di - 0x140));
}

#[test]
fn can_execute_math() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xF6, 0x06, 0x2C, 0x12, 0xFF, // test byte [0x122c],0xff
    ];

    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 1);
    assert_eq!("[085F:0100] F6062C12FF       Test8    byte [ds:0x122C], 0xFF",
               res);

    // XXX also execute
}

#[test]
fn can_execute_and() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB0, 0xF0, // mov al,0xF0
        0xB4, 0x1F, // mov ah,0x1F
        0x20, 0xC4, // and ah,al
    ];

    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] B0F0             Mov8     al, 0xF0
[085F:0102] B41F             Mov8     ah, 0x1F
[085F:0104] 20C4             And8     ah, al",
               res);

    machine.execute_instruction();
    assert_eq!(0xF0, machine.cpu.get_r8(R::AL));

    machine.execute_instruction();
    assert_eq!(0x1F, machine.cpu.get_r8(R::AH));

    machine.execute_instruction();
    assert_eq!(0x10, machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.parity);
}

#[test]
fn can_execute_mul8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB0, 0x40, // mov al,0x40
        0xB3, 0x10, // mov bl,0x10
        0xF6, 0xE3, // mul bl
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0x400, machine.cpu.get_r16(R::AX));
    // XXX flags
}

#[test]
fn can_execute_mul16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x00, 0x80, // mov ax,0x8000
        0xBB, 0x04, 0x00, // mov bx,0x4
        0xF7, 0xE3,       // mul bx
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0x0002, machine.cpu.get_r16(R::DX));
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    // XXX flags
}

#[test]
fn can_execute_div8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x40, 0x00, // mov ax,0x40
        0xB3, 0x10,       // mov bl,0x10
        0xF6, 0xF3,       // div bl
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] B84000           Mov16    ax, 0x0040
[085F:0103] B310             Mov8     bl, 0x10
[085F:0105] F6F3             Div8     bl",
               res);

    machine.execute_instruction();
    assert_eq!(0x40, machine.cpu.get_r8(R::AL));

    machine.execute_instruction();
    assert_eq!(0x10, machine.cpu.get_r8(R::BL));

    machine.execute_instruction();
    assert_eq!(0x04, machine.cpu.get_r8(R::AL)); // quotient
    assert_eq!(0x00, machine.cpu.get_r8(R::AH)); // remainder
}

#[test]
fn can_execute_div16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBA, 0x10, 0x00, // mov dx,0x10
        0xB8, 0x00, 0x40, // mov ax,0x4000
        0xBB, 0x00, 0x01, // mov bx,0x100
        0xF7, 0xF3,       // div bx
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 4);
    assert_eq!("[085F:0100] BA1000           Mov16    dx, 0x0010
[085F:0103] B80040           Mov16    ax, 0x4000
[085F:0106] BB0001           Mov16    bx, 0x0100
[085F:0109] F7F3             Div16    bx",
               res);

    machine.execute_instruction();
    assert_eq!(0x10, machine.cpu.get_r16(R::DX));

    machine.execute_instruction();
    assert_eq!(0x4000, machine.cpu.get_r16(R::AX));

    machine.execute_instruction();
    assert_eq!(0x100, machine.cpu.get_r16(R::BX));

    machine.execute_instruction();
    assert_eq!(0x1040, machine.cpu.get_r16(R::AX)); // quotient
    assert_eq!(0x0000, machine.cpu.get_r16(R::DX)); // remainder
}

#[test]
fn can_execute_idiv8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x00, 0x00,   // mov ax,0x0
        0xB3, 0x02,         // mov bl,0x2
        0xF6, 0xFB,         // idiv bl     ; 0 / 2

        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xB3, 0x02,         // mov bl,0x2
        0xF6, 0xFB,         // idiv bl     ; 0xffff / 2

        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xB3, 0x0F,         // mov bl,0xf
        0xF6, 0xFB,         // idiv bl     ; 0x1 / 0xf
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0x00, machine.cpu.get_r8(R::AL)); // quotient
    assert_eq!(0x00, machine.cpu.get_r8(R::AH)); // remainder

    machine.execute_instructions(3);
    assert_eq!(0x00, machine.cpu.get_r8(R::AL));
    assert_eq!(0xFF, machine.cpu.get_r8(R::AH));

    machine.execute_instructions(3);
    assert_eq!(0x00, machine.cpu.get_r8(R::AL));
    assert_eq!(0x01, machine.cpu.get_r8(R::AH));
}

#[test]
fn can_execute_idiv16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBA, 0xFF, 0xFF,   // mov dx,0xffff
        0xB8, 0x00, 0x00,   // mov ax,0x0
        0xBB, 0x02, 0x00,   // mov bx,0x2
        0xF7, 0xFB,         // idiv bx          ; 0xffff0000 / 2

        0xBA, 0x00, 0x00,   // mov dx,0x0
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xBB, 0x02, 0x00,   // mov bx,0x2
        0xF7, 0xFB,         // idiv bx          ; 0xffff / 2

        0xBA, 0x00, 0x00,   // mov dx,0x0
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xBB, 0x0F, 0x00,   // mov bx,0xf
        0xF7, 0xFB,         // idiv bx          ; 0x1 / 0xf

        0xBA, 0x00, 0x00,   // mov dx,0x0
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xBB, 0xFF, 0xFF,   // mov bx,0xffff
        0xF7, 0xFB,         // idiv bx          ; 0x1 / 0xffff
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(4);
    assert_eq!(0x8000, machine.cpu.get_r16(R::AX)); // quotient
    assert_eq!(0x0000, machine.cpu.get_r16(R::DX)); // remainder

    machine.execute_instructions(4);
    assert_eq!(0x7FFF, machine.cpu.get_r16(R::AX));
    assert_eq!(0x0001, machine.cpu.get_r16(R::DX));

    machine.execute_instructions(4);
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    assert_eq!(0x0001, machine.cpu.get_r16(R::DX));

    machine.execute_instructions(4);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));
    assert_eq!(0x0000, machine.cpu.get_r16(R::DX));
}

#[test]
fn can_execute_idiv32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0xBA, 0x00, 0x00, 0x00, 0x00, // mov edx,0x0
        0x66, 0xB8, 0x00, 0x00, 0x00, 0x44, // mov eax,0x44000000
        0x66, 0xBB, 0x02, 0x00, 0x00, 0x00, // mov ebx,0x2
        0x66, 0xF7, 0xFB,                   // idiv ebx            ; 0x4400_0000 / 2 = 0x2200_0000
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(4);
    assert_eq!(0x2200_0000, machine.cpu.get_r32(R::EAX)); // quotient
    assert_eq!(0x0000_0000, machine.cpu.get_r32(R::EDX)); // remainder
}

#[test]
fn can_execute_lds() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x60,               // mov bx,0x6000
        0xC7, 0x07, 0x22, 0x11,         // mov word [bx],0x1122
        0xC7, 0x47, 0x02, 0x88, 0x66,   // mov word [bx+0x2],0x6688
        0x8C, 0xD9,                     // mov cx,ds   ; backup ds
        0xC5, 0x17,                     // lds dx,[bx]    loads ds and dx with values pointed to at [bx]
        0x8C, 0xD8,                     // mov ax,ds ; save new ds in ax
        0x8E, 0xD9,                     // mov ds,cx   ;  restore ds
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(7);
    assert_eq!(0x1122, machine.cpu.get_r16(R::DX));
    assert_eq!(0x6688, machine.cpu.get_r16(R::AX)); // holds the changed DS value
}

#[test]
fn can_execute_lds_2() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x60,               // mov bx,0x6000
        0xC7, 0x47, 0x06, 0x22, 0x11,   // mov word [bx+0x6],0x1122
        0xC7, 0x47, 0x08, 0x88, 0x66,   // mov word [bx+0x8],0x6688
        0x8C, 0xD9,                     // mov cx,ds   ; backup ds
        0xC5, 0x57, 0x06,               // lds dx,[bx+0x6]   loads ds and dx with values pointed to at [bx+6]
        0x8C, 0xD8,                     // mov ax,ds ; save new ds in ax
        0x8E, 0xD9,                     // mov ds,cx   ;  restore ds
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(7);
    assert_eq!(0x1122, machine.cpu.get_r16(R::DX));
    assert_eq!(0x6688, machine.cpu.get_r16(R::AX)); // holds the changed DS value
}

#[test]
fn can_execute_les() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xC4, 0x06, 0x00, 0x01, // les ax,[0x100]
    ];
    machine.load_executable(&code, 0x085F);
    machine.execute_instruction();
    assert_eq!(0x06C4, machine.cpu.get_r16(R::AX));
    assert_eq!(0x0100, machine.cpu.get_r16(R::ES));
}

#[test]
fn can_execute_cwd() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x00, 0xFE, // mov ax,0xfe00
        0x99,             // cwd
    ];
    machine.load_executable(&code, 0x085F);
    machine.execute_instructions(2);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::DX));
}

#[test]
fn can_execute_aaa() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB0, 0x7E, // mov al,0x7e
        0x37,       // aaa
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0x0104, machine.cpu.get_r16(R::AX));
}

#[test]
fn can_execute_aam() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x44, 0x44,   // mov ax,0x4444
        0xD4, 0x0A,         // aam
        0xB8, 0x00, 0x00,   // mov ax,0x0
        0xD4, 0x0A,         // aam
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xD4, 0x0A,         // aam
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0x0608, machine.cpu.get_r16(R::AX));

    machine.execute_instructions(2);
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));

    machine.execute_instructions(2);
    assert_eq!(0x1905, machine.cpu.get_r16(R::AX));
}

#[test]
fn can_execute_aas() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB0, 0x13, // mov al,0x13
        0x3F,       // aas
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0x0003, machine.cpu.get_r16(R::AX));
}

#[test]
fn can_execute_bts() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x0F, 0xBA, 0x2E, 0xAE, 0x01, 0x0F, // bts word [0x1ae],0xf
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 1);
    assert_eq!("[085F:0100] 0FBA2EAE010F     Bts      word [ds:0x01AE], 0x0F", res);

    // XXX also test emulation
    machine.execute_instructions(1);
}

#[test]
fn can_execute_bsf() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x04, 0x00, // mov ax,0x4
        0x0F, 0xBC, 0xD0, // bsf dx,ax
        0xB8, 0xF0, 0xFF, // mov ax,0xfff0
        0x0F, 0xBC, 0xD0, // bsf dx,ax
        0xB8, 0x00, 0x00, // mov ax,0x0
        0x0F, 0xBC, 0xD0, // bsf dx,ax
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(2, machine.cpu.get_r16(R::DX));
    assert_eq!(false, machine.cpu.regs.flags.zero);

    machine.execute_instructions(2);
    assert_eq!(4, machine.cpu.get_r16(R::DX));
    assert_eq!(false, machine.cpu.regs.flags.zero);

    machine.execute_instructions(2);
    assert_eq!(4, machine.cpu.get_r16(R::DX)); // NOTE: if ax is 0, dx won't change
    assert_eq!(true, machine.cpu.regs.flags.zero);
}

#[test]
fn can_execute_bt() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x02, 0x00, // mov ax,0x2
        0xBA, 0x02, 0x00, // mov dx,0x2
        0x0F, 0xA3, 0xD0, // bt ax,dx
        0xBA, 0x01, 0x00, // mov dx,0x1
        0x0F, 0xA3, 0xD0, // bt ax,dx
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(false, machine.cpu.regs.flags.carry);

    machine.execute_instructions(2);
    assert_eq!(true, machine.cpu.regs.flags.carry);
}

#[test]
fn can_execute_daa() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB0, 0x79, // mov al,0x79
        0xB3, 0x35, // mov bl,0x35
        0x27,      // daa
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0x0079, machine.cpu.get_r16(R::AX)); // XXX, intel manual wants it to be 0x0014
}

#[test]
fn can_execute_das() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB0, 0x35, // mov al,0x35
        0xB3, 0x47, // mov bl,0x47
        0x2F,       // das
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0x0035, machine.cpu.get_r16(R::AX)); // XXX, intel manual wants it to be 0x0088
}

#[test]
fn can_execute_sahf() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x6A, 0x00, // push byte +0x0
        0x9D,       // popf
        0xB4, 0xFF, // mov ah,0xff
        0x9E,       // sahf
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(4);
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.adjust);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
}

#[test]
fn can_execute_pusha_popa() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xE8, 0x03,   // mov ax,0x3e8
        0xB9, 0xE9, 0x03,   // mov cx,0x3e9
        0xBA, 0xEA, 0x03,   // mov dx,0x3ea
        0xBB, 0xEB, 0x03,   // mov bx,0x3eb
        0xBC, 0xEC, 0x03,   // mov sp,0x3ec
        0xBD, 0xED, 0x03,   // mov bp,0x3ed
        0xBE, 0xEE, 0x03,   // mov si,0x3ee
        0xBF, 0xEF, 0x03,   // mov di,0x3ef
        0x60,               // pusha
        0x61,               // popa
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(8);
    machine.execute_instruction(); // pusha
    machine.execute_instruction(); // popa
    assert_eq!(1000, machine.cpu.get_r16(R::AX));
    assert_eq!(1001, machine.cpu.get_r16(R::CX));
    assert_eq!(1002, machine.cpu.get_r16(R::DX));
    assert_eq!(1003, machine.cpu.get_r16(R::BX));
    assert_eq!(1004, machine.cpu.get_r16(R::SP));
    assert_eq!(1005, machine.cpu.get_r16(R::BP));
    assert_eq!(1006, machine.cpu.get_r16(R::SI));
    assert_eq!(1007, machine.cpu.get_r16(R::DI));
}

#[test]
fn can_execute_dec() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBD, 0x00, 0x02, // mov bp,0x200
        0x4D,             // dec bp
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instruction();
    assert_eq!(0x200, machine.cpu.get_r16(R::BP));

    machine.execute_instruction();
    assert_eq!(0x1FF, machine.cpu.get_r16(R::BP));
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(true, machine.cpu.regs.flags.parity);
}

#[test]
fn can_execute_neg() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBB, 0x23, 0x01, // mov bx,0x123
        0xF7, 0xDB,       // neg bx
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instruction();
    assert_eq!(0x0123, machine.cpu.get_r16(R::BX));

    machine.execute_instruction();
    assert_eq!(0xFEDD, machine.cpu.get_r16(R::BX));
    // assert_eq!(true, machine.cpu.regs.flags.carry);  // XXX dosbox = TRUE
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
    assert_eq!(true, machine.cpu.regs.flags.adjust);
    assert_eq!(true, machine.cpu.regs.flags.parity);
}

#[test]
fn can_execute_sbb16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x48, 0xF0, // mov ax,0xf048
        0x1D, 0x45, 0x44, // sbb ax,0x4445
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0xAC03, machine.cpu.get_r16(R::AX));

    // 3286 (xp)     =  0b11_0010_1000_0110
    // 7286 (dosbox) = 0b111_0010_1000_0110
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.adjust);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
}

#[test]
fn can_execute_jmp_far() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xEA, 0x00, 0x06, 0x00, 0x00, // jmp word 0x0:0x600
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 1);
    assert_eq!("[085F:0100] EA00060000       JmpFar   0000:0600", res);

    machine.execute_instruction();
    assert_eq!(0x0000, machine.cpu.get_r16(R::CS));
    assert_eq!(0x0600, machine.cpu.regs.ip);
}

#[test]
fn can_execute_jmp_far_mem() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x31, 0xC0,       // xor ax,ax
        0xBE, 0x88, 0x88, // mov si,0x8888
        0xBB, 0x22, 0x44, // mov bx,0x4422
        0xC6, 0x00, 0x40, // mov byte [bx+si],0x40
        0xFF, 0x28,       // jmp far [bx+si]
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(6);
    assert_eq!(0xCCAB, machine.cpu.regs.ip);
    assert_eq!(0x0001, machine.cpu.get_r16(R::AX));
}

#[test]
fn can_execute_setc() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x0F, 0x92, 0xC0, // setc al
    ];

    machine.load_executable(&code, 0x085F);
    machine.cpu.regs.flags.carry = true;
    machine.execute_instruction();
    assert_eq!(0x01, machine.cpu.get_r8(R::AL));

    machine.load_executable(&code, 0x085F);
    machine.cpu.regs.flags.carry = false;
    machine.execute_instruction();
    assert_eq!(0x00, machine.cpu.get_r8(R::AL));
}

#[test]
fn can_execute_movzx() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB4, 0xFF,       // mov ah,0xff
        0x0F, 0xB6, 0xDC, // movzx bx,ah

    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] B4FF             Mov8     ah, 0xFF
[085F:0102] 0FB6DC           Movzx16  bx, ah",
               res);

    machine.execute_instruction();
    assert_eq!(0xFF, machine.cpu.get_r8(R::AH));

    machine.execute_instruction();
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::BX));
}

#[test]
fn can_execute_rol8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB4, 0xFE,         // mov ah,0xfe
        0xC0, 0xC4, 0x01,   // rol ah,byte 0x1
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xC4, 0xFF,   // rol ah,byte 0xff
        0xB4, 0x01,         // mov ah,0x1
        0xC0, 0xC4, 0x04,   // rol ah,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0xFD, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0xFF, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    // overflow undefined with non-1 shift count

    machine.execute_instructions(2);
    assert_eq!(0x10,  machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    // overflow undefined with non-1 shift count
}

#[test]
fn can_execute_rol16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFE, 0xFF,   // mov ax,0xfffe
        0xC1, 0xC0, 0x01,   // rol ax,byte 0x1
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xC0, 0xFF,   // rol ax,byte 0xff
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xC1, 0xC0, 0x04,   // rol ax,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0xFFFD, machine.cpu.get_r16(R::AX));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    // overflow undefined with non-1 shift count

    machine.execute_instructions(2);
    assert_eq!(0x0010, machine.cpu.get_r16(R::AX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    // overflow undefined with non-1 shift count
}

#[test]
fn can_execute_ror8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB4, 0xFE,         // mov ah,0xfe
        0xC0, 0xCC, 0x01,   // ror ah,byte 0x1
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xCC, 0xFF,   // ror ah,byte 0xff
        0xB4, 0x01,         // mov ah,0x1
        0xC0, 0xCC, 0x04,   // ror ah,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0x7F, machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0xFF, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    // overflow undefined with non-1 shift count

    machine.execute_instructions(2);
    assert_eq!(0x10, machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    // overflow undefined with non-1 shift count
}

#[test]
fn can_execute_ror16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFE, 0xFF,   // mov ax,0xfffe
        0xC1, 0xC8, 0x01,   // ror ax,byte 0x1
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xC8, 0xFF,   // ror ax,byte 0xff
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xC1, 0xC8, 0x04,   // ror ax,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0x7FFF, machine.cpu.get_r16(R::AX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    // overflow undefined with non-1 shift count

    machine.execute_instructions(2);
    assert_eq!(0x1000, machine.cpu.get_r16(R::AX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    // overflow undefined with non-1 shift count
}

#[test]
fn can_execute_rcl8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB4, 0xFE,         // mov ah,0xfe
        0xF9,               // stc
        0xC0, 0xD4, 0x01,   // rcl ah,byte 0x1
        0xB4, 0xFF,         // mov ah,0xff
        0xF9,               // stc
        0xC0, 0xD4, 0xFF,   // rcl ah,byte 0xff
        0xB4, 0x01,         // mov ah,0x1
        0xF9,               // stc
        0xC0, 0xD4, 0x04,   // rcl ah,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0xFD, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(3);
    assert_eq!(0xFF, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(3);
    assert_eq!(0x18, machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
}

#[test]
fn can_execute_rcl16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFE, 0xFF,   // mov ax,0xfffe
        0xF9,               // stc
        0xC1, 0xD0, 0x01,   // rcl ax,byte 0x1
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xF9,               // stc
        0xC1, 0xD0, 0xFF,   // rcl ax,byte 0xff
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xF9,               // stc
        0xC1, 0xD0, 0x04,   // rcl ax,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0xFFFD, machine.cpu.get_r16(R::AX));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(3);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(3);
    assert_eq!(0x0018, machine.cpu.get_r16(R::AX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
}

#[test]
fn can_execute_rcr8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB4, 0xFE,         // mov ah,0xfe
        0xF9,               // stc
        0xC0, 0xDC, 0x01,   // rcr ah,byte 0x1

        0xB4, 0xFE,         // mov ah,0xfe
        0xF8,               // clc
        0xC0, 0xDC, 0x01,   // rcr ah,byte 0x1

        0xB4, 0xFF,         // mov ah,0xff
        0xF9,               // stc
        0xC0, 0xDC, 0xFF,   // rcr ah,byte 0xff

        0xB4, 0x01,         // mov ah,0x1
        0xF9,               // stc
        0xC0, 0xDC, 0x04,   // rcr ah,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0xFF,  machine.cpu.get_r8(R::AH));
    // 3002 = 0b11_0000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(3);
    assert_eq!(0x7F,  machine.cpu.get_r8(R::AH));
    // 3802 = 0b11_1000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(3);
    assert_eq!(0xFF,  machine.cpu.get_r8(R::AH));
    // 3703 = 0b11_0111_0000_0011 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(3);
    assert_eq!(0x30,  machine.cpu.get_r8(R::AH));
    // 3802 = 0b11_1000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);   // XXX win-xp sets overflow here. seems wrong? verify on real hw
}

#[test]
fn can_execute_rcr16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFE, 0xFF,   // mov ax,0xfffe
        0xF9,               // stc
        0xC1, 0xD8, 0x01,   // rcr ax,byte 0x1

        0xB8, 0xFE, 0xFF,   // mov ax,0xfffe
        0xF8,               // sclctc
        0xC1, 0xD8, 0x01,   // rcr ax,byte 0x1

        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xF9,               // stc
        0xC1, 0xD8, 0xFF,   // rcr ax,byte 0xff

        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xF9,               // stc
        0xC1, 0xD8, 0x04,   // rcr ax,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));
    // 3002 = 0b11_0000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(3);
    assert_eq!(0x7FFF, machine.cpu.get_r16(R::AX));
    // 3802 = 0b11_1000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(3);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));
    // 3003 = 0b11_0000_0000_0011 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(3);
    assert_eq!(0x3000, machine.cpu.get_r16(R::AX));
    // 3802 = 0b11_1000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.overflow);  // XXX win-xp sets overflow here. seems wrong? verify on real hw
}

#[test]
fn can_execute_shl8() {
    // XXX shl8 emulation is incomplete / incorrect
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xE4, 0x01,   // shl ah,byte 0x1
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xE4, 0xFF,   // shl ah,byte 0xff
        0xB4, 0x01,         // mov ah,0x1
        0xC0, 0xE4, 0x04,   // shl ah,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0xFE, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    //assert_eq!(false, machine.cpu.regs.flags.overflow); // XXX true in dustbox, false in dosbox?

    machine.execute_instructions(2);
    assert_eq!(0x00, machine.cpu.get_r8(R::AH));
    // assert_eq!(false, machine.cpu.regs.flags.carry); // XXX false in dosbox. true in dustbox!?
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow); // XXX true in dosbox
    // flag bug, reported at https://github.com/joncampbell123/dosbox-x/issues/469
    // win-xp:   flg 3046 = 0b11_0000_0100_0110       xp does not set aux or overflow
    // dosbox-x: flg 0856 =    0b1000_0101_0110       dosbox-x changes aux flag (bug?), and sets overflow (bug?)
    //                           O       A

    machine.execute_instructions(2);
    assert_eq!(0x10, machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
}

#[test]
fn can_execute_shl16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xE0, 0x01,   // shl ax,byte 0x1
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xE0, 0xFF,   // shl ax,byte 0xff
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xC1, 0xE0, 0x04,   // shl ax,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0xFFFE, machine.cpu.get_r16(R::AX));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0x0010, machine.cpu.get_r16(R::AX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
}

#[test]
fn can_execute_shr8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xEC, 0x01,   // shr ah,byte 0x1
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xEC, 0xFF,   // shr ah,byte 0xff
        0xB4, 0x01,         // mov ah,0x1
        0xC0, 0xEC, 0x04,   // shr ah,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0x7F, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(false, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(true, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0x00, machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(true, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0x00, machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
}

#[test]
fn can_execute_shr16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xE8, 0x01,   // shr ax,byte 0x1
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xE8, 0xFF,   // shr ax,byte 0xff
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xC1, 0xE8, 0x04,   // shr ax,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0x7FFF, machine.cpu.get_r16(R::AX));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(true, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(true, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
}

#[test]
fn can_execute_sar8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB4, 0xFE,         // mov ah,0xfe
        0xC0, 0xFC, 0x01,   // sar ah,byte 0x1
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xFC, 0xFF,   // sar ah,byte 0xff
        0xB4, 0x01,         // mov ah,0x1
        0xC0, 0xFC, 0x04,   // sar ah,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0xFF, machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0xFF, machine.cpu.get_r8(R::AH));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0x00, machine.cpu.get_r8(R::AH));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
}

#[test]
fn can_execute_sar16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFE, 0xFF,   // mov ax,0xfffe
        0xC1, 0xF8, 0x01,   // sar ax,byte 0x1
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xF8, 0xFF,   // sar ax,byte 0xff
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xC1, 0xF8, 0x04,   // sar ax,byte 0x4
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));
    assert_eq!(true, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);

    machine.execute_instructions(2);
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.parity);
    assert_eq!(true, machine.cpu.regs.flags.zero);
    assert_eq!(false, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
}

#[test]
fn can_execute_imul8() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB0, 0xFF,     // mov al,0xff
        0xB3, 0x02,     // mov bl,0x2
        0xF6, 0xEB,     // imul bl
        0xB0, 0x00,     // mov al,0x0
        0xB3, 0x02,     // mov bl,0x2
        0xF6, 0xEB,     // imul bl
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0xFFFE, machine.cpu.get_r16(R::AX));
    // 3082

    machine.execute_instructions(3);
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    // 3706

// XXX flags
}

#[test]
fn can_execute_imul16_1_args() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xBB, 0x02, 0x00,   // mov bx,0x2
        0xF7, 0xEB,         // imul bx

        0xB8, 0x00, 0x00,   // mov ax,0x0
        0xBB, 0x02, 0x00,   // mov bx,0x2
        0xF7, 0xEB,         // imul bx

        0xB8, 0xF0, 0x0F,   // mov ax,0xff0
        0xBB, 0xF0, 0x00,   // mov bx,0xf0
        0xF7, 0xEB,         // imul bx
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::DX)); // hi
    assert_eq!(0xFFFE, machine.cpu.get_r16(R::AX)); // lo
    // 3082

    machine.execute_instructions(3);
    assert_eq!(0x0000, machine.cpu.get_r16(R::DX));
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    // 3706

    machine.execute_instructions(3);
    assert_eq!(0x000E, machine.cpu.get_r16(R::DX));
    assert_eq!(0xF100, machine.cpu.get_r16(R::AX));
    // 3887
}

#[test]
fn can_execute_imul16_2_args() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xBB, 0x02, 0x00,   // mov bx,0x2
        0x0F, 0xAF, 0xC3,   // imul ax,bx

        0xB8, 0x00, 0x00,   // mov ax,0x0
        0xBB, 0x02, 0x00,   // mov bx,0x2
        0x0F, 0xAF, 0xC3,   // imul ax,bx

        0xB8, 0xF0, 0x0F,   // mov ax,0xff0
        0xBB, 0xF0, 0x00,   // mov bx,0xf0
        0x0F, 0xAF, 0xC3,   // imul ax,bx
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0xFFFE, machine.cpu.get_r16(R::AX));
    // 3082

    machine.execute_instructions(3);
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    // 3706

    machine.execute_instructions(3);
    assert_eq!(0xF100, machine.cpu.get_r16(R::AX));
    // 3887
}

#[test]
fn can_execute_imul16_3_args() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,       // mov ax,0xffff
        0x6B, 0xC0, 0x02,       // imul ax,ax,byte +0x2

        0xB8, 0x00, 0x00,       // mov ax,0x0
        0x6B, 0xC0, 0x02,       // imul ax,ax,byte +0x2

        0xB8, 0xF0, 0x0F,       // mov ax,0xff0
        0x69, 0xC0, 0xF0, 0x00, // imul ax,ax,word 0xf0
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0xFFFE, machine.cpu.get_r16(R::AX));
    // 3082

    machine.execute_instructions(2);
    assert_eq!(0x0000, machine.cpu.get_r16(R::AX));
    // 3706

    machine.execute_instructions(2);
    assert_eq!(0xF100, machine.cpu.get_r16(R::AX));
    // 3887
}

#[test]
fn can_execute_imul32_1_args() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0xB8, 0xFF, 0xFF, 0x00, 0x00, // mov eax,0xffff
        0x66, 0xBB, 0x02, 0x00, 0x00, 0x00, // mov ebx,0x2
        0x66, 0xF7, 0xEB,                   // imul ebx
        0x66, 0xB8, 0x00, 0x00, 0x00, 0x00, // mov eax,0x0
        0x66, 0xBB, 0x02, 0x00, 0x00, 0x00, // mov ebx,0x2
        0x66, 0xF7, 0xEB,                   // imul ebx
        0x66, 0xB8, 0xF0, 0x0F, 0x00, 0x00, // mov eax,0xff0
        0x66, 0xBB, 0xF0, 0x00, 0x00, 0x00, // mov ebx,0xf0
        0x66, 0xF7, 0xEB,                   // imul ebx
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(3);
    assert_eq!(0x0000_0000, machine.cpu.get_r32(R::EDX)); // hi
    assert_eq!(0x0001_FFFE, machine.cpu.get_r32(R::EAX)); // lo

    machine.execute_instructions(3);
    assert_eq!(0x0000_0000, machine.cpu.get_r32(R::EDX));
    assert_eq!(0x0000_0000, machine.cpu.get_r32(R::EAX));

    machine.execute_instructions(3);
    assert_eq!(0x0000_0000, machine.cpu.get_r32(R::EDX));
    assert_eq!(0x000E_F100, machine.cpu.get_r32(R::EAX));
}

#[test]
fn can_execute_imul32_3_args() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x66, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF,         // mov eax,0xffffffff
        0x66, 0x6B, 0xC0, 0x02,                     // imul eax,eax,byte +0x2
        0x66, 0xB8, 0x00, 0x00, 0x00, 0x00,         // mov eax,0x0
        0x66, 0x6B, 0xC0, 0x02,                     // imul eax,eax,byte +0x2
        0x66, 0xB8, 0xF0, 0x0F, 0x00, 0x00,         // mov eax,0xff0
        0x66, 0x69, 0xC0, 0xF0, 0x00, 0x00, 0x00,   // imul eax,eax,dword 0xf0
    ];
    machine.load_executable(&code, 0x085F);

    machine.execute_instructions(2);
    assert_eq!(0xFFFF_FFFE, machine.cpu.get_r32(R::EAX));

    machine.execute_instructions(2);
    assert_eq!(0x0000_0000, machine.cpu.get_r32(R::EAX));

    machine.execute_instructions(2);
    assert_eq!(0x000E_F100, machine.cpu.get_r32(R::EAX));
}

#[test]
fn can_execute_int_iret() {
    // should hit a default interrupt handler (iret) for int 0x72
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xCD, 0x72, // int 0x72
    ];
    machine.load_executable(&code, 0x085F);

    assert_eq!(0x085F, machine.cpu.get_r16(R::CS));
    machine.execute_instruction();
    assert_eq!(0xF000, machine.cpu.get_r16(R::CS));
    assert_eq!(0x0072, machine.cpu.regs.ip);

    machine.execute_instruction(); // IRET
    assert_eq!(0x085F, machine.cpu.get_r16(R::CS));
    assert_eq!(0x0102, machine.cpu.regs.ip);
}

#[test]
fn can_execute_xlatb() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBB, 0x40, 0x02,               // mov bx,0x240
        0xC6, 0x06, 0x40, 0x02, 0x80,   // mov [0x0240], byte 0x80
        0xD7,                           // xlatb
    ];
    machine.load_executable(&code, 0x085F);
    machine.execute_instructions(3);
    assert_eq!(0x80, machine.cpu.get_r8(R::AL)); // al = [ds:bx]
}

#[test]
fn can_execute_cmpsw() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBE, 0x30, 0x30,           // mov si,0x3030
        0xC7, 0x04, 0x22, 0x22,     // mov word [si],0x2222
        0xBF, 0x40, 0x30,           // mov di,0x3040
        0xC7, 0x05, 0x11, 0x11,     // mov word [di],0x1111
        0xA7,                       // cmpsw   ; compare byte at address DS:(E)SI with byte at address ES:(E)DI
    ];
    machine.load_executable(&code, 0x085F);
    machine.execute_instructions(5);
    // xxx only results in regs ...
    // dosbox regs:
    //assert_eq!(false, machine.cpu.regs.flags.carry); // XXX
    //assert_eq!(false, machine.cpu.regs.flags.zero);
    //assert_eq!(false, machine.cpu.regs.flags.sign);
    //assert_eq!(true, machine.cpu.regs.flags.overflow);
    //assert_eq!(false, machine.cpu.regs.flags.adjust);
    //assert_eq!(true, machine.cpu.regs.flags.parity);
}

#[test]
fn can_execute_shld() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBB, 0x88, 0x44,           // mov bx,0x4488
        0xBF, 0x33, 0x22,           // mov di,0x2233
        0x0F, 0xA4, 0xFB, 0x08,     // shld bx,di,0x8
    ];
    machine.load_executable(&code, 0x085F);
    machine.execute_instructions(3);
    assert_eq!(0x8822, machine.cpu.get_r16(R::BX));
    assert_eq!(false, machine.cpu.regs.flags.carry);
    assert_eq!(true, machine.cpu.regs.flags.overflow);
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    // assert_eq!(false, machine.cpu.regs.flags.adjust); // XXX dosbox: C0 Z0 S1 O1 A0 P1
    assert_eq!(true, machine.cpu.regs.flags.parity);
}

#[test]
fn can_execute_scasb() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x00, 0x40,       // mov ax,0x4000
        0x8E, 0xC0,             // mov es,ax
        0xBF, 0x00, 0x00,       // mov di,0x0
        0x26, 0xC6, 0x05, 0xFF, // mov byte [es:di],0xff
        0xB0, 0xFF,             // mov al,0xff
        0xAE                    // scasb
    ];
    machine.load_executable(&code, 0x085F);
    machine.execute_instructions(6);
    assert_eq!(0x0001, machine.cpu.get_r16(R::DI));
}

#[test]
fn can_execute_scasw() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x00, 0x40,               // mov ax,0x4000
        0x8E, 0xC0,                     // mov es,ax
        0xBF, 0x00, 0x00,               // mov di,0x0
        0x26, 0xC7, 0x05, 0xFF, 0xFF,   // mov word [es:di],0xffff
        0xB8, 0xFF, 0xFF,               // mov ax,0xffff
        0xAF                            // scasw
    ];
    machine.load_executable(&code, 0x085F);
    machine.execute_instructions(6);
    assert_eq!(0x0002, machine.cpu.get_r16(R::DI));
}

#[test]
fn can_execute_movsx16() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB7, 0xFE,             // mov bh,0xfe
        0x0F, 0xBE, 0xC7,       // movsx ax,bh
    ];
    machine.load_executable(&code, 0x085F);
    machine.execute_instructions(2);
    assert_eq!(0xFFFE, machine.cpu.get_r16(R::AX));
}

#[test]
fn can_execute_movsx32() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB7, 0xFE,             // mov bh,0xfe
        0x66, 0x0F, 0xBE, 0xC7, // movsx eax,bh
    ];
    machine.load_executable(&code, 0x085F);
    machine.execute_instructions(2);
    assert_eq!(0xFFFF_FFFE, machine.cpu.get_r32(R::EAX));
}

#[test]
fn can_execute_mov_ds_addressing() {
    // NOTE: this test demonstrates a emulation bug described in https://github.com/martinlindhe/dustbox-rs/issues/9#issuecomment-355609424
    // BUG: "mov [bx+si],dx" writes to the CS segment instead of DS
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x68, 0x00, 0x80,   // push word 0x8000
        0x1F,               // pop ds
        0xBB, 0x10, 0x00,   // mov bx,0x10
        0xBE, 0x01, 0x00,   // mov si,0x1
        0xBA, 0x99, 0x99,   // mov dx,0x9999
        0x89, 0x10,         // mov [bx+si],dx
    ];
    machine.load_executable(&code, 0x085F);
    
    machine.execute_instructions(6);

    let cs = machine.cpu.get_r16(R::CS);
    let ds = machine.cpu.get_r16(R::DS);
    assert_eq!(0x0000, machine.mmu.read_u16(cs, 0x10 + 0x1));
    assert_eq!(0x9999, machine.mmu.read_u16(ds, 0x10 + 0x1));
}

#[test]
fn can_execute_shrd() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,       // mov ax,0xffff
        0xBA, 0xFF, 0xFF,       // mov dx,0xffff
        0x0F, 0xAC, 0xD0, 0x0E, // shrd ax,dx,0xe
    ];
    machine.load_executable(&code, 0x085F);

    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] B8FFFF           Mov16    ax, 0xFFFF
[085F:0103] BAFFFF           Mov16    dx, 0xFFFF
[085F:0106] 0FACD00E         Shrd     ax, dx, 0x0E",
                res);

    machine.execute_instruction();
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));

    machine.execute_instruction();
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::DX));

    machine.execute_instruction();
    assert_eq!(0xFFFF, machine.cpu.get_r16(R::AX));

    // assert_eq!(true, machine.cpu.regs.flags.carry); xxx should be set
    assert_eq!(false, machine.cpu.regs.flags.zero);
    assert_eq!(true, machine.cpu.regs.flags.sign);
    assert_eq!(false, machine.cpu.regs.flags.overflow);
    assert_eq!(false, machine.cpu.regs.flags.adjust);
    assert_eq!(true, machine.cpu.regs.flags.parity);
}

#[test]
fn can_execute_ret_imm() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xE8, 0x03, 0x00,   // 000100: call 0x106
        0xB9, 0x34, 0x12,   // 000103: mov cx,0x1234
        0xC2, 0x01, 0x00,   // 000106: ret 0x1
    ];
    machine.load_executable(&code, 0x085F);

    let stack_offset = machine.cpu.get_r16(R::SP);
    assert_eq!(0x0100, machine.cpu.regs.ip);
    assert_eq!(stack_offset, machine.cpu.get_r16(R::SP));

    machine.execute_instruction(); // call
    assert_eq!(0x0106, machine.cpu.regs.ip);
    assert_eq!(stack_offset - 2, machine.cpu.get_r16(R::SP));

    machine.execute_instruction(); // ret 0x1
    assert_eq!(stack_offset + 1, machine.cpu.get_r16(R::SP));
    assert_eq!(0x0103, machine.cpu.regs.ip);
}

#[test]
fn can_execute_call_far() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x9A, 0xD3, 0x00, 0x00, 0x00,   // call 0x0:0xd3
    ];
    machine.load_executable(&code, 0x085F);
    machine.execute_instruction(); // call

    assert_eq!(0x0000, machine.cpu.get_r16(R::CS));
    assert_eq!(0x00D3, machine.cpu.regs.ip);
}

#[test]
fn can_execute_sldt() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x0F, 0x00, 0x00,   // sldt [bx+si]
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 1);
    assert_eq!("[085F:0100] 0F0000           Sldt     word [ds:bx+si]", res);

    machine.execute_instruction();

    // XXX actually test emulation
}

#[test]
fn can_execute_operand_prefix() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x67, 0xC7, 0x02, 0x22, 0x44,   // mov word [edx],0x4422
        0x67, 0x8B, 0x02,               // mov ax,[edx]
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 2);
    assert_eq!("[085F:0100] 67C7022244       Mov16    word [ds:edx], 0x4422
[085F:0105] 678B02           Mov16    ax, word [ds:edx]", res);

    machine.execute_instruction();
    machine.execute_instruction();
    assert_eq!(0x4422, machine.cpu.get_r16(R::AX));
}

#[test]
fn can_execute_operand_and_address_prefix() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0x67, 0xC7, 0x02, 0x22, 0x44,                   // mov word [edx],0x4422
        0x66, 0x67, 0x81, 0x02, 0x00, 0x00, 0x33, 0x88, // add dword [edx],0x88330000
        0x66, 0x67, 0x8B, 0x02,                         // mov eax,[edx]
    ];
    machine.load_executable(&code, 0x085F);
    let res = machine.cpu.decoder.disassemble_block_to_str(&mut machine.mmu, 0x85F, 0x100, 3);
    assert_eq!("[085F:0100] 67C7022244       Mov16    word [ds:edx], 0x4422
[085F:0105] 6667810200003388 Add32    dword [ds:edx], 0x88330000
[085F:010D] 66678B02         Mov32    eax, dword [ds:edx]", res);

    machine.execute_instructions(3);
    assert_eq!(0x88334422, machine.cpu.get_r32(R::EAX));
}

#[test]
fn estimate_mips() {
    use std::time::Instant;

    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB9, 0xFF, 0xFF, // mov cx,0xffff
        0x49,             // dec cx
        0xEB, 0xFA,       // jmp short 0x100
    ];

    machine.load_executable(&code, 0x085F);

    // run for 1 sec
    const RUN_SECONDS: u64 = 1;
    let start = Instant::now();
    loop {
        machine.execute_instruction();
        if  start.elapsed().as_secs() >= RUN_SECONDS {
            break
        }
    }

    let mips = (machine.cpu.instruction_count as f64) / 1_000_000.;
    println!("MIPS: {}", mips);
}
