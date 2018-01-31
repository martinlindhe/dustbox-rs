use cpu::CPU;
use register::{AX, BX, CX, DX, SI, DI, BP, SP, CS, DS, ES, FS};
use segment::Segment;
use mmu::MMU;
use std::num::Wrapping;

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

    assert_eq!(0xFFFE, cpu.r16[SP].val);
    cpu.execute_instruction(); // push
    assert_eq!(0xFFFC, cpu.r16[SP].val);
    cpu.execute_instruction(); // pop
    assert_eq!(0xFFFE, cpu.r16[SP].val);

    assert_eq!(0x107, cpu.ip);
    assert_eq!(0x8888, cpu.r16[AX].val);
    assert_eq!(0x8888, cpu.sreg16[DS]);
    assert_eq!(0x8888, cpu.sreg16[ES]);
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
    assert_eq!(0x102, cpu.ip);
    assert_eq!(0x13, cpu.r16[DX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x104, cpu.ip);
    assert_eq!(0x13, cpu.r16[AX].lo_u8());
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
    assert_eq!(0x105, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x99, cpu.r16[AX].hi_u8());
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
    assert_eq!(0x123, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x123, cpu.r16[SP].val);
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
    assert_eq!(0x123, cpu.r16[CX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x123, cpu.sreg16[ES]);
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
    assert_eq!(0x1234, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x1234, cpu.sreg16[ES]);

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);
    let cs = cpu.sreg16[CS];
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
    let cs = cpu.sreg16[CS];
    assert_eq!(0x38, cpu.mmu.read_u8(cs, 0x1031));
}

#[test]
fn can_execute_mov_es_segment() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x26, 0x88, 0x25, // mov [es:di],ah
        0x26, 0x8A, 0x05, // mov al,[es:di]

        0x26, 0x8A, 0x45, 0x01, // mov al,[es:di+0x1]
        0x26, 0x8A, 0x5D, 0xFF, // mov bl,[es:di-0x1]

        0x26, 0x8A, 0x85, 0x40, 0x01, // mov al,[es:di+0x140]
        0x26, 0x8A, 0x9D, 0xC0, 0xFE, // mov bl,[es:di-0x140]
    ];

    cpu.load_com(&code);
    let es = 0x4040;
    let di = 0x0200;
    cpu.sreg16[ES] = es;
    cpu.r16[DI].val = di;

    cpu.r16[AX].set_hi(0x88);
    cpu.execute_instruction(); // mov [es:di],ah
    assert_eq!(0x88, cpu.mmu.read_u8(es, di));

    cpu.execute_instruction(); // mov al,[es:di]
    assert_eq!(0x88, cpu.r16[AX].lo_u8());

    cpu.mmu.write_u8(es, di + 1, 0x1);
    cpu.mmu.write_u8(es, di - 1, 0xFF);
    cpu.execute_instruction(); // mov al,[es:di+0x1]
    assert_eq!(0x1, cpu.r16[AX].lo_u8());
    cpu.execute_instruction(); // mov bl,[es:di-0x1]
    assert_eq!(0xFF, cpu.r16[BX].lo_u8());

    cpu.mmu.write_u8(es, di + 0x140, 0x22);
    cpu.mmu.write_u8(es, di - 0x140, 0x88);
    cpu.execute_instruction(); // mov al,[es:di+0x140]
    assert_eq!(0x22, cpu.r16[AX].lo_u8());
    cpu.execute_instruction(); // mov bl,[es:di-0x140]
    assert_eq!(0x88, cpu.r16[BX].lo_u8());
}

#[test]
fn can_execute_mov_fs_segment() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x64, 0x88, 0x05, // mov [fs:di],al
    ];

    cpu.load_com(&code);
    let fs = 0x4040;
    let di = 0x0200;
    cpu.sreg16[FS] = fs;
    cpu.r16[DI].val = di;
    cpu.r16[AX].set_lo(0xFF);
    cpu.execute_instruction(); // mov [fs:di],al
    assert_eq!(0xFF, cpu.mmu.read_u8(fs, di));
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
    assert_eq!(0x0100, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x106, cpu.ip);
    assert_eq!(0x013A, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);
    assert_eq!(0x0100, cpu.r16[DI].val);
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
    assert_eq!(0xFE, cpu.r16[AX].hi_u8());
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(false, cpu.flags.auxiliary_carry);
    assert_eq!(false, cpu.flags.parity);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x00, cpu.r16[AX].hi_u8());
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
    assert_eq!(0, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0, cpu.r16[DI].val);

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
        0x91, // xchg ax,cx
    ];

    cpu.load_com(&code);

    cpu.r16[AX].val = 0x1234;
    cpu.r16[CX].val = 0xFFFF;

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.r16[AX].val);
    assert_eq!(0x1234, cpu.r16[CX].val);
}

