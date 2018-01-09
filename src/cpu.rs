#[allow(unused_imports)]

use std::{mem, u8};
use std::num::Wrapping;

use register::Register16;
use flags::Flags;
use memory::Memory;
use segment::Segment;
use instruction::{Instruction, InstructionInfo, Parameter, ParameterPair, Op, ModRegRm, seg_offs_as_flat};
use int10;
use int16;
use int21;
use int33;
use gpu::GPU;
use register::{AX, BX, CX, DX, SI, DI, BP, SP, AL, CL, CS, DS, ES, FS, GS, SS};

#[cfg(test)]
#[path = "./cpu_test.rs"]
mod cpu_test;

#[derive(Clone)]
pub struct CPU {
    pub ip: u16,
    pub instruction_count: usize,
    pub memory: Memory,
    pub r16: [Register16; 8], // general purpose registers
    pub sreg16: [u16; 6], // segment registers
    pub flags: Flags,
    breakpoints: Vec<usize>,
    pub gpu: GPU,
    rom_base: usize,
    pub fatal_error: bool, // for debugging: signals to debugger we hit an error
    pub deterministic: bool, // for testing: toggles non-deterministic behaviour
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            ip: 0,
            instruction_count: 0,
            memory: Memory::new(),
            r16: [Register16 { val: 0 }; 8],
            sreg16: [0; 6],
            flags: Flags::new(),
            breakpoints: vec![0; 0],
            gpu: GPU::new(),
            rom_base: 0,
            fatal_error: false,
            deterministic: false,
        }
    }

    pub fn add_breakpoint(&mut self, bp: usize) -> Option<usize> {
        if let None = self.breakpoints.iter().find(|&&x| x ==bp) {
            self.breakpoints.push(bp);
            Some(bp)
        } else {
            None
        }
    }

    pub fn remove_breakpoint(&mut self, bp: usize) -> Option<usize> {
        // TODO later: simplify when https://github.com/rust-lang/rust/issues/40062 is stable
        match self.breakpoints.iter().position(|x| *x == bp) {
            Some(pos) => {
                self.breakpoints.remove(pos);
                Some(bp)
            },
            None => None,
        }
    }

    pub fn get_breakpoints(&self) -> Vec<usize> {
        self.breakpoints.clone()
    }

    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    pub fn reset(&mut self) {
        let cpu = CPU::new();
        *self = cpu;
    }

    pub fn load_bios(&mut self, data: &[u8]) {
        self.sreg16[CS] = 0xF000;
        self.ip = 0x0000;
        let min = 0xF_0000;
        let max = min + data.len();
        println!("loading bios to {:06X}..{:06X}", min, max);
        self.rom_base = min;

        self.memory.memory[min..max].copy_from_slice(data);
    }

    // load .com program into CS:0100 and set IP to program start
    pub fn load_com(&mut self, data: &[u8]) {
        // CS,DS,ES,SS = PSP segment
        let psp_segment = 0x085F; // is what dosbox used
        self.sreg16[CS] = psp_segment;
        self.sreg16[DS] = psp_segment;
        self.sreg16[ES] = psp_segment;
        self.sreg16[SS] = psp_segment;

        // offset of last word available in first 64k segment
        self.r16[SP].val = 0xFFFE;
        self.r16[BP].val = 0x091C; // is what dosbox used

        // This is what dosbox initializes the registers to
        // at program load
        self.r16[CX].val = 0xFF;
        self.r16[DX].val = psp_segment;
        self.r16[SI].val = 0x100;
        self.r16[DI].val = 0xFFFE;

        self.ip = 0x0100;
        let min = self.get_offset();
        let max = min + data.len();
        // println!("loading rom to {:06X}..{:06X}", min, max);
        self.rom_base = min;

        self.memory.memory[min..max].copy_from_slice(data);
    }

    // base address the rom was loaded to
    pub fn get_rom_base(&self) -> usize {
        self.rom_base
    }

    pub fn print_registers(&mut self) -> String {
        let mut res = String::new();

        res += format!("AX:{:04X}  SI:{:04X}  DS:{:04X}  IP:{:04X}  cnt:{}\n",
                       self.r16[AX].val,
                       self.r16[SI].val,
                       self.sreg16[DS],
                       self.ip,
                       self.instruction_count)
                .as_ref();
        res += format!("BX:{:04X}  DI:{:04X}  CS:{:04X}  fl:{:04X}\n",
                       self.r16[BX].val,
                       self.r16[DI].val,
                       self.sreg16[CS],
                       self.flags.u16())
                .as_ref();
        res += format!("CX:{:04X}  BP:{:04X}  ES:{:04X}  GS:{:04X}\n",
                       self.r16[CX].val,
                       self.r16[BP].val,
                       self.sreg16[ES],
                       self.sreg16[GS])
                .as_ref();
        res += format!("DX:{:04X}  SP:{:04X}  FS:{:04X}  SS:{:04X}\n",
                       self.r16[DX].val,
                       self.r16[SP].val,
                       self.sreg16[FS],
                       self.sreg16[SS])
                .as_ref();
        res += format!("C{} Z{} S{} O{} A{} P{} D{} I{}",
                       self.flags.carry_numeric(),
                       self.flags.zero_numeric(),
                       self.flags.sign_numeric(),
                       self.flags.overflow_numeric(),
                       self.flags.auxiliary_numeric(),
                       self.flags.parity_numeric(),
                       self.flags.direction_numeric(),
                       self.flags.interrupt_numeric())
                .as_ref();

        res
    }

    pub fn execute_instruction(&mut self) {
        let op = self.decode_instruction(Segment::DS());
        /*
        let cs = self.sreg16[CS];
        let ip = self.ip;
        println!("[{:04X}:{:04X}] <exec> {}", cs, ip, op);
        */
        match op.command {
            Op::Unknown() => {
                self.fatal_error = true;
                println!("executed unknown op, stopping. {} instructions executed",
                         self.instruction_count);
            }
            _ => self.execute(&op),
        }

        // XXX need instruction timing to do this properly
        if self.instruction_count % 100 == 0 {
            self.gpu.progress_scanline();
        }
    }

    pub fn execute_n_instructions(&mut self, n: usize) {
        for _ in 0..n {
            //let op = self.disasm_instruction();
            //println!("{}", op.pretty_string());
            //println!("{}", self.print_registers());
            self.execute_instruction();
            if self.fatal_error {
                return;
            }
            if self.is_ip_at_breakpoint() {
                self.fatal_error = true;
                println!("Breakpoint, ip = {:04X}:{:04X}",
                    self.sreg16[CS],
                    self.ip);
                return;
            }
        }
    }

    pub fn disassemble_block(&mut self, origin: u16, count: usize) -> String {
        let old_ip = self.ip;
        self.ip = origin as u16;
        let mut res = String::new();

        for _ in 0..count {
            let op = self.disasm_instruction();
            res.push_str(&op.to_string());
            res.push_str("\n");
            self.ip += op.length as u16;
        }

        self.ip = old_ip;
        res
    }

    pub fn disasm_instruction(&mut self) -> InstructionInfo {
        let old_ip = self.ip;
        let op = self.decode_instruction(Segment::Default());
        let length = self.ip - old_ip;
        self.ip = old_ip;
        let offset = seg_offs_as_flat(self.sreg16[CS], old_ip);

        InstructionInfo {
            segment: self.sreg16[CS] as usize,
            offset: old_ip as usize,
            length: length as usize,
            text: format!("{}", op),
            bytes: self.read_u8_slice(offset, length as usize),
            instruction: op,
        }
    }

    // used by aaa, aas
    fn adjb(&mut self, param1: i8, param2: i8) {
        if self.flags.auxiliary_carry || (self.r16[AX].lo_u8() & 0xf) > 9 {
            let al = self.r16[AX].lo_u8();
            let ah = self.r16[AX].hi_u8();
            self.r16[AX].set_lo((u16::from(al) + param1 as u16) as u8);
            self.r16[AX].set_hi((u16::from(ah) + param2 as u16) as u8);
            self.flags.auxiliary_carry = true;
            self.flags.carry = true;
        } else {
            self.flags.auxiliary_carry = false;
            self.flags.carry = false;
        }
        let al = self.r16[AX].lo_u8();
        self.r16[AX].set_lo(al & 0x0F);
    }

    // used by daa, das
    fn adj4(&mut self, param1: i8, param2: i8) {
        let old_al = self.r16[AX].lo_u8();
        let old_cf = self.flags.carry;
        self.flags.carry = false;

        if (old_al & 0x0F) > 9 || self.flags.auxiliary_carry {
            let tmp = u16::from(old_al) + param1 as u16;
            self.r16[AX].set_lo(tmp as u8);
            self.flags.carry = tmp & 0x100 != 0;
            self.flags.auxiliary_carry = true;
        } else {
            self.flags.auxiliary_carry = false;
        }

        if old_al > 0x99 || old_cf {
            self.r16[AX].set_lo((u16::from(old_al) + param2 as u16) as u8);
            self.flags.carry = true;
        }
    }

    fn execute(&mut self, op: &Instruction) {
        self.instruction_count += 1;
        match op.command {
            Op::Aaa() => {
                // ASCII Adjust After Addition
                let v = if self.r16[AX].lo_u8() > 0xf9 {
                    2
                 } else {
                    1
                };
                self.adjb(6, v);
            }
            Op::Aas() => {
                // ASCII Adjust AL After Subtraction
                let v = if self.r16[AX].lo_u8() < 6 {
                    -2
                } else {
                    -1
                };
                self.adjb(-6, v);
            }
            Op::Adc8() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let carry = if self.flags.carry { 1 } else { 0 };
                let res = (Wrapping(dst) + Wrapping(src) + Wrapping(carry)).0;
                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u8(res, src + carry, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src + carry, dst);
                self.flags.set_carry_u8(res);
                self.flags.set_parity(res);
            }
            Op::Adc16() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let carry = if self.flags.carry { 1 } else { 0 };
                let res = (Wrapping(dst) + Wrapping(src) + Wrapping(carry)).0;
                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u16(res, src + carry, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src + carry, dst);
                self.flags.set_carry_u16(res);
                self.flags.set_parity(res);
            }
            Op::Add8() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_carry_u8(res);
                self.flags.set_parity(res);

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Add16() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_carry_u16(res);
                self.flags.set_parity(res);

                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);
            }
            Op::And8() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst & src;

                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.flags.overflow = false;
                self.flags.carry = false;
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_parity(res);
                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::And16() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst & src;

                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.flags.overflow = false;
                self.flags.carry = false;
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_parity(res);
                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);
            }
            Op::Arpl() => {
                // Adjust RPL Field of Segment Selector
                println!("XXX impl arpl: {}", op);
                /*
                // NOTE: RPL is the low two bits of the address
                let src = self.read_parameter_value(&op.params.src);
                let mut dst = self.read_parameter_value(&op.params.dst);
                if dst & 3 < src & 3 {
                    self.flags.zero = true;
                    dst = (dst & 0xFFFC) + (src & 3);
                    self.write_parameter_u16(op.segment, &op.params.dst, (dst & 0xFFFF) as u16);
                } else {
                    self.flags.zero = false;
                }
                */
            }
            Op::Bound() => {
                println!("XXX impl {}", op);
            }
            Op::CallNear() => {
                // call near rel
                let old_ip = self.ip;
                let temp_ip = self.read_parameter_value(&op.params.dst);
                self.push16(old_ip);
                self.ip = temp_ip as u16;
            }
            Op::Cbw() => {
                // Convert Byte to Word
                if self.r16[AX].lo_u8() & 0x80 != 0 {
                    self.r16[AX].set_hi(0xFF);
                } else {
                    self.r16[AX].set_hi(0x00);
                }
            }
            Op::Clc() => {
                // Clear Carry Flag
                self.flags.carry = false;
            }
            Op::Cld() => {
                // Clear Direction Flag
                self.flags.direction = false;
            }
            Op::Cli() => {
                // Clear Interrupt Flag
                self.flags.interrupt = false;
            }
            Op::Cmc() => {
                // Complement Carry Flag
                self.flags.carry = !self.flags.carry;
            }
            Op::Cmp8() => {
                // two parameters
                // Modify status flags in the same manner as the SUB instruction
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF, OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_carry_u8(res);
                self.flags.set_overflow_sub_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
            }
            Op::Cmp16() => {
                // two parameters
                // Modify status flags in the same manner as the SUB instruction
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF, OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_carry_u16(res);
                self.flags.set_overflow_sub_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
            }
            Op::Cwd() => {
                // Convert Word to Doubleword
                // DX:AX ← sign-extend of AX.
                self.r16[DX].val = if self.r16[AX].val & 0x8000 != 0 {
                    0xFFFF
                } else {
                    0
                };
            }
            Op::Daa() => {
                // Decimal Adjust AL after Addition
                self.adj4(6, 0x60);
            }
            Op::Das() => {
                // Decimal Adjust AL after Subtraction
                self.adj4(-6, -0x60);
            }
            Op::Dec8() => {
                // single parameter (dst)
                let dst = self.read_parameter_value(&op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.flags.set_overflow_sub_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Dec16() => {
                // single parameter (dst)
                let dst = self.read_parameter_value(&op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.flags.set_overflow_sub_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);
            }
            Op::Div8() => {
                let dst = self.r16[AX].val as usize; // AX
                let src = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) / Wrapping(src)).0;
                let rem = (Wrapping(dst) % Wrapping(src)).0;

                // The CF, OF, SF, ZF, AF, and PF flags are undefined.

                // result stored in AL ← Quotient, AH ← Remainder.
                self.r16[AX].set_lo((res & 0xFF) as u8);
                self.r16[AX].set_hi((rem & 0xFF) as u8);
            }
            Op::Div16() => {
                let dst = ((self.r16[DX].val as usize) << 16) + self.r16[AX].val as usize; // DX:AX
                let src = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) / Wrapping(src)).0;
                let rem = (Wrapping(dst) % Wrapping(src)).0;

                // The CF, OF, SF, ZF, AF, and PF flags are undefined.

                // result stored in AX ← Quotient, DX ← Remainder.
                self.r16[AX].val = (res & 0xFFFF) as u16;
                self.r16[DX].val = (rem & 0xFFFF) as u16;
            }
            Op::Hlt() => {
                println!("XXX impl hlt: {}", op);
            }
            Op::Idiv8() => {
                let mut dst = self.r16[AX].val as usize; // AX
                let src = self.read_parameter_value(&op.params.dst);
                let quo = (Wrapping(dst) / Wrapping(src)).0;
                let rem = (Wrapping(dst) % Wrapping(src)).0;
                if dst > 0xFF {
                    println!("XXX idiv8 INTERRUPT0 (div by 0)");
                } else {
                    self.r16[AX].set_lo((quo & 0xFF) as u8);
                    self.r16[AX].set_hi((rem & 0xFF) as u8);
                }
            }
            Op::Idiv16() => {
                let mut dst = ((self.r16[DX].val as usize) << 16) | self.r16[AX].val as usize; // DX:AX
                let src = self.read_parameter_value(&op.params.dst);
                let quo = (Wrapping(dst) / Wrapping(src)).0;
                let rem = (Wrapping(dst) % Wrapping(src)).0;
                if dst > 0xFFFF {
                    println!("XXX idiv16 INTERRUPT0 (div by 0)");
                } else {
                    self.r16[AX].val = (quo & 0xFFFF) as u16;
                    self.r16[DX].val = (rem & 0xFFFF) as u16;
                }
            }
            Op::Imul8() => {
                // NOTE: only 1-parameter imul8 instruction exists
                // IMUL r/m8               : AX← AL ∗ r/m byte.
                let dst = self.read_parameter_value(&op.params.dst) as i8;
                let tmp = (self.r16[AX].lo_u8() as i8) as isize * dst as isize;
                self.r16[AX].val = tmp as u16;

                // XXX flags
                if self.r16[DX].val != 0 {
                    self.flags.carry = true;
                    self.flags.overflow = true;
                } else {
                    self.flags.carry = false;
                    self.flags.overflow = false;
                }
            }
            Op::Imul16() => {
                match op.params.count() {
                    1 => {
                        // IMUL r/m16               : DX:AX ← AX ∗ r/m word.
                        let a = self.read_parameter_value(&op.params.dst) as i16;
                        let tmp = (self.r16[AX].val as i16) as isize * a as isize;
                        self.r16[AX].val = tmp as u16;
                        self.r16[DX].val = (tmp >> 16) as u16;
                    }
                    2 => {
                        // IMUL r16, r/m16          : word register ← word register ∗ r/m16.
                        let a = self.read_parameter_value(&op.params.dst);
                        let b = self.read_parameter_value(&op.params.src);
                        let tmp = a as isize * b as isize;
                        self.write_parameter_u16(op.segment, &op.params.dst, (tmp & 0xFFFF) as u16);
                    }
                    3 => {
                        // IMUL r16, r/m16, imm8    : word register ← r/m16 ∗ sign-extended immediate byte.
                        // IMUL r16, r/m16, imm16   : word register ← r/m16 ∗ immediate word.
                        let a = self.read_parameter_value(&op.params.src);
                        let b = self.read_parameter_value(&op.params.src2);
                        let tmp = b as isize * a as isize;
                        self.write_parameter_u16(op.segment, &op.params.dst, (tmp & 0xFFFF) as u16);
                    }
                    _ => {
                        panic!("imul16 with {} parameters: {}", op.params.count(), op);
                    }
                }

                // XXX flags
                if self.r16[DX].val != 0 {
                    self.flags.carry = true;
                    self.flags.overflow = true;
                } else {
                    self.flags.carry = false;
                    self.flags.overflow = false;
                }
            }
            Op::In8() => {
                // Input from Port
                // two parameters (dst=AL)
                let src = self.read_parameter_value(&op.params.src);
                let data = self.in_port(src as u16);
                self.write_parameter_u8(&op.params.dst, data);
            }
            Op::Inc8() => {
                let dst = self.read_parameter_value(&op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Inc16() => {
                let dst = self.read_parameter_value(&op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);
            }
            Op::Insb() => {
                println!("XXX impl insb: {}", op);
            }
            Op::Int() => {
                let int = self.read_parameter_value(&op.params.dst);
                self.int(int as u8);
            }
            Op::Ja() => {
                // Jump if above (CF=0 and ZF=0).    (alias: jnbe)
                if !self.flags.carry & !self.flags.zero {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jc() => {
                // Jump if carry (CF=1).    (alias: jb, jnae)
                if self.flags.carry {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jcxz() => {
                // Jump if CX register is 0.
                if self.r16[CX].val == 0 {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jg() => {
                // Jump if greater (ZF=0 and SF=OF).    (alias: jnle)
                if !self.flags.zero & self.flags.sign == self.flags.overflow {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jl() => {
                // Jump if less (SF ≠ OF).    (alias: jnge)
                if self.flags.sign != self.flags.overflow {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::JmpFar() => {
                // dst=Ptr16Imm
                match op.params.dst {
                    Parameter::Ptr16Imm(ip, seg) => {
                        self.sreg16[CS] = seg;
                        self.ip = ip;
                    }
                    _ => {
                        println!("FATAL jmp far with unexpected type {:?}", op.params.dst);
                    }
                }
            }
            Op::JmpNear() | Op::JmpShort() => {
                self.ip = self.read_parameter_value(&op.params.dst) as u16;
            }
            Op::Jna() => {
                // Jump if not above (CF=1 or ZF=1).    (alias: jbe)
                if self.flags.carry | self.flags.zero {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jnc() => {
                // Jump if not carry (CF=0).    (alias: jae, jnb)
                if !self.flags.carry {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jng() => {
                // Jump if not greater (ZF=1 or SF ≠ OF).    (alias: jle)
                if self.flags.zero | self.flags.sign != self.flags.overflow {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jnl() => {
                // Jump if not less (SF=OF).    (alias: jge)
                if self.flags.sign == self.flags.overflow {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jns() => {
                // Jump if not sign (SF=0).
                if !self.flags.sign {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jno() => {
                // Jump if not overflow (OF=0).
                if !self.flags.overflow {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jnz() => {
                // Jump if not zero (ZF=0).    (alias: jne)
                if !self.flags.zero {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Js() => {
                // Jump if sign (SF=1).
                if self.flags.sign {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jo() => {
                // Jump if overflow (OF=1).
                if self.flags.overflow {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jpe() => {
                // Jump short if parity even (PF=1)
                if self.flags.parity {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jpo() => {
                // Jump short if parity odd (PF=0).
                 if !self.flags.parity {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jz() => {
                // Jump if zero (ZF ← 1).    (alias: je)
                if self.flags.zero {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Lea16() => {
                // Load Effective Address
                let src = self.read_parameter_address(&op.params.src) as u16;
                self.write_parameter_u16(op.segment, &op.params.dst, src);
            }
            Op::Lds() => {
                // Load DS:r16 with far pointer from memory.
                let seg = self.read_parameter_address(&op.params.src) as u16;
                let val = self.read_parameter_value(&op.params.src) as u16;
                self.sreg16[DS] = seg;
                self.write_parameter_u16(op.segment, &op.params.dst, val);
            }
            Op::Les() => {
                // les ax, [0x104]
                // Load ES:r16 with far pointer from memory.
                let seg = self.read_parameter_address(&op.params.src) as u16;
                let val = self.read_parameter_value(&op.params.src) as u16;
                self.sreg16[ES] = seg;
                self.write_parameter_u16(op.segment, &op.params.dst, val);
            }
            Op::Lodsb() => {
                // no arguments
                // For legacy mode, load byte at address DS:(E)SI into AL.
                let offset = seg_offs_as_flat(self.sreg16[DS], self.r16[SI].val);
                let val = self.peek_u8_at(offset);
                self.r16[AX].set_lo(val); // AL =
                self.r16[SI].val = if !self.flags.direction {
                    (Wrapping(self.r16[SI].val) + Wrapping(1)).0
                } else {
                    (Wrapping(self.r16[SI].val) - Wrapping(1)).0
                };
            }
            Op::Lodsw() => {
                // no arguments
                // For legacy mode, Load word at address DS:(E)SI into AX.
                let offset = seg_offs_as_flat(self.sreg16[DS], self.r16[SI].val);
                let val = self.peek_u16_at(offset);
                self.r16[AX].val = val;
                self.r16[SI].val = if !self.flags.direction {
                    (Wrapping(self.r16[SI].val) + Wrapping(2)).0
                } else {
                    (Wrapping(self.r16[SI].val) - Wrapping(2)).0
                };
            }
            Op::Loop() => {
                let dst = self.read_parameter_value(&op.params.dst) as u16;
                let res = (Wrapping(self.r16[CX].val) - Wrapping(1)).0;
                self.r16[CX].val = res;
                if res != 0 {
                    self.ip = dst;
                }
                // No flags affected.
            }
            Op::Loope() => {
                let dst = self.read_parameter_value(&op.params.dst) as u16;
                let res = (Wrapping(self.r16[CX].val) - Wrapping(1)).0;
                self.r16[CX].val = res;
                if res != 0 && self.flags.zero {
                    self.ip = dst;
                }
                // No flags affected.
            }
            Op::Loopne() => {
                let dst = self.read_parameter_value(&op.params.dst) as u16;
                let res = (Wrapping(self.r16[CX].val) - Wrapping(1)).0;
                self.r16[CX].val = res;
                if res != 0 && !self.flags.zero {
                    self.ip = dst;
                }
                // No flags affected.
            } 
            Op::Mov8() => {
                // two arguments (dst=reg)
                let data = self.read_parameter_value(&op.params.src) as u8;
                self.write_parameter_u8(&op.params.dst, data);
            }
            Op::Mov16() => {
                // two arguments (dst=reg)
                let data = self.read_parameter_value(&op.params.src) as u16;
                self.write_parameter_u16(op.segment, &op.params.dst, data);
            }
            Op::Movsb() => {
                self.movsb();
            }
            Op::Movsw() => {
                self.movsw();
            }
            Op::Movsx16() => {
                // 80386+
                // Move with Sign-Extension
                // moves a signed value into a register and sign-extends it with 1.
                // two arguments (dst=reg)
                let src = self.read_parameter_value(&op.params.src) as u8;

                let mut data = u16::from(src);
                // XXX should not work identical as Movzx16
                if src & 0x80 != 0 {
                    data += 0xFF00;
                }
                self.write_parameter_u16(op.segment, &op.params.dst, data);
            }
            Op::Movzx16() => {
                // 80386+
                // Move with Zero-Extend
                // moves an unsigned value into a register and zero-extends it with zero.
                // two arguments (dst=reg)
                let src = self.read_parameter_value(&op.params.src) as u8;
                let mut data = u16::from(src);
                if src & 0x80 != 0 {
                    data += 0xFF00;
                }
                self.write_parameter_u16(op.segment, &op.params.dst, data);
            }
            Op::Mul8() => {
                // Unsigned multiply (AX ← AL ∗ r/m8).
                let src = self.r16[AX].lo_u8() as usize; // AL
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) * Wrapping(src)).0;

                // The OF and CF flags are set to 0 if the upper half of the
                // result is 0; otherwise, they are set to 1.
                // The SF, ZF, AF, and PF flags are undefined.
                // XXX flags

                self.r16[AX].val = (res & 0xFFFF) as u16;
            }
            Op::Mul16() => {
                // Unsigned multiply (DX:AX ← AX ∗ r/m16).
                let src = self.r16[AX].val as usize; // AX
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) * Wrapping(src)).0;

                self.r16[AX].val = (res & 0xFFFF) as u16;
                self.r16[DX].val = (res >> 16) as u16;

                let dx_true = self.r16[DX].val != 0;
                self.flags.carry = dx_true;
                self.flags.overflow = dx_true;
                self.flags.zero = (self.r16[AX].val != 0) | (self.r16[DX].val != 0); // XXX ZF is undefined in later docs
            }
            Op::Neg8() => {
                // one argument
                let dst = self.read_parameter_value(&op.params.dst);
                let src = 0;
                let res = (Wrapping(src) - Wrapping(dst)).0;
                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);

                // The CF flag set to 0 if the source operand is 0; otherwise it is set to 1.
                if src == 0 {
                    self.flags.carry = false;
                } else {
                    self.flags.carry = true;
                }
                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_overflow_sub_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
            }
            Op::Neg16() => {
                // one argument
                let dst = self.read_parameter_value(&op.params.dst);
                let src = 0;
                let res = (Wrapping(src) - Wrapping(dst)).0;
                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);

                // The CF flag set to 0 if the source operand is 0; otherwise it is set to 1.
                if src == 0 {
                    self.flags.carry = false;
                } else {
                    self.flags.carry = true;
                }
                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_overflow_sub_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
            }
            Op::Nop() => {}
            Op::Not8() => {
                // one arguments (dst)
                let dst = self.read_parameter_value(&op.params.dst);
                let res = !dst;
                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
                // Flags Affected: None
            }
            Op::Not16() => {
                // one arguments (dst)
                let dst = self.read_parameter_value(&op.params.dst);
                let res = !dst;
                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);
                // Flags Affected: None
            }
            Op::Or8() => {
                // two arguments (dst=AL)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst | src;
                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.flags.overflow = false;
                self.flags.carry = false;
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_parity(res);
                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Or16() => {
                // two arguments (dst=AX)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst | src;
                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.flags.overflow = false;
                self.flags.carry = false;
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_parity(res);
                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);
            }
            Op::Out8() => {
                // two arguments
                let addr = self.read_parameter_value(&op.params.dst) as u16;
                let val = self.read_parameter_value(&op.params.src) as u8;
                self.out_u8(addr, val);
            }
            Op::Out16() => {
                // two arguments
                let addr = self.read_parameter_value(&op.params.dst) as u16;
                let val = self.read_parameter_value(&op.params.src) as u16;
                self.out_u16(addr, val);
            }
            Op::Outsb() => {
                // no arguments
                self.outsb();
            }
            Op::Outsw() => {
                // no arguments
                println!("XXX impl outs word: {}", op);
            }
            Op::Pop16() => {
                // one arguments (dst)
                let data = self.pop16();
                self.write_parameter_u16(op.segment, &op.params.dst, data);
            }
            Op::Popa() => {
                // Pop All General-Purpose Registers
                self.r16[AX].val = self.pop16();
                self.r16[CX].val = self.pop16();
                self.r16[DX].val = self.pop16();
                self.r16[BX].val = self.pop16();
                self.r16[SP].val += 2;
                self.r16[BP].val = self.pop16();
                self.r16[SI].val = self.pop16();
                self.r16[DI].val = self.pop16();
            }
            Op::Popf() => {
                // Pop top of stack into lower 16 bits of EFLAGS.
                let data = self.pop16();
                self.flags.set_u16(data);
            }
            Op::Push8() => {
                // single parameter (dst)
                let data = self.read_parameter_value(&op.params.dst) as u8;
                self.push8(data);
            }
            Op::Push16() => {
                // single parameter (dst)
                let data = self.read_parameter_value(&op.params.dst) as u16;
                self.push16(data);
            }
            Op::Pusha() => {
                // Push All General-Purpose Registers
                let ax = self.r16[AX].val;
                let cx = self.r16[CX].val;
                let dx = self.r16[DX].val;
                let bx = self.r16[BX].val;
                let sp = self.r16[SP].val;
                let bp = self.r16[BP].val;
                let si = self.r16[SI].val;
                let di = self.r16[DI].val;

                self.push16(ax);
                self.push16(cx);
                self.push16(dx);
                self.push16(bx);
                self.push16(sp);
                self.push16(bp);
                self.push16(si);
                self.push16(di);
            }
            Op::Pushf() => {
                // push FLAGS register onto stack
                let data = self.flags.u16();
                self.push16(data);
            }
            Op::Rcl8() => {
                // two arguments
                // rotate 9 bits (CF, `src`) times
                let mut res = self.read_parameter_value(&op.params.dst);
                let mut count = self.read_parameter_value(&op.params.src);

                while count > 0 {
                    let c = if self.flags.carry {
                        1
                    } else {
                        0
                    };
                    res = (res << 1) | c;
	                self.flags.set_carry_u8(res);
                    count -= 1;
                }

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Rcl16() => {
                // two arguments
                // rotate 9 bits (CF, `src`) times
                let mut res = self.read_parameter_value(&op.params.dst);
                let mut count = self.read_parameter_value(&op.params.src);

                while count > 0 {
                    let c = if self.flags.carry {
                        1
                    } else {
                        0
                    };
                    res = (res << 1) | c;
	                self.flags.set_carry_u16(res);
                    count -= 1;
                }

                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);
            }
            Op::Rcr8() => {
                // two arguments
                // rotate 9 bits right `src` times
                let mut res = self.read_parameter_value(&op.params.dst);
                let mut count = self.read_parameter_value(&op.params.src);

                while count > 0 {
                    let c = if self.flags.carry {
                        0x100
                    } else {
                        0
                    };
                    res |= c;
	                self.flags.carry = res & 1 != 0;
                    res >>= 1;
                    count -= 1;
                }

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Rcr16() => {
                // two arguments
                // rotate 9 bits right `src` times
                let mut res = self.read_parameter_value(&op.params.dst);
                let mut count = self.read_parameter_value(&op.params.src);

                while count > 0 {
                    let c = if self.flags.carry {
                        0x1_0000
                    } else {
                        0
                    };
                    res |= c;
	                self.flags.carry = res & 1 != 0;
                    res >>= 1;
                    count -= 1;
                }

                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);
            }
            Op::RepMovsb() => {
                // rep movs byte
                // Move (E)CX bytes from DS:[(E)SI] to ES:[(E)DI].
                loop {
                    self.movsb();
                    self.r16[CX].val -= 1;
                    if self.r16[CX].val == 0 {
                        break;
                    }
                }
            }
            Op::RepMovsw() => {
                // rep movs word
                // Move (E)CX bytes from DS:[(E)SI] to ES:[(E)DI].
                loop {
                    self.movsw();
                    self.r16[CX].val -= 1;
                    if self.r16[CX].val == 0 {
                        break;
                    }
                }
            }
            Op::RepOutsb() => {
                // rep outs byte
                // Output (E)CX bytes from DS:[(E)SI] to port DX.
                loop {
                    self.outsb();
                    self.r16[CX].val -= 1;
                    if self.r16[CX].val == 0 {
                        break;
                    }
                }
            }
            Op::RepStosb() => {
                // rep stos byte
                // Fill (E)CX bytes at ES:[(E)DI] with AL.
                loop {
                    self.stosb();
                    self.r16[CX].val -= 1;
                    if self.r16[CX].val == 0 {
                        break;
                    }
                }
            }
            Op::RepStosw() => {
                // rep stos word
                // Fill (E)CX words at ES:[(E)DI] with AX.
                loop {
                    self.stosw();
                    self.r16[CX].val -= 1;
                    if self.r16[CX].val == 0 {
                        break;
                    }
                }
            }
            Op::RepneScasb() => {
                // Find AL, starting at ES:[(E)DI].
                println!("XXX impl repne scas byte: {}", op);
            }
            Op::Retf() => {
                //no arguments
                self.ip = self.pop16();
                self.sreg16[CS] = self.pop16();
            }
            Op::Retn() => {
                // no arguments
                self.ip = self.pop16();
            }
            Op::Rol8() => {
                // Rotate 8 bits of 'dst' left for 'src' times.
                // two arguments
                let mut res = self.read_parameter_value(&op.params.dst);
                let mut count = self.read_parameter_value(&op.params.src);

                while count > 0 {
                    let val = res & 0x80 != 0;
	                self.flags.carry = val;
                    res = (res & 0xFF) << 1;
                    if val {
	                    res |= 1;
                    }
                    count -= 1;
                }

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);

                // XXX flags
            }
            Op::Rol16() => {
                // Rotate 16 bits of 'dst' left for 'src' times.
                // two arguments
                let mut res = self.read_parameter_value(&op.params.dst);
                let mut count = self.read_parameter_value(&op.params.src);

                while count > 0 {
                    let val = res & 0x8000 != 0;
	                self.flags.carry = val;
                    res <<= 1;
                    if val {
	                    res |= 1;
                    }
                    count -= 1;
                }

                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);

                // XXX flags:
                // If the masked count is 0, the flags are not affected.
                // If the masked count is 1, then the OF flag is affected,
                // otherwise (masked count is greater than 1) the OF flag is undefined.
                // The CF flag is affected when the masked count is non- zero. The SF, ZF,
                // AF, and PF flags are always unaffected.
            }
            Op::Ror8() => {
                // two arguments
                // Rotate 8 bits of 'dst' right for 'src' times.
                let mut res = self.read_parameter_value(&op.params.dst);
                let mut count = self.read_parameter_value(&op.params.src);

                while count > 0 {
	                self.flags.carry = res & 0x1 != 0;
                    res >>= 1;
                    if self.flags.carry {
	                    res |= 0x80;
                    }
                    count -= 1;
                }

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
                // XXX flags
            }
            Op::Ror16() => {
                // Rotate 16 bits of 'dst' right for 'src' times.
                // two arguments
                let mut res = self.read_parameter_value(&op.params.dst);
                let mut count = self.read_parameter_value(&op.params.src);

                while count > 0 {
                    let val = res & 0x1 != 0;
	                self.flags.carry = val;
                    res >>= 1;
                     if self.flags.carry {
	                    res |= 0x8000;
                    }
                    count -= 1;
                }

                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);

                // XXX flags
            }
            Op::Sahf() => {
                // Store AH into Flags

                // Loads the SF, ZF, AF, PF, and CF flags of the EFLAGS register with values
                // from the corresponding bits in the AH register (bits 7, 6, 4, 2, and 0, respectively).
                let ah = self.r16[AX].hi_u8();
                self.flags.carry = ah & 0x1 != 0; // bit 0
                self.flags.parity = ah & 0x4 != 0; // bit 2
                self.flags.auxiliary_carry = ah & 0x10 != 0; // bit 4
                self.flags.zero = ah & 0x40 != 0; // bit 6
                self.flags.sign = ah & 0x80 != 0; // bit 7
            }
            Op::Salc() => {
                // "setalc" is not documented in intel docs,
                // but mentioned in http://ref.x86asm.net/coder32.html#gen_note_u_SALC_D6
                // used in dos-software-decoding/demo-256/luminous/luminous.com

                println!("XXX imp salc: {}", op);
            }
            Op::Sar8() => {
                // Signed divide* r/m8 by 2, imm8 times.
                let dst = self.read_parameter_value(&op.params.dst);
                let mut count = self.read_parameter_value(&op.params.src);
                if count > 8 {
                    count = 8;
                }

                let res = if dst & 0x80 != 0 {
                    let x = 0xFF as usize;
                    dst.rotate_right(count as u32) | x.rotate_left(8 - count as u32)
                } else {
                    dst.rotate_right(count as u32)
                };

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);

                // The CF flag contains the value of the last bit shifted out of the destination operand.
                // The OF flag is affected only for 1-bit shifts; otherwise, it is undefined.
                // The SF, ZF, and PF flags are set according to the result.
                // If the count is 0, the flags are not affected. For a non-zero count, the AF flag is undefined.
                self.flags.carry = (dst & 1) != 0;
                if count == 1 {
                    self.flags.overflow = false;
                }
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_parity(res);
            }
            Op::Sar16() => {
                // Signed divide* r/m8 by 2, imm8 times.
                // two arguments
                let dst = self.read_parameter_value(&op.params.dst);
                let mut count = self.read_parameter_value(&op.params.src);
                if count > 16 {
                    count = 16;
                }
                let res = if dst & 0x8000 != 0 {
                    let x = 0xFFFF as usize;
                    dst.rotate_right(count as u32) | x.rotate_left(16 - count as u32)
                } else {
                    dst.rotate_right(count as u32)
                };

                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);

                // The CF flag contains the value of the last bit shifted out of the destination operand.
                // The OF flag is affected only for 1-bit shifts; otherwise, it is undefined.
                // The SF, ZF, and PF flags are set according to the result.
                // If the count is 0, the flags are not affected. For a non-zero count, the AF flag is undefined.
                self.flags.carry = (dst & 1) != 0;
                if count == 1 {
                    self.flags.overflow = false;
                }
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_parity(res);
                // XXX aux flag ?
            }
            Op::Sbb8() => {
                // Integer Subtraction with Borrow
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let cf = if self.flags.carry { 1 } else { 0 };
                let res = (Wrapping(dst) - (Wrapping(src) + Wrapping(cf))).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.flags.set_overflow_sub_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
                self.flags.set_carry_u8(res);

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Setc() => {
                println!("XXX impl setc: {}", op);
            }
            Op::Shl8() => {
                // Multiply `dst` by 2, `src` times.
                // two arguments    (alias: sal)
                let dst = self.read_parameter_value(&op.params.dst);
                let count = self.read_parameter_value(&op.params.src);
                let res = dst.rotate_left(count as u32);
                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);

                self.flags.carry = (dst & 0x80) != 0;
                if count == 1 {
                    self.flags.overflow = false;
                }
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_parity(res);
                // XXX aux flag ?
            }
            Op::Shl16() => {
                // Multiply `dst` by 2, `src` times.
                // two arguments    (alias: sal)
                let dst = self.read_parameter_value(&op.params.dst);
                let count = self.read_parameter_value(&op.params.src);
                let res = dst.rotate_left(count as u32);
                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);

                self.flags.carry = (dst & 0x8000) != 0;
                if count == 1 {
                    self.flags.overflow = false;
                }
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_parity(res);
                // XXX aux flag ?
            }
            Op::Shr8() => {
                // Unsigned divide r/m8 by 2, `src` times.
                // two arguments
                let dst = self.read_parameter_value(&op.params.dst);
                let count = self.read_parameter_value(&op.params.src);

                let res = dst.rotate_right(count as u32);
                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);

                self.flags.carry = (dst & 1) != 0;
                if count == 1 {
                    self.flags.overflow = false;
                }
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_parity(res);
                // XXX aux flag ?
            }
            Op::Shr16() => {
                // two arguments
                let dst = self.read_parameter_value(&op.params.dst);
                let count = self.read_parameter_value(&op.params.src);

                let res = dst.rotate_right(count as u32);
                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);

                // The CF flag contains the value of the last bit shifted out of the destination
                // operand; it is undefined for SHL and SHR instructions where the count is greater
                // than or equal to the size (in bits) of the destination operand. The OF flag is
                // affected only for 1-bit shifts; otherwise, it is undefined. The SF, ZF, and PF
                // flags are set according to the result. If the count is 0, the flags are not
                // affected. For a non-zero count, the AF flag is undefined.

                self.flags.carry = (dst & 1) != 0;
                if count == 1 {
                    self.flags.overflow = false;
                }
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_parity(res);
                // XXX aux flag ?
            }
            Op::Shrd() => {
                // Double Precision Shift Right
                // 3 arguments

                let dst = self.read_parameter_value(&op.params.dst);
                let count = self.read_parameter_value(&op.params.src2);
                if count == 0 {
                    return;
                }
                let src = self.read_parameter_value(&op.params.src);

                // Shift `dst` to right `count` places while shifting bits from `src` in from the left
                let res = (src & count_to_bitmask(count) as usize) << (16-count) | (dst >> count);

                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);

                if count >= 1 {
                    // XXX carry if count is >= 1

                    // If the count is 1 or greater, the CF flag is filled with the last bit shifted out
                    // of the destination operand

                    self.flags.carry = (dst & 1) != 0; // XXX this would be the first bit.. which is wrong
                }

                // SF, ZF, and PF flags are set according to the value of the result.
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_parity(res);

                if count == 1 {
                    // XXX overflow if count == 1

                    // For a 1-bit shift, the OF flag is set if a sign change occurred; otherwise, it is cleared.
                    // For shifts greater than 1 bit, the OF flag is undefined. 
                }

                // If a shift occurs, the AF flag is undefined. If the count is greater than the operand size,
                // the flags are undefined.
            }
            Op::Stc() => {
                // Set Carry Flag
                self.flags.carry = true;
            }
            Op::Std() => {
                // Set Direction Flag
                self.flags.direction = true;
            }
            Op::Sti() => {
                // Set Interrupt Flag
                self.flags.interrupt = true;
            }
            Op::Stosb() => {
                // no parameters
                // store AL at ES:(E)DI
                self.stosb();
            }
            Op::Stosw() => {
                // no parameters
                // store AX at address ES:(E)DI
                self.stosw();
            }
            Op::Sub8() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.flags.set_overflow_sub_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
                self.flags.set_carry_u8(res);

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Sub16() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.flags.set_overflow_sub_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
                self.flags.set_carry_u16(res);

                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);
            }
            Op::Test8() => {
                // two parameters
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst & src;
                // set SF, ZF, PF according to result.
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_parity(res);
            }
            Op::Test16() => {
                // two parameters
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst & src;
                // set SF, ZF, PF according to result.
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_parity(res);
            }
            Op::Xchg8() => {
                // two parameters (registers)
                let mut src = self.read_parameter_value(&op.params.src);
                let mut dst = self.read_parameter_value(&op.params.dst);
                mem::swap(&mut src, &mut dst);
                self.write_parameter_u8(&op.params.dst, dst as u8);
                self.write_parameter_u8(&op.params.src, src as u8);
            }
            Op::Xchg16() => {
                // two parameters (registers)
                let mut src = self.read_parameter_value(&op.params.src);
                let mut dst = self.read_parameter_value(&op.params.dst);
                mem::swap(&mut src, &mut dst);
                self.write_parameter_u16(op.segment, &op.params.dst, dst as u16);
                self.write_parameter_u16(op.segment, &op.params.src, src as u16);
            }
            Op::Xlatb() => {
                // println!("XXX impl xlatb: {}", op);
            }
            Op::Xor8() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst ^ src;

                // The OF and CF flags are cleared; the SF, ZF,
                // and PF flags are set according to the result.
                self.flags.overflow = false;
                self.flags.carry = false;
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_parity(res);

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Xor16() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst ^ src;

                // The OF and CF flags are cleared; the SF, ZF,
                // and PF flags are set according to the result.
                self.flags.overflow = false;
                self.flags.carry = false;
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_parity(res);

                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);
            }
            _ => {
                println!("execute error: unhandled: {:?} at {:06X}",
                         op.command,
                         self.get_offset());
            }
        }
    }

    fn decode_instruction(&mut self, seg: Segment) -> Instruction {
        let b = self.read_u8();
        let mut op = Instruction {
            segment: seg,
            command: Op::Unknown(),
            params: ParameterPair {
                dst: Parameter::None(),
                src: Parameter::None(),
                src2: Parameter::None(),
            },
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
                        println!("op 0F, unknown {:02X}: at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                            b,
                            self.sreg16[CS],
                            self.ip - 1,
                            self.get_offset() - 1,
                            self.instruction_count);
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
            0x1A => {
                // sbb r8, r/m8
                op.command = Op::Sbb8();
                op.params = self.r8_rm8(op.segment);
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
                op = self.decode_instruction(Segment::ES());
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
                op = self.decode_instruction(Segment::CS());
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
                op = self.decode_instruction(Segment::SS());
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
                op = self.decode_instruction(Segment::DS());
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
            0x62 => {
                // bound r16, m16&16
                op.command = Op::Bound();
                op.params = self.r16_m16(op.segment);
            }
            0x63 => {
                // arpl r/m16, r16
                op.command = Op::Arpl();
                op.params = self.rm16_r16(op.segment);
            }
            0x64 => {
                // fs segment prefix
                op = self.decode_instruction(Segment::FS());
            }
            0x65 => {
                // gs segment prefix
                op = self.decode_instruction(Segment::GS());
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
                        println!("op 8F unknown reg = {}: at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                            x.reg,
                            self.sreg16[CS],
                            self.ip - 1,
                            self.get_offset() - 1,
                            self.instruction_count);
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
                    _ => Op::Unknown(),
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
                    _ => Op::Unknown(),
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
                        println!("op C6 unknown reg = {} at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                            x.reg,
                            self.sreg16[CS],
                            self.ip - 1,
                            self.get_offset() - 1,
                            self.instruction_count);
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
                        println!("op C7 unknown reg = {} at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                            x.reg,
                            self.sreg16[CS],
                            self.ip - 1,
                            self.get_offset() - 1,
                            self.instruction_count);
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
                    _ => Op::Unknown(),
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
                    _ => Op::Unknown(),
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
                    _ => Op::Unknown(),
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
                    _ => Op::Unknown(),
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
                        println!("op F2 error: unhandled op {:02X}", b);
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
                        println!("op F3 error: unhandled op {:02X}", b);
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
                        println!("op F6 unknown reg={}", x.reg);
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
                        println!("op F7 unknown reg={} at {:04X}:{:04X}",
                                 x.reg,
                                 self.sreg16[CS],
                                 self.ip);
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
                        println!("op FE, unknown reg {}: at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                            x.reg,
                            self.sreg16[CS],
                            self.ip - 1,
                            self.get_offset() - 1,
                            self.instruction_count);
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
                        println!("op FF, unknown reg {}: at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                            x.reg,
                            self.sreg16[CS],
                            self.ip - 1,
                            self.get_offset() - 1,
                            self.instruction_count);
                    }
                }
            }
            _ => {
                println!("decode_instruction: unknown op {:02X} at {:04X}:{:04X} ({:06X} flat), {} instructions executed",
                         b,
                         self.sreg16[CS],
                         self.ip - 1,
                         self.get_offset() - 1,
                         self.instruction_count);
            }
        }
        op
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

    // decode r16, r/m16
    fn r16_rm16(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: Parameter::Reg16(x.reg as usize),
            src: self.rm16(seg, x.rm, x.md),
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
            println!("r16_m16 error: invalid encoding, ip={:04X}", self.ip);
        }
        ParameterPair {
            dst: Parameter::Reg16(x.reg as usize),
            src: self.rm16(seg, x.rm, x.md),
            src2: Parameter::None(),
        }
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

    fn push8(&mut self, data: u8) {
        let sp = (Wrapping(self.r16[SP].val) - Wrapping(1)).0;
        self.r16[SP].val = sp;
        let offset = seg_offs_as_flat(self.sreg16[SS], self.r16[SP].val);
        /*
        println!("push8 {:02X}  to {:04X}:{:04X}  =>  {:06X}       instr {}",
                 data,
                 self.sreg16[SS],
                 self.r16[SP].val,
                 offset,
                 self.instruction_count);
        */
        self.write_u8(offset, data);
    }

    fn push16(&mut self, data: u16) {
        let sp = (Wrapping(self.r16[SP].val) - Wrapping(2)).0;
        self.r16[SP].val = sp;
        let offset = seg_offs_as_flat(self.sreg16[SS], self.r16[SP].val);
        /*
        println!("push16 {:04X}  to {:04X}:{:04X}  =>  {:06X}       instr {}",
                 data,
                 self.sreg16[SS],
                 self.r16[SP].val,
                 offset,
                 self.instruction_count);
        */
        self.write_u16(offset, data);
    }

    fn pop16(&mut self) -> u16 {
        let offset = seg_offs_as_flat(self.sreg16[SS], self.r16[SP].val);
        let data = self.peek_u16_at(offset);
        /*
        println!("pop16 {:04X}  from {:04X}:{:04X}  =>  {:06X}       instr {}",
                 data,
                 self.sreg16[SS],
                 self.r16[SP].val,
                 offset,
                 self.instruction_count);
        */
        let sp = (Wrapping(self.r16[SP].val) + Wrapping(2)).0;
        self.r16[SP].val = sp;
        data
    }

    fn read_mod_reg_rm(&mut self) -> ModRegRm {
        let b = self.read_u8();
        ModRegRm {
            md: b >> 6, // high 2 bits
            reg: (b >> 3) & 7, // mid 3 bits
            rm: b & 7, // low 3 bits
        }
    }

    pub fn get_offset(&self) -> usize {
        seg_offs_as_flat(self.sreg16[CS], self.ip)
    }

    fn read_u8(&mut self) -> u8 {
        let offset = self.get_offset();
        let b = self.memory.memory[offset];
        /*
        println!("___ DBG: read u8 {:02X} from {:06X} ... {:04X}:{:04X}",
              b,
              offset,
              self.sreg16[CS],
              self.ip);
        */

        // self.ip = (Wrapping(self.ip) + Wrapping(1)).0;  // XXX what do if ip wraps?
        self.ip += 1;
        b
    }

    fn read_u16(&mut self) -> u16 {
        let lo = self.read_u8();
        let hi = self.read_u8();
        u16::from(hi) << 8 | u16::from(lo)
    }

    fn read_s8(&mut self) -> i8 {
        self.read_u8() as i8
    }

    fn read_s16(&mut self) -> i16 {
        self.read_u16() as i16
    }

    fn read_rel8(&mut self) -> u16 {
        let val = self.read_u8() as i8;
        (self.ip as i16 + i16::from(val)) as u16
    }

    fn read_rel16(&mut self) -> u16 {
        let val = self.read_u16() as i16;
        (self.ip as i16 + val) as u16
    }

    pub fn peek_u8_at(&mut self, pos: usize) -> u8 {
        // println!("peek_u8_at   pos {:04X}  = {:02X}", pos, self.memory[pos]);
        self.memory.memory[pos]
    }

    fn peek_u16_at(&mut self, pos: usize) -> u16 {
        let lo = self.peek_u8_at(pos);
        let hi = self.peek_u8_at(pos + 1);
        u16::from(hi) << 8 | u16::from(lo)
    }

    fn write_u8(&mut self, offset: usize, data: u8) {
        // println!("write_u8 [{:06X}] = {:02X}", offset, data);

        // break if we hit a breakpoint
        if self.is_offset_at_breakpoint(offset) {
            self.fatal_error = true;
            println!("Breakpoint (memory write to {:06X} = {:02X}), ip = {:04X}:{:04X}",
                offset,
                data,
                self.sreg16[CS],
                self.ip);
        }

        self.memory.memory[offset] = data;
    }

    fn write_u16(&mut self, offset: usize, data: u16) {
        // println!("write_u16 [{:06X}] = {:04X}", offset, data);
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.write_u8(offset, lo);
        self.write_u8(offset + 1, hi);
    }

    // returns the address of pointer, used by LEA, LDS, LES
    fn read_parameter_address(&mut self, p: &Parameter) -> usize {
        match *p {
            Parameter::Ptr16Amode(_, r) => self.amode16(r) as usize,
            Parameter::Ptr16AmodeS8(_, r, imm) => (Wrapping(self.amode16(r) as usize) + Wrapping(imm as usize)).0,
            Parameter::Ptr16AmodeS16(_, r, imm) => (Wrapping(self.amode16(r) as usize) + Wrapping(imm as usize)).0,
            Parameter::Ptr16(_, imm) => imm as usize,
            _ => {
                println!("read_parameter_address error: unhandled parameter: {:?} at {:06X}",
                         p,
                         self.get_offset());
                0
            }
        }
    }

    fn read_parameter_value(&mut self, p: &Parameter) -> usize {
        match *p {
            Parameter::Imm8(imm) => imm as usize,
            Parameter::Imm16(imm) => imm as usize,
            Parameter::ImmS8(imm) => imm as usize,
            Parameter::Ptr8(seg, imm) => {
                let offset = seg_offs_as_flat(self.segment(seg), imm);
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr16(seg, imm) => {
                let offset = seg_offs_as_flat(self.segment(seg), imm);
                self.peek_u16_at(offset) as usize
            }
            Parameter::Ptr8Amode(seg, r) => {
                let offset = seg_offs_as_flat(self.segment(seg), self.amode16(r));
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr8AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(seg_offs_as_flat(self.segment(seg), self.amode16(r))) + Wrapping(imm as usize)).0;
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr8AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(seg_offs_as_flat(self.segment(seg), self.amode16(r))) + Wrapping(imm as usize)).0;
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr16Amode(seg, r) => {
                let offset = seg_offs_as_flat(self.segment(seg), self.amode16(r));
                self.peek_u16_at(offset) as usize
            }
            Parameter::Ptr16AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(seg_offs_as_flat(self.segment(seg), self.amode16(r))) + Wrapping(imm as usize)).0;
                self.peek_u16_at(offset) as usize
            }
            Parameter::Ptr16AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(seg_offs_as_flat(self.segment(seg), self.amode16(r))) + Wrapping(imm as usize)).0;
                self.peek_u16_at(offset) as usize
            }
            Parameter::Reg8(r) => {
                let lor = r & 3;
                if r & 4 == 0 {
                    self.r16[lor].lo_u8() as usize
                } else {
                    self.r16[lor].hi_u8() as usize
                }
            }
            Parameter::Reg16(r) => self.r16[r].val as usize,
            Parameter::SReg16(r) => self.sreg16[r] as usize,
            _ => {
                println!("read_parameter_value error: unhandled parameter: {:?} at {:06X}",
                         p,
                         self.get_offset());
                0
            }
        }
    }

    fn write_parameter_u8(&mut self, p: &Parameter, data: u8) {
        match *p {
            Parameter::Reg8(r) => {
                let lor = r & 3;
                if r & 4 == 0 {
                    self.r16[lor].set_lo(data);
                } else {
                    self.r16[lor].set_hi(data);
                }
            }
            Parameter::Ptr8(seg, imm) => {
                let offset = seg_offs_as_flat(self.segment(seg), imm);
                self.write_u8(offset, data);
            }
            Parameter::Ptr8Amode(seg, r) => {
                let offset = seg_offs_as_flat(self.segment(seg), self.amode16(r));
                self.write_u8(offset, data);
            }
            Parameter::Ptr8AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(seg_offs_as_flat(self.segment(seg), self.amode16(r)) ) + Wrapping(imm as usize)).0;
                self.write_u8(offset, data);
            }
            Parameter::Ptr8AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(seg_offs_as_flat(self.segment(seg), self.amode16(r)) ) + Wrapping(imm as usize)).0;
                self.write_u8(offset, data);
            }
            _ => {
                println!("write_parameter_u8 unhandled type {:?} at {:06X}",
                         p,
                         self.get_offset());
            }
        }
    }

    fn write_parameter_u16(&mut self, segment: Segment, p: &Parameter, data: u16) {
        match *p {
            Parameter::Reg16(r) => {
                self.r16[r].val = data;
            }
            Parameter::SReg16(r) => {
                self.sreg16[r] = data;
            }
            Parameter::Imm16(imm) => {
                let offset = seg_offs_as_flat(self.segment(segment), imm);
                self.write_u16(offset, data);
            }
            Parameter::Ptr16(seg, imm) => {
                let offset = seg_offs_as_flat(self.segment(seg), imm);
                self.write_u16(offset, data);
            }
            Parameter::Ptr16Amode(seg, r) => {
                let offset = seg_offs_as_flat(self.segment(seg), self.amode16(r));
                self.write_u16(offset, data);
            }
            Parameter::Ptr16AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(seg_offs_as_flat(self.segment(seg), self.amode16(r)) ) + Wrapping(imm as usize)).0;
                self.write_u16(offset, data);
            }
            Parameter::Ptr16AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(seg_offs_as_flat(self.segment(seg), self.amode16(r)) ) + Wrapping(imm as usize)).0;
                self.write_u16(offset, data);
            }
            _ => {
                println!("write_u16_param unhandled type {:?} at {:06X}",
                         p,
                         self.get_offset());
            }
        }
    }

    // used by disassembler
    pub fn read_u8_slice(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let mut res = vec![0u8; length];
        res[0..length].clone_from_slice(&self.memory.memory[offset..offset + length]);
        res
    }

    fn segment(&self, seg: Segment) -> u16 {
        match seg {
            Segment::DS() |
            Segment::Default() => self.sreg16[DS],
            Segment::CS() => self.sreg16[CS],
            Segment::ES() => self.sreg16[ES],
            Segment::FS() => self.sreg16[FS],
            Segment::GS() => self.sreg16[GS],
            Segment::SS() => self.sreg16[SS],
        }
    }

    fn amode16(&mut self, idx: usize) -> u16 {
        match idx {
            0 => (Wrapping(self.r16[BX].val) + Wrapping(self.r16[SI].val)).0,
            1 => (Wrapping(self.r16[BX].val) + Wrapping(self.r16[DI].val)).0,
            2 => (Wrapping(self.r16[BP].val) + Wrapping(self.r16[SI].val)).0,
            3 => (Wrapping(self.r16[BP].val) + Wrapping(self.r16[DI].val)).0,
            4 => self.r16[SI].val,
            5 => self.r16[DI].val,
            6 => self.r16[BP].val,
            7 => self.r16[BX].val,
            _ => {
                println!("Impossible amode16, idx {}", idx);
                0
            }
        }
    }

    pub fn is_ip_at_breakpoint(&self) -> bool {
        let offset = self.get_offset();
        self.is_offset_at_breakpoint(offset)
    }

    pub fn is_offset_at_breakpoint(&self, offset: usize) -> bool {
        self.breakpoints.iter().any(|&x| x == offset)
    }

    // used for OUTSB instruction
    fn outsb(&mut self) {
        let src = seg_offs_as_flat(self.sreg16[DS], self.r16[SI].val);
        let val = self.peek_u8_at(src);
        let port = self.r16[DX].val;
        self.out_u8(port, val);

        self.r16[SI].val = if !self.flags.direction {
            (Wrapping(self.r16[SI].val) + Wrapping(1)).0
        } else {
            (Wrapping(self.r16[SI].val) - Wrapping(1)).0
        };
    }

    fn movsb(&mut self) {
        let src = seg_offs_as_flat(self.sreg16[DS], self.r16[SI].val);
        let dst = seg_offs_as_flat(self.sreg16[ES], self.r16[DI].val);
        let b = self.peek_u8_at(src);
        self.r16[SI].val = if !self.flags.direction {
            (Wrapping(self.r16[SI].val) + Wrapping(1)).0
        } else {
            (Wrapping(self.r16[SI].val) - Wrapping(1)).0
        };
        self.write_u8(dst, b);
        self.r16[DI].val = if !self.flags.direction {
            (Wrapping(self.r16[DI].val) + Wrapping(1)).0
        } else {
            (Wrapping(self.r16[DI].val) - Wrapping(1)).0
        };
    }

    // used for MOVSW
    fn movsw(&mut self) {
        let src = seg_offs_as_flat(self.sreg16[DS], self.r16[SI].val);
        let dst = seg_offs_as_flat(self.sreg16[ES], self.r16[DI].val);
        let b = self.peek_u16_at(src);
        self.r16[SI].val = if !self.flags.direction {
            (Wrapping(self.r16[SI].val) + Wrapping(2)).0
        } else {
            (Wrapping(self.r16[SI].val) - Wrapping(2)).0
        };
        self.write_u16(dst, b);
        self.r16[DI].val = if !self.flags.direction {
            (Wrapping(self.r16[DI].val) + Wrapping(2)).0
        } else {
            (Wrapping(self.r16[DI].val) - Wrapping(2)).0
        };
    }

    fn stosb(&mut self) {
        let data = self.r16[AX].lo_u8(); // = AL
        let dst = seg_offs_as_flat(self.sreg16[ES], self.r16[DI].val);
        self.write_u8(dst, data);
        self.r16[DI].val = if !self.flags.direction {
            (Wrapping(self.r16[DI].val) + Wrapping(1)).0
        } else {
            (Wrapping(self.r16[DI].val) - Wrapping(1)).0
        };
    }

    fn stosw(&mut self) {
        let data = self.r16[AX].val;
        let dst = seg_offs_as_flat(self.sreg16[ES], self.r16[DI].val);
        self.write_u16(dst, data);

        self.r16[DI].val = if !self.flags.direction {
            (Wrapping(self.r16[DI].val) + Wrapping(2)).0
        } else {
            (Wrapping(self.r16[DI].val) - Wrapping(2)).0
        };
    }

    // output byte `data` to I/O port
    fn out_u8(&mut self, dst: u16, data: u8) {
        match dst {
            0x03C8 => {
                // (VGA,MCGA) PEL address register
                // Sets DAC in write mode and assign start of color register
                // index (0..255) for following write accesses to 3C9h.
                // Next access to 03C8h will stop pending mode immediately.
                self.gpu.dac_index = data;
                // println!("dac index = {}", data);
            }
            0x03C9 => {
                // (VGA,MCGA) PEL data register
                // Three consecutive writes in the order: red, green, blue.
                // The internal DAC index is incremented each 3rd access.
                if self.gpu.dac_color > 2 {
                    let i = self.gpu.dac_index as usize;
                    self.gpu.pal[i].r = self.gpu.dac_current_pal[0];
                    self.gpu.pal[i].g = self.gpu.dac_current_pal[1];
                    self.gpu.pal[i].b = self.gpu.dac_current_pal[2];

                    if self.gpu.dac_index == 0 {
                        println!("DAC palette {} = {}, {}, {}",
                                self.gpu.dac_index,
                                self.gpu.pal[i].r,
                                self.gpu.pal[i].g,
                                self.gpu.pal[i].b);
                    }

                    self.gpu.dac_color = 0;
                    self.gpu.dac_index = (Wrapping(self.gpu.dac_index) + Wrapping(1)).0;
                }
                // map 6-bit color into 8 bits
                self.gpu.dac_current_pal[self.gpu.dac_color] = data << 2;
                self.gpu.dac_color += 1;
            }
            _ => {
                println!("XXX unhandled out_u8 to {:04X}, data {:02X}", dst, data);
            }
        }
    }

    // output word `data` to I/O port
    fn out_u16(&mut self, dst: u16, data: u16) {
        match dst {
            0x03C4 => {
                // XXX
                /*
                03C4  -W  EGA	TS index register
                        bit7-3 : reserved (VGA only)
                        bit2-0 : current TS index
                03C4  RW  VGA	sequencer register index (see #P0670)
                */
            }
            /*
            0x03C5 => {
                03C5  -W  EGA	TS data register
                03C5  RW  VGA	sequencer register data
            }
            PORT 03D4-03D5 - COLOR VIDEO - CRT CONTROL REGISTERS
            */
            0x03D4 => {
                // 03D4  rW  CRT (6845) register index   (CGA/MCGA/color EGA/color VGA)
                // selects which register (0-11h) is to be accessed through 03D5
                // this port is r/w on some VGA, e.g. ET4000
                //        bit 7-6 =0: (VGA) reserved
                //        bit 5   =0: (VGA) reserved for testage
                //        bit 4-0   : selects which register is to be accessed through 03D5
            }  
            /*
                03D5  -W  CRT (6845) data register   (CGA/MCGA/color EGA/color VGA) (see #P0708)
                    selected by PORT 03D4h. registers 0C-0F may be read
                    (see also PORT 03B5h)
                    MCGA, native EGA and VGA use very different defaults from those
                    mentioned for the other adapters; for additional notes and
                    registers 00h-0Fh and EGA/VGA registers 10h-18h and ET4000
                    registers 32h-37h see PORT 03B5h (see #P0654)
                    registers 10h-11h on CGA, EGA, VGA and 12h-14h on EGA, VGA are
                    conflictive with MCGA (see #P0710)
            */
             _ => {
                println!("XXX unhandled out_u16 to {:04X}, data {:02X}", dst, data);
            }
        }
    }

    // read byte from I/O port
    fn in_port(&mut self, port: u16) -> u8 {
        /*
        println!("in_port: read from {:04X} at {:06X}",
                 port,
                 self.get_offset());
        */
        match port {
            0x0040 => {
                // Programmable Interval Timer
                // http://wiki.osdev.org/Programmable_Interval_Timer

                0 // XXX
            },
            0x0060 => {
                // "8042" PS/2 Controller (keyboard & mice)
                // http://wiki.osdev.org/%228042%22_PS/2_Controller
                0 // XXX
            },
            0x03DA => {
                // R-  CGA status register
                // color EGA/VGA: input status 1 register
                //
                // Bitfields for CGA status register:
                // Bit(s)	Description	(Table P0818)
                // 7-6	not used
                // 7	(C&T Wingine) vertical sync in progress (if enabled by XR14)
                // 5-4	color EGA, color ET4000, C&T: diagnose video display feedback, select
                //      from color plane enable
                // 3	in vertical retrace
                //      (C&T Wingine) video active (retrace/video selected by XR14)
                // 2	(CGA,color EGA) light pen switch is off
                //      (MCGA,color ET4000) reserved (0)
                //      (VGA) reserved (1)
                // 1	(CGA,color EGA) positive edge from light pen has set trigger
                //      (VGA,MCGA,color ET4000) reserved (0)
                // 0	horizontal retrace in progress
                //    =0  do not use memory
                //    =1  memory access without interfering with display
                //        (VGA,Genoa SuperEGA) horizontal or vertical retrace
                //    (C&T Wingine) display enabled (retrace/DE selected by XR14)
                let mut flags = 0;

                // HACK: fake bit 0 and 3 (retrace in progress)
                if self.gpu.scanline == 0 {
                    flags |= 0b0000_0001; // set bit 0
                    flags |= 0b0000_1000; // set bit 3
                } else {
                    flags &= 0b1111_1110; // clear bit 0
                    flags &= 0b1111_0111; // clear bit 3
                }

                flags
            }
            _ => {
                println!("in_port: unhandled in8 {:04X} at {:06X}",
                         port,
                         self.get_offset());
                0
            }
        }
    }

    fn int(&mut self, int: u8) {
        // XXX jump to offset 0x21 in interrupt table (look up how hw does this)
        // http://wiki.osdev.org/Interrupt_Vector_Table   XXX or is those just for real-mode interrupts?
        let deterministic = self.deterministic;
        match int {
            0x03 => {
                // debugger interrupt
                // http://www.ctyme.com/intr/int-03.htm
                println!("INT 3 - debugger interrupt. AX={:04X}", self.r16[AX].val);
                self.fatal_error = true; // stops running debugger
            }
            0x10 => int10::handle(self),
            0x16 => int16::handle(self),
            0x20 => {
                // DOS 1+ - TERMINATE PROGRAM
                // NOTE: Windows overloads INT 20
                println!("INT 20 - Terminating program");
                self.fatal_error = true; // stops running debugger
            }
            0x21 => int21::handle(self, deterministic),
            0x33 => int33::handle(self),
            _ => {
                println!("int error: unknown interrupt {:02X}, AX={:04X}, BX={:04X}",
                         int,
                         self.r16[AX].val,
                         self.r16[BX].val);
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
        _ => panic!("unhandled {}", v)
    }
}
