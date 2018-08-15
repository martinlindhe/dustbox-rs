use std::{mem, u8};
use std::num::Wrapping;
use std::marker::PhantomData;

use cpu::flag::Flags;
use cpu::instruction::{Instruction, InstructionInfo, ModRegRm, RepeatMode};
use cpu::parameter::{Parameter, ParameterSet};
use cpu::op::{Op, Invalid};
use cpu::register::{R, AMode, RegisterSnapshot};
use cpu::decoder::Decoder;
use cpu::segment::Segment;
use memory::{MMU, MemoryAddress};
use interrupt;
use gpu::GPU;
use machine::Machine;
use hardware::Hardware;

#[cfg(test)]
#[path = "./interpreter_test.rs"]
mod interpreter_test;

#[derive(Debug)]
enum Exception {
    // http://wiki.osdev.org/Interrupt_Vector_Table
    DIV0 = 0,    // Divide by 0
    UD = 6,      // Invalid opcode (UD2)
    DF = 8,      // Double fault
    TS = 10,     // Invalid TSS
    NP = 11,     // Segment not present
    SS = 12,     // Stack-segment fault
    GP = 13,     // General protection fault
    PF = 14,     // Page fault
}

pub struct CPU {
    pub instruction_count: usize,
    pub cycle_count: usize,
    pub regs: RegisterSnapshot, // general purpose registers, segment registers, ip
    pub rom_base: u32,
    pub fatal_error: bool, // for debugging: signals to debugger we hit an error
    pub deterministic: bool, // for testing: toggles non-deterministic behaviour
    pub decoder: Decoder,
    pub clock_hz: usize,
}

impl CPU {
    pub fn default() -> Self {
        CPU {
            instruction_count: 0,
            cycle_count: 0,
            regs: RegisterSnapshot::default(),
            rom_base: 0,
            fatal_error: false,
            deterministic: false,
            decoder: Decoder::default(),
            clock_hz: 5_000_000, // Intel 8086: 0.330 MIPS at 5.000 MHz
        }
    }

    pub fn get_r8(&self, r: R) -> u8 {
        self.regs.get_r8(r)
    }

    pub fn set_r8(&mut self, r: R, val: u8) {
        self.regs.set_r8(r, val);
    }

    pub fn get_r16(&self, r: R) -> u16 {
        self.regs.get_r16(r)
    }

    pub fn set_r16(&mut self, r: R, val: u16) {
        self.regs.set_r16(r, val);
    }

    pub fn get_r32(&self, r: R) -> u32 {
        self.regs.get_r32(r)
    }

    pub fn set_r32(&mut self, r: R, val: u32) {
        self.regs.set_r32(r, val);
    }

    /// base address the rom was loaded to
    pub fn get_rom_base(&self) -> u32 {
        self.rom_base
    }

