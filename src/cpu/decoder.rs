use std::rc::Rc;
use std::cell::RefCell;

use cpu::instruction::{Instruction, InstructionInfo, ModRegRm, RepeatMode};
use cpu::parameter::{Parameter, ParameterSet};
use cpu::op::{Op, InvalidOp};
use cpu::register::{R8, R16, SR};
use cpu::segment::Segment;
use memory::mmu::{MMU, MemoryAddress};

const DEBUG_DECODER: bool = false;

#[cfg(test)]
#[path = "./decoder_test.rs"]
mod decoder_test;

#[derive(Clone)]
pub struct Decoder {
    c_seg: u16,
    c_offset: u16,
}

impl Decoder {
    pub fn new() -> Self {
        Decoder {
            c_seg: 0,
            c_offset: 0,
        }
    }

    pub fn decode_to_block(&mut self, mut mmu: &mut MMU, seg: u16, offset: u16, n: usize) -> Vec<InstructionInfo> {
        let mut ops: Vec<InstructionInfo> = Vec::new();
        let mut inst_offset = 0;
        for _ in 0..n {
            let op = self.decode_instruction(&mut mmu, seg, offset+inst_offset);
            inst_offset += op.bytes.len() as u16;
            ops.push(op);
        }
        ops
    }

    pub fn disassemble_block_to_str(&mut self, mut mmu: &mut MMU, seg: u16, offset: u16, n: usize) -> String {
        let ops = self.decode_to_block(&mut mmu, seg, offset, n);
        instruction_info_to_str(&ops)
    }

    pub fn decode_instruction(&mut self, mut mmu: &mut MMU, iseg: u16, ioffset: u16) -> InstructionInfo {
        let (op, length) = self.get_instruction(&mut mmu, Segment::Default, iseg, ioffset);
        if DEBUG_DECODER {
            println!("decode_instruction at {:06x}: {:?}", MemoryAddress::RealSegmentOffset(iseg, ioffset).value(), op);
        }
        InstructionInfo {
            segment: iseg as usize,
            offset: ioffset as usize,
            bytes: mmu.read(iseg, ioffset, length),
            instruction: op
        }
    }

    pub fn get_instruction(&mut self, mut mmu: &mut MMU, seg: Segment, iseg: u16, ioffset: u16) -> (Instruction, usize) {
        self.c_seg = iseg;
        self.c_offset = ioffset;
        self.decode(&mut mmu, seg)
    }

