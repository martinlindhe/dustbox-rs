use std::fmt;

use crate::cpu::instruction::{Instruction, ModRegRm};
use crate::cpu::parameter::{Parameter, ParameterSet};
use crate::cpu::segment::Segment;
use crate::cpu::register::R;
use crate::cpu::op::{Op};

#[cfg(test)]
#[path = "./encoder_test.rs"]
mod encoder_test;

#[derive(Debug)]
pub enum EncodeError {
    UnhandledOp(Op),
    UnhandledParameter(Parameter),
    UnexpectedDstType(Parameter),
    Text(String),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EncodeError::UnhandledOp(op) => write!(f, "unhandled op: {:?}", op),
            EncodeError::UnhandledParameter(p) => write!(f, "unhandled param: {:?}", p),
            EncodeError::UnexpectedDstType(p) => write!(f, "unexpected dst type: {:?}", p),
            EncodeError::Text(s) => write!(f, "text: {}", s),
        }
    }
}

#[derive(Default)]
pub struct Encoder {
}

impl Encoder {
    pub fn new() -> Self {
        Encoder {
        }
    }

    pub fn encode_vec(&self, ops: &[Instruction]) -> Result<Vec<u8>, EncodeError> {
        let mut out = vec!();
        for op in ops {
            let enc = self.encode(op);
            if let Ok(data) = enc {
                out.extend(data);
            } else {
                return enc;
            }
        }
        Ok(out)
    }