#[test]
fn can_execute_rep_movsb() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xF3, 0xA4,             // rep movsb
    ];
    cpu.load_com(&code);

    cpu.r16[SI].val = 0x100;
    cpu.r16[DI].val = 0x200;
    cpu.r16[CX].val = 4;

    // copy first 4 bytes from DS:0x100 to ES:0x200
    cpu.execute_instruction(); // rep movsb
    cpu.execute_instruction(); // rep movsb
    cpu.execute_instruction(); // rep movsb
    cpu.execute_instruction(); // rep movsb
    assert_eq!(0x0, cpu.r16[CX].val);
    let min = 0x100;
    let max = min + 4;
    for i in min..max {
        assert_eq!(
            cpu.mmu.read_u8(cpu.sreg16[ES], i),
            cpu.mmu.read_u8(cpu.sreg16[ES], i+0x100));
    }
}

#[test]
fn can_execute_rep_outsb() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xF3, 0x6E,       // rep outsb
    ];
    cpu.load_com(&code);

    cpu.r16[SI].val = 0x100;
    cpu.r16[DX].val = 0x03C8;
    cpu.r16[CX].val = 2;

    assert_eq!(0, cpu.gpu.pel_address);

    cpu.execute_instruction(); // rep outsb
    assert_eq!(0xF3, cpu.gpu.pel_address);

    cpu.execute_instruction(); // rep outsb
    assert_eq!(0x6E, cpu.gpu.pel_address);

    assert_eq!(0x0, cpu.r16[CX].val);
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
    execute_instructions(&mut cpu, 6);
    assert_eq!(0x09, cpu.gpu.pel_address);
}

