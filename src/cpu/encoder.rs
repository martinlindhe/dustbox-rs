use simple_error::SimpleError;

use cpu::instruction::{Instruction, ModRegRm};
use cpu::parameter::{Parameter, ParameterSet};
use cpu::register::{R8, R16};
use cpu::segment::Segment;
use cpu::op::{Op};

#[cfg(test)]
#[path = "./encoder_test.rs"]
mod encoder_test;

quick_error! {
    #[derive(Debug)]
    pub enum EncodeError {
        UnhandledOp(op: Op) {
            description("unhandled op")
            display("unhandled op: {:?}", op)
        }
        UnhandledParameter(param: Parameter) {
            description("unhandled param")
        }
        Text(s: String) {
            description("text")
            display("{}", s)
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
            Op::Aaa => out.push(0x37),
            Op::Aas => out.push(0x3F),
            Op::Dec8 | Op::Inc8 => {
                // 0xFE: r/m8
                out.push(0xFE);
                out.extend(self.encode_rm(&op.params.dst, op.command.feff_index()));
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
                    out.extend(self.encode_rm(&op.params.dst, op.command.feff_index()));
                }
            }
            Op::Int() => {
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
            Op::Mov8 => {
                match op.params.dst {
                    Parameter::Reg8(r) => {
                        if r == R8::AL {
                            if let Parameter::Ptr8(_, imm16) = op.params.src {
                                // 0xA0: mov AL, [moffs8]
                                out.push(0xA0);
                                out.push((imm16 & 0xFF) as u8);
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
                            out.extend(self.encode_r8_rm8(&op.params));
                        } else {
                            // 0x88: mov r/m8, r8
                            out.push(0x88);
                            out.extend(self.encode_rm8_r8(&op.params));
                        }
                    }
                    Parameter::Ptr8(_, _) |
                    Parameter::Ptr8Amode(_, _) |
                    Parameter::Ptr8AmodeS8(_, _, _) |
                    Parameter::Ptr8AmodeS16(_, _, _) => {
                        if let Parameter::Ptr8(_, imm16) = op.params.dst {
                            if let Parameter::Reg8(r) =  op.params.src {
                                if r == R8::AL {
                                    // 0xA2: mov [moffs8], AL
                                    out.push(0xA2);
                                    out.push((imm16 & 0xFF) as u8);
                                    out.push((imm16 >> 8) as u8);
                                    return Ok(out);
                                }
                            }
                        }

                        // 0x88: mov r/m8, r8
                        out.push(0x88);
                        out.extend(self.encode_rm8_r8(&op.params));
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

                if op.params.dst.is_reg() && op.params.src.is_imm() {
                    //0xB8...0xBF: mov r16, u16
                    if let Parameter::Reg16(ref r) = op.params.dst {
                        out.push(0xB8 | r.index() as u8);
                    } else {
                        return Err(EncodeError::UnhandledParameter(op.params.dst.clone()));
                    }
                    if let Parameter::Imm16(imm16) = op.params.src {
                        out.push((imm16 & 0xFF) as u8);
                        out.push((imm16 >> 8) as u8);
                    } else {
                        return Err(EncodeError::UnhandledParameter(op.params.src.clone()));
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
            Op::And8 | Op::Or8 | Op::Add8 | Op::Adc8 | Op::Sub8 | Op::Sbb8 | Op::Cmp8 | Op::Xor8 => {
                match self.arith_instr8(op) {
                    Ok(data) => out.extend(data),
                    Err(why) => return Err(EncodeError::Text(why.as_str().to_owned())),
                }
            }
            Op::Test8 | Op::Not8 | Op::Neg8 | Op::Mul8 | Op::Imul8 | Op::Div8 | Op::Idiv8 => {
                match self.math_instr8(op) {
                    Ok(data) => out.extend(data),
                    Err(why) => return Err(EncodeError::Text(why.as_str().to_owned())),
                }
            }
            Op::Rol8 | Op::Ror8 | Op::Rcl8 | Op::Rcr8 |
            Op::Shl8 | Op::Shr8 | Op::Sar8 => {
                match self.bitshift_instr8(op) {
                    Ok(data) => out.extend(data),
                    Err(why) => return Err(EncodeError::Text(why.as_str().to_owned())),
                }
            }
            Op::Push16 => {
                if let Parameter::Imm16(imm16) = op.params.dst {
                    // 0x68: push imm16
                    out.push(0x68);
                    out.push((imm16 & 0xFF) as u8);
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

    fn arith_instr8(&self, ins: &Instruction) -> Result<Vec<u8>, SimpleError> {
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
            if r == R8::AL {
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
                    out.extend(self.encode_r8_rm8(&ins.params));
                } else {
                    // 0x08: or r/m8, r8
                    // 0x10: adc r/m8, r8
                    // 0x18: sbb r/m8, r8
                    // 0x20: and r/m8, r8
                    // 0x28: sub r/m8, r8
                    // 0x30: xor r/m8, r8
                    // 0x38: cmp r/m8, r8
                    out.push(idx);
                    out.extend(self.encode_rm8_r8(&ins.params));
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
                out.extend(self.encode_rm8_r8(&ins.params));
                Ok(out)
            }
            _ => Err(SimpleError::new(format!("unhandled param {:?}", ins.params.dst))),
        }
    }

    fn math_instr8(&self, ins: &Instruction) -> Result<Vec<u8>, SimpleError> {
        // XXX 0xD2: bit shift byte by CL
        let mut out = vec!();
        match ins.params.dst {
            Parameter::Reg8(r) => {
                if ins.command == Op::Test8 {
                    if let Parameter::Imm8(i) = ins.params.src {
                        if r == R8::AL {
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
                        out.extend(self.encode_rm8_r8(&ins.params));
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
                    out.extend(self.encode_rm8_r8(&ins.params));
                } else {
                    // 0xF6: not r/m8
                    out.push(0xF6);
                    out.extend(self.encode_rm(&ins.params.dst, ins.command.f6_index()));
                }
                Ok(out)
            }
            _ => Err(SimpleError::new(format!("unexpected dst type: {:?}", ins.params.dst))),
        }
    }

    fn bitshift_instr8(&self, ins: &Instruction) -> Result<Vec<u8>, SimpleError> {
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
            _ => Err(SimpleError::new(format!("unexpected dst type: {:?}", ins.params.dst))),
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
            Op::Test8 => 0,
            Op::Not8  => 2,
            Op::Neg8 => 3,
            Op::Mul8 => 4,
            Op::Imul8 => 5,
            Op::Div8 => 6,
            Op::Idiv8 => 7,
            _ => panic!("math_get_index {:?}", op),
        }
    }

    fn encode_r8_rm8(&self, params: &ParameterSet) -> Vec<u8> {
        if let Parameter::Reg8(r) = params.dst {
            self.encode_rm(&params.src, r.index() as u8)
        } else {
            unreachable!();
        }
    }

    fn encode_rm8_r8(&self, params: &ParameterSet) -> Vec<u8> {
        if let Parameter::Reg8(r) = params.src {
            self.encode_rm(&params.dst, r.index() as u8)
        } else {
            panic!("unexpected parameter type: {:?}", params.src);
        }
    }

    fn encode_rm(&self, dst: &Parameter, reg: u8) -> Vec<u8> {
        let mut out = Vec::new();
        match *dst {
            Parameter::Ptr8(_, imm16) => {
                let mut mrr = ModRegRm{md: 0, rm: 6, reg: reg};
                out.push(mrr.u8());
                out.push((imm16 & 0xFF) as u8);
                out.push((imm16 >> 8) as u8);
            }
            Parameter::Ptr8Amode(_, ref amode) => {
                // XXX how doe md:0, rm: 0 not collide with above one...
                let mut mrr = ModRegRm{md: 0, rm: amode.index() as u8, reg: reg};
                out.push(mrr.u8());
            }
            Parameter::Ptr8AmodeS8(_, ref amode, imm) |
            Parameter::Ptr16AmodeS8(_, ref amode, imm) => {
                let mut mrr = ModRegRm{md: 1, rm: amode.index() as u8, reg: reg};
                out.push(mrr.u8());
                out.push(imm as u8);
            },
            Parameter::Ptr8AmodeS16(_, ref amode, imm16) => {
                let mut mrr = ModRegRm{md: 2, rm: amode.index() as u8, reg: reg};
                out.push(mrr.u8());
                out.push((imm16 & 0xFF) as u8);
                out.push((imm16 >> 8) as u8);
            }
            Parameter::Reg8(r) => {
                let mut mrr = ModRegRm{md: 3, rm: r as u8, reg: reg};
                out.push(mrr.u8());
            }
            _ => {
                panic!("encode_rm: unhandled md {:?}", dst);
            }
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

use std::io::{self, Read, Write};
pub fn ndisasm_bytes(bytes: &[u8]) -> Result<String, io::Error> { // XXX own top-level module named ndisasm
    use std::str;
    use std::fs::File;
    use std::process::Command;
    use tempdir::TempDir;
    let tmp_dir = TempDir::new("ndisasm")?;
    let file_path = tmp_dir.path().join("binary.bin");
    let file_str = file_path.to_str().unwrap();
    let mut tmp_file = File::create(&file_path)?;

    tmp_file.write_all(bytes)?;

    let output = Command::new("ndisasm")
        .args(&["-b", "16", file_str])
        .output()
        .expect("failed to execute process");

    drop(tmp_file);
    tmp_dir.close()?;

    let s = str::from_utf8(&output.stdout).unwrap().trim();

    // parse syntax "00000000  CD21              int 0x21", return third column
    let mut col = 0;
    let mut spacing = false;
    let mut res = String::new();
    for c in s.chars() {
        if c == ' ' {
            if !spacing && col < 2 {
                col += 1;
                spacing = true;
            }
        } else {
            spacing = false;
        }
        if col == 2 {
            res.push(c);
        }
    }

    Ok(res.trim().to_owned())
}

/// disasm the encoded instruction with external ndisasm command
fn ndisasm_instruction(op: &Instruction) -> Result<String, io::Error> {
    let encoder = Encoder::new();
    if let Ok(data) = encoder.encode(op) {
        return ndisasm_bytes(&data);
    } else {
        panic!("invalid byte sequence");
    }
}