    /// encodes Instruction to a valid byte sequence
    pub fn encode(&self, op: &Instruction) -> Result<Vec<u8>, EncodeError> {
        let mut out = vec!();
        if op.lock {
            out.push(0xF0);
        }
        match op.segment_prefix {
            Segment::Default => {},
            Segment::ES => out.push(0x26),
            Segment::CS => out.push(0x2E),
            Segment::SS => out.push(0x36),
            Segment::DS => out.push(0x3E),
            Segment::FS => out.push(0x64),
            Segment::GS => out.push(0x65),
        }

        match op.command {
            Op::Daa => out.push(0x27),
            Op::Das => out.push(0x2F),
            Op::Aaa => out.push(0x37),
            Op::Aas => out.push(0x3F),
            Op::Bsf => {
                // bsf r16, r/m16
                out.push(0x0F);
                out.push(0xBC);
                out.extend(self.encode_r_rm(&op.params));
            }
            Op::Bt => {
                // bt r/m16, r16
                out.push(0x0F);
                out.push(0xA3); // XXX BT r/m16, imm8   form is 0xBA
                out.extend(self.encode_rm_r(&op.params));
            }
            Op::Cmc => out.push(0xF5),
            Op::Clc => out.push(0xF8),
            Op::Stc => out.push(0xF9),
            Op::Cli => out.push(0xFA),
            Op::Sti => out.push(0xFB),
            Op::Cld => out.push(0xFC),
            Op::Std => out.push(0xFD),
            Op::Cbw => out.push(0x98),
            Op::Cwd16 => out.push(0x99),
            Op::Sahf => out.push(0x9E),
            Op::Lahf => out.push(0x9F),
            Op::Nop => out.push(0x90),
            Op::Salc => out.push(0xD6),
            Op::Xlatb => out.push(0xD7),
            Op::Cmpsb => out.push(0xA6),
            Op::Cmpsw => out.push(0xA7),
            Op::Aad => {
                if let Parameter::Imm8(imm) = op.params.dst {
                    out.push(0xD5);
                    out.push(imm);
                } else {
                    unreachable!();
                }
            }
            Op::Aam => {
                if let Parameter::Imm8(imm) = op.params.dst {
                    out.push(0xD4);
                    out.push(imm);
                } else {
                    unreachable!();
                }
            }
            Op::Dec8 | Op::Inc8 => {
                // 0xFE: r/m8
                out.push(0xFE);
                out.extend(self.encode_rm(&op.params.dst, Encoder::feff_index(&op.command)));
            }
            Op::Dec16 | Op::Inc16 => {
                if let Parameter::Reg16(ref r) = op.params.dst {
                    match op.command {
                        Op::Inc16 => out.push(0x40 | r.index() as u8), // 0x40...0x47: inc r16
                        Op::Dec16 => out.push(0x48 | r.index() as u8), // 0x48...0x4F: dec r16
                        // 0x50...0x57: push r16
                        // 0x58...0x5F: pop r16
                        _ => panic!("unhandled {:?}", op.command),
                    }
                } else {
                    // 0xFF: // 0xFF: r/m16
                    out.push(0xFF);
                    out.extend(self.encode_rm(&op.params.dst, Encoder::feff_index(&op.command)));
                }
            }
            Op::Dec32 | Op::Inc32 => {
                out.push(0x66);
                if let Parameter::Reg32(ref r) = op.params.dst {
                    match op.command {
                        Op::Inc32 => out.push(0x40 | r.index() as u8), // 0x40...0x47: inc r32
                        Op::Dec32 => out.push(0x48 | r.index() as u8), // 0x48...0x4F: dec r32
                        // 0x50...0x57: push r16
                        // 0x58...0x5F: pop r16
                        _ => panic!("unhandled {:?}", op.command),
                    }
                } else {
                    // 0xFF: // 0xFF: r/m16
                    out.push(0xFF);
                    out.extend(self.encode_rm(&op.params.dst, Encoder::feff_index(&op.command)));
                }
            }
            Op::Int => {
                if let Parameter::Imm8(imm) = op.params.dst {
                    if imm == 1 {
                        out.push(0xF1);
                    } else if imm == 3 {
                        out.push(0xCC);
                    } else {
                        out.push(0xCD);
                        out.push(imm);
                    }
                } else {
                    return Err(EncodeError::UnhandledParameter(op.params.dst.clone()));
                }
            }
            /*
            Op::Cmp16 => {
                // 0x39: cmp r/m16, r16
                // 0x3B: cmp r16, r/m16
                // 0x3D: cmp AX, imm16
                // 0x81: <arithmetic> r/m16, imm16
                // 0x83: <arithmetic> r/m16, imm8
            }
            */
            Op::Xchg8 => {
                // 0x86: xchg r/m8, r8
                out.push(0x86);
                out.extend(self.encode_rm_r(&op.params));
            }
            Op::Lea16 => {
                 out.push(0x8D);
                 out.extend(self.encode_r_rm(&op.params)); // XXX 16-bit ver?!
                // lea r16, m        di, [bx]  = 0b11_1111
            }
            Op::Shld => {
                // shld r/m16, r16, imm8
                out.push(0x0F);
                out.push(0xA4);
                out.extend(self.encode_rm_r_imm(&op.params));
            }
            Op::Shrd => {
                out.push(0x0F);
                out.push(0xAC);
                out.extend(self.encode_rm_r_imm(&op.params));
            }
            Op::Mov8 => {
                match op.params.dst {
                    Parameter::Reg8(r) => {
                        if r == R::AL {
                            if let Parameter::Ptr8(_, imm16) = op.params.src {
                                // 0xA0: mov AL, [moffs8]
                                out.push(0xA0);
                                out.push(imm16 as u8);
                                out.push((imm16 >> 8) as u8);
                                return Ok(out);
                            }
                        }
                        if let Parameter::Imm8(i) = op.params.src {
                            // 0xB0...0xB7: mov r8, u8
                            out.push(0xB0 | r as u8);
                            out.push(i as u8);
                        } else if op.params.src.is_ptr() {
                            // 0x8A: mov r8, r/m8
                            out.push(0x8A);
                            out.extend(self.encode_r_rm(&op.params));
                        } else {
                            // 0x88: mov r/m8, r8
                            out.push(0x88);
                            out.extend(self.encode_rm_r(&op.params));
                        }
                    }
                    Parameter::Ptr8(_, _) |
                    Parameter::Ptr8Amode(_, _) |
                    Parameter::Ptr8AmodeS8(_, _, _) |
                    Parameter::Ptr8AmodeS16(_, _, _) => {
                        if let Parameter::Ptr8(_, imm16) = op.params.dst {
                            if let Parameter::Reg8(r) =  op.params.src {
                                if r == R::AL {
                                    // 0xA2: mov [moffs8], AL
                                    out.push(0xA2);
                                    out.push(imm16 as u8);
                                    out.push((imm16 >> 8) as u8);
                                    return Ok(out);
                                }
                            }
                        }

                        // 0x88: mov r/m8, r8
                        out.push(0x88);
                        out.extend(self.encode_rm_r(&op.params));
                    }
                    _ => {
                        return Err(EncodeError::UnhandledParameter(op.params.dst.clone()));
                    }
                }
            }
            Op::Mov16 => {
                //0x89: mov r/m16, r16
                //0x8B: mov r16, r/m16
                //0x8C: mov r/m16, sreg
                //0x8E: mov sreg, r/m16
                //0xC7: mov r/m16, imm16    reg = 0 for MOV.

                if op.params.src.is_imm() {
                    if let Parameter::Reg16(ref r) = op.params.dst {
                        //0xB8...0xBF: mov r16, u16
                        out.push(0xB8 | r.index() as u8);
                        if let Parameter::Imm16(imm16) = op.params.src {
                            out.push(imm16 as u8);
                            out.push((imm16 >> 8) as u8);
                        } else {
                            return Err(EncodeError::UnhandledParameter(op.params.src.clone()));
                        }
                    } else {
                        return Err(EncodeError::UnhandledParameter(op.params.dst.clone()));
                    }
                    /*
                } else if op.params.src.is_ptr() {
                    // 0x8A: mov r8, r/m8
                    out.push(0x8A);
                    out.extend(self.encode_r8_rm8(&op.params));
                } else {
                    // 0x88: mov r/m8, r8
                    out.push(0x88);
                    out.extend(self.encode_rm8_r8(&op.params));
                }*/
                } else {
                    return Err(EncodeError::UnhandledParameter(op.params.dst.clone()));
                }
            }
            Op::Mov32 => {
                // XXX TODO handle more forms
                out.push(0x66);
                if op.params.src.is_imm() {
                    if let Parameter::Reg32(ref r) = op.params.dst {
                        //0x66 0xB8...0xBF: mov r32, u32
                        out.push(0xB8 | r.index() as u8);
                        if let Parameter::Imm32(imm32) = op.params.src {
                            out.push(imm32 as u8);
                            out.push((imm32 >> 8) as u8);
                            out.push((imm32 >> 16) as u8);
                            out.push((imm32 >> 24) as u8);
                        } else {
                            return Err(EncodeError::UnhandledParameter(op.params.src.clone()));
                        }
                    } else {
                        return Err(EncodeError::UnhandledParameter(op.params.dst.clone()));
                    }
                } else {
                    return Err(EncodeError::UnhandledParameter(op.params.dst.clone()));
                }
            }
            Op::And8 | Op::Or8 | Op::Add8 | Op::Adc8 | Op::Sub8 | Op::Sbb8 | Op::Cmp8 | Op::Xor8 => {
                match self.arith_instr8(op) {
                    Ok(data) => out.extend(data),
                    Err(why) => return Err(why),
                }
            }
            Op::Test8 | Op::Not8 | Op::Neg8 | Op::Mul8 | Op::Imul8 | Op::Div8 | Op::Idiv8 => {
                match self.math_instr8(op) {
                    Ok(data) => out.extend(data),
                    Err(why) => return Err(why),
                }
            }
            Op::Rol8 | Op::Ror8 | Op::Rcl8 | Op::Rcr8 |
            Op::Shl8 | Op::Shr8 | Op::Sar8 => {
                match self.bitshift_instr8(op) {
                    Ok(data) => out.extend(data),
                    Err(why) => return Err(why),
                }
            }
            Op::Test16 | Op::Div16 | Op::Idiv16 | Op::Mul16 | Op::Imul16 => {
                match self.math_instr16(op) {
                    Ok(data) => out.extend(data),
                    Err(why) => return Err(why),
                }
            }
            Op::Push16 => {
                if let Parameter::Imm16(imm16) = op.params.dst {
                    // 0x68: push imm16
                    out.push(0x68);
                    out.push(imm16 as u8);
                    out.push((imm16 >> 8) as u8);
                } else {
                    return Err(EncodeError::UnhandledParameter(op.params.dst.clone()));
                }
            }
            Op::Popf => out.push(0x9D),
            _ => {
                return Err(EncodeError::UnhandledOp(op.command.clone()));
            }
        }
        Ok(out)
    }

