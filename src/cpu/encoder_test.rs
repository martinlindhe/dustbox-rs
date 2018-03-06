use std::fs::File;
use std::io::{self, Read, Write};
use std::process::Command;
use std::str;
use std::collections::HashMap;
use std::time::Instant;
use std::path::{Path, PathBuf};
use std::iter::FromIterator;

use tempdir::TempDir;
use tera::Context;
use rand::{Rng, XorShiftRng};

use cpu::CPU;
use cpu::encoder::{Encoder, ndisasm_bytes};
use cpu::segment::Segment;
use cpu::parameter::Parameter;
use cpu::instruction::{Instruction, InstructionInfo, RepeatMode};
use cpu::op::Op;
use cpu::register::{R, AMode};
use machine::Machine;
use memory::MMU;
use hex::hex_bytes;

#[test] #[ignore] // expensive test
fn can_encode_random_seq() {
    let mut rng = XorShiftRng::new_unseeded();
    let mut code = vec![0u8; 10];

    let mut machine = Machine::new();

    for _ in 0..1000 {
        for mut b in &mut code {
            *b = rng.gen();
        }

        machine.load_com(&code);

        let encoder = Encoder::new();

        // randomizes a byte sequence and tries to decode the first instruction
        let cs = machine.cpu.get_r16(&R::CS);
        let ops = machine.cpu.decoder.decode_to_block(&mut machine.hw.mmu, cs, 0x100, 1);
        let op = &ops[0];
        if op.instruction.command.is_valid() {
            // - if successful, try to encode. all valid decodings should be mapped for valid
            //   encoding for implemented ops (this should find all missing cases)
            let try_enc = encoder.encode(&op.instruction);
            match try_enc {
                Ok(enc) => {
                    let in_bytes = Vec::from_iter(code[0..enc.len()].iter().cloned());
                    if enc != in_bytes {
                        let ndisasm_of_input = ndisasm_bytes(&in_bytes).unwrap();
                        let ndisasm_of_encode = ndisasm_bytes(&enc).unwrap();
                        if ndisasm_of_input != ndisasm_of_encode {
                            panic!("encoding resulted in wrong sequence.\n\ninput  {:?}\noutput {:?}\ninstr {:?}\nndisasm of\ninput '{}'\nencode '{}'",
                                hex_bytes(&in_bytes),
                                hex_bytes(&enc),
                                op.instruction,
                                ndisasm_of_input,
                                ndisasm_of_encode);
                        }
                    }

                    // - if encode was successful, try to decode that seq again and make sure the resulting
                    //   ops are the same (this should ensure all cases code 2-way to the same values)
                    machine.load_com(&enc);
                    let decoded = machine.cpu.decoder.decode_to_block(&mut machine.hw.mmu, cs, 0x100, 1);
                    let reencoded_op = &decoded[0];
                    if op.instruction != reencoded_op.instruction {
                        panic!("re-encoding failed.\n\nexpected {:?},\noutput   {:?}",
                        op.instruction, reencoded_op.instruction);
                    }
                }
                _ => {
                    // NOTE: commented out for now because encoder.rs handles so few instructions
                    // println!("ERROR: found unsuccessful encode for {:?}: reason {:?}", op, try_enc);
                }
            }
        } else {
            // println!("NOTICE: skipping invalid sequence: {:?}: {}", code, op);
        }
    }
}

#[test]
fn can_encode_xchg8() {
    // r/m8, r8
    let op = Instruction::new2(Op::Xchg8, Parameter::Reg8(R::BH), Parameter::Reg8(R::DL));
    assert_encdec(&op, "xchg dl,bh", vec!(0x86, 0xD7));
    // XXX NOTE: nasm encodes differently:
    // 00000100  86FA              xchg bh,dl       nasm        FA = 0b1111_1010
    // 00000102  86D7              xchg dl,bh       us          D7 = 0b1101_0111
}

