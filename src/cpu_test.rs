use test::Bencher;

use cpu::CPU;
use register::{AX, BX, CX, DX, SI, DI, BP, SP, CS, DS, ES};
use instruction::seg_offs_as_flat;
use segment::Segment;

#[test]
fn can_handle_stack() {
    let mut cpu = CPU::new();
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
    let mut cpu = CPU::new();
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
    let mut cpu = CPU::new();
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
    let mut cpu = CPU::new();
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
    let mut cpu = CPU::new();
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
    let mut cpu = CPU::new();
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
    assert_eq!(0x1234, cpu.peek_u16_at(seg_offs_as_flat(cs, 0x0109)));
}

#[test]
fn can_execute_mov_data() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xC6, 0x06, 0x31, 0x10, 0x38,       // mov byte [0x1031],0x38
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    let cs = cpu.sreg16[CS];
    assert_eq!(0x38, cpu.peek_u8_at(seg_offs_as_flat(cs, 0x1031)));
}

#[test]
fn can_execute_segment_prefixed() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x34, 0x12, // mov bx,0x1234
        0x8E, 0xC3,       // mov es,bx
        0xB4, 0x88,       // mov ah,0x88
        0x26, 0x88, 0x25, // mov [es:di],ah
        0x26, 0x8A, 0x05, // mov al,[es:di]
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x1234, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x1234, cpu.sreg16[ES]);

    cpu.execute_instruction();
    assert_eq!(0x107, cpu.ip);
    assert_eq!(0x88, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0x10A, cpu.ip);
    let offset = seg_offs_as_flat(cpu.segment(Segment::ES()), cpu.amode16(5)); // 5=amode DI
    assert_eq!(0x88, cpu.peek_u8_at(offset));

    cpu.execute_instruction();
    assert_eq!(0x10D, cpu.ip);
    assert_eq!(0x88, cpu.r16[AX].lo_u8());
}

#[test]
fn can_execute_imms8() {
    let mut cpu = CPU::new();
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
    let mut cpu = CPU::new();
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
    let mut cpu = CPU::new();
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
    let mut cpu = CPU::new();
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
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        // copy first 5 bytes into 0x200
        0x8D, 0x36, 0x00, 0x01, // lea si,[0x100]
        0x8D, 0x3E, 0x00, 0x02, // lea di,[0x200]
        0xB9, 0x05, 0x00,       // mov cx,0x5
        0xF3, 0xA4,             // rep movsb
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x100, cpu.r16[SI].val);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x5, cpu.r16[CX].val);

    cpu.execute_instruction(); // rep movsb
    assert_eq!(0x0, cpu.r16[CX].val);
    let min = seg_offs_as_flat(cpu.sreg16[CS], 0x100);
    let max = min + 5;
    for i in min..max {
        assert_eq!(cpu.memory.memory[i], cpu.memory.memory[i + 0x100]);
    }
}

#[test]
fn can_execute_rep_outsb() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBE, 0x00, 0x01, // mov si,0x100
        0xBA, 0xC9, 0x03, // mov dx,0x3c9
        0xB9, 0x20, 0x00, // mov cx,0x20
        0xF3, 0x6E,       // rep outsb
    ];
    cpu.load_com(&code);

    assert_eq!(0, cpu.gpu.pal[1].r);
    assert_eq!(0, cpu.gpu.pal[1].g);
    assert_eq!(0, cpu.gpu.pal[1].b);

    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction(); // rep outsb
    assert_eq!(0x0, cpu.r16[CX].val);

    // we verify by checking for change in pal[1], indicating > 1 successful "rep outsb" operation
    assert_eq!(0xE8, cpu.gpu.pal[1].r);
    assert_eq!(0x24, cpu.gpu.pal[1].g);
    assert_eq!(0x0C, cpu.gpu.pal[1].b);
}