#[test]
fn can_execute_addressing() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x02,             // mov bx,0x200
        0xC6, 0x47, 0x2C, 0xFF,       // mov byte [bx+0x2c],0xff  ; rm8 [amode+s8]
        0x8D, 0x36, 0x00, 0x01,       // lea si,[0x100]
        0x8B, 0x14,                   // mov dx,[si]              ; rm16 [reg]
        0x8B, 0x47, 0x2C,             // mov ax,[bx+0x2c]         ; rm16 [amode+s8]
        0x89, 0x87, 0x30, 0x00,       // mov [bx+0x0030],ax       ; rm [amode+s16]
        0x89, 0x05,                   // mov [di],ax              ; rm16 [amode]
        0xC6, 0x85, 0xAE, 0x06, 0xFE, // mov byte [di+0x6ae],0xfe ; rm8 [amode+s16]
        0x8A, 0x85, 0xAE, 0x06,       // mov al,[di+0x6ae]
    ];

    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 9);
    assert_eq!("[085F:0100] BB0002           Mov16    bx, 0x0200
[085F:0103] C6472CFF         Mov8     byte [ds:bx+0x2C], 0xFF
[085F:0107] 8D360001         Lea16    si, word [ds:0x0100]
[085F:010B] 8B14             Mov16    dx, word [ds:si]
[085F:010D] 8B472C           Mov16    ax, word [ds:bx+0x2C]
[085F:0110] 89873000         Mov16    word [ds:bx+0x0030], ax
[085F:0114] 8905             Mov16    word [ds:di], ax
[085F:0116] C685AE06FE       Mov8     byte [ds:di+0x06AE], 0xFE
[085F:011B] 8A85AE06         Mov8     al, byte [ds:di+0x06AE]",
               res);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.r16[BX].val);

    cpu.execute_instruction();
    let ds = cpu.sreg16[DS];
    assert_eq!(0xFF, cpu.mmu.read_u8(ds, 0x22C));

    cpu.execute_instruction();
    assert_eq!(0x100, cpu.r16[SI].val);

    cpu.execute_instruction();
    // should have read word at [0x100]
    assert_eq!(0x00BB, cpu.r16[DX].val);

    cpu.execute_instruction();
    // should have read word at [0x22C]
    assert_eq!(0x00FF, cpu.r16[AX].val);

    cpu.execute_instruction();
    // should have written word to [0x230]
    assert_eq!(0x00FF, cpu.mmu.read_u16(ds, 0x230));

    cpu.execute_instruction();
    // should have written ax to [di]
    let di = cpu.r16[DI].val;
    assert_eq!(0x00FF, cpu.mmu.read_u16(ds, di));

    cpu.execute_instruction();
    // should have written byte to [di+0x06AE]
    assert_eq!(0xFE, cpu.mmu.read_u8(ds, (Wrapping(di) +
                                     Wrapping(0x06AE)).0));

    cpu.execute_instruction();
    // should have read byte from [di+0x06AE] to al
    assert_eq!(0xFE, cpu.r16[AX].lo_u8());
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
    assert_eq!(0xF0, cpu.r16[AX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x1F, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0x10, cpu.r16[AX].hi_u8());
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

    execute_instructions(&mut cpu, 3);
    assert_eq!(0x400, cpu.r16[AX].val);
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

    execute_instructions(&mut cpu, 3);
    assert_eq!(0x0002, cpu.r16[DX].val);
    assert_eq!(0x0000, cpu.r16[AX].val);
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
    assert_eq!(0x40, cpu.r16[AX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x10, cpu.r16[BX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x04, cpu.r16[AX].lo_u8()); // quotient
    assert_eq!(0x00, cpu.r16[AX].hi_u8()); // remainder
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
    assert_eq!(0x10, cpu.r16[DX].val);

    cpu.execute_instruction();
    assert_eq!(0x4000, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0x100, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x1040, cpu.r16[AX].val); // quotient
    assert_eq!(0x0000, cpu.r16[DX].val); // remainder
}

#[test]
fn can_execute_idiv8() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB8, 0x07, 0x00, // mov ax,0x7
        0xB3, 0x03,       // mov bl,0x3
        0xF6, 0xFB,       // idiv bl
    ];
    cpu.load_com(&code);

    execute_instructions(&mut cpu, 3);
    assert_eq!(0x02, cpu.r16[AX].lo_u8()); // quotient
    assert_eq!(0x01, cpu.r16[AX].hi_u8()); // remainder
}

