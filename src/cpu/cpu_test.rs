use std::num::Wrapping;

use cpu::CPU;
use cpu::register::{R8, R16, SR};
use cpu::segment::Segment;
use memory::mmu::MMU;

#[test]
fn can_handle_stack() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0x88, 0x88, // mov ax,0x8888
        0x8E, 0xD8,       // mov ds,ax
        0x1E,             // push ds
        0x07,             // pop es
    ];
    cpu.load_com(&code);

    cpu.execute_instruction(); // mov
    cpu.execute_instruction(); // mov

    assert_eq!(0xFFFE, cpu.get_r16(&R16::SP));
    cpu.execute_instruction(); // push
    assert_eq!(0xFFFC, cpu.get_r16(&R16::SP));
    cpu.execute_instruction(); // pop
    assert_eq!(0xFFFE, cpu.get_r16(&R16::SP));

    assert_eq!(0x107, cpu.ip);
    assert_eq!(0x8888, cpu.get_r16(&R16::AX));
    assert_eq!(0x8888, cpu.get_sr(&SR::DS));
    assert_eq!(0x8888, cpu.get_sr(&SR::ES));
}

#[test]
fn can_execute_add8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0x00, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0x00, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0xFF, cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.auxiliary_carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0xFE, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.parity);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
}

#[test]
fn can_execute_add16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.auxiliary_carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0xFFFE, cpu.get_r16(&R16::AX));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.parity);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
}

#[test]
fn can_execute_mov_r8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB2, 0x13, // mov dl,0x13
        0x88, 0xD0, // mov al,dl
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x13, cpu.get_r8(&R8::DL));

    cpu.execute_instruction();
    assert_eq!(0x13, cpu.get_r8(&R8::AL));
}

#[test]
fn can_execute_mov_r8_rm8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x05, 0x01, // mov bx,0x105
        0x8A, 0x27,       // mov ah,[bx]   | r8, r/m8
        0x99,             // db 0x99
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x105, cpu.get_r16(&R16::BX));

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x99, cpu.get_r8(&R8::AH));
}

#[test]
fn can_execute_mv_r16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0x23, 0x01, // mov ax,0x123
        0x8B, 0xE0,       // mov sp,ax   | r16, r16
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x123, cpu.get_r16(&R16::AX));

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x123, cpu.get_r16(&R16::SP));
}

#[test]
fn can_execute_mov_r16_rm16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB9, 0x23, 0x01, // mov cx,0x123
        0x8E, 0xC1,       // mov es,cx   | r/m16, r16
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x123, cpu.get_r16(&R16::CX));

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x123, cpu.get_sr(&SR::ES));
}

#[test]
fn can_execute_mov_rm16_sreg() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x34, 0x12,       // mov bx,0x1234
        0x8E, 0xC3,             // mov es,bx
        0x8C, 0x06, 0x09, 0x01, // mov [0x109],es  | r/m16, sreg
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x1234, cpu.get_r16(&R16::BX));

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x1234, cpu.get_sr(&SR::ES));

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);
    let cs = cpu.get_sr(&SR::CS);
    assert_eq!(0x1234, cpu.mmu.read_u16(cs, 0x0109));
}

#[test]
fn can_execute_mov_data() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xC6, 0x06, 0x31, 0x10, 0x38,       // mov byte [0x1031],0x38
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    let cs = cpu.get_sr(&SR::CS);
    assert_eq!(0x38, cpu.mmu.read_u8(cs, 0x1031));
}

#[test]
fn can_execute_mov_es_segment() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(4);

    let es = cpu.get_sr(&SR::ES);
    let di = cpu.get_r16(&R16::DI);
    cpu.execute_instruction(); // mov [es:di],ah
    assert_eq!(0x88, cpu.mmu.read_u8(es, di));

    cpu.execute_instruction(); // mov al,[es:di]
    assert_eq!(0x88, cpu.get_r8(&R8::AL));

    cpu.mmu.write_u8(es, di + 1, 0x1);
    cpu.mmu.write_u8(es, di - 1, 0xFF);
    cpu.execute_instruction(); // mov al,[es:di+0x1]
    assert_eq!(0x1, cpu.get_r8(&R8::AL));
    cpu.execute_instruction(); // mov bl,[es:di-0x1]
    assert_eq!(0xFF, cpu.get_r8(&R8::BL));

    cpu.mmu.write_u8(es, di + 0x140, 0x22);
    cpu.mmu.write_u8(es, di - 0x140, 0x88);
    cpu.execute_instruction(); // mov al,[es:di+0x140]
    assert_eq!(0x22, cpu.get_r8(&R8::AL));
    cpu.execute_instruction(); // mov bl,[es:di-0x140]
    assert_eq!(0x88, cpu.get_r8(&R8::BL));
}