#[test]
fn can_execute_addressing() {
    let mut cpu = CPU::new();
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

    let res = cpu.disassemble_block(0x100, 9);
    assert_eq!("[085F:0100] BB0002           Mov16    bx, 0x0200
[085F:0103] C6472CFF         Mov8     byte [bx+0x2C], 0xFF
[085F:0107] 8D360001         Lea16    si, word [0x0100]
[085F:010B] 8B14             Mov16    dx, word [si]
[085F:010D] 8B472C           Mov16    ax, word [bx+0x2C]
[085F:0110] 89873000         Mov16    word [bx+0x0030], ax
[085F:0114] 8905             Mov16    word [di], ax
[085F:0116] C685AE06FE       Mov8     byte [di+0x06AE], 0xFE
[085F:011B] 8A85AE06         Mov8     al, byte [di+0x06AE]
",
               res);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.r16[BX].val);

    cpu.execute_instruction();
    let cs = cpu.sreg16[CS];
    assert_eq!(0xFF, cpu.peek_u8_at(seg_offs_as_flat(cs, 0x22C)));

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
    assert_eq!(0x00FF, cpu.peek_u16_at(seg_offs_as_flat(cs, 0x230)));

    cpu.execute_instruction();
    // should have written ax to [di]
    let di = cpu.r16[DI].val;
    assert_eq!(0x00FF, cpu.peek_u16_at(seg_offs_as_flat(cs, di)));

    cpu.execute_instruction();
    // should have written byte to [di+0x06AE]
    assert_eq!(0xFE, cpu.peek_u8_at(seg_offs_as_flat(cs, di) + 0x06AE));

    cpu.execute_instruction();
    // should have read byte from [di+0x06AE] to al
    assert_eq!(0xFE, cpu.r16[AX].lo_u8());
}

#[test]
fn can_execute_math() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xF6, 0x06, 0x2C, 0x12, 0xFF, // test byte [0x122c],0xff
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 1);
    assert_eq!("[085F:0100] F6062C12FF       Test8    byte [0x122C], 0xFF
",
               res);

    // XXX also execute
}

#[test]
fn can_execute_and() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB0, 0xF0, // mov al,0xF0
        0xB4, 0x1F, // mov ah,0x1F
        0x20, 0xC4, // and ah,al
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 3);

    assert_eq!("[085F:0100] B0F0             Mov8     al, 0xF0
[085F:0102] B41F             Mov8     ah, 0x1F
[085F:0104] 20C4             And8     ah, al
",
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
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB0, 0x40, // mov al,0x40
        0xB3, 0x10, // mov bl,0x10
        0xF6, 0xE3, // mul bl
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x400, cpu.r16[AX].val);
    // XXX flags
}

#[test]
fn can_execute_mul16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x00, 0x80, // mov ax,0x8000
        0xBB, 0x04, 0x00, // mov bx,0x4
        0xF7, 0xE3,       // mul bx
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x0002, cpu.r16[DX].val);
    assert_eq!(0x0000, cpu.r16[AX].val);
    // XXX flags
}

#[test]
fn can_execute_div8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x40, 0x00, // mov ax,0x40
        0xB3, 0x10,       // mov bl,0x10
        0xF6, 0xF3,       // div bl
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 3);

    assert_eq!("[085F:0100] B84000           Mov16    ax, 0x0040
[085F:0103] B310             Mov8     bl, 0x10
[085F:0105] F6F3             Div8     bl
",
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
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBA, 0x10, 0x00, // mov dx,0x10
        0xB8, 0x00, 0x40, // mov ax,0x4000
        0xBB, 0x00, 0x01, // mov bx,0x100
        0xF7, 0xF3,       // div bx
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 4);

    assert_eq!("[085F:0100] BA1000           Mov16    dx, 0x0010
[085F:0103] B80040           Mov16    ax, 0x4000
[085F:0106] BB0001           Mov16    bx, 0x0100
[085F:0109] F7F3             Div16    bx
",
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
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x30, 0x00, // mov ax,0x30
        0xB3, 0x02,       // mov bl,0x2
        0xF6, 0xFB,       // idiv bl
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x18, cpu.r16[AX].lo_u8()); // quotient
    assert_eq!(0x00, cpu.r16[AX].hi_u8()); // remainder
}