#[test]
fn can_encode_test8() {
    // AL, imm8
    let op = Instruction::new2(Op::Test8, Parameter::Reg8(R::AL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "test al,0xff", vec!(0xA8, 0xFF));
    
    // r8, imm8
    let op = Instruction::new2(Op::Test8, Parameter::Reg8(R::BH), Parameter::Imm8(0xFF));
    assert_encdec(&op, "test bh,0xff", vec!(0xF6, 0xC7, 0xFF));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Test8, Parameter::Reg8(R::BH), Parameter::Reg8(R::DL));
    assert_encdec(&op, "test bh,dl", vec!(0x84, 0xD7));

    // r/m8, r8: NOTE can only be decoded to this form. parameter order does not matter
    let op = Instruction::new2(Op::Test8, Parameter::Ptr8(Segment::Default, 0xC365), Parameter::Reg8(R::BH));
    assert_encdec(&op, "test [0xc365],bh", vec!(0x84, 0x3E, 0x65, 0xC3));
}

#[test]
fn can_encode_not8() {
    // r/m8
    let op = Instruction::new1(Op::Not8, Parameter::Reg8(R::BH));
    assert_encdec(&op, "not bh", vec!(0xF6, 0xD7));

    // r/m8
    let op = Instruction::new1(Op::Not8, Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "not byte [0xc365]", vec!(0xF6, 0x16, 0x65, 0xC3));
}

#[test]
fn can_encode_neg8() {
    // r/m8
    let op = Instruction::new1(Op::Neg8, Parameter::Reg8(R::BH));
    assert_encdec(&op, "neg bh", vec!(0xF6, 0xDF));

    // r/m8
    let op = Instruction::new1(Op::Neg8, Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "neg byte [0xc365]", vec!(0xF6, 0x1E, 0x65, 0xC3));
}

#[test]
fn can_encode_mul8() {
    // r/m8
    let op = Instruction::new1(Op::Mul8, Parameter::Reg8(R::BH));
    assert_encdec(&op, "mul bh", vec!(0xF6, 0xE7));

    // r/m8
    let op = Instruction::new1(Op::Mul8, Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "mul byte [0xc365]", vec!(0xF6, 0x26, 0x65, 0xC3));
}

#[test]
fn can_encode_imul8() {
    // r/m8
    let op = Instruction::new1(Op::Imul8, Parameter::Reg8(R::BH));
    assert_encdec(&op, "imul bh", vec!(0xF6, 0xEF));

    // r/m8
    let op = Instruction::new1(Op::Imul8, Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "imul byte [0xc365]", vec!(0xF6, 0x2E, 0x65, 0xC3));
}

#[test]
fn can_encode_div8() {
    // r/m8
    let op = Instruction::new1(Op::Div8, Parameter::Reg8(R::BH));
    assert_encdec(&op, "div bh", vec!(0xF6, 0xF7));

    // r/m8
    let op = Instruction::new1(Op::Div8, Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "div byte [0xc365]", vec!(0xF6, 0x36, 0x65, 0xC3));
}

#[test]
fn can_encode_idiv8() {
    // r/m8
    let op = Instruction::new1(Op::Idiv8, Parameter::Reg8(R::BH));
    assert_encdec(&op, "idiv bh", vec!(0xF6, 0xFF));

    // r/m8
    let op = Instruction::new1(Op::Idiv8, Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "idiv byte [0xc365]", vec!(0xF6, 0x3E, 0x65, 0xC3));
}