#[test]
fn can_execute_mov_fs_segment() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x68, 0x40, 0x40,   // push word 0x4040
        0x0F, 0xA1,         // pop fs
        0xBF, 0x00, 0x02,   // mov di,0x200
        0xB0, 0xFF,         // mov al,0xff

        0x64, 0x88, 0x05,   // mov [fs:di],al
    ];

    cpu.load_com(&code);
    cpu.execute_instructions(5); // mov [fs:di],al
    assert_eq!(0xFF, cpu.mmu.read_u8(cpu.get_sr(&SR::FS), cpu.get_r16(&R16::DI)));
}

#[test]
fn can_execute_imms8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBF, 0x00, 0x01, // mov di,0x100
        0x83, 0xC7, 0x3A, // add di,byte +0x3a
        0x83, 0xC7, 0xC6, // add di,byte -0x3a
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x0100, cpu.get_r16(&R16::DI));

    cpu.execute_instruction();
    assert_eq!(0x106, cpu.ip);
    assert_eq!(0x013A, cpu.get_r16(&R16::DI));

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);
    assert_eq!(0x0100, cpu.get_r16(&R16::DI));
}

#[test]
fn can_execute_with_flags() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB4, 0xFE,       // mov ah,0xfe
        0x80, 0xC4, 0x02, // add ah,0x2   - OF and ZF should be set
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x102, cpu.ip);
    assert_eq!(0xFE, cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(false, cpu.flags.auxiliary_carry);
    assert_eq!(false, cpu.flags.parity);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x00, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.parity);
}

#[test]
fn can_execute_cmp() {
    // make sure we dont overflow (0 - 0x2000 = overflow)
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x00,       // mov bx,0x0
        0x89, 0xDF,             // mov di,bx
        0x81, 0xFF, 0x00, 0x20, // cmp di,0x2000
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0, cpu.get_r16(&R16::BX));

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0, cpu.get_r16(&R16::DI));

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);

    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(false, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.parity);
}

#[test]
fn can_execute_xchg() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0x34, 0x12,   // mov ax,0x1234
        0xB9, 0xFF, 0xFF,   // mov cx,0xffff
        0x91,               // xchg ax,cx
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));
    assert_eq!(0x1234, cpu.get_r16(&R16::CX));
}

#[test]
fn can_execute_rep_movsb() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBE, 0x00, 0x01,   // mov si,0x100
        0xBF, 0x00, 0x02,   // mov di,0x200
        0xB9, 0x04, 0x00,   // mov cx,0x4
        0xF3, 0xA4,         // rep movsb
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(3);

    // copy first 4 bytes from DS:0x100 to ES:0x200
    cpu.execute_instructions(4);
    cpu.execute_instruction(); // rep movsb
    cpu.execute_instruction(); // rep movsb
    cpu.execute_instruction(); // rep movsb
    assert_eq!(0x0, cpu.get_r16(&R16::CX));
    let min = 0x100;
    let max = min + 4;
    for i in min..max {
        assert_eq!(
            cpu.mmu.read_u8(cpu.get_sr(&SR::ES), i),
            cpu.mmu.read_u8(cpu.get_sr(&SR::ES), i+0x100));
    }
}

#[test]
fn can_execute_rep_outsb() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBE, 0x00, 0x01,   // mov si,0x100
        0xBA, 0xC8, 0x03,   // mov dx,0x3c8
        0xB9, 0x02, 0x00,   // mov cx,0x2
        0xF3, 0x6E,         // rep outsb
    ];
    cpu.load_com(&code);

    assert_eq!(0, cpu.gpu.pel_address);

    cpu.execute_instructions(3);
    cpu.execute_instruction(); // rep outsb
    assert_eq!(0xBE, cpu.gpu.pel_address);

    cpu.execute_instruction(); // rep outsb
    assert_eq!(0x00, cpu.gpu.pel_address);

    assert_eq!(0x0, cpu.get_r16(&R16::CX));
}

#[test]
fn can_execute_es_outsb() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x68, 0x00, 0x80,       // push word 0x8000
        0x07,                   // pop es
        0xBE, 0x00, 0x01,       // mov si,0x100
        0x26, 0xC6, 0x04, 0x09, // mov byte [es:si],0x9
        0xBA, 0xC8, 0x03,       // mov dx,0x3c8
        0x26, 0x6E,             // es outsb
    ];
    cpu.load_com(&code);

    assert_eq!(0, cpu.gpu.pel_address);
    cpu.execute_instructions(6);
    assert_eq!(0x09, cpu.gpu.pel_address);
}

#[test]
fn can_execute_lea() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x44, 0x44,           // mov bx,0x4444
        0x8D, 0x3F,                 // lea di,[bx]
        0x8D, 0x36, 0x33, 0x22,     // lea si,[0x2233]
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0x4444, cpu.get_r16(&R16::DI));

    cpu.execute_instruction();
    assert_eq!(0x2233, cpu.get_r16(&R16::SI));
}