#[test]
fn can_execute_idiv16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBA, 0x00, 0x00, // mov dx,0x0
        0xB8, 0x00, 0x80, // mov ax,0x8000
        0xBB, 0x04, 0x00, // mov bx,0x4
        0xF7, 0xFB,       // idiv bx
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x2000, cpu.r16[AX].val); // quotient
    assert_eq!(0x0000, cpu.r16[DX].val); // remainder
}

#[test]
fn can_execute_les() {
    let mut cpu = CPU::new();
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
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x00, 0xFE, // mov ax,0xfe00
        0x99,             // cwd
    ];
    cpu.load_com(&code);
    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.r16[DX].val);
}

#[test]
fn can_execute_aaa() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB0, 0x7E, // mov al,0x7e
        0x37,       // aaa
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x0104, cpu.r16[AX].val);
}

#[test]
fn can_execute_aas() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB0, 0x13, // mov al,0x13
        0x3F,       // aas
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x0003, cpu.r16[AX].val);
}

#[test]
fn can_execute_daa() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB0, 0x79, // mov al,0x79
        0xB3, 0x35, // mov bl,0x35
        0x27,      // daa
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x0079, cpu.r16[AX].val); // XXX, intel manual wants it to be 0x0014
}

#[test]
fn can_execute_das() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB0, 0x35, // mov al,0x35
        0xB3, 0x47, // mov bl,0x47
        0x2F,       // das
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x0035, cpu.r16[AX].val); // XXX, intel manual wants it to be 0x0088
}

#[test]
fn can_execute_sahf() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x6A, 0x00, // push byte +0x0
        0x9D,       // popf
        0xB4, 0xFF, // mov ah,0xff
        0x9E,       // sahf
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.parity);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
}

#[test]
fn can_execute_dec() {
    let mut cpu = CPU::new();
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

    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x23, 0x01, // mov bx,0x123
        0xF7, 0xDB,       // neg bx
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 2);

    assert_eq!("[085F:0100] BB2301           Mov16    bx, 0x0123
[085F:0103] F7DB             Neg16    bx
",
               res);

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
fn can_execute_jmp_far() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xEA, 0x00, 0x06, 0x00, 0x00, // jmp word 0x0:0x600
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 1);

    assert_eq!("[085F:0100] EA00060000       JmpFar   0000:0600
",
               res);

    cpu.execute_instruction();
    assert_eq!(0x0000, cpu.sreg16[CS]);
    assert_eq!(0x0600, cpu.ip);
}

#[test]
fn can_execute_movzx() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB4, 0xFF,       // mov ah,0xff
        0x0F, 0xB6, 0xDC, // movzx bx,ah

    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 2);

    assert_eq!("[085F:0100] B4FF             Mov8     ah, 0xFF
[085F:0102] 0FB6DC           Movzx16  bx, ah
",
               res);

    cpu.execute_instruction();
    assert_eq!(0xFF, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.r16[BX].val);
}

#[test]
fn can_execute_rol8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB4, 0x12,       // mov ah,0x12
        0xC0, 0xC4, 0x04, // rol ah,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x12, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0x21,  cpu.r16[AX].hi_u8());

    // XXX flags
}

#[test]
fn can_execute_rol16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x34, 0x12, // mov ax,0x1234
        0xC1, 0xC0, 0x03, // rol ax,byte 0x3
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x91A0,  cpu.r16[AX].val);

    // XXX flags
}

#[test]
fn can_execute_ror8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB4, 0x37,       // mov ah,0x37
        0xC0, 0xCC, 0x03, // ror ah,byte 0x3
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0xE6,  cpu.r16[AX].hi_u8());

    // XXX flags
}

#[test]
fn can_execute_ror16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x56, 0x34, // mov ax,0x3456
        0xC1, 0xC8, 0x03, // ror ax,byte 0x3
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0xC68A,  cpu.r16[AX].val);

    // XXX flags
}

#[test]
fn can_execute_rcl8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB4, 0x12,       // mov ah,0x12
        0xC0, 0xD4, 0x04, // rcl ah,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x12, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0x20,  cpu.r16[AX].hi_u8());

    // XXX flags
}