#[test]
fn can_encode_and8() {
    // AL, imm8
    let op = Instruction::new2(Op::And8, Parameter::Reg8(R::AL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "and al,0xff", vec!(0x24, 0xFF));

    // r8, imm8
    let op = Instruction::new2(Op::And8, Parameter::Reg8(R::BL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "and bl,0xff", vec!(0x80, 0xE3, 0xFF));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::And8, Parameter::Reg8(R::BH), Parameter::Reg8(R::DL));
    assert_encdec(&op, "and bh,dl", vec!(0x20, 0xD7));

    // r8, r/m8
    let op = Instruction::new2(Op::And8, Parameter::Reg8(R::BH), Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "and bh,[0xc365]", vec!(0x22, 0x3E, 0x65, 0xC3));
}

#[test]
fn can_encode_xor8() {
    // AL, imm8
    let op = Instruction::new2(Op::Xor8, Parameter::Reg8(R::AL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "xor al,0xff", vec!(0x34, 0xFF));

    // r8, imm8
    let op = Instruction::new2(Op::Xor8, Parameter::Reg8(R::BL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "xor bl,0xff", vec!(0x80, 0xF3, 0xFF));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Xor8, Parameter::Reg8(R::BH), Parameter::Reg8(R::DL));
    assert_encdec(&op, "xor bh,dl", vec!(0x30, 0xD7));

    // r8, r/m8
    let op = Instruction::new2(Op::Xor8, Parameter::Reg8(R::BH), Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "xor bh,[0xc365]", vec!(0x32, 0x3E, 0x65, 0xC3));
}

#[test]
fn can_encode_or8() {
    // AL, imm8
    let op = Instruction::new2(Op::Or8, Parameter::Reg8(R::AL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "or al,0xff", vec!(0x0C, 0xFF));

    // r8, imm8
    let op = Instruction::new2(Op::Or8, Parameter::Reg8(R::BL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "or bl,0xff", vec!(0x80, 0xCB, 0xFF));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Or8, Parameter::Reg8(R::BH), Parameter::Reg8(R::DL));
    assert_encdec(&op, "or bh,dl", vec!(0x08, 0xD7));

    // r8, r/m8
    let op = Instruction::new2(Op::Or8, Parameter::Reg8(R::BH), Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "or bh,[0xc365]", vec!(0x0A, 0x3E, 0x65, 0xC3));
}

#[test]
fn can_encode_add8() {
    // AL, imm8
    let op = Instruction::new2(Op::Add8, Parameter::Reg8(R::AL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "add al,0xff", vec!(0x04, 0xFF));

    // r8, imm8
    let op = Instruction::new2(Op::Add8, Parameter::Reg8(R::BL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "add bl,0xff", vec!(0x80, 0xC3, 0xFF));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Add8, Parameter::Reg8(R::BH), Parameter::Reg8(R::DL));
    assert_encdec(&op, "add bh,dl", vec!(0x00, 0xD7));

    // r8, r/m8
    let op = Instruction::new2(Op::Add8, Parameter::Reg8(R::BH), Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "add bh,[0xc365]", vec!(0x02, 0x3E, 0x65, 0xC3));
}

#[test]
fn can_encode_sub8() {
    // AL, imm8
    let op = Instruction::new2(Op::Sub8, Parameter::Reg8(R::AL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "sub al,0xff", vec!(0x2C, 0xFF));

    // r8, imm8
    let op = Instruction::new2(Op::Sub8, Parameter::Reg8(R::BL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "sub bl,0xff", vec!(0x80, 0xEB, 0xFF));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Sub8, Parameter::Reg8(R::BH), Parameter::Reg8(R::DL));
    assert_encdec(&op, "sub bh,dl", vec!(0x28, 0xD7));

    // r8, r/m8
    let op = Instruction::new2(Op::Sub8, Parameter::Reg8(R::BH), Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "sub bh,[0xc365]", vec!(0x2A, 0x3E, 0x65, 0xC3));
}

#[test]
fn can_encode_sbb8() {
    // AL, imm8
    let op = Instruction::new2(Op::Sbb8, Parameter::Reg8(R::AL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "sbb al,0xff", vec!(0x1C, 0xFF));

    // r8, imm8
    let op = Instruction::new2(Op::Sbb8, Parameter::Reg8(R::BL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "sbb bl,0xff", vec!(0x80, 0xDB, 0xFF));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Sbb8, Parameter::Reg8(R::BH), Parameter::Reg8(R::DL));
    assert_encdec(&op, "sbb bh,dl", vec!(0x18, 0xD7));

    // r8, r/m8
    let op = Instruction::new2(Op::Sbb8, Parameter::Reg8(R::BH), Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "sbb bh,[0xc365]", vec!(0x1A, 0x3E, 0x65, 0xC3));
}

#[test]
fn can_encode_adc8() {
    // AL, imm8
    let op = Instruction::new2(Op::Adc8, Parameter::Reg8(R::AL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "adc al,0xff", vec!(0x14, 0xFF));

    // r8, imm8
    let op = Instruction::new2(Op::Adc8, Parameter::Reg8(R::BL), Parameter::Imm8(0xFF));
    assert_encdec(&op, "adc bl,0xff", vec!(0x80, 0xD3, 0xFF));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Adc8, Parameter::Reg8(R::BH), Parameter::Reg8(R::DL));
    assert_encdec(&op, "adc bh,dl", vec!(0x10, 0xD7));

    // r8, r/m8
    let op = Instruction::new2(Op::Adc8, Parameter::Reg8(R::BH), Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "adc bh,[0xc365]", vec!(0x12, 0x3E, 0x65, 0xC3));
}

#[test]
fn can_encode_cmp8() {
    // r8, imm8
    let op = Instruction::new2(Op::Cmp8, Parameter::Reg8(R::BH), Parameter::Imm8(0xFF));
    assert_encdec(&op, "cmp bh,0xff", vec!(0x80, 0xFF, 0xFF));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Cmp8, Parameter::Reg8(R::BH), Parameter::Reg8(R::DL));
    assert_encdec(&op, "cmp bh,dl", vec!(0x38, 0xD7));

    // r/m8, r8  (dst is AMode::BP + imm8)
    let op = Instruction::new2(Op::Cmp8, Parameter::Ptr8AmodeS8(Segment::Default, AMode::BP, 0x10), Parameter::Reg8(R::BH));
    assert_encdec(&op, "cmp [bp+0x10],bh", vec!(0x38, 0x7E, 0x10));

    // r/m8, r8  (dst is AMode::BP + imm8)    - reversed
    let op = Instruction::new2(Op::Cmp8, Parameter::Reg8(R::BH), Parameter::Ptr8AmodeS8(Segment::Default, AMode::BP, 0x10));
    assert_encdec(&op, "cmp bh,[bp+0x10]", vec!(0x3A, 0x7E, 0x10));

    // r8, r/m8
    let op = Instruction::new2(Op::Cmp8, Parameter::Reg8(R::BH), Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "cmp bh,[0xc365]", vec!(0x3A, 0x3E, 0x65, 0xC3));

    // r/m8, r8  (dst is AMode::BP + imm8)
    let op = Instruction::new2(Op::Cmp8, Parameter::Ptr8AmodeS16(Segment::Default, AMode::BP, -0x800), Parameter::Reg8(R::BH));
    assert_encdec(&op, "cmp [bp-0x800],bh", vec!(0x38, 0xBE, 0x00, 0xF8));

    // r/m8, r8  (dst is [imm16])
    let op = Instruction::new2(Op::Cmp8, Parameter::Ptr8(Segment::Default, 0x8000), Parameter::Reg8(R::BH));
    assert_encdec(&op, "cmp [0x8000],bh", vec!(0x38, 0x3E, 0x00, 0x80));

    // r/m8, r8  (dst is [bx])
    let op = Instruction::new2(Op::Cmp8, Parameter::Ptr8Amode(Segment::Default, AMode::BX), Parameter::Reg8(R::BH));
    assert_encdec(&op, "cmp [bx],bh", vec!(0x38, 0x3F));
}

#[test]
fn can_encode_cmp16() {
    // XXX cmp16
    // r16, imm16
    //let op = Instruction::new2(Op::Mov16, Parameter::Reg16(R::BX), Parameter::Imm16(0x8844));
    //assert_encdec(&op, "mov bx,0x8844", vec!(0xBB, 0x44, 0x88));
}

#[test]
fn can_encode_inc() {
    let op = Instruction::new1(Op::Inc8, Parameter::Reg8(R::BH));
    assert_encdec(&op, "inc bh", vec!(0xFE, 0xC7));

    let op = Instruction::new1(Op::Inc8, Parameter::Ptr8AmodeS8(Segment::Default, AMode::BP, 0x10));
    assert_encdec(&op, "inc byte [bp+0x10]", vec!(0xFE, 0x46, 0x10));
    
    let op = Instruction::new1(Op::Inc16, Parameter::Reg16(R::BX));
    assert_encdec(&op, "inc bx", vec!(0x43));

    let op = Instruction::new1(Op::Inc16, Parameter::Ptr16AmodeS8(Segment::Default, AMode::BP, 0x10));
    assert_encdec(&op, "inc word [bp+0x10]", vec!(0xFF, 0x46, 0x10));
}

#[test]
fn can_encode_dec() {
    let op = Instruction::new1(Op::Dec8, Parameter::Reg8(R::BH));
    assert_encdec(&op, "dec bh", vec!(0xFE, 0xCF));

    let op = Instruction::new1(Op::Dec16, Parameter::Reg16(R::BX));
    assert_encdec(&op, "dec bx", vec!(0x4B));

    let op = Instruction::new1(Op::Dec16, Parameter::Ptr16AmodeS8(Segment::Default, AMode::BP, 0x10));
    assert_encdec(&op, "dec word [bp+0x10]", vec!(0xFF, 0x4E, 0x10));
}



#[test]
fn can_encode_push() {
    let op = Instruction::new1(Op::Push16, Parameter::Imm16(0x8088));
    assert_encdec(&op, "push word 0x8088", vec!(0x68, 0x88, 0x80));
}

#[test]
fn can_encode_pop() {
    let op = Instruction::new(Op::Popf);
    assert_encdec(&op, "popf", vec!(0x9D));
}

#[test]
fn can_encode_lea() {
    let op = Instruction::new2(Op::Lea16, Parameter::Reg16(R::DI), Parameter::Ptr16Amode(Segment::Default, AMode::BX));
    assert_encdec(&op, "lea di,[bx]", vec!(0x8D, 0x3F));
}

#[test]
fn can_encode_shld() {
    let op = Instruction::new3(Op::Shld, Parameter::Reg16(R::BX), Parameter::Reg16(R::DI), Parameter::Imm8(0x8));
    assert_encdec(&op, "shld bx,di,0x8", vec!(0x0F, 0xA4, 0xFB, 0x08));
}

#[test]
fn can_encode_shrd() {
    let op = Instruction::new3(Op::Shrd, Parameter::Reg16(R::BX), Parameter::Reg16(R::DI), Parameter::Imm8(0x8));
    assert_encdec(&op, "shrd bx,di,0x8", vec!(0x0F, 0xAC, 0xFB, 0x08));
}

#[test]
fn can_encode_bitshift_instructions() {
    let op = Instruction::new2(Op::Shr8, Parameter::Reg8(R::AH), Parameter::Imm8(0xFF));
    assert_encdec(&op, "shr ah,byte 0xff", vec!(0xC0, 0xEC, 0xFF));

    let op = Instruction::new2(Op::Shl8, Parameter::Reg8(R::AH), Parameter::Imm8(0xFF));
    assert_encdec(&op, "shl ah,byte 0xff", vec!(0xC0, 0xE4, 0xFF));
}

#[test]
fn can_encode_int() {
    let op = Instruction::new1(Op::Int, Parameter::Imm8(0x21));
    assert_encdec(&op, "int 0x21", vec!(0xCD, 0x21));
}

#[test]
fn can_encode_mov8() {
    // r8, imm8
    let op = Instruction::new2(Op::Mov8, Parameter::Reg8(R::BH), Parameter::Imm8(0xFF));
    assert_encdec(&op, "mov bh,0xff", vec!(0xB7, 0xFF));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Mov8, Parameter::Reg8(R::BH), Parameter::Reg8(R::DL));
    assert_encdec(&op, "mov bh,dl", vec!(0x88, 0xD7));

    // r/m8, r8  (dst is AMode::BP + imm8)
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8AmodeS8(Segment::Default, AMode::BP, 0x10), Parameter::Reg8(R::BH));
    assert_encdec(&op, "mov [bp+0x10],bh", vec!(0x88, 0x7E, 0x10));

    // r/m8, r8  (dst is AMode::BP + imm8)    - reversed
    let op = Instruction::new2(Op::Mov8, Parameter::Reg8(R::BH), Parameter::Ptr8AmodeS8(Segment::Default, AMode::BP, 0x10));
    assert_encdec(&op, "mov bh,[bp+0x10]", vec!(0x8A, 0x7E, 0x10));

    // r8, r/m8
    let op = Instruction::new2(Op::Mov8, Parameter::Reg8(R::BH), Parameter::Ptr8(Segment::Default, 0xC365));
    assert_encdec(&op, "mov bh,[0xc365]", vec!(0x8A, 0x3E, 0x65, 0xC3));

    // r/m8, r8  (dst is AMode::BP + imm8)
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8AmodeS16(Segment::Default, AMode::BP, -0x800), Parameter::Reg8(R::BH));
    assert_encdec(&op, "mov [bp-0x800],bh", vec!(0x88, 0xBE, 0x00, 0xF8));

    // r/m8, r8  (dst is [imm16]) // XXX no direct amode mapping in resulting Instruction. can we implement a "Instruction.AMode() -> AMode" ?
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8(Segment::Default, 0x8000), Parameter::Reg8(R::BH));
    assert_encdec(&op, "mov [0x8000],bh", vec!(0x88, 0x3E, 0x00, 0x80));

    // r/m8, r8  (dst is [bx])
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8Amode(Segment::Default, AMode::BX), Parameter::Reg8(R::BH));
    assert_encdec(&op, "mov [bx],bh", vec!(0x88, 0x3F));
}