    fn arith_instr8(&self, ins: &Instruction) -> Result<Vec<u8>, EncodeError> {
        let mut out = vec!();
        let idx = match ins.command {
            Op::Add8 => 0x00,
            Op::Or8  => 0x08,
            Op::Adc8 => 0x10,
            Op::Sbb8 => 0x18,
            Op::And8 => 0x20,
            Op::Sub8 => 0x28,
            Op::Xor8 => 0x30,
            Op::Cmp8 => 0x38,
            _ => panic!("unhandled {:?}", ins.command),
        };

        if let Parameter::Reg8(r) = ins.params.dst {
            if r == R::AL {
                if let Parameter::Imm8(imm) = ins.params.src {
                    // 0x0C: or AL, imm8
                    // 0x1C: sbb al, imm8
                    // 0x24: and AL, imm8
                    // 0x34: xor AL, imm8
                    // 0x3C: cmp AL, imm8
                    out.push(idx + 4);
                    out.push(imm);
                    return Ok(out);
                }
            }
        }
        match ins.params.dst {
            Parameter::Reg8(r) => {
                if let Parameter::Imm8(i) = ins.params.src {
                    // 0x80: <arithmetic> r/m8, imm8
                    out.push(0x80);
                    // md 3 = register adressing
                    let mrr = ModRegRm{md: 3, rm: r.index() as u8, reg: self.arith_index(&ins.command)};
                    out.push(mrr.u8());
                    out.push(i);
                } else if ins.params.src.is_ptr() {
                    // 0x0A: or r8, r/m8
                    // 0x1A: sbb r8, r/m8
                    // 0x22: and r8, r/m8
                    // 0x32: xor r8, r/m8
                    // 0x3A: cmp r8, r/m8
                    out.push(idx + 2);
                    out.extend(self.encode_r_rm(&ins.params));
                } else {
                    // 0x08: or r/m8, r8
                    // 0x10: adc r/m8, r8
                    // 0x18: sbb r/m8, r8
                    // 0x20: and r/m8, r8
                    // 0x28: sub r/m8, r8
                    // 0x30: xor r/m8, r8
                    // 0x38: cmp r/m8, r8
                    out.push(idx);
                    out.extend(self.encode_rm_r(&ins.params));
                }
                Ok(out)
            }
            Parameter::Ptr8(_, _) |
            Parameter::Ptr8Amode(_, _) |
            Parameter::Ptr8AmodeS8(_, _, _) |
            Parameter::Ptr8AmodeS16(_, _, _) => {
                // 0x08: or r/m8, r8
                // 0x10: adc r/m8, r8
                // 0x20: and r/m8, r8
                // 0x30: xor r/m8, r8
                // 0x38: cmp r/m8, r8
                out.push(idx);
                out.extend(self.encode_rm_r(&ins.params));
                Ok(out)
            }
            _ => Err(EncodeError::UnhandledParameter(ins.params.dst.clone())),
        }
    }

