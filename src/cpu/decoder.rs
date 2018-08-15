use std::cell::RefCell;
use std::num::Wrapping;
use std::rc::Rc;

use cpu::instruction::{Instruction, InstructionInfo, ModRegRm, RepeatMode};
use cpu::parameter::{Parameter, ParameterSet};
use cpu::op::{Op, Invalid};
use cpu::register::{R, r8, r16, r32, sr};
use cpu::segment::Segment;
use memory::{MMU, MemoryAddress};

const DEBUG_DECODER: bool = false;

#[cfg(test)]
#[path = "./decoder_test.rs"]
mod decoder_test;

#[derive(Clone, Debug, PartialEq)]
pub enum OperandSize {
    _16bit, _32bit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AddressSize {
    _16bit, _32bit,
}

#[derive(Clone, Default)]
pub struct Decoder {
    // starting instruction decoding offset
    current_seg: u16,
    current_offset: u16,
}

impl Decoder {
    pub fn decode_to_block(&mut self, mut mmu: &mut MMU, seg: u16, offset: u16, n: usize) -> Vec<InstructionInfo> {
        let mut ops: Vec<InstructionInfo> = Vec::new();
        let mut inst_offset = 0;
        for _ in 0..n {
            let op = self.get_instruction_info(&mut mmu, seg, offset+inst_offset);
            inst_offset += op.bytes.len() as u16;
            ops.push(op);
        }
        ops
    }

    pub fn disassemble_block_to_str(&mut self, mut mmu: &mut MMU, seg: u16, offset: u16, n: usize) -> String {
        let ops = self.decode_to_block(&mut mmu, seg, offset, n);
        instruction_info_to_str(&ops)
    }

    // decodes op at iseg:ioffset into a InstructionInfo
    pub fn get_instruction_info(&mut self, mut mmu: &mut MMU, iseg: u16, ioffset: u16) -> InstructionInfo {
        let instr = self.get_instruction(&mut mmu, iseg, ioffset);
        if DEBUG_DECODER {
            println!("get_instruction_info at {:06x}: {:?}", MemoryAddress::RealSegmentOffset(iseg, ioffset).value(), instr);
        }
        InstructionInfo {
            segment: iseg as usize,
            offset: ioffset as usize,
            bytes: mmu.read(iseg, ioffset, instr.length as usize),
            instruction: instr,
        }
    }

    // decodes op at iseg:ioffset into a Instruction
    pub fn get_instruction(&mut self, mut mmu: &mut MMU, segment: u16, offset: u16) -> Instruction {
        self.current_seg = segment;
        self.current_offset = offset;
        let mut op = Instruction::new(Op::Uninitialized);
        self.decode(&mut mmu, &mut op);
        op
    }