#[test]
fn can_encode_mov16() {
    // r16, imm16
    let op = Instruction::new2(Op::Mov16, Parameter::Reg16(R::BX), Parameter::Imm16(0x8844));
    assert_encdec(&op, "mov bx,0x8844", vec!(0xBB, 0x44, 0x88));
/*
// XXX:
    let op = Instruction::new2(Op::Mov16, Parameter::Ptr16Amode(Segment::Default, AMode::SI), Parameter::Imm16(0x8844));  // mov [si], word xxxx
    assert_encdec(&op, "mov word [si],0x8844", vec!(0xC7, 0x04, 0x44, 0x88));
*/
}

fn assert_encdec(op :&Instruction, expected_ndisasm: &str, expected_bytes: Vec<u8>) {
    let encoder = Encoder::new();
    let code = encoder.encode(&op).unwrap();
    assert_eq!(expected_bytes, code, "encoded byte sequence does not match expected bytes");

    let mut machine = Machine::new();
    machine.load_com(&code);
    let cs = machine.cpu.get_r16(&R::CS);
    let ops = machine.cpu.decoder.decode_to_block(&mut machine.hw.mmu, cs, 0x100, 1);
    let decoded_op = &ops[0].instruction;
    assert_eq!(op, decoded_op, "decoded resulting op from instruction encode does not match input op");

    assert_eq!(expected_ndisasm.to_owned(), ndisasm_bytes(&code).unwrap(), "disasm of encoded byte sequence does not match expected ndisasm output");
}