    fn math_instr8(&self, ins: &Instruction) -> Result<Vec<u8>, EncodeError> {
        // XXX 0xD2: bit shift byte by CL
        let mut out = vec!();
        match ins.params.dst {
            Parameter::Reg8(r) => {
                if ins.command == Op::Test8 {
                    if let Parameter::Imm8(i) = ins.params.src {
                        if r == R::AL {
                            // 0xA8: test AL, imm8
                            out.push(0xA8);
                        } else {
                            // 0xF6: test r/m8, imm8
                            out.push(0xF6);
                            out.push(ModRegRm::rm_reg(r.index() as u8, self.math_index(&ins.command)));
                        }
                        out.push(i);
                    } else {
                        // 0x84: test r/m8, r8
                        out.push(0x84);
                        out.extend(self.encode_rm_r(&ins.params));
                    }
                } else {
                    // 0xF6: <math> r/m8.  reg = instruction
                    out.push(0xF6);
                    out.push(ModRegRm::rm_reg(r.index() as u8, self.math_index(&ins.command)));
                }
                Ok(out)
            }
            Parameter::Ptr8(_, _) |
            Parameter::Ptr8Amode(_, _) |
            Parameter::Ptr8AmodeS8(_, _, _) |
            Parameter::Ptr8AmodeS16(_, _, _) => {
                if ins.command == Op::Test8 {
                    // 0x84: test r/m8, r8
                    out.push(0x84);
                    out.extend(self.encode_rm_r(&ins.params));
                } else {
                    // 0xF6: not r/m8
                    out.push(0xF6);
                    out.extend(self.encode_rm(&ins.params.dst, Encoder::f6_index(&ins.command)));
                }
                Ok(out)
            }
            _ => Err(EncodeError::UnexpectedDstType(ins.params.dst.clone())),
        }
    }