#[test]
fn can_execute_rcl16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x34, 0x12, // mov ax,0x1234
        0xC1, 0xD0, 0x04, // rcl ax,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x1234, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0x2340,  cpu.r16[AX].val);

    // XXX flags
}

#[test]
fn can_execute_rcr8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB4, 0x12,       // mov ah,0x12
        0xC0, 0xDC, 0x04, // rcr ah,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x12, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0x41,  cpu.r16[AX].hi_u8());
    // XXX flags
}

#[test]
fn can_execute_rcr16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x34, 0x12, // mov ax,0x1234
        0xC1, 0xD8, 0x04, // rcr ax,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x1234, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0x8123, cpu.r16[AX].val);
    // XXX flags
}

#[test]
fn can_execute_shl8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB4, 0x34,       // mov ah,0x34
        0xC0, 0xE4, 0x04, // shl ah,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
   assert_eq!(0x34, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0x40, cpu.r16[AX].hi_u8());
    // XXX flags
}

#[test]
fn can_execute_shl16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x34, 0x12, // mov ax,0x1234
        0xC1, 0xE0, 0x04, // shl ax,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x2340, cpu.r16[AX].val);
    // XXX flags
}

#[test]
fn can_execute_shr8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB4, 0x34,       // mov ah,0x34
        0xC0, 0xEC, 0x04, // shr ah,byte 0x4
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x03, cpu.r16[AX].hi_u8());
    // XXX flags
}

#[test]
fn can_execute_shr16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x34, 0x12, // mov ax,0x1234
        0xC1, 0xE8, 0x04, // shr ax,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x0123, cpu.r16[AX].val);
    // XXX flags
}

#[test]
fn can_execute_sar8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB4, 0x34,       // mov ah,0x34
        0xC0, 0xFC, 0x04, // sar ah,byte 0x4
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0x03,  cpu.r16[AX].hi_u8());
    // XXX flags
}

#[test]
fn can_execute_sar16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0xF5, 0x05, // mov ax,0x5f5
        0xC1, 0xF8, 0x09, // sar ax,byte 0x9
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x05F5, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0x0002,  cpu.r16[AX].val);
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(false, cpu.flags.parity);
}

#[test]
fn can_execute_imul8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB0, 0xF0, // mov al,0xf0
        0xB7, 0xD0, // mov bh,0xd0
        0xF6, 0xEF, // imul bh
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 3);

    assert_eq!("[085F:0100] B0F0             Mov8     al, 0xF0
[085F:0102] B7D0             Mov8     bh, 0xD0
[085F:0104] F6EF             Imul8    bh
",
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
fn can_execute_imul16_3_args() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBF, 0xFF, 0x8F,       // mov di,0x8fff
        0x69, 0xFF, 0x40, 0x01, // imul di,di,word 0x140
    ];
    cpu.load_com(&code);
    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0xFEC0, cpu.r16[DI].val);
    // XXX Carry & overflow is true in dosbox
}

#[test]
fn can_execute_movsx() {
 let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB7, 0xFF,       // mov bh,0xff
        0x0F, 0xBE, 0xC7, // movsx ax,bh
    ];
    cpu.load_com(&code);
    cpu.execute_instruction();
    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.r16[AX].val);
}

#[test]
fn can_execute_mov_ds_addressing() {
    // NOTE: this test demonstrates a emulation bug described in https://github.com/martinlindhe/dustbox-rs/issues/9#issuecomment-355609424
    // BUG: "mov [bx+si],dx" writes to the CS segment instead of DS
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x10, 0x00, // mov bx,0x10
        0xBE, 0x01, 0x00, // mov si,0x1
        0xBA, 0x99, 0x99, // mov dx,0x9999
        0x89, 0x10,       // mov [bx+si],dx
    ];
    cpu.load_com(&code);
    cpu.sreg16[DS] = 0x8000;
    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();
    cpu.execute_instruction();

    let cs = cpu.sreg16[CS];
    let ds = cpu.sreg16[DS];
    assert_eq!(0x0000, cpu.peek_u16_at(seg_offs_as_flat(cs, 0x10 + 0x1)));
    assert_eq!(0x9999, cpu.peek_u16_at(seg_offs_as_flat(ds, 0x10 + 0x1)));
}