#[test]
fn can_execute_addressing() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x02,             // mov bx,0x200
        0xC6, 0x47, 0x2C, 0xFF,       // mov byte [bx+0x2c],0xff  ; rm8 [amode+s8]
        0x8B, 0x14,                   // mov dx,[si]              ; rm16 [reg]
        0x8B, 0x47, 0x2C,             // mov ax,[bx+0x2c]         ; rm16 [amode+s8]
        0x89, 0x87, 0x30, 0x00,       // mov [bx+0x0030],ax       ; rm [amode+s16]
        0x89, 0x05,                   // mov [di],ax              ; rm16 [amode]
        0xC6, 0x85, 0xAE, 0x06, 0xFE, // mov byte [di+0x6ae],0xfe ; rm8 [amode+s16]
        0x8A, 0x85, 0xAE, 0x06,       // mov al,[di+0x6ae]
    ];

    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 8);
    assert_eq!("[085F:0100] BB0002           Mov16    bx, 0x0200
[085F:0103] C6472CFF         Mov8     byte [ds:bx+0x2C], 0xFF
[085F:0107] 8B14             Mov16    dx, word [ds:si]
[085F:0109] 8B472C           Mov16    ax, word [ds:bx+0x2C]
[085F:010C] 89873000         Mov16    word [ds:bx+0x0030], ax
[085F:0110] 8905             Mov16    word [ds:di], ax
[085F:0112] C685AE06FE       Mov8     byte [ds:di+0x06AE], 0xFE
[085F:0117] 8A85AE06         Mov8     al, byte [ds:di+0x06AE]",
               res);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.get_r16(&R16::BX));

    cpu.execute_instruction();
    let ds = cpu.get_sr(&SR::DS);
    assert_eq!(0xFF, cpu.mmu.read_u8(ds, 0x22C));

    cpu.execute_instruction();
    // should have read word at [0x100]
    assert_eq!(0x00BB, cpu.get_r16(&R16::DX));

    cpu.execute_instruction();
    // should have read word at [0x22C]
    assert_eq!(0x00FF, cpu.get_r16(&R16::AX));

    cpu.execute_instruction();
    // should have written word to [0x230]
    assert_eq!(0x00FF, cpu.mmu.read_u16(ds, 0x230));

    cpu.execute_instruction();
    // should have written ax to [di]
    let di = cpu.get_r16(&R16::DI);
    assert_eq!(0x00FF, cpu.mmu.read_u16(ds, di));

    cpu.execute_instruction();
    // should have written byte to [di+0x06AE]
    assert_eq!(0xFE, cpu.mmu.read_u8(ds, (Wrapping(di) +
                                     Wrapping(0x06AE)).0));

    cpu.execute_instruction();
    // should have read byte from [di+0x06AE] to al
    assert_eq!(0xFE, cpu.get_r8(&R8::AL));
}

#[test]
fn can_execute_math() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xF6, 0x06, 0x2C, 0x12, 0xFF, // test byte [0x122c],0xff
    ];

    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 1);
    assert_eq!("[085F:0100] F6062C12FF       Test8    byte [ds:0x122C], 0xFF",
               res);

    // XXX also execute
}

#[test]
fn can_execute_and() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB0, 0xF0, // mov al,0xF0
        0xB4, 0x1F, // mov ah,0x1F
        0x20, 0xC4, // and ah,al
    ];

    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 3);
    assert_eq!("[085F:0100] B0F0             Mov8     al, 0xF0
[085F:0102] B41F             Mov8     ah, 0x1F
[085F:0104] 20C4             And8     ah, al",
               res);

    cpu.execute_instruction();
    assert_eq!(0xF0, cpu.get_r8(&R8::AL));

    cpu.execute_instruction();
    assert_eq!(0x1F, cpu.get_r8(&R8::AH));

    cpu.execute_instruction();
    assert_eq!(0x10, cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.parity);
}

#[test]
fn can_execute_mul8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB0, 0x40, // mov al,0x40
        0xB3, 0x10, // mov bl,0x10
        0xF6, 0xE3, // mul bl
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0x400, cpu.get_r16(&R16::AX));
    // XXX flags
}

#[test]
fn can_execute_mul16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0x00, 0x80, // mov ax,0x8000
        0xBB, 0x04, 0x00, // mov bx,0x4
        0xF7, 0xE3,       // mul bx
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0x0002, cpu.get_r16(&R16::DX));
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    // XXX flags
}

#[test]
fn can_execute_div8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0x40, 0x00, // mov ax,0x40
        0xB3, 0x10,       // mov bl,0x10
        0xF6, 0xF3,       // div bl
    ];
    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 3);
    assert_eq!("[085F:0100] B84000           Mov16    ax, 0x0040
[085F:0103] B310             Mov8     bl, 0x10
[085F:0105] F6F3             Div8     bl",
               res);

    cpu.execute_instruction();
    assert_eq!(0x40, cpu.get_r8(&R8::AL));

    cpu.execute_instruction();
    assert_eq!(0x10, cpu.get_r8(&R8::BL));

    cpu.execute_instruction();
    assert_eq!(0x04, cpu.get_r8(&R8::AL)); // quotient
    assert_eq!(0x00, cpu.get_r8(&R8::AH)); // remainder
}

