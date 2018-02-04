use std::fs::File;
use std::io::{self, Read, Write};
use std::process::Command;
use std::str;
use std::collections::HashMap;
use std::time::Instant;
use std::path::{Path, PathBuf};

use tempdir::TempDir;
use tera::Context;

use cpu::CPU;
use cpu::encoder::Encoder;
use cpu::segment::Segment;
use cpu::parameter::Parameter;
use cpu::instruction::{Instruction, InstructionInfo, RepeatMode};
use cpu::op::Op;
use cpu::register::{R8, AMode, R16};
use memory::mmu::MMU;
use cpu::fuzzer::ndisasm;

#[test]
fn can_encode_push() {
    let encoder = Encoder::new();

    let op = Instruction::new1(Op::Push16, Parameter::Imm16(0x8088));
    assert_eq!(vec!(0x68, 0x88, 0x80), encoder.encode(&op));
    assert_eq!("push word 0x8088".to_owned(), ndisasm(&op).unwrap());
}

#[test]
fn can_encode_pop() {
    let encoder = Encoder::new();

    let op = Instruction::new(Op::Popf);
    assert_eq!(vec!(0x9D), encoder.encode(&op));
    assert_eq!("popf".to_owned(), ndisasm(&op).unwrap());
}

#[test]
fn can_encode_bitshift_instructions() {
    let encoder = Encoder::new();

    let op = Instruction::new2(Op::Shr8, Parameter::Reg8(R8::AH), Parameter::Imm8(0xFF));
    assert_eq!(vec!(0xC0, 0xEC, 0xFF), encoder.encode(&op));
    assert_eq!("shr ah,byte 0xff".to_owned(), ndisasm(&op).unwrap());

    let op = Instruction::new2(Op::Shl8, Parameter::Reg8(R8::AH), Parameter::Imm8(0xFF));
    assert_eq!(vec!(0xC0, 0xE4, 0xFF), encoder.encode(&op));
    assert_eq!("shl ah,byte 0xff".to_owned(), ndisasm(&op).unwrap());
}

#[test]
fn can_encode_int() {
    let encoder = Encoder::new();

    let op = Instruction::new1(Op::Int(), Parameter::Imm8(0x21));
    assert_eq!(vec!(0xCD, 0x21), encoder.encode(&op));
    assert_eq!("int 0x21".to_owned(), ndisasm(&op).unwrap());
}

#[test]
fn can_encode_mov_addressing_modes() {
    let encoder = Encoder::new();

    // r8, imm8
    let op = Instruction::new2(Op::Mov8, Parameter::Reg8(R8::BH), Parameter::Imm8(0xFF));
    assert_eq!("mov bh,0xff".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0xB7, 0xFF), encoder.encode(&op));

    // r16, imm8
    let op = Instruction::new2(Op::Mov16, Parameter::Reg16(R16::BX), Parameter::Imm16(0x8844));
    assert_eq!("mov bx,0x8844".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0xBB, 0x44, 0x88), encoder.encode(&op));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Mov8, Parameter::Reg8(R8::BH), Parameter::Reg8(R8::DL));
    assert_eq!("mov bh,dl".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0xD7), encoder.encode(&op));

    // r/m8, r8  (dst is AMode::BP + imm8)
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8AmodeS8(Segment::Default, AMode::BP, 0x10), Parameter::Reg8(R8::BH));
    assert_eq!("mov [bp+0x10],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0x7E, 0x10), encoder.encode(&op));

    // r/m8, r8  (dst is AMode::BP + imm8)    - reversed
    let op = Instruction::new2(Op::Mov8, Parameter::Reg8(R8::BH), Parameter::Ptr8AmodeS8(Segment::Default, AMode::BP, 0x10));
    assert_eq!(vec!(0x8A, 0x7E, 0x10), encoder.encode(&op));
    assert_eq!("mov bh,[bp+0x10]".to_owned(), ndisasm(&op).unwrap());

    // r/m8, r8  (dst is AMode::BP + imm8)
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8AmodeS16(Segment::Default, AMode::BP, -0x800), Parameter::Reg8(R8::BH));
    assert_eq!("mov [bp-0x800],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0xBE, 0x00, 0xF8), encoder.encode(&op));

    // r/m8, r8  (dst is [imm16]) // XXX no direct amode mapping in resulting Instruction. can we implement a "Instruction.AMode() -> AMode" ?
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8(Segment::Default, 0x8000), Parameter::Reg8(R8::BH));
    assert_eq!("mov [0x8000],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0x3E, 0x00, 0x80), encoder.encode(&op));

    // r/m8, r8  (dst is [bx])
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8Amode(Segment::Default, AMode::BX), Parameter::Reg8(R8::BH));
    assert_eq!("mov [bx],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0x3F), encoder.encode(&op));
}