    pub fn execute(&mut self, mut hw: &mut Hardware, op: &Instruction) {
        let start_ip = self.regs.ip;
        self.regs.ip = (Wrapping(self.regs.ip) + Wrapping(u16::from(op.length))).0;
        self.instruction_count += 1;
        self.cycle_count += 1; // XXX temp hack; we pretend each instruction takes 8 cycles due to lack of timing
        match op.command {
            Op::Aaa => {
                let v = if self.get_r8(R::AL) > 0xf9 {
                    2
                 } else {
                    1
                };
                self.adjb(6, v);
            }
            Op::Aad => {
                // one parameter
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16; // read_parameter_value XXX add param that specify mmu
                let mut ax = u16::from(self.get_r8(R::AH)) * op1;
                ax += u16::from(self.get_r8(R::AL));
                let al = ax as u8;
                self.set_r8(R::AL, al);
                self.set_r8(R::AH, 0);
                // modification of flags A,C,O is undocumented
                self.regs.flags.carry = false;
                self.regs.flags.overflow = false;
                self.regs.flags.adjust = false;
                // The SF, ZF, and PF flags are set according to the resulting binary value in the AL register
                self.regs.flags.sign = al >= 0x80;
                self.regs.flags.zero = al == 0;
                self.regs.flags.set_parity(al as usize);
            }
            Op::Aam => {
                // tempAL ← AL;
                // AH ← tempAL / imm8; (* imm8 is set to 0AH for the AAM mnemonic *)
                // AL ← tempAL MOD imm8;
                let imm8 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u8;
                if imm8 == 0 {
                    return self.exception(&Exception::DIV0, 0);
                }
                let al = self.get_r8(R::AL);
                self.set_r8(R::AH, al / imm8);
                self.set_r8(R::AL, al % imm8);
                // modification of flags A,C,O is undocumented
                self.regs.flags.carry = false;
                self.regs.flags.overflow = false;
                self.regs.flags.adjust = false;
                // The SF, ZF, and PF flags are set according to the resulting binary value in the AL register
                self.regs.flags.sign = al & 0x80 != 0; // XXX
                self.regs.flags.zero = al == 0;
                self.regs.flags.set_parity(al as usize);
            }
            Op::Aas => {
                let v = if self.get_r8(R::AL) < 6 {
                    -2
                } else {
                    -1
                };
                self.adjb(-6, v);
            }
            Op::Adc8 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let carry = if self.regs.flags.carry { 1 } else { 0 };
                let res = (Wrapping(dst) + Wrapping(src) + Wrapping(carry)).0;
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, (res & 0xFF) as u8);

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.regs.flags.set_overflow_add_u8(res, src + carry, dst);
                self.regs.flags.set_sign_u8(res);
                self.regs.flags.set_zero_u8(res);
                self.regs.flags.set_adjust(res, src + carry, dst);
                self.regs.flags.set_carry_u8(res);
                self.regs.flags.set_parity(res);
            }
            Op::Adc16 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let carry = if self.regs.flags.carry { 1 } else { 0 };
                let res = (Wrapping(dst) + Wrapping(src) + Wrapping(carry)).0;
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, (res & 0xFFFF) as u16);

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.regs.flags.set_overflow_add_u16(res, src + carry, dst);
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_adjust(res, src + carry, dst);
                self.regs.flags.set_carry_u16(res);
                self.regs.flags.set_parity(res);
            }
            Op::Add8 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src) as u8;
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst) as u8;
                let res = src as usize + dst as usize;
                self.regs.flags.set_carry_u8(res);
                self.regs.flags.set_parity(res);
                self.regs.flags.set_adjust(res, src as usize, dst as usize);
                self.regs.flags.set_zero_u8(res);
                self.regs.flags.set_sign_u8(res);
                self.regs.flags.set_overflow_add_u8(res, src as usize, dst as usize);
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res as u8);
            }
            Op::Add16 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src) as u16;
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                let res = src as usize + dst as usize;
                self.regs.flags.set_carry_u16(res);
                self.regs.flags.set_parity(res);
                self.regs.flags.set_adjust(res, src as usize, dst as usize);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_overflow_add_u16(res, src as usize, dst as usize);
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Add32 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src) as u32;
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst) as u32;
                let res = src as usize + dst as usize;
                self.regs.flags.set_carry_u32(res);
                self.regs.flags.set_parity(res);
                self.regs.flags.set_adjust(res, src as usize, dst as usize);
                self.regs.flags.set_zero_u32(res);
                self.regs.flags.set_sign_u32(res);
                self.regs.flags.set_overflow_add_u32(res, src as usize, dst as usize);
                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u32);
            }
            Op::And8 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = dst & src;

                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.regs.flags.overflow = false;
                self.regs.flags.carry = false;
                self.regs.flags.set_sign_u8(res);
                self.regs.flags.set_zero_u8(res);
                self.regs.flags.set_parity(res);
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res as u8);
            }
            Op::And16 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = dst & src;

                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.regs.flags.overflow = false;
                self.regs.flags.carry = false;
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_parity(res);
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Arpl => {
                println!("XXX impl {}", op);
                /*
                // NOTE: RPL is the low two bits of the address
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let mut dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                if dst & 3 < src & 3 {
                    self.regs.flags.zero = true;
                    dst = (dst & 0xFFFC) + (src & 3);
                    self.write_parameter_u16(&mut hw.mmu, op.segment, &op.params.dst, (dst & 0xFFFF) as u16);
                } else {
                    self.regs.flags.zero = false;
                }
                */
            }
            Op::Bsf => {
                let mut src = self.read_parameter_value(&hw.mmu, &op.params.src);
                if src == 0 {
                    self.regs.flags.zero = true;
                } else {
                    let mut count = 0;
                    while src & 1 == 0 {
                        count += 1;
                        src >>= 1;
                    }
                    self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, count);
                    self.regs.flags.zero = false;
                }
            }
            Op::Bt => {
                let bit_base = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let bit_offset = self.read_parameter_value(&hw.mmu, &op.params.src);
                self.regs.flags.carry = bit_base & (1 << (bit_offset & 15)) != 0;
            }
            Op::Bound => {
                // XXX throw BR exception if out of bounds
                println!("XXX impl {}", op);
            }
            Op::CallNear => {
                let old_ip = self.regs.ip;
                let temp_ip = self.read_parameter_value(&hw.mmu, &op.params.dst);
                self.push16(&mut hw.mmu, old_ip);
                self.regs.ip = temp_ip as u16;
            }
            Op::CallFar => {
                let old_seg = self.regs.get_r16(R::CS);
                let old_ip = self.regs.ip;
                self.push16(&mut hw.mmu, old_seg);
                self.push16(&mut hw.mmu, old_ip);
                match op.params.dst {
                    Parameter::Ptr16Imm(seg, offs) => {
                        self.regs.ip = offs;
                        self.regs.set_r16(R::CS, seg);
                    }
                    Parameter::Ptr16(seg, offs) => {
                        let seg = self.segment(seg);
                        self.set_r16(R::CS, seg);
                        self.regs.ip = offs;
                    }
                    Parameter::Ptr16AmodeS8(seg, ref amode, imm) => {
                        let seg = self.segment(seg);
                        self.set_r16(R::CS, seg);
                        self.regs.ip = (self.amode(amode) as isize + imm as isize) as u16;
                    }
                    _ => panic!("CallFar unhandled type {:?}", op.params.dst),
                }
            }
            Op::Cbw => {
                let ah = if self.get_r8(R::AL) & 0x80 != 0 {
                    0xFF
                } else {
                    0x00
                };
                self.set_r8(R::AH, ah);
            }
            Op::Clc => {
                self.regs.flags.carry = false;
            }
            Op::Cld => {
                self.regs.flags.direction = false;
            }
            Op::Cli => {
                self.regs.flags.interrupt = false;
            }
            Op::Cmc => {
                self.regs.flags.carry = !self.regs.flags.carry;
            }
            Op::Cmp8 => {
                // two parameters
                // Modify status flags in the same manner as the SUB instruction
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                self.cmp8(dst, src);
            }
            Op::Cmp16 => {
                // two parameters
                // Modify status flags in the same manner as the SUB instruction
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                self.cmp16(dst, src);
            }
            Op::Cmp32 => {
                // two parameters
                // Modify status flags in the same manner as the SUB instruction
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                self.cmp32(dst, src);
            }
            Op::Cmpsw => {
                // no parameters
                // Compare word at address DS:(E)SI with word at address ES:(E)DI
                // The DS segment may be overridden with a segment override prefix, but the ES segment cannot be overridden.
                let src = hw.mmu.read_u16(self.segment(op.segment_prefix), self.get_r16(R::SI)) as usize;
                let dst = hw.mmu.read_u16(self.get_r16(R::ES), self.get_r16(R::DI)) as usize;
                self.cmp16(dst, src);

                let si = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::SI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.get_r16(R::SI)) - Wrapping(2)).0
                };
                self.set_r16(R::SI, si);
                let di = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::DI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.get_r16(R::DI)) - Wrapping(2)).0
                };
                self.set_r16(R::DI, di);
            }
            Op::Cwd16 => {
                // DX:AX ← sign-extend of AX.
                let dx = if self.get_r16(R::AX) & 0x8000 != 0 {
                    0xFFFF
                } else {
                    0
                };
                self.set_r16(R::DX, dx);
            }
            Op::Cwde32 => {
                // EAX ← sign-extend of AX.
                let ax = self.get_r16(R::AX) as i16;
                self.set_r32(R::EAX, ax as u32);
            }
            Op::Daa => {
                self.adj4(6, 0x60);
            }
            Op::Das => {
                self.adj4(-6, -0x60);
            }
            Op::Dec8 => {
                // single parameter (dst)
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.regs.flags.set_overflow_sub_u8(res, src, dst);
                self.regs.flags.set_sign_u8(res);
                self.regs.flags.set_zero_u8(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);

                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res as u8);
            }
            Op::Dec16 => {
                // single parameter (dst)
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.regs.flags.set_overflow_sub_u16(res, src, dst);
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);

                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Dec32 => {
                // single parameter (dst)
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.regs.flags.set_overflow_sub_u32(res, src, dst);
                self.regs.flags.set_sign_u32(res);
                self.regs.flags.set_zero_u32(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);

                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u32);
            }
            Op::Div8 => {
                // Unsigned divide AX by r/m8, with result stored in AL ← Quotient, AH ← Remainder.
                let ax = self.get_r16(R::AX) as u16;
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                if op1 == 0 {
                    return self.exception(&Exception::DIV0, 0);
                }
                let quotient = ax / op1;
                let remainder = (ax % op1) as u8;
                let quo8 = (quotient & 0xFF) as u8;
                if quotient > 0xFF {
                    return self.exception(&Exception::DIV0, 0);
                }
                self.set_r8(R::AH, remainder);
                self.set_r8(R::AL, quo8);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Div16 => {
                // Unsigned divide DX:AX by r/m16, with result stored in AX ← Quotient, DX ← Remainder.
                let num = (u32::from(self.get_r16(R::DX)) << 16) + u32::from(self.get_r16(R::AX)); // DX:AX
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u32;
                if op1 == 0 {
                    return self.exception(&Exception::DIV0, 0);
                }
                let remainder = (num % op1) as u16;
                let quotient = num / op1;
                let quo16 = (quotient & 0xFFFF) as u16;
                if quotient != u32::from(quo16) {
                    return self.exception(&Exception::DIV0, 0);
                }
                self.set_r16(R::DX, remainder);
                self.set_r16(R::AX, quo16);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Div32 => {
                // Unsigned divide EDX:EAX by r/m32, with result stored in EAX ← Quotient, EDX ← Remainder.
                let num = (u64::from(self.get_r32(R::EDX)) << 32) + u64::from(self.get_r32(R::EAX)); // EDX:EAX
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u64;
                if op1 == 0 {
                    return self.exception(&Exception::DIV0, 0);
                }
                let remainder = (num % op1) as u32;
                let quotient = num / op1;
                let quo32 = (quotient & 0xFFFF) as u32;
                if quotient != u64::from(quo32) {
                    return self.exception(&Exception::DIV0, 0);
                }
                self.set_r32(R::EDX, remainder);
                self.set_r32(R::EAX, quo32);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Enter => {
                // Make Stack Frame for Procedure Parameters
                // Create a stack frame with optional nested pointers for a procedure.
                // XXX test this
                let alloc_size = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                let mut nesting_level = self.read_parameter_value(&hw.mmu, &op.params.src);

                nesting_level &= 0x1F; // XXX "mod 32" says docs
                let bp = self.get_r16(R::BP);
                self.push16(&mut hw.mmu, bp);
                let frame_temp = self.get_r16(R::SP);

                if nesting_level != 0 {
                    for i in 0..nesting_level {
                        let bp = self.get_r16(R::BP) - 2;
                        self.set_r16(R::BP, bp);
                        let val = hw.mmu.read_u16(self.get_r16(R::SS), self.get_r16(R::BP));
                        println!("XXX ENTER: pushing {} = {:04X}", i, val);
                        self.push16(&mut hw.mmu, val);
                    }
                    self.push16(&mut hw.mmu, frame_temp);
                }

                self.set_r16(R::BP, frame_temp);
                let sp = self.get_r16(R::SP) - alloc_size;
                self.set_r16(R::SP, sp);
            }
            Op::Hlt => {
                // println!("XXX impl {}", op);
                // self.fatal_error = true;
            }
            Op::Idiv8 => {
                let ax = self.get_r16(R::AX) as i16; // dividend
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as i8;
                if op1 == 0 {
                    return self.exception(&Exception::DIV0, 0);
                }
                let rem = (ax % i16::from(op1)) as i8;
                let quo = ax / i16::from(op1);
                let quo8s = (quo & 0xFF) as i8;
                if quo != i16::from(quo8s) {
                    return self.exception(&Exception::DIV0, 0);
                }
                self.set_r8(R::AL, quo as u8);
                self.set_r8(R::AH, rem as u8);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Idiv16 => {
                let dividend = ((u32::from(self.get_r16(R::DX)) << 16) | u32::from(self.get_r16(R::AX))) as i32; // DX:AX
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as i16;
                if op1 == 0 {
                    return self.exception(&Exception::DIV0, 0);
                }
                let quo = dividend / i32::from(op1);
                let rem = (dividend % i32::from(op1)) as i16;
                let quo16s = quo as i16;
	            if quo != i32::from(quo16s) {
                    return self.exception(&Exception::DIV0, 0);
                }
                self.set_r16(R::AX, quo16s as u16);
                self.set_r16(R::DX, rem as u16);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Idiv32 => {
                let dividend = ((u64::from(self.get_r32(R::EDX)) << 32) | u64::from(self.get_r32(R::EAX))) as i64; // EDX:EAX
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as i32;
                if op1 == 0 {
                    return self.exception(&Exception::DIV0, 0);
                }
                let quo = dividend / i64::from(op1);
                let rem = (dividend % i64::from(op1)) as i32;
                let quo32s = quo as i32;
	            if quo != i64::from(quo32s) {
                    return self.exception(&Exception::DIV0, 0);
                }
                self.set_r32(R::EAX, quo32s as u32);
                self.set_r32(R::EDX, rem as u32);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Imul8 => {
                // NOTE: only 1-parameter imul8 instruction exists
                // IMUL r/m8               : AX← AL ∗ r/m byte.
                let f1 = self.get_r8(R::AL) as i8;
                let f2 = self.read_parameter_value(&hw.mmu, &op.params.dst) as i8;
                let ax = (i16::from(f1) * i16::from(f2)) as u16; // product
                self.set_r16(R::AX, ax);

                // For the one operand form of the instruction, the CF and OF flags are set when significant
                // bits are carried into the upper half of the result and cleared when the result fits
                // exactly in the lower half of the result.
                if (ax & 0xFF80) == 0xFF80 || (ax & 0xFF80) == 0x0000 {
                    self.regs.flags.carry = false;
                    self.regs.flags.overflow = false;
                } else {
                    self.regs.flags.carry = true;
                    self.regs.flags.overflow = true;
                }
            }
            Op::Imul16 => {
                match op.params.count() {
                    1 => {
                        // IMUL r/m16               : DX:AX ← AX ∗ r/m word.
                        let a = self.read_parameter_value(&hw.mmu, &op.params.dst) as i16;
                        let tmp = (self.get_r16(R::AX) as i16) as isize * a as isize;
                        self.set_r16(R::AX, tmp as u16);
                        self.set_r16(R::DX, (tmp >> 16) as u16);
                    }
                    2 => {
                        // IMUL r16, r/m16          : word register ← word register ∗ r/m16.
                        let a = self.read_parameter_value(&hw.mmu, &op.params.dst);
                        let b = self.read_parameter_value(&hw.mmu, &op.params.src);
                        let tmp = a as isize * b as isize;
                        self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, tmp as u16);
                    }
                    3 => {
                        // IMUL r16, r/m16, imm8    : word register ← r/m16 ∗ sign-extended immediate byte.
                        // IMUL r16, r/m16, imm16   : word register ← r/m16 ∗ immediate word.
                        let a = self.read_parameter_value(&hw.mmu, &op.params.src);
                        let b = self.read_parameter_value(&hw.mmu, &op.params.src2);
                        let tmp = b as isize * a as isize;
                        self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, tmp as u16);
                    }
                    _ => unreachable!(),
                }

                // XXX flags
                // Flags Affected
                // For the one operand form of the instruction, the CF and OF flags are set when significant bits are carried
                // into the upper half of the result and cleared when the result fits exactly in the lower half of the result.
                // For the two- and three-operand forms of the instruction, the CF and OF flags are set when the result must be
                // truncated to fit in the destination operand size and cleared when the result fits exactly in the destination
                // operand size. The SF, ZF, AF, and PF flags are undefined.
            }
            Op::Imul32 => {
                match op.params.count() {
                    1 => {
                        // IMUL r/m32               : EDX:EAX ← EAX ∗ r/m32.
                        let a = self.read_parameter_value(&hw.mmu, &op.params.dst) as i32;
                        let tmp = (self.get_r32(R::EAX) as i32) as isize * a as isize;
                        self.set_r32(R::EAX, tmp as u32);
                        self.set_r32(R::EDX, (tmp >> 32) as u32);
                    }
                    2 => {
                        // IMUL r32, r/m32          : doubleword register ← doubleword register ∗ r/m32.
                        let a = self.read_parameter_value(&hw.mmu, &op.params.dst);
                        let b = self.read_parameter_value(&hw.mmu, &op.params.src);
                        let tmp = a as isize * b as isize;
                        self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, tmp as u32);
                    }
                    3 => {
                        // IMUL r32, r/m32, imm8     : doubleword register ← r/m32 ∗ sign- extended immediate byte.
                        // IMUL r32, r/m32, imm32    : doubleword register ← r/m32 ∗ immediate doubleword.
                        let a = self.read_parameter_value(&hw.mmu, &op.params.src);
                        let b = self.read_parameter_value(&hw.mmu, &op.params.src2);
                        let tmp = b as isize * a as isize;
                        self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, tmp as u32);
                    }
                    _ => unreachable!(),
                }
                // XXX flags
            }
            Op::In8 => {
                // two parameters (dst=AL)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let data = hw.in_u8(src as u16);
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, data);
            }
            Op::In16 => {
                // two parameters (dst=AX)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let data = hw.in_u16(src as u16);
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Inc8 => {
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.regs.flags.set_overflow_add_u8(res, src, dst);
                self.regs.flags.set_sign_u8(res);
                self.regs.flags.set_zero_u8(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);

                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res as u8);
            }
            Op::Inc16 => {
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.regs.flags.set_overflow_add_u16(res, src, dst);
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);

                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Inc32 => {
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.regs.flags.set_overflow_add_u32(res, src, dst);
                self.regs.flags.set_sign_u32(res);
                self.regs.flags.set_zero_u32(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);

                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u32);
            }
            Op::Insb => {
                // Input byte from I/O port specified in DX into memory location specified in ES:DI.
                // The ES segment cannot be overridden with a segment override prefix.
                let dx = self.get_r16(R::DX);
                let data = hw.in_u8(dx);
                hw.mmu.write_u8(self.get_r16(R::ES), self.get_r16(R::DI), data);
                let di = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::DI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.get_r16(R::DI)) - Wrapping(1)).0
                };
                self.set_r16(R::DI, di);
            }
            Op::Int => {
                let int = self.read_parameter_imm(&op.params.dst);
                self.int(&mut hw, int as u8);
            }
            Op::Ja => {
                if !self.regs.flags.carry & !self.regs.flags.zero {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jc => {
                if self.regs.flags.carry {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jcxz => {
                if self.get_r16(R::CX) == 0 {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jg => {
                if !self.regs.flags.zero & self.regs.flags.sign == self.regs.flags.overflow {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jl => {
                if self.regs.flags.sign != self.regs.flags.overflow {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::JmpFar => {
                match op.params.dst {
                    Parameter::Ptr16Imm(seg, imm) => {
                        self.set_r16(R::CS, seg);
                        self.regs.ip = imm;
                    }
                    Parameter::Ptr16Amode(seg, ref amode) => {
                        let seg = self.segment(seg);
                        self.set_r16(R::CS, seg);
                        self.regs.ip = self.amode(amode) as u16;
                    }
                    Parameter::Ptr16AmodeS8(seg, ref amode, imm) => {
                        let seg = self.segment(seg);
                        self.set_r16(R::CS, seg);
                        self.regs.ip = (self.amode(amode) as isize + imm as isize) as u16;
                    }
                    _ => panic!("jmp far with unexpected type {:?}", op.params.dst),
                }
            }
            Op::JmpNear | Op::JmpShort => {
                self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
            }
            Op::Jna => {
                if self.regs.flags.carry | self.regs.flags.zero {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jnc => {
                if !self.regs.flags.carry {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jng => {
                if self.regs.flags.zero | self.regs.flags.sign != self.regs.flags.overflow {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jnl => {
                if self.regs.flags.sign == self.regs.flags.overflow {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jno => {
                if !self.regs.flags.overflow {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jns => {
                if !self.regs.flags.sign {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jnz => {
                if !self.regs.flags.zero {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jo => {
                if self.regs.flags.overflow {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jpe => {
                if self.regs.flags.parity {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jpo => {
                 if !self.regs.flags.parity {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Js => {
                if self.regs.flags.sign {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jz => {
                if self.regs.flags.zero {
                    self.regs.ip = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                }
            }
            Op::Lahf => {
                // Load: AH ← EFLAGS(SF:ZF:0:AF:0:PF:1:CF).
                let mut val = 0 as u8;
                if self.regs.flags.carry {
                    val |= 1;
                }
                val |= 1 << 1;
                if self.regs.flags.parity {
                    val |= 1 << 2;
                }
                if self.regs.flags.adjust {
                    val |= 1 << 4;
                }
                if self.regs.flags.zero {
                    val |= 1 << 6;
                }
                if self.regs.flags.sign {
                    val |= 1 << 7;
                }
                self.set_r8(R::AH, val);
            }
            Op::Lds => {
                // Load DS:r16 with far pointer from memory.
                let (segment, offset) = self.read_segment_selector(&hw.mmu, &op.params.src);
                self.set_r16(R::DS, segment);
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, offset);
            }
            Op::Lea16 => {
                let src = self.read_parameter_address(&op.params.src) as u16;
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, src);
            }
            Op::Leave => {
                // High Level Procedure Exit
                // Set SP to BP, then pop BP.
                // XXX test this
                let bp = self.get_r16(R::BP);
                self.set_r16(R::SP, bp);
                let bp = self.pop16(&mut hw.mmu);
                self.set_r16(R::BP, bp);
            }
            Op::Les => {
                // Load ES:r16 with far pointer from memory.
                let (segment, offset) = self.read_segment_selector(&hw.mmu, &op.params.src);
                self.set_r16(R::ES, segment);
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, offset);
            }
            Op::Lodsb => {
                // no arguments
                // Load byte at address DS:(E)SI into AL.
                // The DS segment may be over-ridden with a segment override prefix.
                let val = hw.mmu.read_u8(self.segment(op.segment_prefix), self.get_r16(R::SI));

                self.set_r8(R::AL, val);
                let si = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::SI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.get_r16(R::SI)) - Wrapping(1)).0
                };
                self.set_r16(R::SI, si);
            }
            Op::Lodsw => {
                // no arguments
                // Load word at address DS:(E)SI into AX.
                // The DS segment may be over-ridden with a segment override prefix.
                let val = hw.mmu.read_u16(self.segment(op.segment_prefix), self.get_r16(R::SI));

                self.set_r16(R::AX, val);
                let si = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::SI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.get_r16(R::SI)) - Wrapping(2)).0
                };
                self.set_r16(R::SI, si);
            }
            Op::Lodsd => {
                // no arguments
                // Load dword at address DS:(E)SI into EAX.
                // The DS segment may be over-ridden with a segment override prefix.
                let val = hw.mmu.read_u32(self.segment(op.segment_prefix), self.get_r16(R::SI));

                self.set_r32(R::EAX, val);
                let si = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::SI)) + Wrapping(4)).0
                } else {
                    (Wrapping(self.get_r16(R::SI)) - Wrapping(4)).0
                };
                self.set_r16(R::SI, si);
            }
            Op::Loop => {
                // Decrement count; jump short if count ≠ 0.
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                let cx = (Wrapping(self.get_r16(R::CX)) - Wrapping(1)).0;
                self.set_r16(R::CX, cx);
                if cx != 0 {
                    self.regs.ip = dst;
                }
            }
            Op::Loope => {
                // Decrement count; jump short if count ≠ 0 and ZF = 1.
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                let cx = (Wrapping(self.get_r16(R::CX)) - Wrapping(1)).0;
                self.set_r16(R::CX, cx);
                if cx != 0 && self.regs.flags.zero {
                    self.regs.ip = dst;
                }
            }
            Op::Loopne => {
                // Decrement count; jump short if count ≠ 0 and ZF = 0.
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                let cx = (Wrapping(self.get_r16(R::CX)) - Wrapping(1)).0;
                self.set_r16(R::CX, cx);
                if cx != 0 && !self.regs.flags.zero {
                    self.regs.ip = dst;
                }
            } 
            Op::Mov8 => {
                // two arguments (dst=reg)
                let data = self.read_parameter_value(&hw.mmu, &op.params.src) as u8;
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, data);
            }
            Op::Mov16 => {
                // two arguments (dst=reg)
                let data = self.read_parameter_value(&hw.mmu, &op.params.src) as u16;
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Mov32 => {
                // two arguments (dst=reg)
                let data = self.read_parameter_value(&hw.mmu, &op.params.src) as u32;
                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Movsb => {
                // move byte from address DS:(E)SI to ES:(E)DI.
                // The DS segment may be overridden with a segment override prefix, but the ES segment cannot be overridden.
                let val = hw.mmu.read_u8(self.segment(op.segment_prefix), self.get_r16(R::SI));
                let si = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::SI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.get_r16(R::SI)) - Wrapping(1)).0
                };
                self.set_r16(R::SI, si);
                let es = self.get_r16(R::ES);
                let di = self.get_r16(R::DI);
                hw.mmu.write_u8(es, di, val);
                let di = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::DI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.get_r16(R::DI)) - Wrapping(1)).0
                };
                self.set_r16(R::DI, di);
            }
            Op::Movsw => {
                // move word from address DS:(E)SI to ES:(E)DI.
                // The DS segment may be overridden with a segment override prefix, but the ES segment cannot be overridden.
                let val = hw.mmu.read_u16(self.segment(op.segment_prefix), self.get_r16(R::SI));
                let si = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::SI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.get_r16(R::SI)) - Wrapping(2)).0
                };
                self.set_r16(R::SI, si);
                let es = self.get_r16(R::ES);
                let di = self.get_r16(R::DI);
                hw.mmu.write_u16(es, di, val);
                let di = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::DI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.get_r16(R::DI)) - Wrapping(2)).0
                };
                self.set_r16(R::DI, di);
            }
            Op::Movsd => {
                // move dword from address DS:(E)SI to ES:(E)DI
                // The DS segment may be overridden with a segment override prefix, but the ES segment cannot be overridden.
                let val = hw.mmu.read_u32(self.segment(op.segment_prefix), self.get_r16(R::SI));
                let si = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::SI)) + Wrapping(4)).0
                } else {
                    (Wrapping(self.get_r16(R::SI)) - Wrapping(4)).0
                };
                self.set_r16(R::SI, si);
                let es = self.get_r16(R::ES);
                let di = self.get_r16(R::DI);
                hw.mmu.write_u32(es, di, val);
                let di = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::DI)) + Wrapping(4)).0
                } else {
                    (Wrapping(self.get_r16(R::DI)) - Wrapping(4)).0
                };
                self.set_r16(R::DI, di);
            }
            Op::Movsx16 => {
                // 80386+
                // moves a signed value into a register and sign-extends it with 1.
                // two arguments (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src) as u8;

                let mut data = u16::from(src);
                // XXX should not work identical as Movzx16
                if src & 0x80 != 0 {
                    data += 0xFF00;
                }
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Movsx32 => {
                // 80386+
                // moves a signed value into a register and sign-extends it with 1.
                // two arguments (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src) as u8;

                let mut data = u32::from(src);
                // XXX should not work identical as Movzx16
                if src & 0x80 != 0 {
                    data += 0xFFFF_FF00;
                }
                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Movzx16 => {
                // 80386+
                // moves an unsigned value into a register and zero-extends it with zero.
                // two arguments (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src) as u8;
                let mut data = u16::from(src);
                if src & 0x80 != 0 {
                    data += 0xFF00;
                }
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Movzx32 => {
                // 80386+
                // moves an unsigned value into a register and zero-extends it with zero.
                // two arguments (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src) as u8;
                let mut data = u32::from(src);
                if src & 0x80 != 0 {
                    data += 0xFFFF_FF00;
                }
                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Mul8 => {
                // Unsigned multiply (AX ← AL ∗ r/m8).
                let al = self.get_r8(R::AL) as usize;
                let arg1 = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let ax = (Wrapping(al) * Wrapping(arg1)).0 as u16;
                self.set_r16(R::AX, ax);
                // The OF and CF flags are set to 0 if the upper half of the
                // result is 0; otherwise, they are set to 1.
                // The SF, ZF, AF, and PF flags are undefined.
                if ax & 0xFF00 != 0 {
                    self.regs.flags.carry = true;
                    self.regs.flags.overflow = true;
                } else {
                    self.regs.flags.carry = false;
                    self.regs.flags.overflow = false;
                }
            }
            Op::Mul16 => {
                // Unsigned multiply (DX:AX ← AX ∗ r/m16).
                let src = self.get_r16(R::AX) as usize;
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = (Wrapping(dst) * Wrapping(src)).0;

                self.set_r16(R::AX, res as u16);
                let dx = (res >> 16) as u16;
                self.set_r16(R::DX, dx);

                self.regs.flags.carry = dx != 0;
                self.regs.flags.overflow = dx != 0;
            }
            Op::Mul32 => {
                // Unsigned multiply (EDX:EAX ← EAX ∗ r/m32)
                let src = self.get_r32(R::EAX) as usize;
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = (Wrapping(dst) * Wrapping(src)).0;

                self.set_r32(R::EAX, res as u32);
                let edx = (res >> 32) as u32;
                self.set_r32(R::EDX, edx);

                self.regs.flags.carry = edx != 0;
                self.regs.flags.overflow = edx != 0;
            }
            Op::Neg8 => {
                // Two's Complement Negation
                // one argument
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let src = 0;
                let res = (Wrapping(src) - Wrapping(dst)).0;
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res as u8);

                self.regs.flags.carry = dst != 0;
                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.regs.flags.overflow = res == 0x80;
                self.regs.flags.set_sign_u8(res);
                self.regs.flags.set_zero_u8(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);
            }
            Op::Neg16 => {
                // one argument
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let src = 0;
                let res = (Wrapping(src) - Wrapping(dst)).0;
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);

                self.regs.flags.carry = dst != 0;
                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.regs.flags.overflow = res == 0x8000;
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);
            }
            Op::Neg32 => {
                // one argument
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let src = 0;
                let res = (Wrapping(src) - Wrapping(dst)).0;
                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u32);

                self.regs.flags.carry = dst != 0;
                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.regs.flags.overflow = res == 0x8000_0000;
                self.regs.flags.set_sign_u32(res);
                self.regs.flags.set_zero_u32(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);
            }
            Op::Nop => {}
            Op::Not8 => {
                // one arguments (dst)
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = !dst;
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, (res & 0xFF) as u8);
                // Flags Affected: None
            }
            Op::Not16 => {
                // one arguments (dst)
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = !dst;
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, (res & 0xFFFF) as u16);
                // Flags Affected: None
            }
            Op::Or8 => {
                // two arguments (dst=AL)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = dst | src;
                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.regs.flags.overflow = false;
                self.regs.flags.carry = false;
                self.regs.flags.set_sign_u8(res);
                self.regs.flags.set_zero_u8(res);
                self.regs.flags.set_parity(res);
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, (res & 0xFF) as u8);
            }
            Op::Or16 => {
                // two arguments (dst=AX)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = dst | src;
                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.regs.flags.overflow = false;
                self.regs.flags.carry = false;
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_parity(res);
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, (res & 0xFFFF) as u16);
            }
            Op::Out8 => {
                // two arguments
                let addr = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                let val = self.read_parameter_value(&hw.mmu, &op.params.src) as u8;
                hw.out_u8(addr, val);
            }
            Op::Out16 => {
                // two arguments
                let addr = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                let val = self.read_parameter_value(&hw.mmu, &op.params.src) as u16;
                hw.out_u16(addr, val);
            }
            Op::Outsb => {
                // Output byte from memory location specified in DS:(E)SI or RSI to I/O port specified in DX.
                // no arguments
                let val = hw.mmu.read_u8(self.segment(op.segment_prefix), self.get_r16(R::SI));
                let port = self.get_r16(R::DX);
                hw.out_u8(port, val);
                let si = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::SI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.get_r16(R::SI)) - Wrapping(1)).0
                };
                self.set_r16(R::SI, si);
            }
            Op::Outsw => {
                // Output word from memory location specified in DS:(E)SI or RSI to I/O port specified in DX**.
                // no arguments
                let val = hw.mmu.read_u16(self.segment(op.segment_prefix), self.get_r16(R::SI));
                let port = self.get_r16(R::DX);
                hw.out_u16(port, val);
                let si = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::SI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.get_r16(R::SI)) - Wrapping(2)).0
                };
                self.set_r16(R::SI, si);
            }
            Op::Pop16 => {
                // one arguments (dst)
                let data = self.pop16(&mut hw.mmu);
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Pop32 => {
                // one arguments (dst)
                let data = self.pop32(&mut hw.mmu);
                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Popa16 => {
                let di = self.pop16(&mut hw.mmu);
                self.set_r16(R::DI, di);
                let si = self.pop16(&mut hw.mmu);
                self.set_r16(R::SI, si);
                let bp = self.pop16(&mut hw.mmu);
                self.set_r16(R::BP, bp);
                let sp = self.get_r16(R::SP) + 2; // skip next word of stack
                self.set_r16(R::SP, sp);
                let bx = self.pop16(&mut hw.mmu);
                self.set_r16(R::BX, bx);
                let dx = self.pop16(&mut hw.mmu);
                self.set_r16(R::DX, dx);
                let cx = self.pop16(&mut hw.mmu);
                self.set_r16(R::CX, cx);
                let ax = self.pop16(&mut hw.mmu);
                self.set_r16(R::AX, ax);
            }
            Op::Popad32 => {
                let edi = self.pop32(&mut hw.mmu);
                self.set_r32(R::EDI, edi);
                let esi = self.pop32(&mut hw.mmu);
                self.set_r32(R::ESI, esi);
                let ebp = self.pop32(&mut hw.mmu);
                self.set_r32(R::EBP, ebp);
                let esp = self.get_r32(R::ESP) + 4; // skip next dword of stack
                self.set_r32(R::ESP, esp);
                let ebx = self.pop32(&mut hw.mmu);
                self.set_r32(R::EBX, ebx);
                let edx = self.pop32(&mut hw.mmu);
                self.set_r32(R::EDX, edx);
                let ecx = self.pop32(&mut hw.mmu);
                self.set_r32(R::ECX, ecx);
                let eax = self.pop32(&mut hw.mmu);
                self.set_r32(R::EAX, eax);
            }
            Op::Popf => {
                let data = self.pop16(&mut hw.mmu);
                self.regs.flags.set_u16(data);
            }
            Op::Push16 => {
                // single parameter (dst)
                let data = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                self.push16(&mut hw.mmu, data);
            }
            Op::Push32 => {
                // single parameter (dst)
                let data = self.read_parameter_value(&hw.mmu, &op.params.dst) as u32;
                self.push32(&mut hw.mmu, data);
            }
            Op::Pusha16 => {
                let ax = self.get_r16(R::AX);
                let cx = self.get_r16(R::CX);
                let dx = self.get_r16(R::DX);
                let bx = self.get_r16(R::BX);
                let sp = self.get_r16(R::SP);
                let bp = self.get_r16(R::BP);
                let si = self.get_r16(R::SI);
                let di = self.get_r16(R::DI);
                self.push16(&mut hw.mmu, ax);
                self.push16(&mut hw.mmu, cx);
                self.push16(&mut hw.mmu, dx);
                self.push16(&mut hw.mmu, bx);
                self.push16(&mut hw.mmu, sp);
                self.push16(&mut hw.mmu, bp);
                self.push16(&mut hw.mmu, si);
                self.push16(&mut hw.mmu, di);
            }
            Op::Pushad32 => {
                let eax = self.get_r32(R::EAX);
                let ecx = self.get_r32(R::ECX);
                let edx = self.get_r32(R::EDX);
                let ebx = self.get_r32(R::EBX);
                let esp = self.get_r32(R::ESP);
                let ebp = self.get_r32(R::EBP);
                let esi = self.get_r32(R::ESI);
                let edi = self.get_r32(R::EDI);
                self.push32(&mut hw.mmu, eax);
                self.push32(&mut hw.mmu, ecx);
                self.push32(&mut hw.mmu, edx);
                self.push32(&mut hw.mmu, ebx);
                self.push32(&mut hw.mmu, esp);
                self.push32(&mut hw.mmu, ebp);
                self.push32(&mut hw.mmu, esi);
                self.push32(&mut hw.mmu, edi);
            }
            Op::Pushf => {
                let data = self.regs.flags.u16();
                self.push16(&mut hw.mmu, data);
            }
            Op::Rcl8 => {
                // Rotate 9 bits (CF, r/m8) left imm8 times.
                // two arguments
                let mut count = (self.read_parameter_value(&hw.mmu, &op.params.src) & 0x1F) % 9;
                if count > 0 {
                    let cf = self.regs.flags.carry_val() as u16;
                    let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                    let res = if count == 1 {
                        ((op1 << 1) | cf)
                    } else {
                        ((op1 << count) | (cf << (count - 1)) | (op1 >> (9 - count)))
                    } as u8;
                    self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res);
                    self.regs.flags.carry = (op1 >> (8 - count)) & 1 != 0;
                    // For left rotates, the OF flag is set to the exclusive OR of the CF bit
                    // (after the rotate) and the most-significant bit of the result.
                    self.regs.flags.overflow = self.regs.flags.carry_val() as u16 ^ (u16::from(res) >> 7) != 0;
                }
            }
            Op::Rcl16 => {
                // Rotate 9 bits (CF, r/m8) left imm8 times.
                // two arguments
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                let count = (self.read_parameter_value(&hw.mmu, &op.params.src) & 0x1F) % 17;
                if count > 0 {
                    let cf = self.regs.flags.carry_val() as u16;
                    let res = if count == 1 {
                        (op1 << 1) | cf
                    } else if count == 16 {
                        (cf << 15) | (op1 >> 1)
                    } else {
                        (op1 << count) | (cf << (count - 1)) | (op1 >> (17 - count))
                    };
                    self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
                    self.regs.flags.carry = (op1 >> (16 - count)) & 1 != 0;
                    self.regs.flags.overflow = self.regs.flags.carry_val() as u16 ^ (op1 >> 15) != 0;
                }
            }
            Op::Rcr8 => {
                // two arguments
                // rotate 9 bits right `op1` times
                let mut count = self.read_parameter_value(&hw.mmu, &op.params.src) as u16/* & 0x1F*/;
                if count % 9 != 0 {
                    count %= 9;
                    let cf = self.regs.flags.carry_val() as u16;
                    let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                    let res = (op1 >> count | (cf << (8 - count)) | (op1 << (9 - count))) as u8;
                    self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res);
                    self.regs.flags.carry = (op1 >> (count - 1)) & 1 != 0;
                    // The OF flag is set to the exclusive OR of the two most-significant bits of the result.
                    self.regs.flags.overflow = (res ^ (res << 1)) & 0x80 != 0; // dosbox
                    //self.regs.flags.overflow = (((res << 1) ^ res) >> 7) & 0x1 != 0; // bochs. of = result6 ^ result7
                }
            }
            Op::Rcr16 => {
                // two arguments
                // rotate 9 bits right `op1` times
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let count = (self.read_parameter_value(&hw.mmu, &op.params.src) as u32 & 0x1F) % 17;
                if count > 0 {
                    let cf = self.regs.flags.carry_val();
                    let res = (op1 >> count) | (cf << (16 - count)) | (op1 << (17 - count));
                    self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
                    self.regs.flags.carry = (op1 >> (count - 1)) & 1 != 0;
                    let bit15 = (res >> 15) & 1;
                    let bit14 = (res >> 14) & 1;
                    self.regs.flags.overflow = bit15 ^ bit14 != 0;
                }
            }
            Op::Rcr32 => {
                // two arguments
                // rotate 9 bits right `op1` times
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let count = (self.read_parameter_value(&hw.mmu, &op.params.src) as u32 & 0x1F) % 17;    // XXX
                if count > 0 {
                    let cf = self.regs.flags.carry_val();
                    let res = (op1 >> count) | (cf << (32 - count)) | (op1 << (33 - count));
                    self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u32);
                    self.regs.flags.carry = (op1 >> (count - 1)) & 1 != 0;
                    let bit15 = (res >> 15) & 1; // XXX
                    let bit14 = (res >> 14) & 1;
                    self.regs.flags.overflow = bit15 ^ bit14 != 0;
                }
            }
            Op::Iret => {
                self.regs.ip = self.pop16(&mut hw.mmu);
                let cs = self.pop16(&mut hw.mmu);
                self.set_r16(R::CS, cs);
                let flags = self.pop16(&mut hw.mmu);
                self.regs.flags.set_u16(flags);
                hw.bios.flags_address = MemoryAddress::Unset;
            }
            Op::Retf => {
                if op.params.count() == 1 {
                    // 1 argument: pop imm16 bytes from stack
                    let imm16 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                    let sp = self.get_r16(R::SP) + imm16;
                    self.set_r16(R::SP, sp);
                }
                self.regs.ip = self.pop16(&mut hw.mmu);
                let cs = self.pop16(&mut hw.mmu);
                self.set_r16(R::CS, cs);
            }
            Op::Retn => {
                self.regs.ip = self.pop16(&mut hw.mmu);
                if op.params.count() == 1 {
                    // 1 argument: pop imm16 bytes from stack
                    let imm16 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                    let sp = self.get_r16(R::SP) + imm16;
                    self.set_r16(R::SP, sp);
                }
            }
            Op::Rol8 => {
                // Rotate 8 bits of 'dst' left for 'src' times.
                // two arguments: op1, count
                let mut op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u8;
                let mut count = self.read_parameter_value(&hw.mmu, &op.params.src);
                if count & 0b0_0111 == 0 {
                    if count & 0b1_1000 != 0 {
                        let bit0 = op1 & 1;
                        let bit7 = op1 >> 7;
                        self.regs.flags.overflow = bit0 ^ bit7 != 0;
                        self.regs.flags.carry = bit0 != 0;
                    }
                    // no-op if count is 0
                    return;
                }
                count &= 0x7;
                let res = (op1 << count) | (op1 >> (8 - count));
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res);
                let bit0 = res & 1;
                let bit7 = res >> 7;
                self.regs.flags.overflow = bit0 ^ bit7 != 0;
                self.regs.flags.carry = bit0 != 0;
            }
            Op::Rol16 => {
                // Rotate 16 bits of 'dst' left for 'src' times.
                // two arguments
                let mut res = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                let count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0x1F;
                res = res.rotate_left(count as u32);
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res);
                let bit0 = res & 1;
                let bit15 = (res >> 15) & 1;
                if count == 1 {
                    self.regs.flags.overflow = bit0 ^ bit15 != 0;
                }
                self.regs.flags.carry = bit0 != 0;
            }
            Op::Ror8 => {
                // Rotate 8 bits of 'dst' right for 'src' times.
                // two arguments
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u8;
                let count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0x1F;

                if count & 0b0_0111 == 0 {
                    if count & 0b1_1000 != 0 {
                        let bit6 = (op1 >> 6) & 1;
                        let bit7 = op1 >> 7;
                        self.regs.flags.overflow = bit6 ^ bit7 != 0;
                        self.regs.flags.carry = bit7 != 0;
                    }
                    return;
                }

                let res = op1.rotate_right(count as u32);
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res);
                let bit6 = (res >> 6) & 1;
                let bit7 = res >> 7;
                self.regs.flags.overflow = bit6 ^ bit7 != 0;
                self.regs.flags.carry = bit7 != 0;
            }
            Op::Ror16 => {
                // Rotate 16 bits of 'dst' right for 'src' times.
                // two arguments
                let mut res = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                let mut count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0x1F;
                res = res.rotate_right(count as u32);
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res);
                let bit14 = (res >> 14) & 1;
                let bit15 = (res >> 15) & 1;
                if count == 1 {
                    self.regs.flags.overflow = bit14 ^ bit15 != 0;
                }
                self.regs.flags.carry = bit15 != 0;
            }
            Op::Sahf => {
                // Loads the SF, ZF, AF, PF, and CF flags of the EFLAGS register with values
                // from the corresponding bits in the AH register (bits 7, 6, 4, 2, and 0, respectively).
                let ah = self.get_r8(R::AH);
                self.regs.flags.carry = ah & 0x1 != 0; // bit 0
                self.regs.flags.parity = ah & 0x4 != 0; // bit 2
                self.regs.flags.adjust = ah & 0x10 != 0; // bit 4
                self.regs.flags.zero = ah & 0x40 != 0; // bit 6
                self.regs.flags.sign = ah & 0x80 != 0; // bit 7
            }
            Op::Salc => {
                let al = if self.regs.flags.carry {
                    0xFF
                } else {
                    0
                };
                self.set_r8(R::AL, al);
            }
            Op::Sar8 => {
                // Signed divide r/m8 by 2, imm8 times.
                // two arguments
                let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u8;
                let mut count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0x1F;
                if count > 0 {
                    if count > 8 {
                        count = 8;
                    }

                    let res = if op1 & 0x80 != 0 {
                        ((op1 as usize) >> count) | (0xFF << (8 - count))
                    } else {
                        ((op1 as usize) >> count)
                    };
                    
                    self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res as u8);
                    self.regs.flags.carry = (op1 as isize >> (count - 1)) & 0x1 != 0;
                    self.regs.flags.overflow = false;
                    self.regs.flags.set_sign_u8(res as usize);
                    self.regs.flags.set_zero_u8(res as usize);
                    self.regs.flags.set_parity(res as usize);
                }
            }
            Op::Sar16 => {
                // Signed divide r/m8 by 2, imm8 times.
                // two arguments
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0xF;
                if count > 0 {
                    let res = if dst & 0x8000 != 0 {
                        let x = 0xFFFF as usize;
                        dst.rotate_right(count as u32) | x.rotate_left(16 - count as u32)
                    } else {
                        dst.rotate_right(count as u32)
                    };
                    self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
                    self.regs.flags.carry = (dst as u16 >> (count - 1)) & 0x1 != 0;
                    if count == 1 {
                        self.regs.flags.overflow = false;
                    }
                    self.regs.flags.set_sign_u16(res);
                    self.regs.flags.set_zero_u16(res);
                    self.regs.flags.set_parity(res);
                }
            }
            Op::Sar32 => {
                // Signed divide r/m8 by 2, imm8 times.
                // two arguments
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0xF; // XXX
                if count > 0 {
                    let res = if dst & 0x8000_0000 != 0 {
                        let x = 0xFFFF_FFFF as usize;
                        dst.rotate_right(count as u32) | x.rotate_left(32 - count as u32)
                    } else {
                        dst.rotate_right(count as u32)
                    };
                    self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u32);
                    self.regs.flags.carry = (dst as u32 >> (count - 1)) & 0x1 != 0; // XXX
                    if count == 1 {
                        self.regs.flags.overflow = false;
                    }
                    self.regs.flags.set_sign_u32(res);
                    self.regs.flags.set_zero_u32(res);
                    self.regs.flags.set_parity(res);
                }
            }
            Op::Sbb8 => {
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let cf = if self.regs.flags.carry { 1 } else { 0 };
                let res = (Wrapping(dst) - (Wrapping(src) + Wrapping(cf))).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.regs.flags.set_overflow_sub_u8(res, src, dst);
                self.regs.flags.set_sign_u8(res);
                self.regs.flags.set_zero_u8(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);
                self.regs.flags.set_carry_u8(res);

                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res as u8);
            }
            Op::Sbb16 => {
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let cf = if self.regs.flags.carry { 1 } else { 0 };
                let res = (Wrapping(dst) - (Wrapping(src) + Wrapping(cf))).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.regs.flags.set_overflow_sub_u16(res, src, dst);
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);
                self.regs.flags.set_carry_u16(res);

                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Scasb => {
                // Compare AL with byte at ES:(E)DI then set status flags.
                // ES cannot be overridden with a segment override prefix.
                let src = self.get_r8(R::AL);
                let dst = hw.mmu.read_u8(self.get_r16(R::ES), self.get_r16(R::DI));
                self.cmp8(dst as usize, src as usize);
                let di = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::DI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.get_r16(R::DI)) - Wrapping(1)).0
                };
                self.set_r16(R::DI, di);
            }
            Op::Scasw => {
                // Compare AX with word at ES:(E)DI or RDI then set status flags.
                // ES cannot be overridden with a segment override prefix.
                let src = self.get_r16(R::AX);
                let dst = hw.mmu.read_u16(self.get_r16(R::ES), self.get_r16(R::DI));
                self.cmp16(dst as usize, src as usize);
                let di = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::DI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.get_r16(R::DI)) - Wrapping(2)).0
                };
                self.set_r16(R::DI, di);
            }
            Op::Setc => {
                let val = if self.regs.flags.carry {
                    1
                } else {
                    0
                };
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, val);
            }
            Op::Setnz => {
                let val = if !self.regs.flags.zero {
                    1
                } else {
                    0
                };
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, val);
            }
            Op::Shl8 => {
                // Multiply `dst` by 2, `src` times.
                // two arguments    (alias: sal)
                let count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0b1_1111;
                // XXX differs from dosbox & winxp
                //if count > 0 {
                    let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                    let res = if count < 8 {
                        op1 << count
                    } else {
                        0
                    };
                    let cf = if count > 8 {
                        0
                    } else {
                        op1 >> (8 - count) & 0x1
                    };
                    self.regs.flags.carry = cf != 0;
                    //self.regs.flags.overflow = cf ^ (res >> 7) != 0; // bochs
                    //self.regs.flags.overflow = (op1 ^ res) & 0x80 != 0; // dosbox buggy
                    self.regs.flags.overflow = res >> 7 ^ cf as u16 != 0; // MSB of result XOR CF. WARNING: This only works because FLAGS_CF == 1
                    //self.regs.flags.overflow = ((op1 ^ res) >> (12 - 8)) & 0x800 != 0; // qemu
                    //self.regs.flags.adjust = count & 0x1F != 0; // XXX dosbox. AF not set in winxp
                    self.regs.flags.set_sign_u8(res as usize);
                    self.regs.flags.set_zero_u8(res as usize);
                    self.regs.flags.set_parity(res as usize);
                    self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res as u8);
                //}
            }
            Op::Shl16 => {
                // Multiply `dst` by 2, `src` times.
                // two arguments    (alias: sal)
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0x1F;
                if count > 0 {
                    let res = dst.wrapping_shl(count as u32);
                    self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
                    self.regs.flags.carry = (res & 0x8000) != 0;
                    if count == 1 {
                        self.regs.flags.overflow = self.regs.flags.carry_val() ^ ((res & 0x8000) >> 15) != 0;
                    }
                    self.regs.flags.set_sign_u16(res);
                    self.regs.flags.set_zero_u16(res);
                    self.regs.flags.set_parity(res);
                }
            }
            Op::Shl32 => {
                // Multiply `dst` by 2, `src` times.
                // two arguments    (alias: sal)
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0x1F; // XXX
                if count > 0 {
                    let res = dst.wrapping_shl(count as u32);
                    self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u32);
                    self.regs.flags.carry = (res & 0x8000_0000) != 0;
                    if count == 1 {
                        self.regs.flags.overflow = self.regs.flags.carry_val() ^ ((res & 0x8000) >> 15) != 0; // XXX
                    }
                    self.regs.flags.set_sign_u32(res);
                    self.regs.flags.set_zero_u32(res);
                    self.regs.flags.set_parity(res);
                }
            }
            Op::Shld => {
                // 3 arguments
                let count = self.read_parameter_value(&hw.mmu, &op.params.src2) & 0x1F;
                if count > 0 {
                    let op1 = self.read_parameter_value(&hw.mmu, &op.params.dst) as u16;
                    let op2 = self.read_parameter_value(&hw.mmu, &op.params.src) as u16;
                    // count < 32, since only lower 5 bits used
                    let temp_32 = (u32::from(op1) << 16) | u32::from(op2); // double formed by op1:op2
                    let mut result_32 = temp_32 << count;

                    // hack to act like x86 SHLD when count > 16
                    if count > 16 {
                        // for Pentium processor, when count > 16, actually shifting op1:op2:op2 << count,
                        // it is the same as shifting op2:op2 by count-16
                        // For P6 and later (CPU_LEVEL >= 6), when count > 16, actually shifting op1:op2:op1 << count,
                        // which is the same as shifting op2:op1 by count-16
                        // The behavior is undefined so both ways are correct, we prefer P6 way of implementation
                        result_32 |= u32::from(op1) << (count - 16);
                     }

                    let res16 = (result_32 >> 16) as u16;
                    self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res16);

                    let cf = (temp_32 >> (32 - count)) & 0x1;
                    self.regs.flags.carry = cf != 0;
                    self.regs.flags.overflow = cf ^ (u32::from(res16) >> 15) != 0;
                    self.regs.flags.set_zero_u16(res16 as usize);
                    self.regs.flags.set_sign_u16(res16 as usize);
                    self.regs.flags.set_adjust(res16 as usize, op1 as usize, op2 as usize);
                    self.regs.flags.set_parity(res16 as usize);
                }
            }
            Op::Shr8 => {
                // Unsigned divide r/m8 by 2, `src` times.
                // two arguments
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0x1F;
                if count > 0 {
                    let res = dst.wrapping_shr(count as u32);
                    self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res as u8);
                    self.regs.flags.carry = (dst.wrapping_shr((count - 1) as u32) & 0x1) != 0;
                    self.regs.flags.overflow = dst & 0x80 != 0;
                    self.regs.flags.set_sign_u8(res);
                    self.regs.flags.set_zero_u8(res);
                    self.regs.flags.set_parity(res);
                    /*
                    The CF flag contains the value of the last bit shifted out of the destination operand;
                    it is undefined for SHL and SHR instructions where the count is greater than or equal to the size (in bits) of the destination operand. 

                    The OF flag is affected only for 1-bit shifts (see “Description” above); otherwise, it is undefined. 
                    The SF, ZF, and PF flags are set according to the result. If the count is 0, the flags are not affected.
                    For a non-zero count, the AF flag is undefined.
                    */
                }
            }
            Op::Shr16 => {
                // two arguments
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0x1F;
                if count > 0 {
                    let res = dst.wrapping_shr(count as u32);
                    self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
                    self.regs.flags.carry = (dst.wrapping_shr((count - 1) as u32) & 0x1) != 0;
                    self.regs.flags.overflow = dst & 0x8000 != 0;
                    self.regs.flags.set_sign_u16(res);
                    self.regs.flags.set_zero_u16(res);
                    self.regs.flags.set_parity(res);
                }
            }
            Op::Shr32 => {
                // two arguments
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let count = self.read_parameter_value(&hw.mmu, &op.params.src) & 0x1F; // XXX
                if count > 0 {
                    let res = dst.wrapping_shr(count as u32);
                    self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u32);
                    self.regs.flags.carry = (dst.wrapping_shr((count - 1) as u32) & 0x1) != 0; // XXX
                    self.regs.flags.overflow = dst & 0x8000_0000 != 0;
                    self.regs.flags.set_sign_u32(res);
                    self.regs.flags.set_zero_u32(res);
                    self.regs.flags.set_parity(res);
                }
            }
            Op::Shrd => {
                // 3 arguments

                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let count = self.read_parameter_value(&hw.mmu, &op.params.src2);
                if count == 0 {
                    return;
                }
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);

                // Shift `dst` to right `count` places while shifting bits from `src` in from the left
                let res = (src & count_to_bitmask(count) as usize) << (16-count) | (dst >> count);

                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);

                if count >= 1 {
                    // XXX carry if count is >= 1

                    // If the count is 1 or greater, the CF flag is filled with the last bit shifted out
                    // of the destination operand

                    self.regs.flags.carry = (dst & 1) != 0; // XXX this would be the first bit.. which is wrong
                }

                // SF, ZF, and PF flags are set according to the value of the result.
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_parity(res);

                if count == 1 {
                    // XXX overflow if count == 1
                    // For a 1-bit shift, the OF flag is set if a sign change occurred; otherwise, it is cleared.
                    // For shifts greater than 1 bit, the OF flag is undefined. 
                }

                // If a shift occurs, the AF flag is undefined. If the count is greater than the operand size,
                // the flags are undefined.
            }
            Op::Sldt => {
                println!("XXX impl {:?}", op);
            }
            Op::Stc => {
                self.regs.flags.carry = true;
            }
            Op::Std => {
                self.regs.flags.direction = true;
            }
            Op::Sti => {
                self.regs.flags.interrupt = true;
            }
            Op::Stosb => {
                // no parameters
                // store AL at ES:(E)DI
                // The ES segment cannot be overridden with a segment override prefix.
                let al = self.get_r8(R::AL);
                let es = self.get_r16(R::ES);
                let di = self.get_r16(R::DI);
                hw.mmu.write_u8(es, di, al);
                let di = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::DI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.get_r16(R::DI)) - Wrapping(1)).0
                };
                self.set_r16(R::DI, di);
            }
            Op::Stosw => {
                // no parameters
                // store AX at address ES:(E)DI
                // The ES segment cannot be overridden with a segment override prefix.
                let ax = self.get_r16(R::AX);
                let es = self.get_r16(R::ES);
                let di = self.get_r16(R::DI);
                hw.mmu.write_u16(es, di, ax);
                let di = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::DI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.get_r16(R::DI)) - Wrapping(2)).0
                };
                self.set_r16(R::DI, di);
            }
            Op::Stosd => {
                // no parameters
                // store EAX at address ES:(E)DI
                // The ES segment cannot be overridden with a segment override prefix.
                let eax = self.get_r32(R::EAX);
                let es = self.get_r16(R::ES);
                let di = self.get_r16(R::DI);
                hw.mmu.write_u32(es, di, eax);
                // XXX adjust DI or EDI ?
                let di = if !self.regs.flags.direction {
                    (Wrapping(self.get_r16(R::DI)) + Wrapping(4)).0
                } else {
                    (Wrapping(self.get_r16(R::DI)) - Wrapping(4)).0
                };
                self.set_r16(R::DI, di);
            }
            Op::Sub8 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.regs.flags.set_overflow_sub_u8(res, src, dst);
                self.regs.flags.set_sign_u8(res);
                self.regs.flags.set_zero_u8(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);
                self.regs.flags.set_carry_u8(res);

                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res as u8);
            }
            Op::Sub16 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.regs.flags.set_overflow_sub_u16(res, src, dst);
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);
                self.regs.flags.set_carry_u16(res);

                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Sub32 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.regs.flags.set_overflow_sub_u32(res, src, dst);
                self.regs.flags.set_sign_u32(res);
                self.regs.flags.set_zero_u32(res);
                self.regs.flags.set_adjust(res, src, dst);
                self.regs.flags.set_parity(res);
                self.regs.flags.set_carry_u32(res);

                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u32);
            }
            Op::Test8 => {
                // two parameters
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = dst & src;
                // set SF, ZF, PF according to result.
                self.regs.flags.set_sign_u8(res);
                self.regs.flags.set_zero_u8(res);
                self.regs.flags.set_parity(res);
            }
            Op::Test16 => {
                // two parameters
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = dst & src;
                // set SF, ZF, PF according to result.
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_parity(res);
            }
            Op::Xchg8 => {
                // two parameters (registers)
                let mut src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let mut dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                mem::swap(&mut src, &mut dst);
                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, dst as u8);
                self.write_parameter_u8(&mut hw.mmu, &op.params.src, src as u8);
            }
            Op::Xchg16 => {
                // two parameters (registers)
                let mut src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let mut dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                mem::swap(&mut src, &mut dst);
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, dst as u16);
                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.src, src as u16);
            }
            Op::Xchg32 => {
                // two parameters (registers)
                let mut src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let mut dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                mem::swap(&mut src, &mut dst);
                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, dst as u32);
                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.src, src as u32);
            }
            Op::Xlatb => {
                // no parameters
                // Set AL to memory byte DS:[(E)BX + unsigned AL].
                // The DS segment may be overridden with a segment override prefix.
                let al = hw.mmu.read_u8(self.segment(op.segment_prefix), self.get_r16(R::BX) + u16::from(self.get_r8(R::AL)));
                self.set_r8(R::AL, al);
            }
            Op::Xor8 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = dst ^ src;

                // The OF and CF flags are cleared; the SF, ZF,
                // and PF flags are set according to the result.
                self.regs.flags.overflow = false;
                self.regs.flags.carry = false;
                self.regs.flags.set_sign_u8(res);
                self.regs.flags.set_zero_u8(res);
                self.regs.flags.set_parity(res);

                self.write_parameter_u8(&mut hw.mmu, &op.params.dst, res as u8);
            }
            Op::Xor16 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = dst ^ src;

                // The OF and CF flags are cleared; the SF, ZF,
                // and PF flags are set according to the result.
                self.regs.flags.overflow = false;
                self.regs.flags.carry = false;
                self.regs.flags.set_sign_u16(res);
                self.regs.flags.set_zero_u16(res);
                self.regs.flags.set_parity(res);

                self.write_parameter_u16(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Xor32 => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&hw.mmu, &op.params.src);
                let dst = self.read_parameter_value(&hw.mmu, &op.params.dst);
                let res = dst ^ src;

                // The OF and CF flags are cleared; the SF, ZF,
                // and PF flags are set according to the result.
                self.regs.flags.overflow = false;
                self.regs.flags.carry = false;
                self.regs.flags.set_sign_u32(res);
                self.regs.flags.set_zero_u32(res);
                self.regs.flags.set_parity(res);

                self.write_parameter_u32(&mut hw.mmu, op.segment_prefix, &op.params.dst, res as u32);
            }
            _ => {
                let (seg, off) = self.get_address_pair();
                println!("execute error: unhandled '{}' at {:04X}:{:04X} (flat {:06X})",
                         op,
                         seg,
                         off,
                         self.get_address());
            }
        }

        match op.repeat {
            RepeatMode::Rep => {
                let cx = (Wrapping(self.get_r16(R::CX)) - Wrapping(1)).0;
                self.set_r16(R::CX, cx);
                if cx != 0 {
                    self.regs.ip = start_ip;
                }
            }
            RepeatMode::Repe => {
                let cx = (Wrapping(self.get_r16(R::CX)) - Wrapping(1)).0;
                self.set_r16(R::CX, cx);
                if cx != 0 && self.regs.flags.zero {
                    self.regs.ip = start_ip;
                }
            }
            RepeatMode::Repne => {
                let cx = (Wrapping(self.get_r16(R::CX)) - Wrapping(1)).0;
                self.set_r16(R::CX, cx);
                if cx != 0 && !self.regs.flags.zero {
                    self.regs.ip = start_ip;
                }
            }
            RepeatMode::None => {}
        }

        if op.lock {
            // TODO implement lock
            // println!("XXX FIXME: instruction has LOCK prefix: {}", op);
        }
    }

    fn exception(&mut self, which: &Exception, error: usize) {
        /*
        #define CPU_INT_SOFTWARE    0x1
        #define CPU_INT_EXCEPTION   0x2
        #define CPU_INT_HAS_ERROR   0x4
        #define CPU_INT_NOIOPLCHECK 0x8
        */
        println!("Exception {:?}, error {}", which, error);

        // CPU_Interrupt(which,CPU_INT_EXCEPTION | ((which>=8) ? CPU_INT_HAS_ERROR : 0),reg_eip);
    }

    fn cmp8(&mut self, dst: usize, src: usize) {
        let res = (Wrapping(dst) - Wrapping(src)).0;

        // The CF, OF, SF, ZF, AF, and PF flags are set according to the result.
        self.regs.flags.set_carry_u8(res);
        self.regs.flags.set_overflow_sub_u8(res, src, dst);
        self.regs.flags.set_sign_u8(res);
        self.regs.flags.set_zero_u8(res);
        self.regs.flags.set_adjust(res, src, dst);
        self.regs.flags.set_parity(res);
    }

    fn cmp16(&mut self, dst: usize, src: usize) {
        let res = (Wrapping(dst) - Wrapping(src)).0;

        // The CF, OF, SF, ZF, AF, and PF flags are set according to the result.
        self.regs.flags.set_carry_u16(res);
        self.regs.flags.set_overflow_sub_u16(res, src, dst);
        self.regs.flags.set_sign_u16(res);
        self.regs.flags.set_zero_u16(res);
        self.regs.flags.set_adjust(res, src, dst);
        self.regs.flags.set_parity(res);
    }

    fn cmp32(&mut self, dst: usize, src: usize) {
        let res = (Wrapping(dst) - Wrapping(src)).0;

        // The CF, OF, SF, ZF, AF, and PF flags are set according to the result.
        self.regs.flags.set_carry_u32(res);
        self.regs.flags.set_overflow_sub_u32(res, src, dst);
        self.regs.flags.set_sign_u32(res);
        self.regs.flags.set_zero_u32(res);
        self.regs.flags.set_adjust(res, src, dst);
        self.regs.flags.set_parity(res);
    }

    fn push16(&mut self, mmu: &mut MMU, data: u16) {
        let sp = (Wrapping(self.get_r16(R::SP)) - Wrapping(2)).0;
        self.set_r16(R::SP, sp);
        let ss = self.get_r16(R::SS);
        mmu.write_u16(ss, sp, data);
    }

    fn push32(&mut self, mmu: &mut MMU, data: u32) {
        let sp = (Wrapping(self.get_r16(R::SP)) - Wrapping(4)).0;
        self.set_r16(R::SP, sp);
        let ss = self.get_r16(R::SS);
        mmu.write_u32(ss, sp, data);
    }

    fn pop16(&mut self, mmu: &mut MMU) -> u16 {
        let data = mmu.read_u16(self.get_r16(R::SS), self.get_r16(R::SP));
        let sp = (Wrapping(self.get_r16(R::SP)) + Wrapping(2)).0;
        self.set_r16(R::SP, sp);
        data
    }

    fn pop32(&mut self, mmu: &mut MMU) -> u32 {
        let data = mmu.read_u32(self.get_r16(R::SS), self.get_r16(R::SP));
        let sp = (Wrapping(self.get_r16(R::SP)) + Wrapping(4)).0;
        self.set_r16(R::SP, sp);
        data
    }

    /// returns the absoute address of CS:IP
    pub fn get_address(&self) -> u32 {
        MemoryAddress::RealSegmentOffset(self.get_r16(R::CS), self.regs.ip).value()
    }

    pub fn get_address_pair(&self) -> (u16, u16) {
        (self.get_r16(R::CS), self.regs.ip)
    }

    fn read_u8(&mut self, mmu: &MMU) -> u8 {
        let (seg, off) = self.get_address_pair();
        let b = mmu.read_u8(seg, off);
        self.regs.ip += 1;
        b
    }

    fn read_u16(&mut self, mmu: &MMU) -> u16 {
        let lo = self.read_u8(mmu);
        let hi = self.read_u8(mmu);
        u16::from(hi) << 8 | u16::from(lo)
    }

    fn read_s8(&mut self,  mmu: &MMU) -> i8 {
        self.read_u8(mmu) as i8
    }

    fn read_s16(&mut self, mmu: &MMU) -> i16 {
        self.read_u16(mmu) as i16
    }

    fn read_rel8(&mut self, mmu: &MMU) -> u16 {
        let val = self.read_u8(mmu) as i8;
        (self.regs.ip as i16 + i16::from(val)) as u16
    }

    fn read_rel16(&mut self, mmu: &MMU) -> u16 {
        let val = self.read_u16(mmu) as i16;
        (self.regs.ip as i16 + val) as u16
    }

    /// returns "segment, offset" pair
    fn get_amode_addr(&self, amode: &AMode) -> (u16, u16) {
        match *amode {
            AMode::BX => (self.get_r16(R::DS), self.get_r16(R::BX)),
            AMode::BP => (self.get_r16(R::SS), self.get_r16(R::BP)),
            AMode::SI => (self.get_r16(R::DS), self.get_r16(R::SI)),
            AMode::DI => (self.get_r16(R::DS), self.get_r16(R::DI)),
            AMode::BXSI => (self.get_r16(R::DS), self.get_r16(R::BX) + self.get_r16(R::SI)),
            AMode::BXDI => (self.get_r16(R::DS), self.get_r16(R::BX) + self.get_r16(R::DI)),
            AMode::BPSI => (self.get_r16(R::SS), self.get_r16(R::BP) + self.get_r16(R::SI)),
            AMode::BPDI => (self.get_r16(R::SS), self.get_r16(R::BP) + self.get_r16(R::DI)),
            _ => panic!("xxx"),
        }
    }

    /// used by lds, les
    fn read_segment_selector(&self, mmu: &MMU, p: &Parameter) -> (u16, u16) {
        let (segment, offset) = match *p {
            Parameter::Ptr16(seg, imm) => (self.segment(seg), imm),
            Parameter::Ptr16Amode(_, ref amode) => self.get_amode_addr(amode),
            Parameter::Ptr16AmodeS8(_, ref amode, imms) => {
                let (seg, off) = self.get_amode_addr(amode);
                (seg, (i32::from(off) + i32::from(imms)) as u16)
            }
            /*
            Parameter::Ptr16AmodeS16(_, ref amode, imms) => {
                let (seg, off) = self.get_amode_addr(amode);
                (seg, (i32::from(off) + i32::from(imms)) as u16)
            }
            */
            _ => panic!("unhandled parameter {:?}", p),
        };

        let o_val = mmu.read_u16(segment, offset);
        let s_val = mmu.read_u16(segment, offset + 2);
        (s_val, o_val)
    }

    /// returns the address of pointer, used by LEA
    fn read_parameter_address(&mut self, p: &Parameter) -> usize {
        match *p {
            Parameter::Ptr16Amode(_, ref amode) => self.amode(amode),
            Parameter::Ptr16AmodeS8(_, ref amode, imm) => (Wrapping(self.amode(amode)) + Wrapping(imm as usize)).0,
            Parameter::Ptr16AmodeS16(_, ref amode, imm) => (Wrapping(self.amode(amode)) + Wrapping(imm as usize)).0,
            Parameter::Ptr16(_, imm) => imm as usize,
            _ => panic!("unhandled parameter: {:?} at {:06X}", p, self.get_address()),
        }
    }

    fn read_parameter_imm(&self, p: &Parameter) -> usize {
        match *p {
            Parameter::Imm8(imm) => imm as usize,
            Parameter::Imm16(imm) => imm as usize,
            Parameter::ImmS8(imm) => imm as usize,
            _ => panic!("read_parameter_imm only allows imm-type params: {:?}", p),
        }
    }

    fn read_parameter_value(&mut self, mmu: &MMU, p: &Parameter) -> usize {
        match *p {
            Parameter::Imm8(imm) => imm as usize,
            Parameter::Imm16(imm) => imm as usize,
            Parameter::Imm32(imm) => imm as usize,
            Parameter::ImmS8(imm) => imm as usize,
            Parameter::Reg8(r) => self.get_r8(r) as usize,
            Parameter::Reg16(r) => self.get_r16(r) as usize,
            Parameter::Reg32(r) => self.get_r32(r) as usize,
            Parameter::SReg16(sr) => self.get_r16(sr) as usize,
            Parameter::Ptr8(seg, imm) => mmu.read_u8(self.segment(seg), imm) as usize,
            Parameter::Ptr8Amode(seg, ref amode) => {
                let seg = self.segment(seg);
                let offset = self.amode(amode) as u16;
                mmu.read_u8(seg, offset) as usize
            }
            Parameter::Ptr8AmodeS8(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = (Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16)).0;
                mmu.read_u8(seg, offset) as usize
            }
            Parameter::Ptr8AmodeS16(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = (Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16)).0;
                mmu.read_u8(seg, offset) as usize
            }
            Parameter::Ptr16(seg, imm) => mmu.read_u16(self.segment(seg), imm) as usize,
            Parameter::Ptr16Amode(seg, ref amode) => {
                let seg = self.segment(seg);
                let offset = self.amode(amode) as u16;
                mmu.read_u16(seg, offset) as usize
            }
            Parameter::Ptr16AmodeS8(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = (Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16)).0;
                mmu.read_u16(seg, offset) as usize
            }
            Parameter::Ptr16AmodeS16(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = (Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16)).0;
                mmu.read_u16(seg, offset) as usize
            }
            Parameter::Ptr32(seg, imm) => mmu.read_u32(self.segment(seg), imm) as usize,
            Parameter::Ptr32Amode(seg, ref amode) => {
                let seg = self.segment(seg);
                let offset = self.amode(amode) as u16;
                mmu.read_u32(seg, offset) as usize
            }
            Parameter::Ptr32AmodeS8(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = (Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16)).0;
                mmu.read_u32(seg, offset) as usize
            }
            _ => {
                let (seg, off) = self.get_address_pair();
                panic!("unhandled parameter: {:?} at {:04X}:{:04X} ({:06X} flat)", p, seg, off, self.get_address());
            },
        }
    }

    fn write_parameter_u8(&mut self, mmu: &mut MMU, p: &Parameter, data: u8) {
        match *p {
            Parameter::Reg8(r) => self.set_r8(r, data),
            Parameter::Ptr8(seg, imm) => {
                let seg = self.segment(seg);
                mmu.write_u8(seg, imm, data);
            }
            Parameter::Ptr8Amode(seg, ref amode) => {
                let seg = self.segment(seg);
                let offset = self.amode(amode) as u16;
                mmu.write_u8(seg, offset, data);
            }
            Parameter::Ptr8AmodeS8(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16);
                mmu.write_u8(seg, offset.0, data);
            }
            Parameter::Ptr8AmodeS16(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16);
                mmu.write_u8(seg, offset.0, data);
            }
            _ => panic!("write_parameter_u8 unhandled type {:?} at {:06X}", p, self.get_address()),
        }
    }

    fn write_parameter_u16(&mut self, mmu: &mut MMU, segment: Segment, p: &Parameter, data: u16) {
        match *p {
            Parameter::Reg16(r) |
            Parameter::SReg16(r) => self.set_r16(r, data),
            Parameter::Imm16(imm) => {
                let seg = self.segment(segment);
                mmu.write_u16(seg, imm, data);
            }
            Parameter::Ptr16(seg, imm) => {
                let seg = self.segment(seg);
                mmu.write_u16(seg, imm, data);
            }
            Parameter::Ptr16Amode(seg, ref amode) => {
                let seg = self.segment(seg);
                let offset = self.amode(amode) as u16;
                mmu.write_u16(seg, offset, data);
            }
            Parameter::Ptr16AmodeS8(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16);
                mmu.write_u16(seg, offset.0, data);
            }
            Parameter::Ptr16AmodeS16(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16);
                mmu.write_u16(seg, offset.0, data);
            }
            _ => panic!("unhandled type {:?} at {:06X}", p, self.get_address()),
        }
    }

    fn write_parameter_u32(&mut self, mmu: &mut MMU, _segment: Segment, p: &Parameter, data: u32) {
        match *p {
            Parameter::Reg32(r) => self.set_r32(r, data),
            Parameter::Ptr32(seg, imm) => {
                let seg = self.segment(seg);
                mmu.write_u32(seg, imm, data);
            }
            Parameter::Ptr32Amode(seg, ref amode) => {
                let seg = self.segment(seg);
                let offset = self.amode(amode);
                mmu.write_u32(seg, offset as u16, data);
            }
            Parameter::Ptr32AmodeS8(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16);
                mmu.write_u32(seg, offset.0, data);
            }
            Parameter::Ptr32AmodeS16(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16);
                mmu.write_u32(seg, offset.0, data);
            }
            _ => panic!("unhandled type {:?} at {:06X}", p, self.get_address()),
        }
    }

    /// returns the value of the given segment register
    fn segment(&self, seg: Segment) -> u16 {
        self.get_r16(seg.as_register())
    }

    fn amode(&self, amode: &AMode) -> usize {
        match *amode {
            AMode::BXSI => (Wrapping(self.get_r16(R::BX)) + Wrapping(self.get_r16(R::SI))).0 as usize,
            AMode::BXDI => (Wrapping(self.get_r16(R::BX)) + Wrapping(self.get_r16(R::DI))).0 as usize,
            AMode::BPSI => (Wrapping(self.get_r16(R::BP)) + Wrapping(self.get_r16(R::SI))).0 as usize,
            AMode::BPDI => (Wrapping(self.get_r16(R::BP)) + Wrapping(self.get_r16(R::DI))).0 as usize,
            AMode::SI => self.get_r16(R::SI) as usize,
            AMode::DI => self.get_r16(R::DI) as usize,
            AMode::BP => self.get_r16(R::BP) as usize,
            AMode::BX => self.get_r16(R::BX) as usize,

            AMode::EAX => self.get_r32(R::EAX) as usize,
            AMode::ECX => self.get_r32(R::ECX) as usize,
            AMode::EDX => self.get_r32(R::EDX) as usize,
            AMode::EBX => self.get_r32(R::EBX) as usize,
            AMode::ESP => self.get_r32(R::ESP) as usize,
            AMode::EBP => self.get_r32(R::EBP) as usize,
            AMode::ESI => self.get_r32(R::ESI) as usize,
            AMode::EDI => self.get_r32(R::EDI) as usize,
        }
    }

    /// used by aaa, aas
    fn adjb(&mut self, param1: i8, param2: i8) {
        if self.regs.flags.adjust || (self.get_r8(R::AL) & 0xf) > 9 {
            let al = (i16::from(self.get_r8(R::AL)) + i16::from(param1)) as u8;
            let ah = (i16::from(self.get_r8(R::AH)) + i16::from(param2)) as u8;
            self.set_r8(R::AL, al);
            self.set_r8(R::AH, ah);
            self.regs.flags.adjust = true;
            self.regs.flags.carry = true;
        } else {
            self.regs.flags.adjust = false;
            self.regs.flags.carry = false;
        }
        let al = self.get_r8(R::AL);
        self.set_r8(R::AL, al & 0x0F);
    }

    /// used by daa, das
    fn adj4(&mut self, param1: i16, param2: i16) {
        let mut al = self.get_r8(R::AL);
        if ((al & 0x0F) > 0x09) || self.regs.flags.adjust {
            if (al > 0x99) || self.regs.flags.carry {
                al = (i16::from(al) + param2) as u8;
                self.regs.flags.carry = true;
            } else {
                self.regs.flags.carry = false;
            }
            al = (i16::from(al) + param1) as u8;
            self.regs.flags.adjust = true;
        } else {
            if (al > 0x99) || self.regs.flags.carry {
                al = (i16::from(al) + param2) as u8;
                self.regs.flags.carry = true;
            } else {
                self.regs.flags.carry = false;
            }
            self.regs.flags.adjust = false;
        }
        self.set_r8(R::AL, al);
        self.regs.flags.sign = al & 0x80 != 0;
        self.regs.flags.zero = al == 0;
        self.regs.flags.set_parity(al as usize);
    }

    fn int(&mut self, hw: &mut Hardware, int: u8) {
        let flags = self.regs.flags.u16();
        self.push16(&mut hw.mmu, flags);
        hw.bios.flags_address = MemoryAddress::RealSegmentOffset(self.get_r16(R::SS), self.get_r16(R::SP));

        self.regs.flags.interrupt = false;
        self.regs.flags.trap = false;
        let (cs, ip) = self.get_address_pair();
        self.push16(&mut hw.mmu, cs);
        self.push16(&mut hw.mmu, ip);
        let base = 0;
        let idx = u16::from(int) << 2;
        let ip = hw.mmu.read_u16(base, idx);
        let cs = hw.mmu.read_u16(base, idx + 2);
        // println!("int: jumping to interrupt handler for interrupt {:02X} pos at {:04X}:{:04X} = {:04X}:{:04X}", int, base, idx, cs, ip);
        self.regs.ip = ip;
        self.set_r16(R::CS, cs);
    }

    pub fn handle_interrupt(&mut self, mut hw: &mut Hardware, int: u8) {
        match int {
            0x03 => {
                // debugger interrupt
                // http://www.ctyme.com/intr/int-03.htm
                println!("INT 3 - debugger interrupt. AX={:04X}", self.get_r16(R::AX));
                self.fatal_error = true; // stops execution
            }
            0x10 => interrupt::int10::handle(self, &mut hw),
            0x16 => interrupt::int16::handle(self, &mut hw),
            0x1A => interrupt::int1a::handle(self, &mut hw),
            0x20 => {
                // DOS 1+ - TERMINATE PROGRAM
                // NOTE: Windows overloads INT 20
                println!("INT 20 - Terminating program");
                self.fatal_error = true; // stops execution
            }
            0x21 => interrupt::int21::handle(self, &mut hw),
            0x33 => interrupt::int33::handle(self, &mut hw),
            _ => {
                println!("int error: unknown interrupt {:02X}, AX={:04X}, BX={:04X}",
                        int,
                        self.get_r16(R::AX),
                        self.get_r16(R::BX));
            }
        }
    }
}