#[test]
fn can_execute_div16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBA, 0x10, 0x00, // mov dx,0x10
        0xB8, 0x00, 0x40, // mov ax,0x4000
        0xBB, 0x00, 0x01, // mov bx,0x100
        0xF7, 0xF3,       // div bx
    ];
    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 4);
    assert_eq!("[085F:0100] BA1000           Mov16    dx, 0x0010
[085F:0103] B80040           Mov16    ax, 0x4000
[085F:0106] BB0001           Mov16    bx, 0x0100
[085F:0109] F7F3             Div16    bx",
               res);

    cpu.execute_instruction();
    assert_eq!(0x10, cpu.get_r16(&R16::DX));

    cpu.execute_instruction();
    assert_eq!(0x4000, cpu.get_r16(&R16::AX));

    cpu.execute_instruction();
    assert_eq!(0x100, cpu.get_r16(&R16::BX));

    cpu.execute_instruction();
    assert_eq!(0x1040, cpu.get_r16(&R16::AX)); // quotient
    assert_eq!(0x0000, cpu.get_r16(&R16::DX)); // remainder
}

#[test]
fn can_execute_idiv8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0x00, cpu.get_r8(&R8::AL)); // quotient
    assert_eq!(0x00, cpu.get_r8(&R8::AH)); // remainder

    cpu.execute_instructions(3);
    assert_eq!(0x00, cpu.get_r8(&R8::AL));
    assert_eq!(0xFF, cpu.get_r8(&R8::AH));

    cpu.execute_instructions(3);
    assert_eq!(0x00, cpu.get_r8(&R8::AL));
    assert_eq!(0x01, cpu.get_r8(&R8::AH));
}

#[test]
fn can_execute_idiv16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(4);
    assert_eq!(0x8000, cpu.get_r16(&R16::AX)); // quotient
    assert_eq!(0x0000, cpu.get_r16(&R16::DX)); // remainder

    cpu.execute_instructions(4);
    assert_eq!(0x7FFF, cpu.get_r16(&R16::AX));
    assert_eq!(0x0001, cpu.get_r16(&R16::DX));

    cpu.execute_instructions(4);
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    assert_eq!(0x0001, cpu.get_r16(&R16::DX));

    cpu.execute_instructions(4);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));
    assert_eq!(0x0000, cpu.get_r16(&R16::DX));
}

#[test]
fn can_execute_les() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xC4, 0x06, 0x00, 0x01, // les ax,[0x100]
    ];
    cpu.load_com(&code);
    cpu.execute_instruction();
    assert_eq!(0x06C4, cpu.get_r16(&R16::AX));
    assert_eq!(0x0100, cpu.get_sr(&SR::ES));
}

#[test]
fn can_execute_cwd() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0x00, 0xFE, // mov ax,0xfe00
        0x99,             // cwd
    ];
    cpu.load_com(&code);
    cpu.execute_instructions(2);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::DX));
}

#[test]
fn can_execute_aaa() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB0, 0x7E, // mov al,0x7e
        0x37,       // aaa
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0x0104, cpu.get_r16(&R16::AX));
}

#[test]
fn can_execute_aam() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0x44, 0x44,   // mov ax,0x4444
        0xD4, 0x0A,         // aam
        0xB8, 0x00, 0x00,   // mov ax,0x0
        0xD4, 0x0A,         // aam
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xD4, 0x0A,         // aam
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0x0608, cpu.get_r16(&R16::AX));

    cpu.execute_instructions(2);
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));

    cpu.execute_instructions(2);
    assert_eq!(0x1905, cpu.get_r16(&R16::AX));
}

#[test]
fn can_execute_aas() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB0, 0x13, // mov al,0x13
        0x3F,       // aas
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0x0003, cpu.get_r16(&R16::AX));
}

#[test]
fn can_execute_bsf() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0x04, 0x00, // mov ax,0x4
        0x0F, 0xBC, 0xD0, // bsf dx,ax
        0xB8, 0xF0, 0xFF, // mov ax,0xfff0
        0x0F, 0xBC, 0xD0, // bsf dx,ax
        0xB8, 0x00, 0x00, // mov ax,0x0
        0x0F, 0xBC, 0xD0, // bsf dx,ax
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(2, cpu.get_r16(&R16::DX));
    assert_eq!(false, cpu.flags.zero);

    cpu.execute_instructions(2);
    assert_eq!(4, cpu.get_r16(&R16::DX));
    assert_eq!(false, cpu.flags.zero);

    cpu.execute_instructions(2);
    assert_eq!(4, cpu.get_r16(&R16::DX)); // NOTE: if ax is 0, dx won't change
    assert_eq!(true, cpu.flags.zero);
}

