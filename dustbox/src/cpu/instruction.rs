use std::fmt;

use crate::cpu::Segment;
use crate::cpu::Op;
use crate::cpu::{Parameter, ParameterSet};
use crate::cpu::{OperandSize, AddressSize};
use crate::hex::hex_bytes;
use crate::string::right_pad;

#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    pub command: Op,
    pub params: ParameterSet,
    pub length: u8,
    // op prefixes
    pub segment_prefix: Segment,    // segment prefix opcode
    pub repeat: RepeatMode,         // REPcc prefix
    pub lock: bool,                 // LOCK prefix
    pub op_size: OperandSize,       // 0x66 prefix
    pub address_size: AddressSize,  // 0x67 prefix
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
    pub fn new(op: Op) -> Self {
        Instruction::new3(op, Parameter::None, Parameter::None, Parameter::None)
    }

    pub fn new1(op: Op, dst: Parameter) -> Self {
        Instruction::new3(op, dst, Parameter::None, Parameter::None)
    }

    pub fn new2(op: Op, dst: Parameter, src: Parameter) -> Self {
        Instruction::new3(op, dst, src, Parameter::None)
    }

    pub fn new3(op: Op, dst: Parameter, src: Parameter, src2: Parameter) -> Self {
        let op_size = Instruction::op_size_from_op(&op);
        Instruction {
            command: op,
            segment_prefix: Segment::Default,
            params: ParameterSet {dst, src, src2},
            lock: false,
            repeat: RepeatMode::None,
            op_size,
            address_size: AddressSize::_16bit,
            length: 0,
        }
    }

    // used to decorate tracer
    pub fn is_ret(&self) -> bool {
        match self.command {
            Op::Retn | Op::Retf | Op::RetImm16 => true,
            _ => false,
        }
    }

    // used to decorate tracer
    pub fn is_loop(&self) -> bool {
        match self.command {
            Op::Loop16 |  Op::Loop32 | Op::Loop16e | Op::Loop32e | Op::Loop16ne | Op::Loop32ne => true,
            _ => false,
        }
    }

    // used to decorate tracer
    pub fn is_unconditional_jmp(&self) -> bool {
        match self.command {
            Op::JmpShort | Op::JmpNear | Op::JmpFar => true,
            _ => false,
        }
    }

    fn op_size_from_op(op: &Op) -> OperandSize {
        match *op {
            Op::Mov32 | Op::Inc32 | Op::Dec32 => OperandSize::_32bit,
            _ => OperandSize::_16bit,
        }
    }

    fn hide_segment_prefix(&self) -> bool {
        self.command == Op::Add8 || self.command == Op::Add16 || self.command == Op::Add32 ||
        self.command == Op::Adc8 || self.command == Op::Adc16 || self.command == Op::Adc32 ||
        self.command == Op::Sub8 || self.command == Op::Sub16 || self.command == Op::Sub32 ||
        self.command == Op::Sbb8 || self.command == Op::Sbb16 || self.command == Op::Sbb32 ||
        self.command == Op::Inc8 || self.command == Op::Inc16 || self.command == Op::Inc32 ||
        self.command == Op::Dec8 || self.command == Op::Dec16 || self.command == Op::Dec32 ||
        self.command == Op::Mul8 || self.command == Op::Mul16 || self.command == Op::Mul32 ||
        self.command == Op::Div8 || self.command == Op::Div16 || self.command == Op::Div32 ||
        self.command == Op::Imul8 || self.command == Op::Imul16 || self.command == Op::Imul32 ||
        self.command == Op::Idiv8 || self.command == Op::Idiv16 || self.command == Op::Idiv32 ||
        self.command == Op::And8 || self.command == Op::And16 || self.command == Op::And32 ||
        self.command == Op::Or8 || self.command == Op::Or16 || self.command == Op::Or32 ||
        self.command == Op::Xor8 || self.command == Op::Xor16 || self.command == Op::Xor32 ||
        self.command == Op::Cmp8 || self.command == Op::Cmp16 || self.command == Op::Cmp32 ||
        self.command == Op::Test8 || self.command == Op::Test16 || self.command == Op::Test32 ||
        self.command == Op::Xchg8 || self.command == Op::Xchg16 || self.command == Op::Xchg32 ||
        self.command == Op::Mov8 || self.command == Op::Mov16 || self.command == Op::Mov32 ||
        self.command == Op::Movsx16 || self.command == Op::Movsx32 || self.command == Op::Movzx16
    }

    fn describe_instruction(&self) -> String {
        let op_space = 9;
        let mut prefix = self.repeat.as_str().to_owned();
        if prefix != "" {
            prefix = right_pad(&prefix, op_space);
        }

        match self.params.dst {
            Parameter::None => format!("{}{}", prefix, self.command),
            _ => {
                let cmd = right_pad(&format!("{}{}", prefix, self.command), op_space);

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
    pub segment: u16,
    pub offset: u32,
    pub bytes: Vec<u8>,
    pub instruction: Instruction,
}

impl fmt::Display for InstructionInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{:04X}:{:04X}] {} {}",
            self.segment,
            self.offset,
            right_pad(&hex_bytes(&self.bytes), 16),
            format!("{}", self.instruction),
        )
    }
}

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
            RepeatMode::Rep => "Rep",
            RepeatMode::Repe => "Repe",
            RepeatMode::Repne => "Repne",
        }
    }
}

/// Instruction encoding layout for Scale/Index/Base byte
#[derive(Debug)]
pub struct SIB {
    /// High 2 bits
    pub scale: u8,
    /// Mid 3 bits
    pub index: u8,
    /// Low 3 bits
    pub base: u8,
}

/// Instruction encoding layout for Mod/Reg/RM byte
#[derive(Debug)]
pub struct ModRegRm {
    /// "mod" is correct name, but is reserved keyword
    /// High 2 bits
    pub md: u8,

    /// mid 3 bits
    pub reg: u8,

    /// low 3 bits
    pub rm: u8,
}

impl ModRegRm {
    pub fn u8(&self) -> u8 {
        (self.md << 6) |  // high 2 bits
        (self.reg << 3) | // mid 3 bits
        self.rm           // low 3 bits
    }

    pub fn rm_reg(rm: u8, reg: u8) -> u8 {
        // md 3 = register adressing
        // XXX ModRegRm.rm really should use enum AMode, not like AMode is now. naming there is wrong
        ModRegRm{md: 3, rm, reg}.u8()
    }
}