    fn decode(&mut self, mut mmu: &mut MMU, seg: Segment) -> (Instruction, usize) {
        let ioffset = self.c_offset;
        let b = self.read_u8(mmu);

        let mut op = Instruction {
            command: Op::Unknown,
            params: ParameterSet {
                dst: Parameter::None,
                src: Parameter::None,
                src2: Parameter::None,
            },
            segment_prefix: seg,
            repeat: RepeatMode::None,
            lock: false,
        };

        match b {
            0x00 => {
                // add r/m8, r8
                op.command = Op::Add8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x01 => {
                // add r/m16, r16
                op.command = Op::Add16;
                op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
            }
            0x02 => {
                // add r8, r/m8
                op.command = Op::Add8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x03 => {
                // add r16, r/m16
                op.command = Op::Add16;
                op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
            }
            0x04 => {
                // add AL, imm8
                op.command = Op::Add8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x05 => {
                // add AX, imm16
                op.command = Op::Add16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x06 => {
                // push es
                op.command = Op::Push16;
                op.params.dst = Parameter::SReg16(SR::ES);
            }
            0x07 => {
                // pop es
                op.command = Op::Pop16;
                op.params.dst = Parameter::SReg16(SR::ES);
            }
            0x08 => {
                // or r/m8, r8
                op.command = Op::Or8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x09 => {
                // or r/m16, r16
                op.command = Op::Or16;
                op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
            }
            0x0A => {
                // or r8, r/m8
                op.command = Op::Or8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x0B => {
                // or r16, r/m16
                op.command = Op::Or16;
                op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
            }
            0x0C => {
                // or AL, imm8
                op.command = Op::Or8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x0D => {
                // or AX, imm16
                op.command = Op::Or16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x0E => {
                // push cs
                op.command = Op::Push16;
                op.params.dst = Parameter::SReg16(SR::CS);
            }
            0x0F => {
                let b = self.read_u8(mmu);
                match b {
                    0x82 => {
                        // jc rel16
                        op.command = Op::Jc;
                        op.params.dst = Parameter::Imm16(self.read_rel16(mmu));
                    }
                    0x84 => {
                        // jz rel16
                        op.command = Op::Jz;
                        op.params.dst = Parameter::Imm16(self.read_rel16(mmu));
                    }
                    0x85 => {
                        // jnz rel16
                        op.command = Op::Jnz;
                        op.params.dst = Parameter::Imm16(self.read_rel16(mmu));
                    }
                    0x87 => {
                        // ja rel16
                        op.command = Op::Ja;
                        op.params.dst = Parameter::Imm16(self.read_rel16(mmu));
                    }
                    0x89 => {
                        // jns rel16
                        op.command = Op::Jns;
                        op.params.dst = Parameter::Imm16(self.read_rel16(mmu));
                    }
                    0x92 => {
                        // setc r/m8
                        let x = self.read_mod_reg_rm(mmu);
                        op.command = Op::Setc;
                        op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                    }
                    0x95 => {
                        // setnz r/m8  (alias setne)
                        let x = self.read_mod_reg_rm(mmu);
                        op.command = Op::Setnz;
                        op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                    }
                    0xA0 => {
                        // push fs
                        op.command = Op::Push16;
                        op.params.dst = Parameter::SReg16(SR::FS);
                    }
                    0xA1 => {
                        // pop fs
                        op.command = Op::Pop16;
                        op.params.dst = Parameter::SReg16(SR::FS);
                    }
                    0xA3 => {
                        // bt r/m16, r16
                        op.command = Op::Bt;
                        op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
                    }
                    0xA4 =>{
                        // shld r/m16, r16, imm8
                        op.command = Op::Shld;
                        op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
                        op.params.src2 = Parameter::Imm8(self.read_u8(mmu));
                    }
                    0xA8 => {
                        // push gs
                        op.command = Op::Push16;
                        op.params.dst = Parameter::SReg16(SR::GS);
                    }
                    0xA9 => {
                        // pop gs
                        op.command = Op::Pop16;
                        op.params.dst = Parameter::SReg16(SR::GS);
                    }
                    0xAC => {
                        // shrd r/m16, r16, imm8
                        op.command = Op::Shrd;
                        op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
                        op.params.src2 = Parameter::Imm8(self.read_u8(mmu));
                    }
                    0xAF => {
                        // imul r16, r/m16
                        op.command = Op::Imul16;
                        op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
                    }
                    0xB6 => {
                        // movzx r16, r/m8
                        op.command = Op::Movzx16();
                        op.params = self.r16_rm8(&mut mmu, op.segment_prefix);
                    }
                    0xBC => {
                        // bsf r16, r/m16
                        op.command = Op::Bsf;
                        op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
                    }
                    0xBE => {
                        // movsx r16, r/m8
                        op.command = Op::Movsx16();
                        op.params = self.r16_rm8(&mut mmu, op.segment_prefix);
                    }
                    _ => op.command = Op::Invalid(InvalidOp::Op),
                }
            }
            0x10 => {
                // adc r/m8, r8
                op.command = Op::Adc8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x11 => {
                // adc r/m16, r16
                op.command = Op::Adc16;
                op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
            }
            0x12 => {
                // adc r8, r/m8
                op.command = Op::Adc8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x13 => {
                // adc r16, r/m16
                op.command = Op::Adc16;
                op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
            }
            0x14 => {
                // adc al, imm8
                op.command = Op::Adc8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x15 => {
                // adc ax, imm16
                op.command = Op::Adc16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x16 => {
                // push ss
                op.command = Op::Push16;
                op.params.dst = Parameter::SReg16(SR::SS);
            }
            0x17 => {
                // pop ss
                op.command = Op::Pop16;
                op.params.dst = Parameter::SReg16(SR::SS);
            }
            0x18 => {
                // sbb r/m8, r8
                op.command = Op::Sbb8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x1A => {
                // sbb r8, r/m8
                op.command = Op::Sbb8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x1C => {
                // sbb al, imm8
                op.command = Op::Sbb8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x1D => {
                // sbb ax, imm16
                op.command = Op::Sbb16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x1E => {
                // push ds
                op.command = Op::Push16;
                op.params.dst = Parameter::SReg16(SR::DS);
            }
            0x1F => {
                // pop ds
                op.command = Op::Pop16;
                op.params.dst = Parameter::SReg16(SR::DS);
            }
            0x20 => {
                // and r/m8, r8
                op.command = Op::And8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x21 => {
                // and r/m16, r16
                op.command = Op::And16;
                op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
            }
            0x22 => {
                // and r8, r/m8
                op.command = Op::And8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x23 => {
                // and r16, r/m16
                op.command = Op::And16;
                op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
            }
            0x24 => {
                // and AL, imm8
                op.command = Op::And8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x25 => {
                // and AX, imm16
                op.command = Op::And16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x26 => {
                // es segment prefix
                let (mut op, length) = self.decode(&mut mmu, Segment::ES);
                return (op, length + 1);
            }
            0x27 => op.command = Op::Daa,
            0x28 => {
                // sub r/m8, r8
                op.command = Op::Sub8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x29 => {
                // sub r/m16, r16
                op.command = Op::Sub16;
                op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
            }
            0x2A => {
                // sub r8, r/m8
                op.command = Op::Sub8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x2B => {
                // sub r16, r/m16
                op.command = Op::Sub16;
                op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
            }
            0x2C => {
                // sub AL, imm8
                op.command = Op::Sub8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x2D => {
                // sub AX, imm16
                op.command = Op::Sub16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x2E => {
                // cs segment prefix
                let (mut op, length) = self.decode(&mut mmu, Segment::CS);
                return (op, length + 1);
            }
            0x2F => op.command = Op::Das,
            0x30 => {
                // xor r/m8, r8
                op.command = Op::Xor8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x31 => {
                // xor r/m16, r16
                op.command = Op::Xor16;
                op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
            }
            0x32 => {
                // xor r8, r/m8
                op.command = Op::Xor8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x33 => {
                // xor r16, r/m16
                op.command = Op::Xor16;
                op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
            }
            0x34 => {
                // xor AL, imm8
                op.command = Op::Xor8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x35 => {
                // xor AX, imm16
                op.command = Op::Xor16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x36 => {
                // ss segment prefix
                let (mut op, length) = self.decode(&mut mmu, Segment::SS);
                return (op, length + 1);
            }
            0x37 => op.command = Op::Aaa,
            0x38 => {
                // cmp r/m8, r8
                op.command = Op::Cmp8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x39 => {
                // cmp r/m16, r16
                op.command = Op::Cmp16;
                op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
            }
            0x3A => {
                // cmp r8, r/m8
                op.command = Op::Cmp8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x3B => {
                // cmp r16, r/m16
                op.command = Op::Cmp16;
                op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
            }
            0x3C => {
                // cmp AL, imm8
                op.command = Op::Cmp8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x3D => {
                // cmp AX, imm16
                op.command = Op::Cmp16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x3E => {
                // ds segment prefix
                let (mut op, length) = self.decode(&mut mmu, Segment::DS);
                return (op, length + 1);
            }
            0x3F => op.command = Op::Aas,
            0x40...0x47 => {
                // inc r16
                op.command = Op::Inc16;
                op.params.dst = Parameter::Reg16(Into::into(b & 7));
            }
            0x48...0x4F => {
                // dec r16
                op.command = Op::Dec16;
                op.params.dst = Parameter::Reg16(Into::into(b & 7));
            }
            0x50...0x57 => {
                // push r16
                op.command = Op::Push16;
                op.params.dst = Parameter::Reg16(Into::into(b & 7));
            }
            0x58...0x5F => {
                // pop r16
                op.command = Op::Pop16;
                op.params.dst = Parameter::Reg16(Into::into(b & 7));
            }
            0x60 => op.command = Op::Pusha,
            0x61 => op.command = Op::Popa,
            0x62 => {
                // bound r16, m16&16
                op.command = Op::Bound();
                // XXX not all modes of 2nd argument is valid
                op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
            }
            0x63 => {
                // arpl r/m16, r16
                op.command = Op::Arpl();
                op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
            }
            0x64 => {
                // fs segment prefix
                let (mut op, length) = self.decode(&mut mmu, Segment::FS);
                return (op, length + 1);
            }
            0x65 => {
                // gs segment prefix
                let (mut op, length) = self.decode(&mut mmu, Segment::GS);
                return (op, length + 1);
            }
            0x66 => {
                // 80386+ Operand-size override prefix
                println!("ERROR: unsupported 386 operand-size override prefix");
                op.command = Op::Invalid(InvalidOp::Op);
            }
            0x67 => {
                // 80386+ Address-size override prefix
                println!("ERROR: unsupported 386 address-size override prefix");
                op.command = Op::Invalid(InvalidOp::Op);
            }
            0x68 => {
                // push imm16
                op.command = Op::Push16;
                op.params.dst = Parameter::Imm16(self.read_u16(mmu));
            }
            0x69 => {
                // imul r16, r/m16, imm16
                op.command = Op::Imul16;
                op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
                op.params.src2 = Parameter::Imm16(self.read_u16(mmu));
            }
            0x6A => {
                // push imm8
                op.command = Op::Push16;
                op.params.dst = Parameter::ImmS8(self.read_s8(mmu));
            }
            0x6B => {
                // imul r16, r/m16, imm8
                op.command = Op::Imul16;
                op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
                op.params.src2 = Parameter::Imm8(self.read_u8(mmu));
            }
            0x6C => op.command = Op::Insb,
            0x6D => op.command = Op::Insw,
            0x6E => op.command = Op::Outsb,
            0x6F => op.command = Op::Outsw,
            0x70 => {
                // jo rel8
                op.command = Op::Jo;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x71 => {
                // jno rel8
                op.command = Op::Jno;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x72 => {
                // jc rel8
                op.command = Op::Jc;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x73 => {
                // jnc rel8
                op.command = Op::Jnc;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x74 => {
                // jz rel8
                op.command = Op::Jz;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x75 => {
                // jnz rel8
                op.command = Op::Jnz;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x76 => {
                // jna rel8
                op.command = Op::Jna;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x77 => {
                // ja rel8
                op.command = Op::Ja;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x78 => {
                // js rel8
                op.command = Op::Js;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x79 => {
                // jns rel8
                op.command = Op::Jns;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
	        0x7A => {
                // jpe rel8
		        op.command = Op::Jpe; // alias: jp
		        op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x7B => {
                // jpo rel8
                op.command = Op::Jpo; // alias: jnp
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x7C => {
                // jl rel8
                op.command = Op::Jl;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x7D => {
                // jnl rel8
                op.command = Op::Jnl;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x7E => {
                // jng rel8
                op.command = Op::Jng;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x7F => {
                // jg rel8
                op.command = Op::Jg;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0x80 => {
                // <arithmetic> r/m8, imm8
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
                match x.reg {
                    0 => op.command = Op::Add8,
                    1 => op.command = Op::Or8,
                    2 => op.command = Op::Adc8,
                    3 => op.command = Op::Sbb8,
                    4 => op.command = Op::And8,
                    5 => op.command = Op::Sub8,
                    6 => op.command = Op::Xor8,
                    7 => op.command = Op::Cmp8,
                    _ => {}
                }
            }
            0x81 => {
                // <arithmetic> r/m16, imm16
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm16(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
                match x.reg {
                    0 => op.command = Op::Add16,
                    1 => op.command = Op::Or16,
                    2 => op.command = Op::Adc16,
                    3 => op.command = Op::Sbb16,
                    4 => op.command = Op::And16,
                    5 => op.command = Op::Sub16,
                    6 => op.command = Op::Xor16,
                    7 => op.command = Op::Cmp16,
                    _ => {}
                }
            }
            // 0x82 is unrecognized by objdump & ndisasm, but alias to 0x80 on pre Pentium 4:s according to ref.x86asm.net
            0x83 => {
                // <arithmetic> r/m16, imm8
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm16(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::ImmS8(self.read_s8(mmu));
                match x.reg {
                    0 => op.command = Op::Add16,
                    1 => op.command = Op::Or16,
                    2 => op.command = Op::Adc16,
                    3 => op.command = Op::Sbb16,
                    4 => op.command = Op::And16,
                    5 => op.command = Op::Sub16,
                    6 => op.command = Op::Xor16,
                    7 => op.command = Op::Cmp16,
                    _ => {}
                }
            }
            0x84 => {
                // test r/m8, r8
                op.command = Op::Test8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x85 => {
                // test r/m16, r16
                op.command = Op::Test16;
                op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
            }
            0x86 => {
                // xchg r/m8, r8
                op.command = Op::Xchg8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x87 => {
                // xchg r/m16, r16
                op.command = Op::Xchg16;
                op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
            }
            0x88 => {
                // mov r/m8, r8
                op.command = Op::Mov8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x89 => {
                // mov r/m16, r16
                op.command = Op::Mov16;
                op.params = self.rm16_r16(&mut mmu, op.segment_prefix);
            }
            0x8A => {
                // mov r8, r/m8
                op.command = Op::Mov8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x8B => {
                // mov r16, r/m16
                op.command = Op::Mov16;
                op.params = self.r16_rm16(&mut mmu, op.segment_prefix);
            }
            0x8C => {
                // mov r/m16, sreg
                op.command = Op::Mov16;
                op.params = self.rm16_sreg(&mut mmu, op.segment_prefix);
            }
            0x8D => {
                // lea r16, m
                op.command = Op::Lea16;
                op.params = self.r16_m16(&mut mmu, op.segment_prefix);
            }
            0x8E => {
                // mov sreg, r/m16
                op.command = Op::Mov16;
                op.params = self.sreg_rm16(&mut mmu, op.segment_prefix);
            }
            0x8F => {
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm16(&mut mmu, op.segment_prefix, x.rm, x.md);
                match x.reg {
                    0 => op.command = Op::Pop16, // pop r/m16
                    _ => op.command = Op::Invalid(InvalidOp::Reg(x.reg)),
                }
            }
            0x90 => op.command = Op::Nop,
            0x91...0x97 => {
                // xchg AX, r16 | xchg r16, AX
                // NOTE: "xchg ax,ax" is an alias of "nop"
                op.command = Op::Xchg16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Reg16(Into::into(b & 7));
            }
            0x98 => op.command = Op::Cbw,
            0x99 => op.command = Op::Cwd,
            // 0x9A = "call word imm16:imm16"
            // 0x9B = "wait"
            0x9C => op.command = Op::Pushf,
            0x9D => op.command = Op::Popf,
            0x9E => op.command = Op::Sahf,
            0x9F => op.command = Op::Lahf,
            0xA0 => {
                // mov AL, [moffs8]
                op.command = Op::Mov8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Ptr8(op.segment_prefix, self.read_u16(mmu));
            }
            0xA1 => {
                // mov AX, [moffs16]
                op.command = Op::Mov16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Ptr16(op.segment_prefix, self.read_u16(mmu));
            }
            0xA2 => {
                // mov [moffs8], AL
                op.command = Op::Mov8;
                op.params.dst = Parameter::Ptr8(op.segment_prefix, self.read_u16(mmu));
                op.params.src = Parameter::Reg8(R8::AL);
            }
            0xA3 => {
                // mov [moffs16], AX
                op.command = Op::Mov16;
                op.params.dst = Parameter::Ptr16(op.segment_prefix, self.read_u16(mmu));
                op.params.src = Parameter::Reg16(R16::AX);
            }
            0xA4 => op.command = Op::Movsb,
            0xA5 => op.command = Op::Movsw,
            0xA6 => op.command = Op::Cmpsb,
            0xA7 => op.command = Op::Cmpsw,
            0xA8 => {
                // test AL, imm8
                op.command = Op::Test8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xA9 => {
                // test AX, imm16
                op.command = Op::Test16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0xAA => op.command = Op::Stosb,
            0xAB => op.command = Op::Stosw,
            0xAC => op.command = Op::Lodsb,
            0xAD => op.command = Op::Lodsw,
            0xAE => op.command = Op::Scasb,
            0xAF => op.command = Op::Scasw,
            0xB0...0xB7 => {
                // mov r8, u8
                op.command = Op::Mov8;
                op.params.dst = Parameter::Reg8(Into::into(b & 7));
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xB8...0xBF => {
                // mov r16, u16
                op.command = Op::Mov16;
                op.params.dst = Parameter::Reg16(Into::into(b & 7));
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0xC0 => {
                // r8, byte imm8
                let x = self.read_mod_reg_rm(mmu);
                op.command = match x.reg {
                    0 => Op::Rol8,
                    1 => Op::Ror8,
                    2 => Op::Rcl8,
                    3 => Op::Rcr8,
                    4 => Op::Shl8,
                    5 => Op::Shr8,
                    7 => Op::Sar8,
                    _ => Op::Invalid(InvalidOp::Op),
                };
                op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xC1 => {
                // r16, byte imm8
                let x = self.read_mod_reg_rm(mmu);
                op.command = match x.reg {
                    0 => Op::Rol16,
                    1 => Op::Ror16,
                    2 => Op::Rcl16,
                    3 => Op::Rcr16,
                    4 => Op::Shl16,
                    5 => Op::Shr16,
                    7 => Op::Sar16,
                    _ => Op::Invalid(InvalidOp::Op),
                };
                op.params.dst = self.rm16(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xC2 => {
                // ret [near] imm16
                op.command = Op::Retn;
                op.params.dst = Parameter::Imm16(self.read_u16(mmu));
            }
            0xC3 => op.command = Op::Retn, // ret [near]
            0xC4 => {
                // les r16, m16
                op.command = Op::Les;
                op.params = self.r16_m16(&mut mmu, op.segment_prefix);
            }
            0xC5 => {
                // lds r16, m16
                op.command = Op::Lds;
                op.params = self.r16_m16(&mut mmu, op.segment_prefix);
            }
            0xC6 => {
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
                match x.reg {
                    0 => op.command = Op::Mov8, // mov r/m8, imm8
                    _ => op.command = Op::Invalid(InvalidOp::Reg(x.reg)),
                }
            }
            0xC7 => {
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm16(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
                match x.reg {
                    0 => op.command = Op::Mov16, // mov r/m16, imm16
                    _ => op.command = Op::Invalid(InvalidOp::Reg(x.reg)),
                }
            }
            0xC8 => {
                // enter imm16, imm8
                op.command = Op::Enter;
                op.params.dst = Parameter::Imm16(self.read_u16(mmu));
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xC9 => op.command = Op::Leave,
            0xCA => {
                // ret [far] imm16
                op.command = Op::Retf;
                op.params.dst = Parameter::Imm16(self.read_u16(mmu));
            }
            0xCB => op.command = Op::Retf,
            0xCC => {
                op.command = Op::Int;
                op.params.dst = Parameter::Imm8(3);
            }
            0xCD => {
                // int imm8
                op.command = Op::Int;
                op.params.dst = Parameter::Imm8(self.read_u8(mmu));
            }
            0xCE => op.command = Op::Into(),
	        0xCF => op.command = Op::Iret,
            0xD0 => {
                // bit shift byte by 1
                let x = self.read_mod_reg_rm(mmu);
                op.command = match x.reg {
                    0 => Op::Rol8,
                    1 => Op::Ror8,
                    2 => Op::Rcl8,
                    3 => Op::Rcr8,
                    4 => Op::Shl8,
                    5 => Op::Shr8,
                    7 => Op::Sar8,
                    _ => Op::Invalid(InvalidOp::Op),
                };
                op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Imm8(1);
            }
            0xD1 => {
                // bit shift word by 1
                let x = self.read_mod_reg_rm(mmu);
                op.command = match x.reg {
                    0 => Op::Rol16,
                    1 => Op::Ror16,
                    2 => Op::Rcl16,
                    3 => Op::Rcr16,
                    4 => Op::Shl16,
                    5 => Op::Shr16,
                    7 => Op::Sar16,
                    _ => Op::Invalid(InvalidOp::Op),
                };
                op.params.dst = self.rm16(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Imm16(1);
            }
            0xD2 => {
                // bit shift byte by CL
                let x = self.read_mod_reg_rm(mmu);
                op.command = match x.reg {
                    0 => Op::Rol8,
                    1 => Op::Ror8,
                    2 => Op::Rcl8,
                    3 => Op::Rcr8,
                    4 => Op::Shl8,
                    5 => Op::Shr8,
                    7 => Op::Sar8,
                    _ => Op::Invalid(InvalidOp::Op),
                };
                op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Reg8(R8::CL);
            }
            0xD3 => {
                // bit shift word by CL
                let x = self.read_mod_reg_rm(mmu);
                op.command = match x.reg {
                    0 => Op::Rol16,
                    1 => Op::Ror16,
                    2 => Op::Rcl16,
                    3 => Op::Rcr16,
                    4 => Op::Shl16,
                    5 => Op::Shr16,
                    7 => Op::Sar16,
                    _ => Op::Invalid(InvalidOp::Op),
                };
                op.params.dst = self.rm16(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Reg8(R8::CL);
            }
            0xD4 => {
                // aam imm8
                op.command = Op::Aam;
                op.params.dst = Parameter::Imm8(self.read_u8(mmu));
            }
            0xD5 => {
                op.command = Op::Aad;
                op.params.dst = Parameter::Imm8(self.read_u8(mmu));
            }
            0xD6 => op.command = Op::Salc,
            0xD7 => op.command = Op::Xlatb,
            0xD8...0xDF => {
                // fpu
                println!("ERROR: unsupported FPU opcode {:02X}", b);
                op.command = Op::Invalid(InvalidOp::Op);
            }
            0xE0 => {
                op.command = Op::Loopne;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0xE1 => {
                op.command = Op::Loope;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0xE2 => {
                op.command = Op::Loop;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0xE3 => {
                // jcxz rel8
                op.command = Op::Jcxz;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0xE4 => {
                // in AL, imm8
                op.command = Op::In8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xE5 => {
                // in AX, imm8
                op.command = Op::In16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xE6 => {
                // OUT imm8, AL
                op.command = Op::Out8;
                op.params.dst = Parameter::Imm8(self.read_u8(mmu));
                op.params.src = Parameter::Reg8(R8::AL);
            }
            0xE7 => {
                // OUT imm8, AX
                op.command = Op::Out16;
                op.params.dst = Parameter::Imm8(self.read_u8(mmu));
                op.params.src = Parameter::Reg16(R16::AX);
            }
            0xE8 => {
                // call near s16
                op.command = Op::CallNear;
                op.params.dst = Parameter::Imm16(self.read_rel16(mmu));
            }
            0xE9 => {
                // jmp near rel16
                op.command = Op::JmpNear;
                op.params.dst = Parameter::Imm16(self.read_rel16(mmu));
            }
            0xEA => {
                // jmp far ptr16:16
                op.command = Op::JmpFar;
                let imm = self.read_u16(mmu);
                let seg = self.read_u16(mmu);
                op.params.dst = Parameter::Ptr16Imm(seg, imm);
            }
            0xEB => {
                // jmp short rel8
                op.command = Op::JmpShort;
                op.params.dst = Parameter::Imm16(self.read_rel8(mmu));
            }
            0xEC => {
                // in AL, DX
                op.command = Op::In8;
                op.params.dst = Parameter::Reg8(R8::AL);
                op.params.src = Parameter::Reg16(R16::DX);
            }
            0xED => {
                // in AX, DX
                op.command = Op::In16;
                op.params.dst = Parameter::Reg16(R16::AX);
                op.params.src = Parameter::Reg16(R16::DX);
            }
            0xEE => {
                // out DX, AL
                op.command = Op::Out8;
                op.params.dst = Parameter::Reg16(R16::DX);
                op.params.src = Parameter::Reg8(R8::AL);
            }
            0xEF => {
                // out DX, AX
                op.command = Op::Out16;
                op.params.dst = Parameter::Reg16(R16::DX);
                op.params.src = Parameter::Reg16(R16::AX);
            }
            0xF0 => {
                // lock prefix
                let (mut op, length) = self.decode(&mut mmu, seg);
                op.lock = true;
                return (op, length + 1)
            }
            0xF1 => {
                op.command = Op::Int;
                op.params.dst = Parameter::Imm8(1);
            }
            0xF2 => {
                let b = self.read_u8(mmu);
                match b {
                    0xAE => {
                        op.repeat = RepeatMode::Repne;
                        op.command = Op::Scasb;
                    }
                    _ => op.command = Op::Invalid(InvalidOp::Op),
                }
            }
            0xF3 => {
                let b = self.read_u8(mmu);
                match b {
                    0x6E => {
                        op.repeat = RepeatMode::Rep;
                        op.command = Op::Outsb;
                    }
                    0xA4 => {
                        op.repeat = RepeatMode::Rep;
                        op.command = Op::Movsb;
                    }
                    0xA5 => {
                        op.repeat = RepeatMode::Rep;
                        op.command = Op::Movsw;
                    }
                    0xAA => {
                        op.repeat = RepeatMode::Rep;
                        op.command = Op::Stosb;
                    }
                    0xAB => {
                        op.repeat = RepeatMode::Rep;
                        op.command = Op::Stosw;
                    }
                    _ => op.command = Op::Invalid(InvalidOp::Op),
                }
            }
            0xF4 => op.command = Op::Hlt(),
            0xF5 => op.command = Op::Cmc,
            0xF6 => {
                // <math> r/m8
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                match x.reg {
                    0 | 1 => {
                        // test r/m8, imm8
                        op.command = Op::Test8;
                        op.params.src = Parameter::Imm8(self.read_u8(mmu));
                    }
                    2 => op.command = Op::Not8,
                    3 => op.command = Op::Neg8,
                    4 => op.command = Op::Mul8,
                    5 => op.command = Op::Imul8,
                    6 => op.command = Op::Div8,
                    7 => op.command = Op::Idiv8,
                    _ => unreachable!(),
                }
            }
            0xF7 => {
                // <math> r/m16
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm16(&mut mmu, op.segment_prefix, x.rm, x.md);
                match x.reg {
                    0 | 1 => {
                        // test r/m16, imm16
                        op.command = Op::Test16;
                        op.params.src = Parameter::Imm16(self.read_u16(mmu));
                    }
                    2 => op.command = Op::Not16,
                    3 => op.command = Op::Neg16,
                    4 => op.command = Op::Mul16,
                    5 => op.command = Op::Imul16,
                    6 => op.command = Op::Div16,
                    7 => op.command = Op::Idiv16,
                    _ => unreachable!(),
                }
            }
            0xF8 => op.command = Op::Clc,
            0xF9 => op.command = Op::Stc,
            0xFA => op.command = Op::Cli,
            0xFB => op.command = Op::Sti,
            0xFC => op.command = Op::Cld,
            0xFD => op.command = Op::Std,
            0xFE => {
                // r/m8
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                match x.reg {
                    // NOTE: 2 is a deprecated but valid encoding, example:
                    // https://www.pouet.net/prod.php?which=65203
                    // 00000140  FEC5              inc ch
                    0 | 2 => op.command = Op::Inc8,
                    1 => op.command = Op::Dec8,
                    _ => op.command = Op::Invalid(InvalidOp::Reg(x.reg)),
                }
            }
            0xFF => {
                // r/m16
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm16(&mut mmu, op.segment_prefix, x.rm, x.md);
                match x.reg {
                    0 => op.command = Op::Inc16,
                    1 => op.command = Op::Dec16,
                    2 => op.command = Op::CallNear,
                    // 3 => call far
                    4 => op.command = Op::JmpNear,
                    // 5 => jmp far
                    6 => op.command = Op::Push16,
                    _ => op.command = Op::Invalid(InvalidOp::Reg(x.reg)),
                }
            }
            _ => op.command = Op::Invalid(InvalidOp::Op),
        }

        // calculate instruction length
        let length = (self.c_offset - ioffset) as usize;
        (op, length)
    }

    // decode rm8
    fn rm8(&mut self, mmu: &mut MMU, seg: Segment, rm: u8, md: u8) -> Parameter {
        match md {
            0 => {
                if rm == 6 {
                    // [u16]
                    Parameter::Ptr8(seg, self.read_u16(mmu))
                } else {
                    // [amode]
                    Parameter::Ptr8Amode(seg, Into::into(rm))
                }
            }
            // [amode+s8]
            1 => Parameter::Ptr8AmodeS8(seg, Into::into(rm), self.read_s8(mmu)),
            // [amode+s16]
            2 => Parameter::Ptr8AmodeS16(seg, Into::into(rm), self.read_s16(mmu)),
            // reg
            3 => Parameter::Reg8(Into::into(rm)),
            _ => unreachable!(),
        }
    }

    // decode rm16
    fn rm16(&mut self, mmu: &mut MMU, seg: Segment, rm: u8, md: u8) -> Parameter {
        match md {
            0 => {
                if rm == 6 {
                    // [u16]
                    Parameter::Ptr16(seg, self.read_u16(mmu))
                } else {
                    // [amode]
                    Parameter::Ptr16Amode(seg, Into::into(rm))
                }
            }
            // [amode+s8]
            1 => Parameter::Ptr16AmodeS8(seg, Into::into(rm), self.read_s8(mmu)),
            // [amode+s16]
            2 => Parameter::Ptr16AmodeS16(seg, Into::into(rm), self.read_s16(mmu)),
            // [reg]
            _ => Parameter::Reg16(Into::into(rm)),
        }
    }

    // decode r8, r/m8
    fn r8_rm8(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: Parameter::Reg8(Into::into(x.reg)),
            src: self.rm8(&mut mmu, seg, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r/m8, r8
    fn rm8_r8(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: self.rm8(&mut mmu, seg, x.rm, x.md),
            src: Parameter::Reg8(Into::into(x.reg)),
            src2: Parameter::None,
        }
    }

    // decode Sreg, r/m16
    fn sreg_rm16(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: Parameter::SReg16(Into::into(x.reg)),
            src: self.rm16(&mut mmu, seg, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r/m16, Sreg
    fn rm16_sreg(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: self.rm16(&mut mmu, seg, x.rm, x.md),
            src: Parameter::SReg16(Into::into(x.reg)),
            src2: Parameter::None,
        }
    }

    // decode r16, r/m8 (movzx)
    fn r16_rm8(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: Parameter::Reg16(Into::into(x.reg)),
            src: self.rm8(&mut mmu, seg, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r16, r/m16
    fn r16_rm16(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: Parameter::Reg16(Into::into(x.reg)),
            src: self.rm16(&mut mmu, seg, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r/m16, r16
    fn rm16_r16(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: self.rm16(&mut mmu, seg, x.rm, x.md),
            src: Parameter::Reg16(Into::into(x.reg)),
            src2: Parameter::None,
        }
    }

    // decode r16, m16
    fn r16_m16(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        if x.md == 3 {
            println!("r16_m16 error: invalid encoding, ip={:04X}", self.c_offset);
        }
        ParameterSet {
            dst: Parameter::Reg16(Into::into(x.reg)),
            src: self.rm16(&mut mmu, seg, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    fn read_mod_reg_rm(&mut self, mmu: &MMU) -> ModRegRm {
        let b = mmu.read_u8(self.c_seg, self.c_offset);
        self.c_offset += 1;
        ModRegRm {
            md: b >> 6, // high 2 bits
            reg: (b >> 3) & 7, // mid 3 bits
            rm: b & 7, // low 3 bits
        }
    }

    fn read_rel8(&mut self, mmu: &MMU) -> u16 {
        let val = self.read_s8(mmu);
        (self.c_offset as i16 + i16::from(val)) as u16
    }

    fn read_rel16(&mut self, mmu: &MMU) -> u16 {
        let val = self.read_s16(mmu);
        (self.c_offset as i16 + val) as u16
    }

    fn read_u8(&mut self, mmu: &MMU) -> u8 {
        let b = mmu.read_u8(self.c_seg, self.c_offset);
        self.c_offset += 1;
        b
    }

    fn read_s8(&mut self, mmu: &MMU) -> i8 {
        self.read_u8(mmu) as i8
    }

    fn read_u16(&mut self, mmu: &MMU) -> u16 {
        let lo = self.read_u8(mmu);
        let hi = self.read_u8(mmu);
        u16::from(hi) << 8 | u16::from(lo)
    }

    fn read_s16(&mut self, mmu: &MMU) -> i16 {
        self.read_u16(mmu) as i16
    }
}

pub fn instruction_info_to_str(ops: &[InstructionInfo]) -> String {
    let mut lines = Vec::new();
    for op in ops {
        lines.push(op.to_string())
    }
    lines.join("\n")
}

pub fn instructions_to_str(ops: &[Instruction]) -> String {
    let mut lines = Vec::new();
    for op in ops {
        lines.push(op.to_string())
    }
    lines.join("\n")
}
