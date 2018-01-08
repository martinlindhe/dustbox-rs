use mmu::MMU;
use instruction::{
    Instruction,
    ParameterPair,
    Op,
    Parameter,
    ModRegRm,
    InvalidOp
};
use segment::Segment;
use register::{AX, BX, CX, DX, SI, DI, BP, SP, AL, CL, CS, DS, ES, FS, GS, SS};

pub struct Decoder<'a> {
    mmu: &'a MMU,
    c_seg: u16,
    c_offset: u16,
}

impl<'a> Decoder<'a> {
    pub fn new(mmu: &'a MMU) -> Self {
        Decoder {
            mmu,
            c_seg: 0,
            c_offset: 0,
        }
    }

    pub fn get_instruction(&mut self, iseg: u16, ioffset: u16, seg: Segment) -> Instruction {
        self.c_seg = iseg;
        self.c_offset = ioffset;

        self.decode(seg)
    }

    fn decode(&mut self, seg: Segment) -> Instruction {
        let iseg = self.c_seg;
        let ioffset = self.c_offset;
        let b = self.mmu.read_u8(iseg, ioffset);

        let mut op = Instruction {
            segment: seg,
            command: Op::Unknown(),
            params: ParameterPair {
                dst: Parameter::None(),
                src: Parameter::None(),
                src2: Parameter::None(),
            },
            byte_length: 0,
        };

        match b {
            0x00 => {
                // add r/m8, r8
                op.command = Op::Add8();
                op.params = self.rm8_r8(op.segment);
            }
            0x01 => {
                // add r/m16, r16
                op.command = Op::Add16();
                op.params = self.rm16_r16(op.segment);
            }
            0x02 => {
                // add r8, r/m8
                op.command = Op::Add8();
                op.params = self.r8_rm8(op.segment);
            }
            0x03 => {
                // add r16, r/m16
                op.command = Op::Add16();
                op.params = self.r16_rm16(op.segment);
            }
            0x04 => {
                // add AL, imm8
                op.command = Op::Add8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x05 => {
                // add AX, imm16
                op.command = Op::Add16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0x06 => {
                // push es
                op.command = Op::Push16();
                op.params.dst = Parameter::SReg16(ES);
            }
            0x07 => {
                // pop es
                op.command = Op::Pop16();
                op.params.dst = Parameter::SReg16(ES);
            }
            0x08 => {
                // or r/m8, r8
                op.command = Op::Or8();
                op.params = self.rm8_r8(op.segment);
            }
            0x09 => {
                // or r/m16, r16
                op.command = Op::Or16();
                op.params = self.rm16_r16(op.segment);
            }
            0x0A => {
                // or r8, r/m8
                op.command = Op::Or8();
                op.params = self.r8_rm8(op.segment);
            }
            0x0B => {
                // or r16, r/m16
                op.command = Op::Or16();
                op.params = self.r16_rm16(op.segment);
            }
            0x0C => {
                // or AL, imm8
                op.command = Op::Or8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x0D => {
                // or AX, imm16
                op.command = Op::Or16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0x0E => {
                // push cs
                op.command = Op::Push16();
                op.params.dst = Parameter::SReg16(CS);
            }
            0x0F => {
                let b = self.read_u8();
                match b {
                    0x84 => {
                        // jz rel16
                        op.command = Op::Jz();
                        op.params.dst = Parameter::Imm16(self.read_rel16());
                    }
                    0x85 => {
                        // jnz rel16
                        op.command = Op::Jnz();
                        op.params.dst = Parameter::Imm16(self.read_rel16());
                    }
                    0x92 => {
                        // setc r/m8
                        let x = self.read_mod_reg_rm();
                        op.command = Op::Setc();
                        op.params.dst = self.rm8(op.segment, x.rm, x.md);
                    }
                    0xA0 => {
                        // push fs
                        op.command = Op::Push16();
                        op.params.dst = Parameter::SReg16(FS);
                    }
                    0xA1 => {
                        // pop fs
                        op.command = Op::Pop16();
                        op.params.dst = Parameter::SReg16(FS);
                    }
                    0xA8 => {
                        // push gs
                        op.command = Op::Push16();
                        op.params.dst = Parameter::SReg16(GS);
                    }
                    0xA9 => {
                        // pop gs
                        op.command = Op::Pop16();
                        op.params.dst = Parameter::SReg16(GS);
                    }
                    0xAC => {
                        // shrd r/m16, r16, imm8
                        op.command = Op::Shrd();
                        op.params = self.rm16_r16(op.segment);
                        op.params.src2 = Parameter::Imm8(self.read_u8());
                    }
                    0xAF => {
                        // imul r16, r/m16
                        op.command = Op::Imul16();
                        op.params = self.r16_rm16(op.segment);
                    }
                    0xB6 => {
                        // movzx r16, r/m8
                        op.command = Op::Movzx16();
                        op.params = self.r16_rm8(op.segment);
                    }
                    0xBE => {
                        // movsx r16, r/m8
                        op.command = Op::Movsx16();
                        op.params = self.r16_rm8(op.segment);
                    }
                    _ => {
                        let invalid = InvalidOp::Op(vec![0x0F, b]);
                        op.command = Op::Invalid(invalid);
                        // println!("op 0F, unknown {:02X}: at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                        //     b,
                        //     self.sreg16[CS],
                        //     self.ip - 1,
                        //     self.get_offset() - 1,
                        //     self.instruction_count);
                    }
                }
            }
            0x10 => {
                // adc r/m8, r8
                op.command = Op::Adc8();
                op.params = self.rm8_r8(op.segment);
            }
            0x13 => {
                // adc r16, r/m16
                op.command = Op::Adc16();
                op.params = self.r16_rm16(op.segment);
            }
            0x14 => {
                // adc AL, imm8
                op.command = Op::Adc8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x1C => {
                // sbb AL, imm8
                op.command = Op::Sbb8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x1E => {
                // push ds
                op.command = Op::Push16();
                op.params.dst = Parameter::SReg16(DS);
            }
            0x1F => {
                // pop ds
                op.command = Op::Pop16();
                op.params.dst = Parameter::SReg16(DS);
            }
            0x20 => {
                // and r/m8, r8
                op.command = Op::And8();
                op.params = self.rm8_r8(op.segment);
            }
            0x21 => {
                // and r/m16, r16
                op.command = Op::And16();
                op.params = self.rm16_r16(op.segment);
            }
            0x22 => {
                // and r8, r/m8
                op.command = Op::And8();
                op.params = self.r8_rm8(op.segment);
            }
            0x23 => {
                // and r16, r/m16
                op.command = Op::And16();
                op.params = self.r16_rm16(op.segment);
            }
            0x24 => {
                // and AL, imm8
                op.command = Op::And8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x25 => {
                // and AX, imm16
                op.command = Op::And16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0x26 => {
                // es segment prefix
                op = self.decode(Segment::ES());
            }
            0x27 => {
                // daa
                op.command = Op::Daa();
            }
            0x28 => {
                // sub r/m8, r8
                op.command = Op::Sub8();
                op.params = self.rm8_r8(op.segment);
            }
            0x29 => {
                // sub r/m16, r16
                op.command = Op::Sub16();
                op.params = self.rm16_r16(op.segment);
            }
            0x2A => {
                // sub r8, r/m8
                op.command = Op::Sub8();
                op.params = self.r8_rm8(op.segment);
            }
            0x2B => {
                // sub r16, r/m16
                op.command = Op::Sub16();
                op.params = self.r16_rm16(op.segment);
            }
            0x2C => {
                // sub AL, imm8
                op.command = Op::Sub8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x2D => {
                // sub AX, imm16
                op.command = Op::Sub16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0x2E => {
                // XXX if next op is a Jcc, then this is a "branch not taken" hint
                op = self.decode(Segment::CS());
            }
            0x2F => {
                op.command = Op::Das();
            }
            0x30 => {
                // xor r/m8, r8
                op.command = Op::Xor8();
                op.params = self.rm8_r8(op.segment);
            }
            0x31 => {
                // xor r/m16, r16
                op.command = Op::Xor16();
                op.params = self.rm16_r16(op.segment);
            }
            0x32 => {
                // xor r8, r/m8
                op.command = Op::Xor8();
                op.params = self.r8_rm8(op.segment);
            }
            0x33 => {
                // xor r16, r/m16
                op.command = Op::Xor16();
                op.params = self.r16_rm16(op.segment);
            }
            0x34 => {
                // xor AL, imm8
                op.command = Op::Xor8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x35 => {
                // xor AX, imm16
                op.command = Op::Xor16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0x36 => {
                // ss segment prefix
                op = self.decode(Segment::SS());
            }
            0x37 => {
                op.command = Op::Aaa();
            }
            0x38 => {
                // cmp r/m8, r8
                op.command = Op::Cmp8();
                op.params = self.rm8_r8(op.segment);
            }
            0x39 => {
                // cmp r/m16, r16
                op.command = Op::Cmp16();
                op.params = self.rm16_r16(op.segment);
            }
            0x3A => {
                // cmp r8, r/m8
                op.command = Op::Cmp8();
                op.params = self.r8_rm8(op.segment);
            }
            0x3B => {
                // cmp r16, r/m16
                op.command = Op::Cmp16();
                op.params = self.r16_rm16(op.segment);
            }
            0x3C => {
                // cmp AL, imm8
                op.command = Op::Cmp8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x3D => {
                // cmp AX, imm16
                op.command = Op::Cmp16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0x3E => {
                // ds segment prefix
                // XXX if next op is a Jcc, then this is a "branch taken" hint
                op = self.decode(Segment::DS());
            }
            0x3F => {
                op.command = Op::Aas();
            }
            0x40...0x47 => {
                // inc r16
                op.command = Op::Inc16();
                op.params.dst = Parameter::Reg16((b & 7) as usize);
            }
            0x48...0x4F => {
                // dec r16
                op.command = Op::Dec16();
                op.params.dst = Parameter::Reg16((b & 7) as usize);
            }
            0x50...0x57 => {
                // push r16
                op.command = Op::Push16();
                op.params.dst = Parameter::Reg16((b & 7) as usize);
            }
            0x58...0x5F => {
                // pop r16
                op.command = Op::Pop16();
                op.params.dst = Parameter::Reg16((b & 7) as usize);
            }
            0x60 => {
                // pusha
                op.command = Op::Pusha();
            }
            0x61 => {
                // popa
                op.command = Op::Popa();
            }
            // 0x62 = "bound"
            0x63 => {
                // arpl r/m16, r16
                op.command = Op::Arpl();
                op.params = self.rm16_r16(op.segment);
            }
            0x64 => {
                // fs segment prefix
                op = self.decode(Segment::FS());
            }
            0x65 => {
                // gs segment prefix
                op = self.decode(Segment::GS());
            }
            // 0x66 = 80386+ Operand-size override prefix
            // 0x67 = 80386+ Address-size override prefix
            0x68 => {
                // push imm16
                op.command = Op::Push16();
                op.params.dst = Parameter::Imm16(self.read_u16());
            }
            0x69 => {
                // imul r16, r/m16, imm16
                op.command = Op::Imul16();
                op.params = self.r16_rm16(op.segment);
                op.params.src2 = Parameter::Imm16(self.read_u16());
            }
            0x6A => {
                // push imm8
                op.command = Op::Push8();
                op.params.dst = Parameter::ImmS8(self.read_s8());
            }
            0x6B => {
                // imul r16, r/m16, imm8
                op.command = Op::Imul16();
                op.params = self.r16_rm16(op.segment);
                op.params.src2 = Parameter::Imm8(self.read_u8());
            }
            0x6C => {
                op.command = Op::Insb();
            }
            0x6D => {
                op.command = Op::Insw();
            }
            0x6E => {
                op.command = Op::Outsb();
            }
            0x6F => {
                op.command = Op::Outsw();
            }
            0x70 => {
                // jo rel8
                op.command = Op::Jo();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x71 => {
                // jno rel8
                op.command = Op::Jno();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x72 => {
                // jc rel8
                op.command = Op::Jc();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x73 => {
                // jnc rel8
                op.command = Op::Jnc();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x74 => {
                // jz rel8
                op.command = Op::Jz();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x75 => {
                // jnz rel8
                op.command = Op::Jnz();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x76 => {
                // jna rel8
                op.command = Op::Jna();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x77 => {
                // ja rel8
                op.command = Op::Ja();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x78 => {
                // js rel8
                op.command = Op::Js();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x79 => {
                // jns rel8
                op.command = Op::Jns();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
	        0x7A => {
                // jpe rel8
		        op.command = Op::Jpe(); // alias: jp
		        op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x7B => {
                // jpo rel8
                op.command = Op::Jpo(); // alias: jnp
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x7C => {
                // jl rel8
                op.command = Op::Jl();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x7D => {
                // jnl rel8
                op.command = Op::Jnl();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x7E => {
                // jng rel8
                op.command = Op::Jng();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x7F => {
                // jg rel8
                op.command = Op::Jg();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x80 => {
                // arithmetic 8-bit
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8());
                match x.reg {
                    0 => {
                        // add r/m8, imm8
                        op.command = Op::Add8();
                    }
                    1 => {
                        // or r/m8, imm8
                        op.command = Op::Or8();
                    }
                    2 => {
                        // adc r/m8, imm8
                        op.command = Op::Adc8();
                    }
                    3 => {
                        // sbb r/m8, imm8
                        op.command = Op::Sbb8();
                    }
                    4 => {
                        // and r/m8, imm8
                        op.command = Op::And8();
                    }
                    5 => {
                        // sub r/m8, imm8
                        op.command = Op::Sub8();
                    }
                    6 => {
                        // xor r/m8, imm8
                        op.command = Op::Xor8();
                    }
                    7 => {
                        // cmp r/m8, imm8
                        op.command = Op::Cmp8();
                    }
                    _ => {} // XXX how to get rid of this pattern, x.reg is only 3 bits
                }
            }
            0x81 => {
                // arithmetic 16-bit
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm16(self.read_u16());
                match x.reg {
                    0 => {
                        op.command = Op::Add16();
                    }
                    1 => {
                        op.command = Op::Or16();
                    }
                    2 => {
                        op.command = Op::Adc16();
                    }
                    3 => {
                        op.command = Op::Sbb16();
                    }
                    4 => {
                        op.command = Op::And16();
                    }
                    5 => {
                        op.command = Op::Sub16();
                    }
                    6 => {
                        op.command = Op::Xor16();
                    }
                    7 => {
                        op.command = Op::Cmp16();
                    }
                    _ => {}
                }
            }
            // 0x82 is unrecognized by objdump & ndisasm, but alias to 0x80 on pre Pentium 4:s according to ref.x86asm.net
            0x83 => {
                // arithmetic 16-bit with signed 8-bit value
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::ImmS8(self.read_s8());
                match x.reg {
                    0 => {
                        op.command = Op::Add16();
                    }
                    1 => {
                        op.command = Op::Or16();
                    }
                    2 => {
                        op.command = Op::Adc16();
                    }
                    3 => {
                        op.command = Op::Sbb16();
                    }
                    4 => {
                        op.command = Op::And16();
                    }
                    5 => {
                        op.command = Op::Sub16();
                    }
                    6 => {
                        op.command = Op::Xor16();
                    }
                    7 => {
                        op.command = Op::Cmp16();
                    }
                    _ => {}
                }
            }
            0x84 => {
                // test r/m8, r8
                op.command = Op::Test8();
                op.params = self.rm8_r8(op.segment);
            }
            0x85 => {
                // test r/m16, r16
                op.command = Op::Test16();
                op.params = self.rm16_r16(op.segment);
            }
            0x86 => {
                // xchg r/m8, r8 | xchg r8, r/m8
                op.command = Op::Xchg8();
                op.params = self.rm8_r8(op.segment);
            }
            0x87 => {
                // xchg r/m16, r16 | xchg r16, r/m16
                op.command = Op::Xchg16();
                op.params = self.rm16_r16(op.segment);
            }
            0x88 => {
                // mov r/m8, r8
                op.command = Op::Mov8();
                op.params = self.rm8_r8(op.segment);
            }
            0x89 => {
                // mov r/m16, r16
                op.command = Op::Mov16();
                op.params = self.rm16_r16(op.segment);
            }
            0x8A => {
                // mov r8, r/m8
                op.command = Op::Mov8();
                op.params = self.r8_rm8(op.segment);
            }
            0x8B => {
                // mov r16, r/m16
                op.command = Op::Mov16();
                op.params = self.r16_rm16(op.segment);
            }
            0x8C => {
                // mov r/m16, sreg
                op.command = Op::Mov16();
                op.params = self.rm16_sreg(op.segment);
            }
            0x8D => {
                // lea r16, m
                op.command = Op::Lea16();
                op.params = self.r16_m16(op.segment);
            }
            0x8E => {
                // mov sreg, r/m16
                op.command = Op::Mov16();
                op.params = self.sreg_rm16(op.segment);
            }
            0x8F => {
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                match x.reg {
                    0 => {
                        // pop r/m16
                        op.command = Op::Pop16();
                    }
                    _ => {
                        let invalid = InvalidOp::Reg(x.reg);
                        op.command = Op::Invalid(invalid);
                        // println!("op 8F unknown reg = {}: at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                        //     x.reg,
                        //     self.sreg16[CS],
                        //     self.ip - 1,
                        //     self.get_offset() - 1,
                        //     self.instruction_count);
                    }
                }
            }
            0x90 => {
                // nop
                op.command = Op::Nop();
            }
            0x91...0x97 => {
                // xchg AX, r16  | xchg r16, AX
                // NOTE: ("xchg ax,ax" is an alias of "nop")
                op.command = Op::Xchg16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Reg16((b & 7) as usize);
            }
            0x98 => {
                // cbw
                op.command = Op::Cbw();
            }
            0x99 => {
                op.command = Op::Cwd();
            }
            // 0x9A = "call word imm16:imm16"
            // 0x9B = "wait"
            0x9C => {
                op.command = Op::Pushf();
            }
            0x9D => {
                op.command = Op::Popf();
            }
            0x9E => {
                op.command = Op::Sahf();
            }
            0x9F => {
                op.command = Op::Lahf();
            }
            0xA0 => {
                // mov AL, moffs8
                op.command = Op::Mov8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Ptr8(op.segment, self.read_u16());
            }
            0xA1 => {
                // mov AX, moffs16
                op.command = Op::Mov16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Ptr16(op.segment, self.read_u16());
            }
            0xA2 => {
                // mov moffs8, AL
                op.command = Op::Mov8();
                op.params.dst = Parameter::Ptr8(op.segment, self.read_u16());
                op.params.src = Parameter::Reg8(AL);
            }
            0xA3 => {
                // mov moffs16, AX
                op.command = Op::Mov16();
                op.params.dst = Parameter::Ptr16(op.segment, self.read_u16());
                op.params.src = Parameter::Reg16(AX);
            }
            0xA4 => {
                op.command = Op::Movsb();
            }
            0xA5 => {
                op.command = Op::Movsw();
            }
            0xA6 => {
                op.command = Op::Cmpsb();
            }
            0xA7 => {
		        op.command = Op::Cmpsw();
            }
            0xA8 => {
                // test AL, imm8
                op.command = Op::Test8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0xA9 => {
                // test AX, imm16
                op.command = Op::Test16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0xAA => {
                op.command = Op::Stosb();
            }
            0xAB => {
                op.command = Op::Stosw();
            }
            0xAC => {
                op.command = Op::Lodsb();
            }
            0xAD => {
                op.command = Op::Lodsw();
            }
            0xAE => {
		        op.command = Op::Scasb();
            }
	        0xAF => {
		        op.command = Op::Scasw();
            }
            0xB0...0xB7 => {
                // mov r8, u8
                op.command = Op::Mov8();
                op.params.dst = Parameter::Reg8((b & 7) as usize);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0xB8...0xBF => {
                // mov r16, u16
                op.command = Op::Mov16();
                op.params.dst = Parameter::Reg16((b & 7) as usize);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0xC0 => {
                // r8, byte imm8
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    0 => Op::Rol8(),
                    1 => Op::Ror8(),
                    2 => Op::Rcl8(),
                    3 => Op::Rcr8(),
                    4 => Op::Shl8(),
                    5 => Op::Shr8(),
                    7 => Op::Sar8(),
                    _ => {
                        let invalid = InvalidOp::Op(vec![0xC0, x.reg]);
                        Op::Invalid(invalid)
                    }
                };
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0xC1 => {
                // r16, byte imm8
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    0 => Op::Rol16(),
                    1 => Op::Ror16(),
                    2 => Op::Rcl16(),
                    3 => Op::Rcr16(),
                    4 => Op::Shl16(),
                    5 => Op::Shr16(),
                    7 => Op::Sar16(),
                    _ => {
                        let invalid = InvalidOp::Op(vec![0xC1, x.reg]);
                        Op::Invalid(invalid)
                    }
                };
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            // 0xC2 = ret imm16
            0xC3 => {
                // ret [near]
                op.command = Op::Retn();
            }
            0xC4 => {
                // les r16, m16
                op.command = Op::Les();
                op.params = self.r16_m16(op.segment);
            }
            0xC5 => {
                // lds r16, m16
                op.command = Op::Lds();
                op.params = self.r16_m16(op.segment);
            }
            0xC6 => {
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8());
                match x.reg {
                    0 => {
                        // mov r/m8, imm8
                        op.command = Op::Mov8();
                    }
                    _ => {
                        let invalid = InvalidOp::Reg(x.reg);
                        op.command = Op::Invalid(invalid);
                        // println!("op C6 unknown reg = {} at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                        //     x.reg,
                        //     self.sreg16[CS],
                        //     self.ip - 1,
                        //     self.get_offset() - 1,
                        //     self.instruction_count);
                    }
                }
            }
            0xC7 => {
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm16(self.read_u16());
                match x.reg {
                    0 => {
                        // mov r/m16, imm16
                        op.command = Op::Mov16();
                    }
                    _ => {
                        let invalid = InvalidOp::Reg(x.reg);
                        op.command = Op::Invalid(invalid);
                        // println!("op C7 unknown reg = {} at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                        //     x.reg,
                        //     self.sreg16[CS],
                        //     self.ip - 1,
                        //     self.get_offset() - 1,
                        //     self.instruction_count);
                    }
                }
            }
            // 0xC8 = "enter"
            // 0xC9 = "leave"
            // 0xCA = "retf imm16"
            0xCB => {
                op.command = Op::Retf();
            }
            0xCC => {
                op.command = Op::Int();
                op.params.dst = Parameter::Imm8(3);
            }
            0xCD => {
                // int imm8
                op.command = Op::Int();
                op.params.dst = Parameter::Imm8(self.read_u8());
            }
            // 0xCE = "into"
	        // 0xCF = "iretw"
            0xD0 => {
                // bit shift byte by 1
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    0 => Op::Rol8(),
                    1 => Op::Ror8(),
                    2 => Op::Rcl8(),
                    3 => Op::Rcr8(),
                    4 => Op::Shl8(),
                    5 => Op::Shr8(),
                    7 => Op::Sar8(),
                    _ => {
                        let invalid = InvalidOp::Op(vec![0xD0, x.reg]);
                        Op::Invalid(invalid)
                    }
                };
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm8(1);
            }
            0xD1 => {
                // bit shift word by 1
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    0 => Op::Rol16(),
                    1 => Op::Ror16(),
                    2 => Op::Rcl16(),
                    3 => Op::Rcr16(),
                    4 => Op::Shl16(),
                    5 => Op::Shr16(),
                    7 => Op::Sar16(),
                    _ => {
                        let invalid = InvalidOp::Op(vec![0xD1, x.reg]);
                        Op::Invalid(invalid)
                    }
                };
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm16(1);
            }
            0xD2 => {
                // bit shift byte by CL
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    0 => Op::Rol8(),
                    1 => Op::Ror8(),
                    2 => Op::Rcl8(),
                    3 => Op::Rcr8(),
                    4 => Op::Shl8(),
                    5 => Op::Shr8(),
                    7 => Op::Sar8(),
                    _ => {
                        let invalid = InvalidOp::Op(vec![0xD2, x.reg]);
                        Op::Invalid(invalid)
                    }
                };
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                op.params.src = Parameter::Reg8(CL);
            }
            0xD3 => {
                // bit shift word by CL
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    0 => Op::Rol16(),
                    1 => Op::Ror16(),
                    2 => Op::Rcl16(),
                    3 => Op::Rcr16(),
                    4 => Op::Shl16(),
                    5 => Op::Shr16(),
                    7 => Op::Sar16(),
                    _ => {
                        let invalid = InvalidOp::Op(vec![0xD3, x.reg]);
                        Op::Invalid(invalid)
                    }
                };
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Reg8(CL);
            }
            0xD4 => {
                op.command = Op::Aam();
                op.params.dst = Parameter::Imm8(self.read_u8());
            }
            0xD5 => {
                op.command = Op::Aad();
                op.params.dst = Parameter::Imm8(self.read_u8());
            }
            0xD6 => {
                // NOTE: 0xD6 unassigned in intel docs, but mapped to salc (aka 'setalc') in
                // http://ref.x86asm.net/coder32.html#gen_note_u_SALC_D6
                op.command = Op::Salc();
            }
            0xD7 => {
                op.command = Op::Xlatb();
            }
            /*
            0xD8 => { // fpu
                op.decodeD8(data)
            }
            0xD9 => { // fpu
                op.decodeD9(data)
            }
            0xDA => { // fpu
                op.decodeDA(data)
            }
            0xDB => { // fpu
                op.decodeDB(data)
            }
            0xDC => { // fpu
                op.decodeDC(data)
            }
            0xDD => { // fpu
                op.decodeDD(data)
            }
            0xDE => { // fpu
                op.decodeDE(data)
            }
            0xDF => { // fpu
                op.decodeDF(data)
            }
            */
            0xE0 => {
                op.command = Op::Loopne();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0xE1 => {
                op.command = Op::Loope();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0xE2 => {
                op.command = Op::Loop();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0xE3 => {
                // jcxz rel8
                op.command = Op::Jcxz();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0xE4 => {
                // in AL, imm8
                op.command = Op::In8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0xE5 => {
                // in AX, imm8
                op.command = Op::In16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0xE6 => {
                // OUT imm8, AL
                op.command = Op::Out8();
                op.params.dst = Parameter::Imm8(self.read_u8());
                op.params.src = Parameter::Reg8(AL);
            }
            0xE7 => {
                // OUT imm8, AX
                op.command = Op::Out16();
                op.params.dst = Parameter::Imm8(self.read_u8());
                op.params.src = Parameter::Reg16(AX);
            }
            0xE8 => {
                // call near s16
                op.command = Op::CallNear();
                op.params.dst = Parameter::Imm16(self.read_rel16());
            }
            0xE9 => {
                // jmp near rel16
                op.command = Op::JmpNear();
                op.params.dst = Parameter::Imm16(self.read_rel16());
            }
            0xEA => {
                // jmp far ptr16:16
                op.command = Op::JmpFar();
                op.params.dst = Parameter::Ptr16Imm(self.read_u16(), self.read_u16());
            }
            0xEB => {
                // jmp short rel8
                op.command = Op::JmpShort();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0xEC => {
                // in AL, DX
                op.command = Op::In8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Reg16(DX);
            }
            0xED => {
                // in AX, DX
                op.command = Op::In16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Reg16(DX);
            }
            0xEE => {
                // out DX, AL
                op.command = Op::Out8();
                op.params.dst = Parameter::Reg16(DX);
                op.params.src = Parameter::Reg8(AL);
            }
            0xEF => {
                // out DX, AX
                op.command = Op::Out16();
                op.params.dst = Parameter::Reg16(DX);
                op.params.src = Parameter::Reg16(AX);
            }
            // 0xF0 = "lock" prefix
            0xF1 => {
                op.command = Op::Int();
                op.params.dst = Parameter::Imm8(1);
            }
            0xF2 => {
                // repne  (alias repnz)
                let b = self.read_u8();
                match b {
                    0xAE => {
                        // repne scas byte
                        op.command = Op::RepneScasb();
                    }
                    _ => {
                        let invalid = InvalidOp::Op(vec![0xF2, b]);
                        op.command = Op::Invalid(invalid);
                        // println!("op F2 error: unhandled op {:02X}", b);
                    }
                }
            }
            0xF3 => {
                // rep
                let b = self.read_u8();
                match b {
                    0x6E => {
                        op.command = Op::RepOutsb();
                    }
                    0xA4 => {
                        op.command = Op::RepMovsb();
                    }
                    0xA5 => {
                        op.command = Op::RepMovsw();
                    }
                    0xAA => {
                        op.command = Op::RepStosb();
                    }
                    0xAB => {
                        op.command = Op::RepStosw();
                    }
                    _ => {
                        let invalid = InvalidOp::Op(vec![0xF3, b]);
                        op.command = Op::Invalid(invalid);
                        //println!("op F3 error: unhandled op {:02X}", b);
                    }
                }
            }
            0xF4 => {
                op.command = Op::Hlt();
            }
            0xF5 => {
                op.command = Op::Cmc();
            }
            0xF6 => {
                // byte sized math
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                match x.reg {
                    0 | 1 => {
                        // test r/m8, imm8
                        op.command = Op::Test8();
                        op.params.src = Parameter::Imm8(self.read_u8());
                    }
                    2 => {
                        // not r/m8
                        op.command = Op::Not8();
                    }
                    3 => {
                        // neg r/m8
                        op.command = Op::Neg8();
                    }
                    4 => {
                        // mul r/m8
                        op.command = Op::Mul8();
                    }
                    5 => {
                        // imul r/m8
                        op.command = Op::Imul8();
                    }
                    6 => {
                        // div r/m8
                        op.command = Op::Div8();
                    }
                    7 => {
                        // idiv r/m8
                        op.command = Op::Idiv8();
                    }
                    _ => {
                        let invalid = InvalidOp::Reg(x.reg);
                        op.command = Op::Invalid(invalid);
                        //println!("op F6 unknown reg={}", x.reg);
                    }
                }
            }
            0xF7 => {
                // word sized math
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                match x.reg {
                    0 | 1 => {
                        // test r/m16, imm16
                        op.command = Op::Test16();
                        op.params.src = Parameter::Imm16(self.read_u16());
                    }
                    2 => {
                        // not r/m16
                        op.command = Op::Not16();
                    }
                    3 => {
                        // neg r/m16
                        op.command = Op::Neg16();
                    }
                    4 => {
                        // mul r/m16
                        op.command = Op::Mul16();
                    }
                    5 => {
                        // imul r/m16
                        op.command = Op::Imul16();
                    }
                    6 => {
                        // div r/m16
                        op.command = Op::Div16();
                    }
                    7 => {
                        // idiv r/m16
                        op.command = Op::Idiv16();
                    }
                    _ => {
                        let invalid = InvalidOp::Reg(x.reg);
                        op.command = Op::Invalid(invalid);
                        // println!("op F7 unknown reg={} at {:04X}:{:04X}",
                        //          x.reg,
                        //          self.sreg16[CS],
                        //          self.ip);
                    }
                }
            }
            0xF8 => {
                // clc
                op.command = Op::Clc();
            }
            0xF9 => {
                // stc
                op.command = Op::Stc();
            }
            0xFA => {
                // cli
                op.command = Op::Cli();
            }
            0xFB => {
                // sti
                op.command = Op::Sti();
            }
            0xFC => {
                // cld
                op.command = Op::Cld();
            }
            0xFD => {
                // std
                op.command = Op::Std();
            }
            0xFE => {
                // byte size
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                match x.reg {
                    0 | 2 => {
                        // NOTE: 2 is a old encoding, example user:
                        // https://www.pouet.net/prod.php?which=65203
                        // 00000140  FEC5              inc ch
                        op.command = Op::Inc8();
                    }
                    1 => {
                        op.command = Op::Dec8();
                    }
                    _ => {
                        let invalid = InvalidOp::Reg(x.reg);
                        op.command = Op::Invalid(invalid);
                        // println!("op FE, unknown reg {}: at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                        //     x.reg,
                        //     self.sreg16[CS],
                        //     self.ip - 1,
                        //     self.get_offset() - 1,
                        //     self.instruction_count);
                    }
                }
            }
            0xFF => {
                // word size
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                match x.reg {
                    0 => {
                        // inc r/m16
                        op.command = Op::Inc16();
                    }
                    1 => {
                        // dec r/m16
                        op.command = Op::Dec16();
                    }
                    2 => {
                        // call r/m16
                        op.command = Op::CallNear();
                    }
                    // 3 => call far
                    4 => {
                        // jmp r/m16
                        op.command = Op::JmpNear();
                    }
                    // 5 => jmp far
                    6 => {
                        // push r/m16
                        op.command = Op::Push16();
                    }
                    _ => {
                        let invalid = InvalidOp::Reg(x.reg);
                        op.command = Op::Invalid(invalid);
                        // println!("op FF, unknown reg {}: at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                        //     x.reg,
                        //     self.sreg16[CS],
                        //     self.ip - 1,
                        //     self.get_offset() - 1,
                        //     self.instruction_count);
                    }
                }
            }
            _ => {
                let invalid = InvalidOp::Op(vec![b]);
                op.command = Op::Invalid(invalid);
                // println!("decode_instruction: unknown op {:02X} at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                //          b,
                //          self.sreg16[CS],
                //          self.ip - 1,
                //          self.get_offset() - 1,
                //          self.instruction_count);
            }
        }

        //calculate instruction length
        op.byte_length = self.c_offset - ioffset;
        op
    }

    // decode rm8
    fn rm8(&mut self, seg: Segment, rm: u8, md: u8) -> Parameter {
        match md {
            0 => {
                if rm == 6 {
                    // [u16]
                    Parameter::Ptr8(seg, self.read_u16())
                } else {
                    // [amode]
                    Parameter::Ptr8Amode(seg, rm as usize)
                }
            }
            // [amode+s8]
            1 => Parameter::Ptr8AmodeS8(seg, rm as usize, self.read_s8()),
            // [amode+s16]
            2 => Parameter::Ptr8AmodeS16(seg, rm as usize, self.read_s16()),
            // [reg]
            _ => Parameter::Reg8(rm as usize),
        }
    }

    // decode rm16
    fn rm16(&mut self, seg: Segment, rm: u8, md: u8) -> Parameter {
        match md {
            0 => {
                if rm == 6 {
                    // [u16]
                    Parameter::Ptr16(seg, self.read_u16())
                } else {
                    // [amode]
                    Parameter::Ptr16Amode(seg, rm as usize)
                }
            }
            // [amode+s8]
            1 => Parameter::Ptr16AmodeS8(seg, rm as usize, self.read_s8()),
            // [amode+s16]
            2 => Parameter::Ptr16AmodeS16(seg, rm as usize, self.read_s16()),
            // [reg]
            _ => Parameter::Reg16(rm as usize),
        }
    }

    // decode r8, r/m8
    fn r8_rm8(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: Parameter::Reg8(x.reg as usize),
            src: self.rm8(seg, x.rm, x.md),
            src2: Parameter::None(),
        }
    }

    // decode r/m8, r8
    fn rm8_r8(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: self.rm8(seg, x.rm, x.md),
            src: Parameter::Reg8(x.reg as usize),
            src2: Parameter::None(),
        }
    }

    // decode Sreg, r/m16
    fn sreg_rm16(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: Parameter::SReg16(x.reg as usize),
            src: self.rm16(seg, x.rm, x.md),
            src2: Parameter::None(),
        }
    }

    // decode r/m16, Sreg
    fn rm16_sreg(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: self.rm16(seg, x.rm, x.md),
            src: Parameter::SReg16(x.reg as usize),
            src2: Parameter::None(),
        }
    }

    // decode r16, r/m8 (movzx)
    fn r16_rm8(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: Parameter::Reg16(x.reg as usize),
            src: self.rm8(seg, x.rm, x.md),
            src2: Parameter::None(),
        }
    }

    // decode r16, r/m16
    fn r16_rm16(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: Parameter::Reg16(x.reg as usize),
            src: self.rm16(seg, x.rm, x.md),
            src2: Parameter::None(),
        }
    }

    // decode r/m16, r16
    fn rm16_r16(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: self.rm16(seg, x.rm, x.md),
            src: Parameter::Reg16(x.reg as usize),
            src2: Parameter::None(),
        }
    }

    // decode r16, m16
    fn r16_m16(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        if x.md == 3 {
            println!("r16_m16 error: invalid encoding, ip={:04X}", self.c_offset);
        }
        ParameterPair {
            dst: Parameter::Reg16(x.reg as usize),
            src: self.rm16(seg, x.rm, x.md),
            src2: Parameter::None(),
        }
    }

    fn read_mod_reg_rm(&mut self) -> ModRegRm {
        let b = self.mmu.read_u8(self.c_seg, self.c_offset);
        self.c_offset += 1;
        ModRegRm {
            md: b >> 6, // high 2 bits
            reg: (b >> 3) & 7, // mid 3 bits
            rm: b & 7, // low 3 bits
        }
    }

    fn read_rel8(&mut self) -> u16 {
        let val = self.read_s8();
        (self.c_offset as i16 + i16::from(val)) as u16
    }

    fn read_rel16(&mut self) -> u16 {
        let val = self.read_s16();
        (self.c_offset as i16 + val) as u16
    }

    fn read_u8(&mut self) -> u8 {
        let b = self.mmu.read_u8(self.c_seg, self.c_offset);
        self.c_offset += 1;
        b
    }

    fn read_s8(&mut self) -> i8 {
        self.read_u8() as i8
    }

    fn read_u16(&mut self) -> u16 {
        let lo = self.read_u8();
        let hi = self.read_u8();
        u16::from(hi) << 8 | u16::from(lo)
    }

    fn read_s16(&mut self) -> i16 {
        self.read_u16() as i16
    }
}
