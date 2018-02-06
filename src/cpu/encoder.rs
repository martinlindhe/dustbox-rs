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
        }
        UnhandledParameter(param: Parameter) {
            description("unhandled param")
        }
        Gizmo {
            description("Refrob the Gizmo")
        }
        WidgetNotFound(widget_name: String) {
            description("The widget could not be found")
            display(r#"The widget "{}" could not be found"#, widget_name)
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

            Op::Rol8 | Op::Ror8 | Op::Rcl8 | Op::Rcr8 |
            Op::Shl8 | Op::Shr8 | Op::Sar8 => {
                out.extend(self.bitshift_instr8(op));
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

    fn bitshift_instr8(&self, ins: &Instruction) -> Vec<u8> {
        let mut out = vec!();
        match ins.params.dst {
            Parameter::Reg8(r) => {
                match ins.params.src {
                    Parameter::Imm8(i) => {
                        // md 3 = register adressing
                        // XXX ModRegRm.rm really should use enum AMode, not like AMode is now. naming there is wrong
                        let mrr = ModRegRm{md: 3, rm: r.index() as u8, reg: self.bitshift_get_index(&ins.command)};
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
            }
            _ => {
                panic!("unexpected dst type: {:?}", ins);
            }
        }
        out
    }

    fn bitshift_get_index(&self, op: &Op) -> u8 {
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
            unreachable!();
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
fn ndisasm_bytes(bytes: &[u8]) -> Result<String, io::Error> {
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
