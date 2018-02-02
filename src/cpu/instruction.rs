use std::fmt;
use std::num::Wrapping;

use cpu::segment::Segment;
use cpu::register::{R8, R16, SR, AMode};
use cpu::op::Op;
use cpu::parameter::{Parameter, ParameterPair};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RepeatMode {
    None,
    Rep,
    Repe, // alias repz
    Repne, // alias repnz
}

impl RepeatMode {
    fn as_str(&self) -> &str {
        match *self {
            RepeatMode::None => "",
            RepeatMode::Rep => "Rep ",
            RepeatMode::Repe => "Repe ",
            RepeatMode::Repne => "Repne ",
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Instruction {
    pub command: Op,
    pub params: ParameterPair,
    pub segment_prefix: Segment,
    pub length: u8,
    pub repeat: RepeatMode, // REPcc prefix
    pub lock: bool,         // LOCK prefix
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let instr = self.describe_instruction();
        if self.segment_prefix == Segment::Default || self.hide_segment_prefix() {
            write!(f, "{}", instr)
        } else {
            write!(f, "{} {}", self.segment_prefix.as_str(), instr)
        }
    }
}

impl Instruction {
    pub fn new1(op: Op, dst: Parameter) -> Self {
        Instruction {
            command: op,
            segment_prefix: Segment::Default,
            lock: false,
            repeat: RepeatMode::None,
            params: ParameterPair {
                dst: dst,
                src: Parameter::None,
                src2: Parameter::None,
            },
            length: 0, // XXX remove length here, cannot be known
        }
    }

    pub fn new2(op: Op, dst: Parameter, src: Parameter) -> Self {
        Instruction {
            command: op,
            segment_prefix: Segment::Default,
            lock: false,
            repeat: RepeatMode::None,
            params: ParameterPair {
                dst: dst,
                src: src,
                src2: Parameter::None,
            },
            length: 0, // XXX remove length here, cannot be known
        }
    }

    fn hide_segment_prefix(&self) -> bool {
        self.command == Op::Mov8() ||
        self.command == Op::Mov16()
    }

    fn describe_instruction(&self) -> String {
        let prefix = self.repeat.as_str();

        match self.params.dst {
            Parameter::None => format!("{}{:?}", prefix, self.command),
            _ => {
                let cmd = right_pad(&format!("{}{:?}", prefix, self.command), 9);

                match self.params.src2 {
                    Parameter::None => match self.params.src {
                        Parameter::None => format!("{}{}", cmd, self.params.dst),
                        _ => format!("{}{}, {}", cmd, self.params.dst, self.params.src),
                    },
                    _ => format!(
                        "{}{}, {}, {}",
                        cmd,
                        self.params.dst,
                        self.params.src,
                        self.params.src2
                    ),
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct InstructionInfo {
    pub segment: usize,
    pub offset: usize,
    pub text: String,
    pub bytes: Vec<u8>,
    pub instruction: Instruction,
}

impl fmt::Display for InstructionInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let hex = self.to_hex_string(&self.bytes);
        write!(
            f,
            "[{:04X}:{:04X}] {} {}",
            self.segment,
            self.offset,
            right_pad(&hex, 16),
            self.text
        )
    }
}

impl InstructionInfo {
    fn to_hex_string(&self, bytes: &[u8]) -> String {
        let strs: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
        strs.join("")
    }
}

#[derive(Debug)]
pub struct ModRegRm {
    pub md: u8,  /// "mod" is correct name, but is reserved keyword
    pub reg: u8,
    pub rm: u8,
}

impl ModRegRm {
    pub fn u8(&self) -> u8 {
        (self.md << 6) |  // high 2 bits
        (self.reg << 3) | // mid 3 bits
        self.rm           // low 3 bits
    }
}

fn right_pad(s: &str, len: usize) -> String {
    let mut res = String::new();
    res.push_str(s);
    if s.len() < len {
        let padding_len = len - s.len();
        for _ in 0..padding_len {
            res.push_str(" ");
        }
    }
    res
}