#[test]
fn can_execute_bt() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0x02, 0x00, // mov ax,0x2
        0xBA, 0x02, 0x00, // mov dx,0x2
        0x0F, 0xA3, 0xD0, // bt ax,dx
        0xBA, 0x01, 0x00, // mov dx,0x1
        0x0F, 0xA3, 0xD0, // bt ax,dx
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(false, cpu.flags.carry);

    cpu.execute_instructions(2);
    assert_eq!(true, cpu.flags.carry);
}

#[test]
fn can_execute_daa() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB0, 0x79, // mov al,0x79
        0xB3, 0x35, // mov bl,0x35
        0x27,      // daa
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0x0079, cpu.get_r16(&R16::AX)); // XXX, intel manual wants it to be 0x0014
}

#[test]
fn can_execute_das() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB0, 0x35, // mov al,0x35
        0xB3, 0x47, // mov bl,0x47
        0x2F,       // das
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0x0035, cpu.get_r16(&R16::AX)); // XXX, intel manual wants it to be 0x0088
}

#[test]
fn can_execute_sahf() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x6A, 0x00, // push byte +0x0
        0x9D,       // popf
        0xB4, 0xFF, // mov ah,0xff
        0x9E,       // sahf
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(4);
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
}

#[test]
fn can_execute_pusha_popa() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(8);
    cpu.execute_instruction(); // pusha
    cpu.execute_instruction(); // popa
    assert_eq!(1000, cpu.get_r16(&R16::AX));
    assert_eq!(1001, cpu.get_r16(&R16::CX));
    assert_eq!(1002, cpu.get_r16(&R16::DX));
    assert_eq!(1003, cpu.get_r16(&R16::BX));
    assert_eq!(1004, cpu.get_r16(&R16::SP));
    assert_eq!(1005, cpu.get_r16(&R16::BP));
    assert_eq!(1006, cpu.get_r16(&R16::SI));
    assert_eq!(1007, cpu.get_r16(&R16::DI));
}

#[test]
fn can_execute_dec() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBD, 0x00, 0x02, // mov bp,0x200
        0x4D,             // dec bp
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.get_r16(&R16::BP));

    cpu.execute_instruction();
    assert_eq!(0x1FF, cpu.get_r16(&R16::BP));
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(true, cpu.flags.parity);
}

#[test]
fn can_execute_neg() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x23, 0x01, // mov bx,0x123
        0xF7, 0xDB,       // neg bx
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x0123, cpu.get_r16(&R16::BX));

    cpu.execute_instruction();
    assert_eq!(0xFEDD, cpu.get_r16(&R16::BX));
    // assert_eq!(true, cpu.flags.carry);  // XXX dosbox = TRUE
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.parity);
}

#[test]
fn can_execute_sbb16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0x48, 0xF0, // mov ax,0xf048
        0x1D, 0x45, 0x44, // sbb ax,0x4445
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0xAC03, cpu.get_r16(&R16::AX));

    // 3286 (xp)     =  0b11_0010_1000_0110
    // 7286 (dosbox) = 0b111_0010_1000_0110
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.auxiliary_carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
}

#[test]
fn can_execute_jmp_far() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xEA, 0x00, 0x06, 0x00, 0x00, // jmp word 0x0:0x600
    ];
    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 1);
    assert_eq!("[085F:0100] EA00060000       JmpFar   0000:0600",
               res);

    cpu.execute_instruction();
    assert_eq!(0x0000, cpu.get_sr(&SR::CS));
    assert_eq!(0x0600, cpu.ip);
}

#[test]
fn can_execute_setc() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x0F, 0x92, 0xC0, // setc al
    ];

    cpu.load_com(&code);
    cpu.flags.carry = true;
    cpu.execute_instruction();
    assert_eq!(0x01, cpu.get_r8(&R8::AL));

    cpu.load_com(&code);
    cpu.flags.carry = false;
    cpu.execute_instruction();
    assert_eq!(0x00, cpu.get_r8(&R8::AL));
}

#[test]
fn can_execute_movzx() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB4, 0xFF,       // mov ah,0xff
        0x0F, 0xB6, 0xDC, // movzx bx,ah

    ];
    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 2);
    assert_eq!("[085F:0100] B4FF             Mov8     ah, 0xFF
[085F:0102] 0FB6DC           Movzx16  bx, ah",
               res);

    cpu.execute_instruction();
    assert_eq!(0xFF, cpu.get_r8(&R8::AH));

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.get_r16(&R16::BX));
}

#[test]
fn can_execute_rol8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB4, 0xFE,         // mov ah,0xfe
        0xC0, 0xC4, 0x01,   // rol ah,byte 0x1
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xC4, 0xFF,   // rol ah,byte 0xff
        0xB4, 0x01,         // mov ah,0x1
        0xC0, 0xC4, 0x04,   // rol ah,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0xFD, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0xFF, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    // overflow undefined with non-1 shift count

    cpu.execute_instructions(2);
    assert_eq!(0x10,  cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.carry);
    // overflow undefined with non-1 shift count
}

