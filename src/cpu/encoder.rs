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