    fn math_instr16(&self, ins: &Instruction) -> Result<Vec<u8>, EncodeError> {
        let mut out = vec!();

        match ins.params.src2 {
            Parameter::ImmS8(v) => {
                // 3 operand form: 6B /r ib
                out.push(0x6B);
                out.extend(self.encode_r_rm(&ins.params));
                out.push(v as u8);
                return Ok(out);
            }
            _ => {}
        }

        if ins.command != Op::Test16 {
            match ins.params.src {
                Parameter::Reg16(_) => {
                    // 2 operand form: 0F AF /r
                    out.push(0x0F);
                    out.push(0xAF);
                    out.extend(self.encode_r_rm(&ins.params));
                    return Ok(out);
                }
                _ => {}
            }
        }

        match ins.params.dst {
            Parameter::Reg16(r) => {
                if ins.command == Op::Test16 {
                    if let Parameter::Imm16(i) = ins.params.src {
                        if r == R::AX {
                            out.push(0xA9); // test AX, imm16
                        } else {
                            out.push(0xF7); // test r/m16, imm16
                            out.push(ModRegRm::rm_reg(r.index() as u8, self.math_index(&ins.command)));
                        }
                        out.push(i as u8);
                        out.push((i >> 8) as u8);
                    } else {
                        out.push(0x85); // test r/m16, r16
                        out.extend(self.encode_rm_r(&ins.params));
                    }
                } else {
                    // 1 operand form: F7 /5
                    out.push(0xF7);
                    out.push(ModRegRm::rm_reg(r.index() as u8, self.math_index(&ins.command)));
                }
                Ok(out)
            }
            _ => Err(EncodeError::UnexpectedDstType(ins.params.dst.clone())),
        }
    }

    /// used for 0xF6 encodings
    fn f6_index(op: &Op) -> u8 {
        match *op {
            Op::Test8 => 0,
            Op::Not8  => 2,
            Op::Neg8  => 3,
            Op::Mul8  => 4,
            Op::Imul8 => 5,
            Op::Div8  => 6,
            Op::Idiv8 => 7,
            _ => panic!("f6_index {:?}", op),
        }
    }

    /// used for 0xFE and 0xFF encodings
    fn feff_index(op: &Op) -> u8 {
        match *op {
            Op::Inc8 | Op::Inc16 | Op::Inc32 => 0,
            Op::Dec8 | Op::Dec16 | Op::Dec32 => 1,
            Op::CallNear => 2,
            // 3 => call far
            Op::JmpNear => 4,
            // 5 => jmp far
            Op::Push16 => 6,
            _ => panic!("feff_index {:?}", op),
        }
    }

    fn bitshift_instr8(&self, ins: &Instruction) -> Result<Vec<u8>, EncodeError> {
        let mut out = vec!();
        match ins.params.dst {
            Parameter::Reg8(r) => {
                match ins.params.src {
                    Parameter::Imm8(i) => {
                        // md 3 = register adressing
                        // XXX ModRegRm.rm really should use enum AMode, not like AMode is now. naming there is wrong
                        let mrr = ModRegRm{md: 3, rm: r.index() as u8, reg: self.bitshift_index(&ins.command)};
                        if i == 1 {
                            // 0xD0: bit shift byte by 1
                            out.push(0xD0);
                            out.push(mrr.u8());
                        } else {
                            // 0xC0: r8, byte imm8
                            out.push(0xC0);
                            out.push(mrr.u8());
                            out.push(i as u8);
                        }
                    }
                    _ => {
                        // 0xD2: bit shift byte by CL
                        panic!("bitshift_instr8 {:?}", ins);
                    }
                }
                Ok(out)
            }
            _ => Err(EncodeError::UnexpectedDstType(ins.params.dst.clone())),
        }
    }