#[test]
fn can_execute_rol16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0xFE, 0xFF,   // mov ax,0xfffe
        0xC1, 0xC0, 0x01,   // rol ax,byte 0x1
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xC0, 0xFF,   // rol ax,byte 0xff
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xC1, 0xC0, 0x04,   // rol ax,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0xFFFD, cpu.get_r16(&R16::AX));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));
    assert_eq!(true, cpu.flags.carry);
    // overflow undefined with non-1 shift count

    cpu.execute_instructions(2);
    assert_eq!(0x0010, cpu.get_r16(&R16::AX));
    assert_eq!(false, cpu.flags.carry);
    // overflow undefined with non-1 shift count
}

#[test]
fn can_execute_ror8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB4, 0xFE,         // mov ah,0xfe
        0xC0, 0xCC, 0x01,   // ror ah,byte 0x1
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xCC, 0xFF,   // ror ah,byte 0xff
        0xB4, 0x01,         // mov ah,0x1
        0xC0, 0xCC, 0x04,   // ror ah,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0x7F, cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0xFF, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    // overflow undefined with non-1 shift count

    cpu.execute_instructions(2);
    assert_eq!(0x10, cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.carry);
    // overflow undefined with non-1 shift count
}

#[test]
fn can_execute_ror16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0xFE, 0xFF,   // mov ax,0xfffe
        0xC1, 0xC8, 0x01,   // ror ax,byte 0x1
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xC8, 0xFF,   // ror ax,byte 0xff
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xC1, 0xC8, 0x04,   // ror ax,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0x7FFF, cpu.get_r16(&R16::AX));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));
    assert_eq!(true, cpu.flags.carry);
    // overflow undefined with non-1 shift count

    cpu.execute_instructions(2);
    assert_eq!(0x1000, cpu.get_r16(&R16::AX));
    assert_eq!(false, cpu.flags.carry);
    // overflow undefined with non-1 shift count
}

#[test]
fn can_execute_rcl8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0xFD, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(3);
    assert_eq!(0xFF, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(3);
    assert_eq!(0x18, cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);
}

#[test]
fn can_execute_rcl16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0xFFFD, cpu.get_r16(&R16::AX));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(3);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(3);
    assert_eq!(0x0018, cpu.get_r16(&R16::AX));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);
}

#[test]
fn can_execute_rcr8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0xFF,  cpu.get_r8(&R8::AH));
    // 3002 = 0b11_0000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(3);
    assert_eq!(0x7F,  cpu.get_r8(&R8::AH));
    // 3802 = 0b11_1000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.overflow);

    cpu.execute_instructions(3);
    assert_eq!(0xFF,  cpu.get_r8(&R8::AH));
    // 3703 = 0b11_0111_0000_0011 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(3);
    assert_eq!(0x30,  cpu.get_r8(&R8::AH));
    // 3802 = 0b11_1000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);   // XXX win-xp sets overflow here. seems wrong? verify on real hw
}

#[test]
fn can_execute_rcr16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));
    // 3002 = 0b11_0000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(3);
    assert_eq!(0x7FFF, cpu.get_r16(&R16::AX));
    // 3802 = 0b11_1000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.overflow);

    cpu.execute_instructions(3);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));
    // 3003 = 0b11_0000_0000_0011 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(3);
    assert_eq!(0x3000, cpu.get_r16(&R16::AX));
    // 3802 = 0b11_1000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);  // XXX win-xp sets overflow here. seems wrong? verify on real hw
}

#[test]
fn can_execute_shl8() {
    // XXX shl8 emulation is incomplete / incorrect
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xE4, 0x01,   // shl ah,byte 0x1
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xE4, 0xFF,   // shl ah,byte 0xff
        0xB4, 0x01,         // mov ah,0x1
        0xC0, 0xE4, 0x04,   // shl ah,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0xFE, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    //assert_eq!(false, cpu.flags.overflow); // XXX true in dustbox, false in dosbox?

    cpu.execute_instructions(2);
    assert_eq!(0x00, cpu.get_r8(&R8::AH));
    // assert_eq!(false, cpu.flags.carry); // XXX false in dosbox. true in dustbox!?
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow); // XXX true in dosbox
    // flag bug, reported at https://github.com/joncampbell123/dosbox-x/issues/469
    // win-xp:   flg 3046 = 0b11_0000_0100_0110       xp does not set aux or overflow
    // dosbox-x: flg 0856 =    0b1000_0101_0110       dosbox-x changes aux flag (bug?), and sets overflow (bug?)
    //                           O       A

    cpu.execute_instructions(2);
    assert_eq!(0x10, cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
}