#[test]
fn can_execute_idiv16() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBA, 0x00, 0x00, // mov dx,0x0
        0xB8, 0x07, 0x00, // mov ax,0x7
        0xBB, 0x03, 0x00, // mov bx,0x3
        0xF7, 0xFB,       // idiv bx (dx:ax / bx)
    ];
    cpu.load_com(&code);

    execute_instructions(&mut cpu, 4);
    assert_eq!(0x0002, cpu.r16[AX].val); // quotient
    assert_eq!(0x0001, cpu.r16[DX].val); // remainder
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
    assert_eq!(0x06C4, cpu.r16[AX].val);
    assert_eq!(0x0100, cpu.sreg16[ES]);
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
    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFFFF, cpu.r16[DX].val);
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x0104, cpu.r16[AX].val);
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x0608, cpu.r16[AX].val);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x0000, cpu.r16[AX].val);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x1905, cpu.r16[AX].val);
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x0003, cpu.r16[AX].val);
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(2, cpu.r16[DX].val);
    assert_eq!(false, cpu.flags.zero);

    execute_instructions(&mut cpu, 2);
    assert_eq!(4, cpu.r16[DX].val);
    assert_eq!(false, cpu.flags.zero);

    execute_instructions(&mut cpu, 2);
    assert_eq!(4, cpu.r16[DX].val); // NOTE: if ax is 0, dx won't change
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

    execute_instructions(&mut cpu, 3);
    assert_eq!(false, cpu.flags.carry);

    execute_instructions(&mut cpu, 2);
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

    execute_instructions(&mut cpu, 3);
    assert_eq!(0x0079, cpu.r16[AX].val); // XXX, intel manual wants it to be 0x0014
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

    execute_instructions(&mut cpu, 3);
    assert_eq!(0x0035, cpu.r16[AX].val); // XXX, intel manual wants it to be 0x0088
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

    execute_instructions(&mut cpu, 4);
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
        0x60,       // pusha
        0x61,       // popa
    ];
    cpu.load_com(&code);

    cpu.r16[AX].val = 1000;
    cpu.r16[CX].val = 1001;
    cpu.r16[DX].val = 1002;
    cpu.r16[BX].val = 1003;
    cpu.r16[SP].val = 1004;
    cpu.r16[BP].val = 1005;
    cpu.r16[SI].val = 1006;
    cpu.r16[DI].val = 1007;
    cpu.execute_instruction(); // pusha
    cpu.execute_instruction(); // popa
    assert_eq!(1000, cpu.r16[AX].val);
    assert_eq!(1001, cpu.r16[CX].val);
    assert_eq!(1002, cpu.r16[DX].val);
    assert_eq!(1003, cpu.r16[BX].val);
    assert_eq!(1004, cpu.r16[SP].val);
    assert_eq!(1005, cpu.r16[BP].val);
    assert_eq!(1006, cpu.r16[SI].val);
    assert_eq!(1007, cpu.r16[DI].val);
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
    assert_eq!(0x200, cpu.r16[BP].val);

    cpu.execute_instruction();
    assert_eq!(0x1FF, cpu.r16[BP].val);
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
    assert_eq!(0x0123, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0xFEDD, cpu.r16[BX].val);
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xAC03, cpu.r16[AX].val);

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
    assert_eq!(0x0000, cpu.sreg16[CS]);
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
    assert_eq!(0x01, cpu.r16[AX].lo_u8());

    cpu.load_com(&code);
    cpu.flags.carry = false;
    cpu.execute_instruction();
    assert_eq!(0x00, cpu.r16[AX].lo_u8());
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
    assert_eq!(0xFF, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.r16[BX].val);
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFD, cpu.r16[AX].hi_u8());
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFF, cpu.r16[AX].hi_u8());
    assert_eq!(true, cpu.flags.carry);
    // overflow undefined with non-1 shift count

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x10,  cpu.r16[AX].hi_u8());
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFFFD, cpu.r16[AX].val);
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFFFF, cpu.r16[AX].val);
    assert_eq!(true, cpu.flags.carry);
    // overflow undefined with non-1 shift count

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x0010, cpu.r16[AX].val);
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x7F, cpu.r16[AX].hi_u8());
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFF, cpu.r16[AX].hi_u8());
    assert_eq!(true, cpu.flags.carry);
    // overflow undefined with non-1 shift count

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x10, cpu.r16[AX].hi_u8());
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x7FFF, cpu.r16[AX].val);
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFFFF, cpu.r16[AX].val);
    assert_eq!(true, cpu.flags.carry);
    // overflow undefined with non-1 shift count

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x1000, cpu.r16[AX].val);
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

    execute_instructions(&mut cpu, 3);
    assert_eq!(0xFD, cpu.r16[AX].hi_u8());
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 3);
    assert_eq!(0xFF, cpu.r16[AX].hi_u8());
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 3);
    assert_eq!(0x18, cpu.r16[AX].hi_u8());
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

    execute_instructions(&mut cpu, 3);
    assert_eq!(0xFFFD, cpu.r16[AX].val);
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 3);
    assert_eq!(0xFFFF, cpu.r16[AX].val);
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 3);
    assert_eq!(0x0018, cpu.r16[AX].val);
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

    execute_instructions(&mut cpu, 3);
    assert_eq!(0xFF,  cpu.r16[AX].hi_u8());
    // 3002 = 0b11_0000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 3);
    assert_eq!(0x7F,  cpu.r16[AX].hi_u8());
    // 3802 = 0b11_1000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.overflow);

    execute_instructions(&mut cpu, 3);
    assert_eq!(0xFF,  cpu.r16[AX].hi_u8());
    // 3703 = 0b11_0111_0000_0011 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 3);
    assert_eq!(0x30,  cpu.r16[AX].hi_u8());
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

    execute_instructions(&mut cpu, 3);
    assert_eq!(0xFFFF, cpu.r16[AX].val);
    // 3002 = 0b11_0000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 3);
    assert_eq!(0x7FFF, cpu.r16[AX].val);
    // 3802 = 0b11_1000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.overflow);

    execute_instructions(&mut cpu, 3);
    assert_eq!(0xFFFF, cpu.r16[AX].val);
    // 3003 = 0b11_0000_0000_0011 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 3);
    assert_eq!(0x3000, cpu.r16[AX].val);
    // 3802 = 0b11_1000_0000_0010 (xp)
    //        ____ O___ SZ_A _P_C
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow);  // XXX win-xp sets overflow here. seems wrong? verify on real hw
}