    fn bitshift_index(&self, op: &Op) -> u8 {
        match *op {
            Op::Rol8 => 0,
            Op::Ror8 => 1,
            Op::Rcl8 => 2,
            Op::Rcr8 => 3,
            Op::Shl8 => 4,
            Op::Shr8 => 5,
            Op::Sar8 => 7,
            _ => panic!("bitshift_get_index {:?}", op),
        }
    }

    fn arith_index(&self, op: &Op) -> u8 {
        match *op {
            Op::Add8 => 0,
            Op::Or8  => 1,
            Op::Adc8 => 2,
            Op::Sbb8 => 3,
            Op::And8 => 4,
            Op::Sub8 => 5,
            Op::Xor8 => 6,
            Op::Cmp8 => 7,
            _ => panic!("arith_get_index {:?}", op),
        }
    }

    fn math_index(&self, op: &Op) -> u8 {
        match *op {
            Op::Test8 | Op::Test16 => 0,
            Op::Not8  => 2,
            Op::Neg8 => 3,
            Op::Mul8 | Op::Mul16 => 4,
            Op::Imul8 | Op::Imul16 => 5,
            Op::Div8 | Op::Div16 => 6,
            Op::Idiv8 | Op::Idiv16 => 7,
            _ => panic!("math_get_index {:?}", op),
        }
    }

    fn encode_r_rm(&self, params: &ParameterSet) -> Vec<u8> {
        match params.dst {
            Parameter::Reg8(ref r) |
            Parameter::Reg16(ref r) => self.encode_rm(&params.src, r.index() as u8),
            _ => unreachable!(),
        }
    }

    fn encode_rm_r(&self, params: &ParameterSet) -> Vec<u8> {
        match params.src {
            Parameter::Reg8(ref r) |
            Parameter::Reg16(ref r) => self.encode_rm(&params.dst, r.index() as u8),
            _ => panic!("unexpected parameter type: {:?}", params.src),
        }
    }

    fn encode_rm_r_imm(&self, params: &ParameterSet) -> Vec<u8> {
        let mut out = Vec::new();
        // shld r/m16, r16, imm8
        out.extend(match params.src {
            Parameter::Reg8(ref r) |
            Parameter::Reg16(ref r) => self.encode_rm(&params.dst, r.index() as u8),
            _ => panic!("unexpected parameter type: {:?}", params.src),
        });
        if let Parameter::Imm8(imm) = params.src2 {
            out.push(imm);
        } else {
            unreachable!();
        }
        out
    }

    fn encode_rm(&self, dst: &Parameter, reg: u8) -> Vec<u8> {
        let mut out = Vec::new();
        match *dst {
            Parameter::Ptr8(_, imm16) => {
                out.push(ModRegRm{md: 0, rm: 6, reg}.u8());
                out.push(imm16 as u8);
                out.push((imm16 >> 8) as u8);
            }
            Parameter::Ptr8Amode(_, ref amode) |
            Parameter::Ptr16Amode(_, ref amode) => {
                // XXX how does md:0, rm: 0 not collide with above one...
                out.push(ModRegRm{md: 0, rm: amode.index() as u8, reg}.u8());
            }
            Parameter::Ptr8AmodeS8(_, ref amode, imm) |
            Parameter::Ptr16AmodeS8(_, ref amode, imm) => {
                out.push(ModRegRm{md: 1, rm: amode.index() as u8, reg}.u8());
                out.push(imm as u8);
            },
            Parameter::Ptr8AmodeS16(_, ref amode, imm16) => {
                out.push(ModRegRm{md: 2, rm: amode.index() as u8, reg}.u8());
                out.push(imm16 as u8);
                out.push((imm16 >> 8) as u8);
            }
            Parameter::Reg8(ref r) |
            Parameter::Reg16(ref r) |
            Parameter::Reg32(ref r) => {
                out.push(ModRegRm{md: 3, rm: r.index() as u8, reg}.u8());
            }
            _ => panic!("encode_rm: unhandled md {:?}", dst),
        }
        out
    }

    fn encode_imm8(&self, param: &Parameter) -> Vec<u8> {
        let mut out = Vec::new();
        if let Parameter::Imm8(imm) = *param {
            out.push(imm as u8);
            return out;
        }
        panic!("not imm8 {:?}", param);
    }
}