fn count_to_bitmask(v: usize) -> usize {
    match v {
        0  => 0,
        1  => 0b1,
        2  => 0b11,
        3  => 0b111,
        4  => 0b1111,
        5  => 0b1_1111,
        6  => 0b11_1111,
        7  => 0b111_1111,
        8  => 0b1111_1111,
        9  => 0b1_1111_1111,
        10 => 0b11_1111_1111,
        11 => 0b111_1111_1111,
        12 => 0b1111_1111_1111,
        13 => 0b1_1111_1111_1111,
        14 => 0b11_1111_1111_1111,
        15 => 0b111_1111_1111_1111,
        16 => 0b1111_1111_1111_1111,
        17 => 0b1_1111_1111_1111_1111,
        18 => 0b11_1111_1111_1111_1111,
        19 => 0b111_1111_1111_1111_1111,
        20 => 0b1111_1111_1111_1111_1111,
        21 => 0b1_1111_1111_1111_1111_1111,
        22 => 0b11_1111_1111_1111_1111_1111,
        23 => 0b111_1111_1111_1111_1111_1111,
        24 => 0b1111_1111_1111_1111_1111_1111,
        25 => 0b1_1111_1111_1111_1111_1111_1111,
        26 => 0b11_1111_1111_1111_1111_1111_1111,
        27 => 0b111_1111_1111_1111_1111_1111_1111,
        28 => 0b1111_1111_1111_1111_1111_1111_1111,
        _ => panic!("unhandled {}", v)
    }
}