#[test]
fn can_execute_shl16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xE0, 0x01,   // shl ax,byte 0x1
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xE0, 0xFF,   // shl ax,byte 0xff
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xC1, 0xE0, 0x04,   // shl ax,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0xFFFE, cpu.get_r16(&R16::AX));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0x0010, cpu.get_r16(&R16::AX));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
}

#[test]
fn can_execute_shr8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xEC, 0x01,   // shr ah,byte 0x1
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xEC, 0xFF,   // shr ah,byte 0xff
        0xB4, 0x01,         // mov ah,0x1
        0xC0, 0xEC, 0x04,   // shr ah,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0x7F, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(true, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0x00, cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(true, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0x00, cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
}

#[test]
fn can_execute_shr16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xE8, 0x01,   // shr ax,byte 0x1
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xE8, 0xFF,   // shr ax,byte 0xff
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xC1, 0xE8, 0x04,   // shr ax,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0x7FFF, cpu.get_r16(&R16::AX));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(true, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(true, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
}

#[test]
fn can_execute_sar8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB4, 0xFE,         // mov ah,0xfe
        0xC0, 0xFC, 0x01,   // sar ah,byte 0x1
        0xB4, 0xFF,         // mov ah,0xff
        0xC0, 0xFC, 0xFF,   // sar ah,byte 0xff
        0xB4, 0x01,         // mov ah,0x1
        0xC0, 0xFC, 0x04,   // sar ah,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0xFF, cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0xFF, cpu.get_r8(&R8::AH));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0x00, cpu.get_r8(&R8::AH));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
}

#[test]
fn can_execute_sar16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0xFE, 0xFF,   // mov ax,0xfffe
        0xC1, 0xF8, 0x01,   // sar ax,byte 0x1
        0xB8, 0xFF, 0xFF,   // mov ax,0xffff
        0xC1, 0xF8, 0xFF,   // sar ax,byte 0xff
        0xB8, 0x01, 0x00,   // mov ax,0x1
        0xC1, 0xF8, 0x04,   // sar ax,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    cpu.execute_instructions(2);
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
}

#[test]
fn can_execute_imul8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB0, 0xFF,     // mov al,0xff
        0xB3, 0x02,     // mov bl,0x2
        0xF6, 0xEB,     // imul bl
        0xB0, 0x00,     // mov al,0x0
        0xB3, 0x02,     // mov bl,0x2
        0xF6, 0xEB,     // imul bl
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0xFFFE, cpu.get_r16(&R16::AX));
    // 3082

    cpu.execute_instructions(3);
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    // 3706

// XXX flags
}

#[test]
fn can_execute_imul16_1_args() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::DX)); // hi
    assert_eq!(0xFFFE, cpu.get_r16(&R16::AX)); // lo
    // 3082

    cpu.execute_instructions(3);
    assert_eq!(0x0000, cpu.get_r16(&R16::DX));
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    // 3706

    cpu.execute_instructions(3);
    assert_eq!(0x000E, cpu.get_r16(&R16::DX));
    assert_eq!(0xF100, cpu.get_r16(&R16::AX));
    // 3887
}

#[test]
fn can_execute_imul16_2_args() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
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
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    assert_eq!(0xFFFE, cpu.get_r16(&R16::AX));
    // 3082

    cpu.execute_instructions(3);
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    // 3706

    cpu.execute_instructions(3);
    assert_eq!(0xF100, cpu.get_r16(&R16::AX));
    // 3887
}

#[test]
fn can_execute_imul16_3_args() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,       // mov ax,0xffff
        0x6B, 0xC0, 0x02,       // imul ax,ax,byte +0x2

        0xB8, 0x00, 0x00,       // mov ax,0x0
        0x6B, 0xC0, 0x02,       // imul ax,ax,byte +0x2

        0xB8, 0xF0, 0x0F,       // mov ax,0xff0
        0x69, 0xC0, 0xF0, 0x00, // imul ax,ax,word 0xf0
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(2);
    assert_eq!(0xFFFE, cpu.get_r16(&R16::AX));
    // 3082

    cpu.execute_instructions(2);
    assert_eq!(0x0000, cpu.get_r16(&R16::AX));
    // 3706

    cpu.execute_instructions(2);
    assert_eq!(0xF100, cpu.get_r16(&R16::AX));
    // 3887
}

#[test]
fn can_execute_xlatb() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x40, 0x02,               // mov bx,0x240
        0xC6, 0x06, 0x40, 0x02, 0x80,   // mov [0x0240], byte 0x80
        0xD7,                           // xlatb
    ];
    cpu.load_com(&code);
    cpu.execute_instructions(3);
    assert_eq!(0x80, cpu.get_r8(&R8::AL)); // al = [ds:bx]
}

#[test]
fn can_execute_shld() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x88, 0x44,           // mov bx,0x4488
        0xBF, 0x33, 0x22,           // mov di,0x2233
        0x0F, 0xA4, 0xFB, 0x08,     // shld bx,di,0x8
    ];
    cpu.load_com(&code);
    cpu.execute_instructions(3);
    assert_eq!(0x8822, cpu.get_r16(&R16::BX));
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.overflow);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    // assert_eq!(false, cpu.flags.auxiliary_carry); // XXX dosbox: C0 Z0 S1 O1 A0 P1
    assert_eq!(true, cpu.flags.parity);
}