    // decodes the next instruction
    fn decode(&mut self, mut mmu: &mut MMU, mut op: &mut Instruction) {
        let start_offset = self.current_offset;
        let b = self.read_u8(mmu);
        if DEBUG_DECODER {
            println!("decode op start {:?}", op);
        }

        match b {
            0x00 => {
                // add r/m8, r8
                op.command = Op::Add8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x01 => {
                // add r/m16, r16
                // add r/m32, r32
                self.prefixed_16_32_rm_r(&mut mmu, &mut op, Op::Add16, Op::Add32)
            }
            0x02 => {
                // add r8, r/m8
                op.command = Op::Add8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x03 => {
                // add r16, r/m16
                // add r32, r/m32
                self.prefixed_16_32_r_rm(&mut mmu, &mut op, Op::Add16, Op::Add32)
            }
            0x04 => {
                // add AL, imm8
                op.command = Op::Add8;
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x05 => {
                match op.op_size {
                    OperandSize::_16bit => {
                        // add AX, imm16
                        op.command = Op::Add16;
                        op.params.dst = Parameter::Reg16(R::AX);
                        op.params.src = Parameter::Imm16(self.read_u16(mmu));
                    }
                    OperandSize::_32bit => {
                        // add EAX, imm32
                        op.command = Op::Add32;
                        op.params.dst = Parameter::Reg32(R::EAX);
                        op.params.src = Parameter::Imm32(self.read_u32(mmu));
                    }
                }
            }
            0x06 => {
                // push es
                op.command = Op::Push16;
                op.params.dst = Parameter::SReg16(R::ES);
            }
            0x07 => {
                // pop es
                op.command = Op::Pop16;
                op.params.dst = Parameter::SReg16(R::ES);
            }
            0x08 => {
                // or r/m8, r8
                op.command = Op::Or8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x09 => {
                // or r/m16, r16
                op.command = Op::Or16;
                op.params = self.rm16_r16(&mut mmu, op);
            }
            0x0A => {
                // or r8, r/m8
                op.command = Op::Or8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x0B => {
                // or r16, r/m16
                op.command = Op::Or16;
                op.params = self.r16_rm16(&mut mmu, op);
            }
            0x0C => {
                // or AL, imm8
                op.command = Op::Or8;
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x0D => {
                // or AX, imm16
                op.command = Op::Or16;
                op.params.dst = Parameter::Reg16(R::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x0E => {
                // push cs
                op.command = Op::Push16;
                op.params.dst = Parameter::SReg16(R::CS);
            }
            0x0F => {
                let b2 = self.read_u8(mmu);
                match b2 {
                    0x00 => {
                        let x = self.read_mod_reg_rm(mmu);
                        op.params.dst = self.rm16(&mut mmu, op, x.rm, x.md);
                        op.command = match x.reg {
                            0 => Op::Sldt, // sldt r/m16
                            _ => Op::Invalid(vec!(b, b2), Invalid::Reg(x.reg)),
                        };
                    }
                    0x82 => {
                        // jc rel16
                        op.command = Op::Jc;
                        op.params.dst = Parameter::Imm16(self.read_rel16(mmu));
                    }
                    0x83 => {
                        // jnc rel16
                        op.command = Op::Jnc;
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
                    0x86 => {
                        // jna rel16
                        op.command = Op::Jna;
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
                    0x8C => {
                        // jl rel16
                        op.command = Op::Jl;
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
                        op.params.dst = Parameter::SReg16(R::FS);
                    }
                    0xA1 => {
                        // pop fs
                        op.command = Op::Pop16;
                        op.params.dst = Parameter::SReg16(R::FS);
                    }
                    0xA3 => {
                        // bt r/m16, r16
                        op.command = Op::Bt;
                        op.params = self.rm16_r16(&mut mmu, op);
                    }
                    0xA4 =>{
                        // shld r/m16, r16, imm8
                        op.command = Op::Shld;
                        op.params = self.rm16_r16(&mut mmu, op);
                        op.params.src2 = Parameter::Imm8(self.read_u8(mmu));
                    }
                    0xA8 => {
                        // push gs
                        op.command = Op::Push16;
                        op.params.dst = Parameter::SReg16(R::GS);
                    }
                    0xA9 => {
                        // pop gs
                        op.command = Op::Pop16;
                        op.params.dst = Parameter::SReg16(R::GS);
                    }
                    0xAC => {
                        // shrd r/m16, r16, imm8
                        op.command = Op::Shrd;
                        op.params = self.rm16_r16(&mut mmu, op);
                        op.params.src2 = Parameter::Imm8(self.read_u8(mmu));
                    }
                    0xAF => {
                        // imul r16, r/m16
                        // imul r32, r/m32
                        self.prefixed_16_32_r_rm(&mut mmu, &mut op, Op::Imul16, Op::Imul32)
                    }
                    0xB6 => {
                        match op.op_size {
                            OperandSize::_16bit => {
                                // movzx r16, r/m8
                                op.command = Op::Movzx16;
                                op.params = self.r16_rm8(&mut mmu, op.segment_prefix);
                            }
                            OperandSize::_32bit => {
                                // movzx r32, r/m8
                                op.command = Op::Movzx32;
                                op.params = self.r32_rm8(&mut mmu, op.segment_prefix);
                            }
                        }
                    }
                    0xBA => {
                        // bts r/m16, imm8
                        let x = self.read_mod_reg_rm(mmu);
                        op.command = Op::Bts;
                        op.params.dst = self.rm16(&mut mmu, op, x.rm, x.md);
                        op.params.src = Parameter::Imm8(self.read_u8(mmu));
                    }
                    0xBC => {
                        // bsf r16, r/m16
                        op.command = Op::Bsf;
                        op.params = self.r16_rm16(&mut mmu, op);
                    }
                    0xBE => {
                        match op.op_size {
                            OperandSize::_16bit => {
                                // movsx r16, r/m8
                                op.command = Op::Movsx16;
                                op.params = self.r16_rm8(&mut mmu, op.segment_prefix);
                            }
                            OperandSize::_32bit => {
                                // movsx r32, r/m8
                                op.command = Op::Movsx32;
                                op.params = self.r32_rm8(&mut mmu, op.segment_prefix);
                            }
                        }
                    }
                    0xBF => {
                        match op.op_size {
                            OperandSize::_16bit => op.command = Op::Invalid(vec!(b, b2), Invalid::Op),
                            OperandSize::_32bit => {
                                // movsx r32, r/m16
                                op.command = Op::Movsx32;
                                op.params = self.r32_rm16(&mut mmu, op);
                            }
                        }
                    }
                    _ => op.command = Op::Invalid(vec!(b, b2), Invalid::Op),
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
                op.params = self.rm16_r16(&mut mmu, op);
            }
            0x12 => {
                // adc r8, r/m8
                op.command = Op::Adc8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x13 => {
                // adc r16, r/m16
                op.command = Op::Adc16;
                op.params = self.r16_rm16(&mut mmu, op);
            }
            0x14 => {
                // adc al, imm8
                op.command = Op::Adc8;
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x15 => {
                // adc ax, imm16
                op.command = Op::Adc16;
                op.params.dst = Parameter::Reg16(R::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x16 => {
                // push ss
                op.command = Op::Push16;
                op.params.dst = Parameter::SReg16(R::SS);
            }
            0x17 => {
                // pop ss
                op.command = Op::Pop16;
                op.params.dst = Parameter::SReg16(R::SS);
            }
            0x18 => {
                // sbb r/m8, r8
                op.command = Op::Sbb8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x19 => {
                // sbb r/m16, r16
                op.command = Op::Sbb16;
                op.params = self.rm16_r16(&mut mmu, op);
            }
            0x1A => {
                // sbb r8, r/m8
                op.command = Op::Sbb8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x1B => {
                // sbb r16, r/m16
                op.command = Op::Sbb16;
                op.params = self.r16_rm16(&mut mmu, op);
            }
            0x1C => {
                // sbb al, imm8
                op.command = Op::Sbb8;
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x1D => {
                // sbb ax, imm16
                op.command = Op::Sbb16;
                op.params.dst = Parameter::Reg16(R::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x1E => {
                // push ds
                op.command = Op::Push16;
                op.params.dst = Parameter::SReg16(R::DS);
            }
            0x1F => {
                // pop ds
                op.command = Op::Pop16;
                op.params.dst = Parameter::SReg16(R::DS);
            }
            0x20 => {
                // and r/m8, r8
                op.command = Op::And8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x21 => {
                // and r/m16, r16
                op.command = Op::And16;
                op.params = self.rm16_r16(&mut mmu, op);
            }
            0x22 => {
                // and r8, r/m8
                op.command = Op::And8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x23 => {
                // and r16, r/m16
                op.command = Op::And16;
                op.params = self.r16_rm16(&mut mmu, op);
            }
            0x24 => {
                // and AL, imm8
                op.command = Op::And8;
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x25 => {
                // and AX, imm16
                op.command = Op::And16;
                op.params.dst = Parameter::Reg16(R::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x26 => {
                // es segment prefix
                op.segment_prefix = Segment::ES;
                self.decode(&mut mmu, &mut op);
                op.length += 1;
                return;
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
                op.params = self.rm16_r16(&mut mmu, op);
            }
            0x2A => {
                // sub r8, r/m8
                op.command = Op::Sub8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x2B => {
                // sub r16, r/m16
                // sub r32, r/m32
                self.prefixed_16_32_r_rm(&mut mmu, &mut op, Op::Sub16, Op::Sub32)
            }
            0x2C => {
                // sub AL, imm8
                op.command = Op::Sub8;
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x2D => {
                match op.op_size {
                    OperandSize::_16bit => {
                        // sub AX, imm16
                        op.command = Op::Sub16;
                        op.params.dst = Parameter::Reg16(R::AX);
                        op.params.src = Parameter::Imm16(self.read_u16(mmu));
                    }
                    OperandSize::_32bit => {
                        // sub EAX, imm32
                        op.command = Op::Sub32;
                        op.params.dst = Parameter::Reg32(R::EAX);
                        op.params.src = Parameter::Imm32(self.read_u32(mmu));
                    }
                }
            }
            0x2E => {
                // cs segment prefix
                op.segment_prefix = Segment::CS;
                self.decode(&mut mmu, &mut op);
                op.length += 1;
                return;
            }
            0x2F => op.command = Op::Das,
            0x30 => {
                // xor r/m8, r8
                op.command = Op::Xor8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x31 => {
                // xor r/m16, r16
                // xor r/m32, r32
                self.prefixed_16_32_rm_r(&mut mmu, &mut op, Op::Xor16, Op::Xor32)
            }
            0x32 => {
                // xor r8, r/m8
                op.command = Op::Xor8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x33 => {
                // xor r16, r/m16
                // xor r32, r/m32
                self.prefixed_16_32_r_rm(&mut mmu, &mut op, Op::Xor16, Op::Xor32)
            }
            0x34 => {
                // xor AL, imm8
                op.command = Op::Xor8;
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x35 => {
                // xor AX, imm16
                op.command = Op::Xor16;
                op.params.dst = Parameter::Reg16(R::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0x36 => {
                // ss segment prefix
                op.segment_prefix = Segment::SS;
                self.decode(&mut mmu, &mut op);
                op.length += 1;
                return;
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
                op.params = self.rm16_r16(&mut mmu, op);
            }
            0x3A => {
                // cmp r8, r/m8
                op.command = Op::Cmp8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x3B => {
                // cmp r16, r/m16
                // cmp r32, r/m32
                self.prefixed_16_32_r_rm(&mut mmu, &mut op, Op::Cmp16, Op::Cmp32)
            }
            0x3C => {
                // cmp AL, imm8
                op.command = Op::Cmp8;
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0x3D => {
                match op.op_size {
                    OperandSize::_16bit => {
                        // cmp AX, imm16
                        op.command = Op::Cmp16;
                        op.params.dst = Parameter::Reg16(R::AX);
                        op.params.src = Parameter::Imm16(self.read_u16(mmu));
                    }
                    OperandSize::_32bit => {
                        // cmp EAX, imm32
                        op.command = Op::Cmp32;
                        op.params.dst = Parameter::Reg32(R::EAX);
                        op.params.src = Parameter::Imm32(self.read_u32(mmu));
                    }
                }
            }
            0x3E => {
                // ds segment prefix
                op.segment_prefix = Segment::DS;
                self.decode(&mut mmu, &mut op);
                op.length += 1;
                return;
            }
            0x3F => op.command = Op::Aas,
            0x40...0x47 => match op.op_size {
                OperandSize::_16bit => {
                    // inc r16
                    op.command = Op::Inc16;
                    op.params.dst = Parameter::Reg16(r16(b & 7));
                }
                OperandSize::_32bit => {
                    // inc r32
                    op.command = Op::Inc32;
                    op.params.dst = Parameter::Reg32(r32(b & 7));
                }
            },
            0x48...0x4F => match op.op_size {
                OperandSize::_16bit => {
                    // dec r16
                    op.command = Op::Dec16;
                    op.params.dst = Parameter::Reg16(r16(b & 7));
                }
                OperandSize::_32bit => {
                    // dec r32
                    op.command = Op::Dec32;
                    op.params.dst = Parameter::Reg32(r32(b & 7));
                }
            },
            0x50...0x57 => match op.op_size {
                OperandSize::_16bit => {
                    // push r16
                    op.command = Op::Push16;
                    op.params.dst = Parameter::Reg16(r16(b & 7));
                }
                OperandSize::_32bit => {
                    // push r32
                    op.command = Op::Push32;
                    op.params.dst = Parameter::Reg32(r32(b & 7));
                }
            },
            0x58...0x5F => match op.op_size {
                OperandSize::_16bit => {
                    // pop r16
                    op.command = Op::Pop16;
                    op.params.dst = Parameter::Reg16(r16(b & 7));
                }
                OperandSize::_32bit => {
                    // pop r32
                    op.command = Op::Pop32;
                    op.params.dst = Parameter::Reg32(r32(b & 7));
                }
            },
            0x60 => op.command = match op.op_size {
                OperandSize::_16bit => Op::Pusha16,
                OperandSize::_32bit => Op::Pushad32,
            },
            0x61 => op.command = match op.op_size {
                OperandSize::_16bit => Op::Popa16,
                OperandSize::_32bit => Op::Popad32,
            },
            0x62 => {
                // bound r16, m16&16
                op.command = Op::Bound;
                // XXX not all modes of 2nd argument is valid
                op.params = self.r16_rm16(&mut mmu, op);
            }
            0x63 => {
                // arpl r/m16, r16
                op.command = Op::Arpl;
                op.params = self.rm16_r16(&mut mmu, op);
            }
            0x64 => {
                // fs segment prefix
                op.segment_prefix = Segment::FS;
                self.decode(&mut mmu, &mut op);
                op.length += 1;
                return;
            }
            0x65 => {
                // gs segment prefix
                op.segment_prefix = Segment::GS;
                self.decode(&mut mmu, &mut op);
                op.length += 1;
                return;
            }
            0x66 => {
                // 80386+ Operand-size override prefix
                op.op_size = OperandSize::_32bit;
                self.decode(&mut mmu, &mut op);
                op.length += 1;
                return;
            }
            0x67 => {
                // 80386+ Address-size override prefix
                op.address_size = AddressSize::_32bit;
                self.decode(&mut mmu, &mut op);
                op.length += 1;
                return;
            }
            0x68 => {
                // push imm16
                op.command = Op::Push16;
                op.params.dst = Parameter::Imm16(self.read_u16(mmu));
            }
            0x69 => match op.op_size {
                OperandSize::_16bit => {
                    // imul r16, r/m16, imm16
                    op.command = Op::Imul16;
                    op.params = self.r16_rm16(&mut mmu, op);
                    op.params.src2 = Parameter::Imm16(self.read_u16(mmu));
                }
                OperandSize::_32bit => {
                    // imul r32, r/m32, imm32
                    op.command = Op::Imul32;
                    op.params = self.r32_rm32(&mut mmu, op);
                    op.params.src2 = Parameter::Imm32(self.read_u32(mmu));
                }
            },
            0x6A => {
                // push imm8
                op.command = Op::Push16;
                op.params.dst = Parameter::ImmS8(self.read_s8(mmu));
            }
            0x6B => match op.op_size {
                OperandSize::_16bit => {
                    // imul r16, r/m16, imm8
                    op.command = Op::Imul16;
                    op.params = self.r16_rm16(&mut mmu, op);
                    op.params.src2 = Parameter::Imm8(self.read_u8(mmu));
                }
                OperandSize::_32bit => {
                    // imul r32, r/m32, imm8
                    op.command = Op::Imul32;
                    op.params = self.r32_rm32(&mut mmu, op);
                    op.params.src2 = Parameter::Imm8(self.read_u8(mmu));
                }
            },
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
                op.command = match x.reg {
                    0 => Op::Add8,
                    1 => Op::Or8,
                    2 => Op::Adc8,
                    3 => Op::Sbb8,
                    4 => Op::And8,
                    5 => Op::Sub8,
                    6 => Op::Xor8,
                    7 => Op::Cmp8,
                    _ => unreachable!(),
                };
            }
            0x81 => {
                let x = self.read_mod_reg_rm(mmu);
                match op.op_size {
                    OperandSize::_16bit => {
                        // <arithmetic> r/m16, imm16
                        op.params.dst = self.rm16(&mut mmu, op, x.rm, x.md);
                        op.params.src = Parameter::Imm16(self.read_u16(mmu));
                        op.command = match x.reg {
                            0 => Op::Add16,
                            1 => Op::Or16,
                            2 => Op::Adc16,
                            3 => Op::Sbb16,
                            4 => Op::And16,
                            5 => Op::Sub16,
                            6 => Op::Xor16,
                            7 => Op::Cmp16,
                            _ => unreachable!(),
                        };
                    }
                    OperandSize::_32bit => {
                        // <arithmetic> r/m32, imm32
                        op.params.dst = self.rm32(&mut mmu, op, x.rm, x.md);
                        op.params.src = Parameter::Imm32(self.read_u32(mmu));
                        op.command = match x.reg {
                            0 => Op::Add32,
                            5 => Op::Sub32,
                            7 => Op::Cmp32,
                            _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                        };
                    }
                }
            }
            // 0x82 is unrecognized by objdump & ndisasm, but alias to 0x80 on pre Pentium 4:s according to ref.x86asm.net
            0x83 => {
                let x = self.read_mod_reg_rm(mmu);
                match op.op_size {
                    OperandSize::_16bit => {
                        // <arithmetic> r/m16, imm8
                        op.params.dst = self.rm16(&mut mmu, op, x.rm, x.md);
                        op.params.src = Parameter::ImmS8(self.read_s8(mmu));
                        op.command = match x.reg {
                            0 => Op::Add16,
                            1 => Op::Or16,
                            2 => Op::Adc16,
                            3 => Op::Sbb16,
                            4 => Op::And16,
                            5 => Op::Sub16,
                            6 => Op::Xor16,
                            7 => Op::Cmp16,
                            _ => unreachable!(),
                        };
                    }
                    OperandSize::_32bit => {
                        // <arithmetic> r/m32, imm8
                        op.params.dst = self.rm32(&mut mmu, op, x.rm, x.md);
                        op.params.src = Parameter::ImmS8(self.read_s8(mmu));
                        op.command = match x.reg {
                            0 => Op::Add32,
                            _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                        };
                    }
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
                op.params = self.rm16_r16(&mut mmu, op);
            }
            0x86 => {
                // xchg r/m8, r8
                op.command = Op::Xchg8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x87 => {
                // xchg r/m16, r16
                op.command = Op::Xchg16;
                op.params = self.rm16_r16(&mut mmu, op);
            }
            0x88 => {
                // mov r/m8, r8
                op.command = Op::Mov8;
                op.params = self.rm8_r8(&mut mmu, op.segment_prefix);
            }
            0x89 => {
                // mov r/m16, r16
                // mov r/m32, r32
                self.prefixed_16_32_rm_r(&mut mmu, &mut op, Op::Mov16, Op::Mov32)
            }
            0x8A => {
                // mov r8, r/m8
                op.command = Op::Mov8;
                op.params = self.r8_rm8(&mut mmu, op.segment_prefix);
            }
            0x8B => {
                // mov r16, r/m16
                // mov r32, r/m32
                self.prefixed_16_32_r_rm(&mut mmu, &mut op, Op::Mov16, Op::Mov32)
            }
            0x8C => {
                // mov r/m16, sreg
                op.command = Op::Mov16;
                op.params = self.rm16_sreg(&mut mmu, op);
            }
            0x8D => {
                // lea r16, m
                op.command = Op::Lea16;
                op.params = self.r16_m16(&mut mmu, op);
            }
            0x8E => {
                // mov sreg, r/m16
                op.command = Op::Mov16;
                op.params = self.sreg_rm16(&mut mmu, op);
            }
            0x8F => {
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm16(&mut mmu, op, x.rm, x.md);
                op.command = match x.reg {
                    0 => Op::Pop16, // pop r/m16
                    _ => Op::Invalid(vec!(b), Invalid::FPUOp),
                };
            }
            0x90 => op.command = Op::Nop,
            0x91...0x97 => match op.op_size {
                OperandSize::_16bit => {
                    // xchg AX, r16 | xchg r16, AX
                    // NOTE: "xchg ax,ax" is an alias of "nop"
                    op.command = Op::Xchg16;
                    op.params.dst = Parameter::Reg16(R::AX);
                    op.params.src = Parameter::Reg16(r16(b & 7));
                }
                OperandSize::_32bit => {
                    op.command = Op::Xchg32;
                    op.params.dst = Parameter::Reg32(R::EAX);
                    op.params.src = Parameter::Reg32(r32(b & 7));
                }
            },
            0x98 => {
                op.command = match op.op_size {
                    OperandSize::_16bit => Op::Cbw,
                    OperandSize::_32bit => Op::Cwde32,
                };
            }
            0x99 => op.command = Op::Cwd16,
            0x9A => {
                // call ptr16:16
                op.command = Op::CallFar;
                let imm = self.read_u16(mmu);
                let seg = self.read_u16(mmu);
                op.params.dst = Parameter::Ptr16Imm(seg, imm);
            }
            0x9B => {
                // TODO fpu instructions
                op.command = Op::Invalid(vec!(b), Invalid::FPUOp);
            }
            0x9C => op.command = Op::Pushf,
            0x9D => op.command = Op::Popf,
            0x9E => op.command = Op::Sahf,
            0x9F => op.command = Op::Lahf,
            0xA0 => {
                // mov AL, [moffs8]
                op.command = Op::Mov8;
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Ptr8(op.segment_prefix, self.read_u16(mmu));
            }
            0xA1 => match op.op_size {
                OperandSize::_16bit => {
                    // mov AX, [moffs16]
                    op.command = Op::Mov16;
                    op.params.dst = Parameter::Reg16(R::AX);
                    op.params.src = Parameter::Ptr16(op.segment_prefix, self.read_u16(mmu));
                }
                OperandSize::_32bit => {
                    // mov EAX, [moffs32]
                    op.command = Op::Mov32;
                    op.params.dst = Parameter::Reg32(R::EAX);
                    op.params.src = Parameter::Ptr32(op.segment_prefix, self.read_u16(mmu));
                }
            },
            0xA2 => {
                // mov [moffs8], AL
                op.command = Op::Mov8;
                op.params.dst = Parameter::Ptr8(op.segment_prefix, self.read_u16(mmu));
                op.params.src = Parameter::Reg8(R::AL);
            }
            0xA3 => match op.op_size {
                OperandSize::_16bit => {
                    // mov [moffs16], AX
                    op.command = Op::Mov16;
                    op.params.dst = Parameter::Ptr16(op.segment_prefix, self.read_u16(mmu));
                    op.params.src = Parameter::Reg16(R::AX);
                }
                OperandSize::_32bit => {
                    // mov [moffs32], EAX
                    op.command = Op::Mov32;
                    op.params.dst = Parameter::Ptr32(op.segment_prefix, self.read_u16(mmu));
                    op.params.src = Parameter::Reg32(R::EAX);
                }
            },
            0xA4 => op.command = Op::Movsb,
            0xA5 => op.command = match op.op_size {
                OperandSize::_16bit => Op::Movsw,
                OperandSize::_32bit => Op::Movsd,
            },
            0xA6 => op.command = Op::Cmpsb,
            0xA7 => op.command = Op::Cmpsw,
            0xA8 => {
                // test AL, imm8
                op.command = Op::Test8;
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xA9 => {
                // test AX, imm16
                op.command = Op::Test16;
                op.params.dst = Parameter::Reg16(R::AX);
                op.params.src = Parameter::Imm16(self.read_u16(mmu));
            }
            0xAA => op.command = Op::Stosb,
            0xAB => op.command = match op.op_size {
                OperandSize::_16bit => Op::Stosw,
                OperandSize::_32bit => Op::Stosd,
            },
            0xAC => op.command = Op::Lodsb,
            0xAD => op.command = match op.op_size {
                OperandSize::_16bit => Op::Lodsw,
                OperandSize::_32bit => Op::Lodsd,
            },
            0xAE => op.command = Op::Scasb,
            0xAF => op.command = Op::Scasw,
            0xB0...0xB7 => {
                // mov r8, u8
                op.command = Op::Mov8;
                op.params.dst = Parameter::Reg8(r8(b & 7));
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xB8...0xBF => match op.op_size {
                OperandSize::_16bit => {
                    // mov r16, u16
                    op.command = Op::Mov16;
                    op.params.dst = Parameter::Reg16(r16(b & 7));
                    op.params.src = Parameter::Imm16(self.read_u16(mmu));
                }
                OperandSize::_32bit => {
                    // mov r32, u32
                    op.command = Op::Mov32;
                    op.params.dst = Parameter::Reg32(r32(b & 7));
                    op.params.src = Parameter::Imm32(self.read_u32(mmu));
                }
            },
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
                    _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                };
                op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xC1 => {
                let x = self.read_mod_reg_rm(mmu);
                match op.op_size {
                    OperandSize::_16bit => {
                        // r16, byte imm8
                        op.command = match x.reg {
                            0 => Op::Rol16,
                            1 => Op::Ror16,
                            2 => Op::Rcl16,
                            3 => Op::Rcr16,
                            4 => Op::Shl16,
                            5 => Op::Shr16,
                            7 => Op::Sar16,
                            _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                        };
                        op.params.dst = self.rm16(&mut mmu, op, x.rm, x.md);
                        op.params.src = Parameter::Imm8(self.read_u8(mmu));
                    }
                    OperandSize::_32bit => {
                        // r32, byte imm8
                        op.command = match x.reg {
                            3 => Op::Rcr32,
                            4 => Op::Shl32,
                            5 => Op::Shr32,
                            7 => Op::Sar32,
                            _ => panic!("unhandled 0xc1 32bit reg {}, ip = {:04X}:{:04X}", x.reg, self.current_seg, self.current_offset),
                        };
                        op.params.dst = self.rm32(&mut mmu, op, x.rm, x.md);
                        op.params.src = Parameter::Imm8(self.read_u8(mmu));
                    }
                }
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
                op.params = self.r16_m16(&mut mmu, op);
            }
            0xC5 => {
                // lds r16, m16
                op.command = Op::Lds;
                op.params = self.r16_m16(&mut mmu, op);
            }
            0xC6 => {
                let x = self.read_mod_reg_rm(mmu);
                op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
                op.command = match x.reg {
                    0 => Op::Mov8, // mov r/m8, imm8
                    _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                };
            }
            0xC7 => {
                let x = self.read_mod_reg_rm(mmu);
                match op.op_size {
                    OperandSize::_16bit => {
                        op.params.dst = self.rm16(&mut mmu, op, x.rm, x.md);
                        op.params.src = Parameter::Imm16(self.read_u16(mmu));
                        op.command = match x.reg {
                            0 => Op::Mov16, // mov r/m16, imm16
                            _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                        };
                    }
                    OperandSize::_32bit => {
                        op.params.dst = self.rm32(&mut mmu, op, x.rm, x.md);
                        op.params.src = Parameter::Imm32(self.read_u32(mmu));
                        op.command = match x.reg {
                            0 => Op::Mov32, // mov r/m32, imm32
                            _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                        };
                    }
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
            0xCE => op.command = Op::Into,
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
                    _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
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
                    _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                };
                op.params.dst = self.rm16(&mut mmu, op, x.rm, x.md);
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
                    _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                };
                op.params.dst = self.rm8(&mut mmu, op.segment_prefix, x.rm, x.md);
                op.params.src = Parameter::Reg8(R::CL);
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
                    _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                };
                op.params.dst = self.rm16(&mut mmu, op, x.rm, x.md);
                op.params.src = Parameter::Reg8(R::CL);
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
                op.command = Op::Invalid(vec!(b), Invalid::FPUOp);
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
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xE5 => {
                // in AX, imm8
                op.command = Op::In16;
                op.params.dst = Parameter::Reg16(R::AX);
                op.params.src = Parameter::Imm8(self.read_u8(mmu));
            }
            0xE6 => {
                // OUT imm8, AL
                op.command = Op::Out8;
                op.params.dst = Parameter::Imm8(self.read_u8(mmu));
                op.params.src = Parameter::Reg8(R::AL);
            }
            0xE7 => {
                // OUT imm8, AX
                op.command = Op::Out16;
                op.params.dst = Parameter::Imm8(self.read_u8(mmu));
                op.params.src = Parameter::Reg16(R::AX);
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
                op.params.dst = Parameter::Reg8(R::AL);
                op.params.src = Parameter::Reg16(R::DX);
            }
            0xED => {
                // in AX, DX
                op.command = Op::In16;
                op.params.dst = Parameter::Reg16(R::AX);
                op.params.src = Parameter::Reg16(R::DX);
            }
            0xEE => {
                // out DX, AL
                op.command = Op::Out8;
                op.params.dst = Parameter::Reg16(R::DX);
                op.params.src = Parameter::Reg8(R::AL);
            }
            0xEF => {
                // out DX, AX
                op.command = Op::Out16;
                op.params.dst = Parameter::Reg16(R::DX);
                op.params.src = Parameter::Reg16(R::AX);
            }
            0xF0 => {
                // lock prefix
                op.lock = true;
                self.decode(&mut mmu, &mut op);
                op.length += 1;
                return;
            }
            0xF1 => {
                op.command = Op::Int;
                op.params.dst = Parameter::Imm8(1);
            }
            0xF2 => {
                // repne (cmps, scas) prefix
                op.repeat = RepeatMode::Repne;
                self.decode(&mut mmu, &mut op);
                op.length += 1;
                return;
            }
            0xF3 => {
                // rep (ins, movs, outs, lods, stos), repe (cmps, scas) prefix
                self.decode(&mut mmu, &mut op);
                op.length += 1;
                match op.command {
                    Op::Insb | Op::Insw |
                    Op::Outsb | Op::Outsw |
                    Op::Movsb | Op::Movsw | Op::Movsd |
                    Op::Stosb | Op::Stosw | Op::Stosd |
                    Op::Lodsb | Op::Lodsw | Op::Lodsd => {
                        op.repeat = RepeatMode::Rep;
                    }
                    Op::Cmpsb | Op::Cmpsw |
                    Op::Scasb | Op::Scasw => {
                        op.repeat = RepeatMode::Repe;
                    }
                    _ => op.command = Op::Invalid(vec!(b), Invalid::Op), // XXX not encoding the instruction bytes after 0xf3 prefix
                }
                return;
            }
            0xF4 => op.command = Op::Hlt,
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
                let x = self.read_mod_reg_rm(mmu);
                match op.op_size {
                    OperandSize::_16bit => {
                        // <math> r/m16
                        op.params.dst = self.rm16(&mut mmu, op, x.rm, x.md);
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
                    OperandSize::_32bit => {
                        op.params.dst = self.rm32(&mut mmu, op, x.rm, x.md);
                        op.command = match x.reg {
                            3 => Op::Neg32,
                            4 => Op::Mul32,
                            5 => Op::Imul32,
                            6 => Op::Div32,
                            7 => Op::Idiv32,
                            _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                        };
                    }
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
                op.command = match x.reg {
                    // NOTE: 2 is a deprecated but valid encoding, example:
                    // https://www.pouet.net/prod.php?which=65203
                    // 00000140  FEC5              inc ch
                    0 | 2 => Op::Inc8,
                    1 => Op::Dec8,
                    _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                };
            }
            0xFF => {
                let x = self.read_mod_reg_rm(mmu);
                match op.op_size {
                    OperandSize::_16bit => {
                        // r/m16
                        op.params.dst = self.rm16(&mut mmu, op, x.rm, x.md);
                        op.command = match x.reg {
                            0 => Op::Inc16,
                            1 => Op::Dec16,
                            2 => Op::CallNear,
                            3 => Op::CallFar,
                            4 => Op::JmpNear,
                            5 => Op::JmpFar,
                            6 => Op::Push16,
                            _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                        };
                    }
                    OperandSize::_32bit => {
                        op.params.dst = self.rm32(&mut mmu, op, x.rm, x.md);
                        op.command = match x.reg {
                            0 => Op::Inc32,
                            1 => Op::Dec32,
                            _ => Op::Invalid(vec!(b), Invalid::Reg(x.reg)),
                        };
                    }
                }
            }
            _ => op.command = Op::Invalid(vec!(b), Invalid::Op),
        }
        // calculate instruction length
        op.length = (Wrapping(u16::from(op.length)) + Wrapping(self.current_offset) - Wrapping(start_offset)).0 as u8;
        if DEBUG_DECODER {
            println!("decode op end {:?}", op);
        }
    }

    fn prefixed_16_32_rm_r(&mut self, mut mmu: &mut MMU, op: &mut Instruction, op16: Op, op32: Op) {
        match op.op_size {
            OperandSize::_16bit => {
                op.command = op16;
                op.params = self.rm16_r16(&mut mmu, op);
            }
            OperandSize::_32bit => {
                op.command = op32;
                op.params = self.rm32_r32(&mut mmu, op);
            }
        }
    }

    fn prefixed_16_32_r_rm(&mut self, mut mmu: &mut MMU, op: &mut Instruction, op16: Op, op32: Op) {
        match op.op_size {
            OperandSize::_16bit => {
                op.command = op16;
                op.params = self.r16_rm16(&mut mmu, op);
            }
            OperandSize::_32bit => {
                op.command = op32;
                op.params = self.r32_rm32(&mut mmu, op);
            }
        }
    }

    // decode rm8
    fn rm8(&mut self, mmu: &mut MMU, seg: Segment, rm: u8, md: u8) -> Parameter {
        let adr_size = AddressSize::_16bit;
        match md {
            0 => {
                if rm == 6 {
                    // [u16]
                    Parameter::Ptr8(seg, self.read_u16(mmu))
                } else {
                    // [amode]
                    Parameter::Ptr8Amode(seg, adr_size.amode_from(rm))
                }
            }
            // [amode+s8]
            1 => Parameter::Ptr8AmodeS8(seg, adr_size.amode_from(rm), self.read_s8(mmu)),
            // [amode+s16]
            2 => Parameter::Ptr8AmodeS16(seg, adr_size.amode_from(rm), self.read_s16(mmu)),
            // reg
            3 => Parameter::Reg8(r8(rm)),
            _ => unreachable!(),
        }
    }

    // decode rm16
    fn rm16(&mut self, mmu: &mut MMU, op: &Instruction, rm: u8, md: u8) -> Parameter {
        match md {
            0 => {
                if rm == 6 {
                    // [u16]
                    Parameter::Ptr16(op.segment_prefix, self.read_u16(mmu))
                } else {
                    // [amode]
                    Parameter::Ptr16Amode(op.segment_prefix, op.address_size.amode_from(rm))
                }
            }
            // [amode+s8]
            1 => Parameter::Ptr16AmodeS8(op.segment_prefix, op.address_size.amode_from(rm), self.read_s8(mmu)),
            // [amode+s16]
            2 => Parameter::Ptr16AmodeS16(op.segment_prefix, op.address_size.amode_from(rm), self.read_s16(mmu)),
            // [reg]
            3 => Parameter::Reg16(r16(rm)),
            _ => unreachable!(),
        }
    }

    // decode rm32
    fn rm32(&mut self, mmu: &mut MMU, op: &Instruction, rm: u8, md: u8) -> Parameter {
        match md {
            0 => {
                if rm == 6 {
                    // [u16]
                    Parameter::Ptr32(op.segment_prefix, self.read_u16(mmu))
                } else {
                    // [amode]
                    Parameter::Ptr32Amode(op.segment_prefix, op.address_size.amode_from(rm))
                }
            }
            // [amode+s8]
            1 => Parameter::Ptr32AmodeS8(op.segment_prefix, op.address_size.amode_from(rm), self.read_s8(mmu)),
            // [amode+s16]
            2 => Parameter::Ptr32AmodeS16(op.segment_prefix, op.address_size.amode_from(rm), self.read_s16(mmu)),
            // [reg]
            3 => Parameter::Reg32(r32(rm)),
            _ => unreachable!(),
        }
    }

    // decode r8, r/m8
    fn r8_rm8(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: Parameter::Reg8(r8(x.reg)),
            src: self.rm8(&mut mmu, seg, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r/m8, r8
    fn rm8_r8(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: self.rm8(&mut mmu, seg, x.rm, x.md),
            src: Parameter::Reg8(r8(x.reg)),
            src2: Parameter::None,
        }
    }

    // decode Sreg, r/m16
    fn sreg_rm16(&mut self, mut mmu: &mut MMU, op: &Instruction) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: Parameter::SReg16(sr(x.reg)),
            src: self.rm16(&mut mmu, op, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r/m16, Sreg
    fn rm16_sreg(&mut self, mut mmu: &mut MMU, op: &Instruction) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: self.rm16(&mut mmu, op, x.rm, x.md),
            src: Parameter::SReg16(sr(x.reg)),
            src2: Parameter::None,
        }
    }

    // decode r16, r/m8 (movzx)
    fn r16_rm8(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: Parameter::Reg16(r16(x.reg)),
            src: self.rm8(&mut mmu, seg, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r32, r/m8 (movzx)
    fn r32_rm8(&mut self, mut mmu: &mut MMU, seg: Segment) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: Parameter::Reg32(r32(x.reg)),
            src: self.rm8(&mut mmu, seg, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r32, r/m16 (movsx)
    fn r32_rm16(&mut self, mut mmu: &mut MMU, op: &Instruction) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: Parameter::Reg32(r32(x.reg)),
            src: self.rm16(&mut mmu, op, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r16, r/m16
    fn r16_rm16(&mut self, mut mmu: &mut MMU, op: &Instruction) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: Parameter::Reg16(r16(x.reg)),
            src: self.rm16(&mut mmu, op, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r/m16, r16
    fn rm16_r16(&mut self, mut mmu: &mut MMU, op: &Instruction) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: self.rm16(&mut mmu, op, x.rm, x.md),
            src: Parameter::Reg16(r16(x.reg)),
            src2: Parameter::None,
        }
    }

    // decode r16, m16
    fn r16_m16(&mut self, mut mmu: &mut MMU, op: &Instruction) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        if x.md == 3 {
            println!("r16_m16 error: invalid encoding, ip={:04X}", self.current_offset);
        }
        ParameterSet {
            dst: Parameter::Reg16(r16(x.reg)),
            src: self.rm16(&mut mmu, op, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r32, r/m32
    fn r32_rm32(&mut self, mut mmu: &mut MMU, op: &Instruction) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: Parameter::Reg32(r32(x.reg)),
            src: self.rm32(&mut mmu, op, x.rm, x.md),
            src2: Parameter::None,
        }
    }

    // decode r/m32, r32
    fn rm32_r32(&mut self, mut mmu: &mut MMU, op: &Instruction) -> ParameterSet {
        let x = self.read_mod_reg_rm(mmu);
        ParameterSet {
            dst: self.rm32(&mut mmu, op, x.rm, x.md),
            src: Parameter::Reg32(r32(x.reg)),
            src2: Parameter::None,
        }
    }

    fn read_mod_reg_rm(&mut self, mmu: &MMU) -> ModRegRm {
        let b = mmu.read_u8(self.current_seg, self.current_offset);
        self.current_offset += 1;
        let res = ModRegRm {
            md: b >> 6, // high 2 bits
            reg: (b >> 3) & 7, // mid 3 bits
            rm: b & 7, // low 3 bits
        };
        if DEBUG_DECODER {
            println!("read_mod_reg_rm byte {:02X} = mod {}, reg {}, rm {}", b, res.md, res.reg, res.rm);
        }
        res
    }

    fn read_rel8(&mut self, mmu: &MMU) -> u16 {
        let val = self.read_s8(mmu);
        (self.current_offset as i16 + i16::from(val)) as u16
    }

    fn read_rel16(&mut self, mmu: &MMU) -> u16 {
        let val = self.read_s16(mmu);
        (self.current_offset as i16 + val) as u16
    }

    fn read_u8(&mut self, mmu: &MMU) -> u8 {
        let b = mmu.read_u8(self.current_seg, self.current_offset);
        self.current_offset = (Wrapping(self.current_offset) + Wrapping(1)).0;
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

    fn read_u32(&mut self, mmu: &MMU) -> u32 {
        let lo = self.read_u16(mmu);
        let hi = self.read_u16(mmu);
        u32::from(hi) << 16 | u32::from(lo)
    }

    fn read_s16(&mut self, mmu: &MMU) -> i16 {
        self.read_u16(mmu) as i16
    }

    // returns the flat starting offset of the instruction being decoded
    fn current_flat(&self) -> u32 {
        MemoryAddress::RealSegmentOffset(self.current_seg, self.current_offset).value()
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
