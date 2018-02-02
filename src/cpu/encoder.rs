use cpu::instruction::{Instruction, ModRegRm};
use cpu::parameter::{Parameter, ParameterSet};
use cpu::op::{Op};

#[cfg(test)]
#[path = "./encoder_test.rs"]
mod encoder_test;

pub struct Encoder {
}

impl Encoder {
    pub fn new() -> Self {
        Encoder {
        }
    }

    /// encodes Instruction to a valid byte sequence
    pub fn encode(&self, op: &Instruction) -> Vec<u8> {
        let mut out = vec!();
        match op.command {
            Op::Int() => {
                out.push(0xCD);
                out.extend(self.encode_imm8(&op.params.dst).iter().cloned());
            }
            Op::Mov8() => {
                if op.params.dst.is_reg() && op.params.src.is_imm() {
                    // 0xB0...0xB7
                    // mov r8, u8
                    if let Parameter::Reg8(r) = op.params.dst {
                        out.push(0xB0 | r as u8);
                    }
                    if let Parameter::Imm8(i) = op.params.src {
                        out.push(i as u8);
                    }
                } else if op.params.src.is_ptr() {
                    // 0x8A: mov r8, r/m8
                    out.push(0x8A);
                    out.extend(self.encode_r8_rm8(&op.params).iter().cloned());
                } else {
                    // 0x88: mov r/m8, r8
                    out.push(0x88);
                    out.extend(self.encode_rm8_r8(&op.params).iter().cloned());
                }
            }

            Op::Rol8 | Op::Ror8 | Op::Rcl8 | Op::Rcr8 |
            Op::Shl8 | Op::Shr8 | Op::Sar8 => {
                out.extend(self.bitshift_instr8(&op).iter().cloned());
            }
            _ => {
                panic!("encode: unhandled op {}", op);
            }
        }
        out
    }

    fn bitshift_instr8(&self, ins: &Instruction) -> Vec<u8> {
        let mut out = vec!();
        if ins.params.dst.is_reg() && ins.params.src.is_imm() {
            if let Parameter::Imm8(i) = ins.params.src {
                // 0xC0: r8, byte imm8
                out.push(0xC0);
                if let Parameter::Reg8(r) = ins.params.dst {
                    // md 3 = register adressing, rm = the register value
                    let mut mrr = ModRegRm{md: 3, rm: r.index() as u8, reg: self.bitshift_get_index(&ins.command)};
                    out.push(mrr.u8());
                } else {
                    unreachable!();
                }
                out.push(i as u8);
            } else {
                unreachable!();
            }
        } else {
            // 0xD0: bit shift byte by 1
            // 0xD2: bit shift byte by CL
            panic!("bitshift_instr8 {:?}", ins);
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
        self.encode_rm8(&params.src, &params.dst)
    }

    fn encode_rm8_r8(&self, params: &ParameterSet) -> Vec<u8> {
        self.encode_rm8(&params.dst, &params.src)
    }

    fn encode_rm8(&self, dst: &Parameter, src: &Parameter) -> Vec<u8> {
        let mut out = Vec::new();
        match dst {
            &Parameter::Ptr8(_, imm16) => {
                let mut mrr = ModRegRm{md: 0, rm: 6, reg: 0};
                if let &Parameter::Reg8(src_r) = src {
                    mrr.reg = src_r as u8
                } else {
                    unreachable!();
                }
                out.push(mrr.u8());
                out.push((imm16 & 0xFF) as u8);
                out.push((imm16 >> 8) as u8);
            }
            &Parameter::Ptr8Amode(_, ref amode) => {
                // XXX how doe md:0, rm: 0 not collide with above one...
                let mut mrr = ModRegRm{md: 0, rm: amode.index() as u8, reg: 0};
                if let &Parameter::Reg8(src_r) = src {
                    mrr.reg = src_r as u8
                } else {
                    unreachable!();
                }
                out.push(mrr.u8());
            }
            &Parameter::Ptr8AmodeS8(_, ref amode, imm) => {
                let mut mrr = ModRegRm{md: 1, rm: amode.index() as u8, reg: 0};
                if let &Parameter::Reg8(reg) = src {
                    mrr.reg = reg as u8;
                } else {
                    unreachable!();
                }
                out.push(mrr.u8());
                out.push(imm as u8);
            },
            &Parameter::Ptr8AmodeS16(_, ref amode, imm16) => {
                let mut mrr = ModRegRm{md: 2, rm: amode.index() as u8, reg: 0};
                if let &Parameter::Reg8(reg) = src {
                    mrr.reg = reg as u8;
                } else {
                    unreachable!();
                }
                out.push(mrr.u8());
                out.push((imm16 & 0xFF) as u8);
                out.push((imm16 >> 8) as u8);
            }
            &Parameter::Reg8(r) => {
                let mut mrr = ModRegRm{md: 3, rm: r as u8, reg: 0};
                if let &Parameter::Reg8(src_r) = src {
                    mrr.reg = src_r as u8
                } else {
                    unreachable!();
                }
                out.push(mrr.u8());
            }
            _ => {
                panic!("XXX unhandled md encoding {:?}", dst);
            }
        }

        out
    }

    fn encode_imm8(&self, param: &Parameter) -> Vec<u8> {
        let mut out = Vec::new();
        if let &Parameter::Imm8(imm) = param {
            out.push(imm as u8);
            return out;
        }
        panic!("not imm8 {:?}", param);
    }
}
