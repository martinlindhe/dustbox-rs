use cpu::Instruction;
use cpu::Parameter;
use cpu::Op;

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
            Op::Mov8() => {
                // 0x88: mov r/m8, r8
                // 0x8A: mov r8, r/m8

                // NOTE: r8, r8 can use either encoding.
                if op.params.src.is_ptr() {
                    out.push(0x8A);
                } else {
                    out.push(0x88);
                }
            }
            Op::Int() => out.push(0xCD),
            _ => {
                panic!("encode: unhandled op {}", op);
            }
        }
        match op.params.dst {
            Parameter::Imm8(imm) => {
                out.push(imm);
            }
            _ => {
                panic!("encode: unhandled param {}", op.params.dst);
            }
        }
        out
    }
}