#[test]
fn can_execute_shrd() {
let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0xFF, 0xFF,       // mov ax,0xffff
        0xBA, 0xFF, 0xFF,       // mov dx,0xffff
        0x0F, 0xAC, 0xD0, 0x0E, // shrd ax,dx,0xe
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 3);

    assert_eq!("[085F:0100] B8FFFF           Mov16    ax, 0xFFFF
[085F:0103] BAFFFF           Mov16    dx, 0xFFFF
[085F:0106] 0FACD00E         Shrd     ax, dx, 0x0E
",
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
fn can_execute_imul16_1_arg() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x8F, 0x79, // mov bx,0x798f
        0xB8, 0xD9, 0xFF, // mov ax,0xffd9
        0xF7, 0xEB,       // imul bx
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 3);

    assert_eq!("[085F:0100] BB8F79           Mov16    bx, 0x798F
[085F:0103] B8D9FF           Mov16    ax, 0xFFD9
[085F:0106] F7EB             Imul16   bx
",
               res);

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
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xE8, 0x05, 0x00, // call l_0x108   ; call a later offset
        0xBA, 0x0B, 0x01, // mov dx,0x10b
        0xB4, 0x09,       // mov ah,0x9
        0xCD, 0x21,       // l_0x108: int 0x21
        0xE8, 0xFB, 0xFF, // call l_0x108   ; call an earlier offset
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 5);

    assert_eq!("[085F:0100] E80500           CallNear 0x0108
[085F:0103] BA0B01           Mov16    dx, 0x010B
[085F:0106] B409             Mov8     ah, 0x09
[085F:0108] CD21             Int      0x21
[085F:010A] E8FBFF           CallNear 0x0108
",
               res);
}

#[test]
fn can_disassemble_lea() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x8D, 0x47, 0x80, // lea ax,[bx-0x80]
 ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 1);

    assert_eq!("[085F:0100] 8D4780           Lea16    ax, word [bx-0x80]
",
               res);
}

#[test]
fn can_disassemble_segment_prefixed() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x26, 0x88, 0x25, // mov [es:di],ah
        0x26, 0x8A, 0x25, // mov ah,[es:di]
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 2);

    assert_eq!("[085F:0100] 268825           Mov8     byte [es:di], ah
[085F:0103] 268A25           Mov8     ah, byte [es:di]
",
               res);
}

#[test]
fn can_disassemble_arithmetic() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x81, 0xC7, 0xC0, 0x00,       // add di,0xc0
        0x83, 0xC7, 0x3A,             // add di,byte +0x3a
        0x83, 0xC7, 0xC6,             // add di,byte -0x3a
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 4);

    assert_eq!("[085F:0100] 803E311000       Cmp8     byte [0x1031], 0x00
[085F:0105] 81C7C000         Add16    di, 0x00C0
[085F:0109] 83C73A           Add16    di, byte +0x3A
[085F:010C] 83C7C6           Add16    di, byte -0x3A
",
               res);
}

#[test]
fn can_disassemble_jz_rel() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x74, 0x04, // jz 0x106
        0x74, 0xFE, // jz 0x102
        0x74, 0x00, // jz 0x106
        0x74, 0xFA, // jz 0x102
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 4);

    assert_eq!("[085F:0100] 7404             Jz       0x0106
[085F:0102] 74FE             Jz       0x0102
[085F:0104] 7400             Jz       0x0106
[085F:0106] 74FA             Jz       0x0102
",
               res);
}

#[test]
fn estimate_mips() {
    use std::time::Instant;

    let mut cpu = CPU::new();
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

#[bench]
fn bench_simple_loop(b: &mut Bencher) {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB9, 0xFF, 0xFF, // mov cx,0xffff
        0x49,             // dec cx
        0xEB, 0xFA,       // jmp short 0x100
    ];

    cpu.load_com(&code);

    b.iter(|| cpu.execute_instruction())
}

#[bench]
fn bench_disasm_small_prog(b: &mut Bencher) {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
    ];
    cpu.load_com(&code);

    b.iter(|| cpu.disassemble_block(0x100, 8))
}