#[test]
fn can_execute_shl8() {
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFE, cpu.r16[AX].hi_u8());
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x00, cpu.r16[AX].hi_u8());
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    // flag bug, reported at https://github.com/joncampbell123/dosbox-x/issues/469
    // win-xp:   flg 3046 = 0b11_0000_0100_0110       xp does not set aux or overflow
    // dosbox-x: flg 0856 =    0b1000_0101_0110       dosbox-x changes aux flag (bug?), and sets overflow (bug?)
    //                           O       A

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x10, cpu.r16[AX].hi_u8());
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFFFE, cpu.r16[AX].val);
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x0000, cpu.r16[AX].val);
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x0010, cpu.r16[AX].val);
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x7F, cpu.r16[AX].hi_u8());
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(true, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x00, cpu.r16[AX].hi_u8());
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(true, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x00, cpu.r16[AX].hi_u8());
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x7FFF, cpu.r16[AX].val);
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(true, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x0000, cpu.r16[AX].val);
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(true, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x0000, cpu.r16[AX].val);
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFF, cpu.r16[AX].hi_u8());
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFF, cpu.r16[AX].hi_u8());
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x00, cpu.r16[AX].hi_u8());
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

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFFFF, cpu.r16[AX].val);
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFFFF, cpu.r16[AX].val);
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);

    execute_instructions(&mut cpu, 2);
    assert_eq!(0x0000, cpu.r16[AX].val);
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
        0xB0, 0xF0, // mov al,0xf0
        0xB7, 0xD0, // mov bh,0xd0
        0xF6, 0xEF, // imul bh
    ];
    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 3);
    assert_eq!("[085F:0100] B0F0             Mov8     al, 0xF0
[085F:0102] B7D0             Mov8     bh, 0xD0
[085F:0104] F6EF             Imul8    bh",
               res);

    cpu.execute_instruction();
    assert_eq!(0xF0, cpu.r16[AX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0xD0, cpu.r16[BX].hi_u8());

    cpu.execute_instruction();
    // AX = AL ∗ r/m byte.
    assert_eq!(0x0300, cpu.r16[AX].val);
    // XXX Carry & overflow is true in dosbox
}

#[test]
fn can_execute_imul16_2_args() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB9, 0x0A, 0x00, // mov cx,0xa
        0xBF, 0x14, 0x00, // mov di,0x14
        0x0F, 0xAF, 0xCF, // imul cx,di
    ];
    cpu.load_com(&code);
    execute_instructions(&mut cpu, 3);
    assert_eq!(0x00C8, cpu.r16[CX].val);
}

#[test]
fn can_execute_imul16_3_args() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBF, 0xFF, 0x8F,       // mov di,0x8fff
        0x69, 0xFF, 0x40, 0x01, // imul di,di,word 0x140
    ];
    cpu.load_com(&code);
    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFEC0, cpu.r16[DI].val);
    // XXX Carry & overflow is true in dosbox
}

#[test]
fn can_execute_xlatb() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x40, 0x02, // mov bx,0x240
        0xD7,             // xlatb
    ];
    cpu.load_com(&code);
    // prepare ds:bx with expected value
    let ds = cpu.sreg16[DS];
    cpu.mmu.write_u16(ds, 0x0240, 0x80);

    execute_instructions(&mut cpu, 2); // xlatb: al = [ds:bx]
    assert_eq!(0x80, cpu.r16[AX].lo_u8());
}

#[test]
fn can_execute_movsx() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB7, 0xFF,       // mov bh,0xff
        0x0F, 0xBE, 0xC7, // movsx ax,bh
    ];
    cpu.load_com(&code);
    execute_instructions(&mut cpu, 2);
    assert_eq!(0xFFFF, cpu.r16[AX].val);
}