/*
#[test]
fn can_execute_lds() {
    // STATUS: broken
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x60,           // mov bx,0x6000
        0xC7, 0x07, 0x22, 0x11,     // mov word [bx],0x1122
        0xC5, 0x17,                 // lds dx,[bx]
    ];
    cpu.load_com(&code);

    cpu.execute_instructions(3);
    // XXX writes to registers ds and dx.
    // dx = value from [bx]. ds = segment selector
    assert_eq!(0x1122, cpu.get_r16(&R16::DX));
    assert_eq!(0x18CC, cpu.get_sr(&SR::DS));   // XXX ?! "segment selector". dustbox = 0x6000. dosbox = 0x18cc. winxp = 0x150a
}
*/

#[test]
fn can_execute_movsx() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB7, 0xFF,       // mov bh,0xff
        0x0F, 0xBE, 0xC7, // movsx ax,bh
    ];
    cpu.load_com(&code);
    cpu.execute_instructions(2);
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));
}

#[test]
fn can_execute_mov_ds_addressing() {
    // NOTE: this test demonstrates a emulation bug described in https://github.com/martinlindhe/dustbox-rs/issues/9#issuecomment-355609424
    // BUG: "mov [bx+si],dx" writes to the CS segment instead of DS
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x68, 0x00, 0x80,   // push word 0x8000
        0x1F,               // pop ds
        0xBB, 0x10, 0x00,   // mov bx,0x10
        0xBE, 0x01, 0x00,   // mov si,0x1
        0xBA, 0x99, 0x99,   // mov dx,0x9999
        0x89, 0x10,         // mov [bx+si],dx
    ];
    cpu.load_com(&code);
    
    cpu.execute_instructions(6);

    let cs = cpu.get_sr(&SR::CS);
    let ds = cpu.get_sr(&SR::DS);
    assert_eq!(0x0000, cpu.mmu.read_u16(cs, 0x10 + 0x1));
    assert_eq!(0x9999, cpu.mmu.read_u16(ds, 0x10 + 0x1));
}

#[test]
fn can_execute_shrd() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,       // mov ax,0xffff
        0xBA, 0xFF, 0xFF,       // mov dx,0xffff
        0x0F, 0xAC, 0xD0, 0x0E, // shrd ax,dx,0xe
    ];
    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 3);
    assert_eq!("[085F:0100] B8FFFF           Mov16    ax, 0xFFFF
[085F:0103] BAFFFF           Mov16    dx, 0xFFFF
[085F:0106] 0FACD00E         Shrd     ax, dx, 0x0E",
                res);

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.get_r16(&R16::DX));

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.get_r16(&R16::AX));

    // assert_eq!(true, cpu.flags.carry); xxx should be set
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(false, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.parity);
}

#[test]
fn can_execute_ret_imm() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xE8, 0x00, 0x00,   // call 0x103
        0x50,               // push ax
        0xC2, 0x02, 0x00,   // ret 0x2
    ];
    cpu.load_com(&code);

    assert_eq!(0xFFFE, cpu.get_r16(&R16::SP));
    cpu.execute_instruction(); // call
    assert_eq!(0xFFFC, cpu.get_r16(&R16::SP));
    cpu.execute_instruction(); // push
    assert_eq!(0xFFFA, cpu.get_r16(&R16::SP));
    cpu.execute_instruction(); // ret 0x2
    assert_eq!(0xFFFE, cpu.get_r16(&R16::SP));
}

#[test]
fn can_execute_imul16_1_arg() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x8F, 0x79, // mov bx,0x798f
        0xB8, 0xD9, 0xFF, // mov ax,0xffd9
        0xF7, 0xEB,       // imul bx
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x798F, cpu.get_r16(&R16::BX));

    cpu.execute_instruction();
    assert_eq!(0xFFD9, cpu.get_r16(&R16::AX));

    cpu.execute_instruction();
    assert_eq!(0xFFED, cpu.get_r16(&R16::DX));
    assert_eq!(0x7B37, cpu.get_r16(&R16::AX));
}

#[test]
fn estimate_mips() {
    use std::time::Instant;

    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB9, 0xFF, 0xFF, // mov cx,0xffff
        0x49,             // dec cx
        0xEB, 0xFA,       // jmp short 0x100
    ];

    cpu.load_com(&code);

    // run for 1 sec
    const RUN_SECONDS: u64 = 1;
    let start = Instant::now();
    loop {
        cpu.execute_instruction();
        if  start.elapsed().as_secs() >= RUN_SECONDS {
            break
        }
    }

    let mips = (cpu.instruction_count as f64) / 1_000_000.;
    println!("MIPS: {}", mips);
}