#[test]
fn can_execute_mov_ds_addressing() {
    // NOTE: this test demonstrates a emulation bug described in https://github.com/martinlindhe/dustbox-rs/issues/9#issuecomment-355609424
    // BUG: "mov [bx+si],dx" writes to the CS segment instead of DS
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xBB, 0x10, 0x00, // mov bx,0x10
        0xBE, 0x01, 0x00, // mov si,0x1
        0xBA, 0x99, 0x99, // mov dx,0x9999
        0x89, 0x10,       // mov [bx+si],dx
    ];
    cpu.load_com(&code);
    cpu.sreg16[DS] = 0x8000;
    execute_instructions(&mut cpu, 4);

    let cs = cpu.sreg16[CS];
    let ds = cpu.sreg16[DS];
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
    assert_eq!(0xFFFF, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.r16[DX].val);

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.r16[AX].val);

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

    assert_eq!(0xFFFE, cpu.r16[SP].val);
    cpu.execute_instruction(); // call
    assert_eq!(0xFFFC, cpu.r16[SP].val);
    cpu.execute_instruction(); // push
    assert_eq!(0xFFFA, cpu.r16[SP].val);
    cpu.execute_instruction(); // ret 0x2
    assert_eq!(0xFFFE, cpu.r16[SP].val);
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
    assert_eq!(0x798F, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0xFFD9, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0xFFED, cpu.r16[DX].val);
    assert_eq!(0x7B37, cpu.r16[AX].val);
}

#[test]
fn can_disassemble_basic() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xE8, 0x05, 0x00, // call l_0x108   ; call a later offset
        0xBA, 0x0B, 0x01, // mov dx,0x10b
        0xB4, 0x09,       // mov ah,0x9
        0xCD, 0x21,       // l_0x108: int 0x21
        0xE8, 0xFB, 0xFF, // call l_0x108   ; call an earlier offset
    ];
    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 5);
    assert_eq!("[085F:0100] E80500           CallNear 0x0108
[085F:0103] BA0B01           Mov16    dx, 0x010B
[085F:0106] B409             Mov8     ah, 0x09
[085F:0108] CD21             Int      0x21
[085F:010A] E8FBFF           CallNear 0x0108",
               res);
}

#[test]
fn can_disassemble_lea() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x8D, 0x47, 0x80, // lea ax,[bx-0x80]
 ];
    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 1);
    assert_eq!("[085F:0100] 8D4780           Lea16    ax, word [ds:bx-0x80]",
               res);
}

#[test]
fn can_disassemble_segment_prefixed() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x26, 0x88, 0x25, // mov [es:di],ah
        0x26, 0x8A, 0x25, // mov ah,[es:di]
    ];
    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 2);
    assert_eq!("[085F:0100] 268825           Mov8     byte [es:di], ah
[085F:0103] 268A25           Mov8     ah, byte [es:di]",
               res);
}

#[test]
fn can_disassemble_values() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x81, 0xC7, 0xC0, 0x00,       // add di,0xc0
        0x83, 0xC7, 0x3A,             // add di,byte +0x3a
        0x83, 0xC7, 0xC6,             // add di,byte -0x3a
    ];
    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 4);
    assert_eq!("[085F:0100] 803E311000       Cmp8     byte [ds:0x1031], 0x00
[085F:0105] 81C7C000         Add16    di, 0x00C0
[085F:0109] 83C73A           Add16    di, byte +0x3A
[085F:010C] 83C7C6           Add16    di, byte -0x3A",
               res);
}

#[test]
fn can_disassemble_relative_short_jumps() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x74, 0x04, // jz 0x106
        0x74, 0xFE, // jz 0x102
        0x74, 0x00, // jz 0x106
        0x74, 0xFA, // jz 0x102
    ];
    cpu.load_com(&code);

    let res = cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 4);
    assert_eq!("[085F:0100] 7404             Jz       0x0106
[085F:0102] 74FE             Jz       0x0102
[085F:0104] 7400             Jz       0x0106
[085F:0106] 74FA             Jz       0x0102",
               res);
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

// executes n instructions of the cpu. only used in tests
fn execute_instructions(cpu: &mut CPU, count: usize) {
    for _ in 0..count {
        cpu.execute_instruction()
    }
}
