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
use decoder::Decoder;
use mmu::MMU;

#[cfg(test)]
#[path = "./cpu_test.rs"]
mod cpu_test;

#[derive(Clone)]
pub struct CPU {
    pub ip: u16,
    pub instruction_count: usize,
    pub mmu: MMU,
    pub r16: [Register16; 8], // general purpose registers
    pub sreg16: [u16; 6], // segment registers
    pub flags: Flags,
    breakpoints: Vec<usize>,
    pub gpu: GPU,
    rom_base: usize,
    pub fatal_error: bool, // for debugging: signals to debugger we hit an error
    pub deterministic: bool, // for testing: toggles non-deterministic behaviour
    pub decoder: Decoder,
}

impl CPU {
    pub fn new(mmu: MMU) -> Self {
        CPU {
            ip: 0,
            instruction_count: 0,
            r16: [Register16 { val: 0 }; 8],
            sreg16: [0; 6],
            flags: Flags::new(),
            breakpoints: vec![0; 0],
            gpu: GPU::new(),
            rom_base: 0,
            fatal_error: false,
            deterministic: false,
            mmu: mmu.clone(),
            decoder: Decoder::new(mmu)
        }
    }

    pub fn add_breakpoint(&mut self, bp: usize) {
        self.breakpoints.push(bp);
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

    //reset the CPU but keep the memory
    pub fn soft_reset(&mut self) {
        let cpu = CPU::new(self.mmu.clone());
        *self = cpu;
    }

    pub fn reset(&mut self, mmu: MMU) {
        let cpu = CPU::new(mmu);
        *self = cpu;
    }

    pub fn load_bios(&mut self, data: &[u8]) {
        self.sreg16[CS] = 0xF000;
        self.ip = 0x0000;
        let end = self.ip + data.len() as u16;
        println!("loading bios to {:06X}..{:06X}", self.ip, end);
        self.rom_base = self.ip as usize;

        self.mmu.write(self.sreg16[CS], self.ip, data);
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
        // println!("loading rom to {:06X}..{:06X}", min, max);
        self.rom_base = min;

        self.mmu.write(self.sreg16[CS], self.ip, data);
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
        let cs = self.sreg16[CS];
        let ip = self.ip;
        let op = self.decoder
            .get_instruction(cs, ip, Segment::CS());

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
        self.ip += op.byte_length;
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
                dst = (Wrapping(dst) / Wrapping(src)).0;
                let rem = (Wrapping(dst) % Wrapping(src)).0;
                if dst > 0xFF {
                    println!("XXX idiv8 INTERRUPT0 (div by 0)");
                } else {
                    self.r16[AX].set_lo((dst & 0xFF) as u8);
                    self.r16[AX].set_hi((rem & 0xFF) as u8);
                }
            }
            Op::Idiv16() => {
                let mut dst = ((self.r16[DX].val as usize) << 16) + self.r16[AX].val as usize; // DX:AX
                let src = self.read_parameter_value(&op.params.dst);
                dst = (Wrapping(dst) / Wrapping(src)).0;
                let rem = (Wrapping(dst) % Wrapping(src)).0;
                if dst > 0xFFFF {
                    println!("XXX idiv16 INTERRUPT0 (div by 0)");
                } else {
                    self.r16[AX].val = (dst & 0xFFFF) as u16;
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
                if op.params.count() == 1 {
                    // IMUL r/m16               : DX:AX ← AX ∗ r/m word.
                    let a = self.read_parameter_value(&op.params.dst) as i16;
                    let tmp = (self.r16[AX].val as i16) as isize * a as isize;
                    self.r16[AX].val = tmp as u16;
                    self.r16[DX].val = (tmp >> 16) as u16;
                } else if op.params.count() == 3 {
                    // IMUL r16, r/m16, imm8    : word register ← r/m16 ∗ sign-extended immediate byte.
                    // IMUL r16, r/m16, imm16   : word register ← r/m16 ∗ immediate word.
                    let a = self.read_parameter_value(&op.params.src);
                    let b = self.read_parameter_value(&op.params.src2);
                    let tmp = b as isize * a as isize;
                    self.write_parameter_u16(op.segment, &op.params.dst, (tmp & 0xFFFF) as u16);
                } else {
                    // IMUL r16, r/m16          : word register ← word register ∗ r/m16.
                    println!("XXX impl imul16 with {} parameters: {}", op.params.count(), op);
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
                let val = self.mmu
                    .read_u8(self.sreg16[DS], self.r16[SI].val);

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
                let val = self.mmu
                    .read_u16(self.sreg16[DS], self.r16[SI].val);

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
                let count = self.read_parameter_value(&op.params.src);

                let res = dst.rotate_right(count as u32);
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
                let count = self.read_parameter_value(&op.params.src);

                let res = dst.rotate_right(count as u32);
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
        let data = self.mmu.read_u16(self.sreg16[SS], self.r16[SP].val);
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

    pub fn get_offset(&self) -> usize {
        self.mmu.s_translate(self.sreg16[CS], self.ip)
    }

    fn read_u8(&mut self) -> u8 {
        let b = self.mmu.read_u8(
            self.sreg16[CS],
            self.ip);

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

    // pub fn peek_u8_at(&mut self, pos: usize) -> u8 {
    //     // println!("peek_u8_at   pos {:04X}  = {:02X}", pos, self.memory[pos]);
    //     self.memory.memory[pos]
    // }

    // fn peek_u16_at(&mut self, pos: usize) -> u16 {
    //     let lo = self.peek_u8_at(pos);
    //     let hi = self.peek_u8_at(pos + 1);
    //     u16::from(hi) << 8 | u16::from(lo)
    // }

    fn write_u8(&mut self, offset: usize, data: u8) {
        // break if we hit a breakpoint
        if self.is_offset_at_breakpoint(offset) {
            self.fatal_error = true;
            println!("Breakpoint (memory write to {:06X} = {:02X}), ip = {:04X}:{:04X}",
                offset,
                data,
                self.sreg16[CS],
                self.ip);
        }

        self.mmu.write_byte_flat(offset, data);
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
                self.mmu.read_u8(self.segment(seg), imm) as usize
            }
            Parameter::Ptr16(seg, imm) => {
                self.mmu.read_u16(self.segment(seg), imm) as usize
            }
            Parameter::Ptr8Amode(seg, r) => {
                let seg = self.segment(seg);
                let offset = self.amode16(r);
                self.mmu.read_u8(seg, offset) as usize
            }
            Parameter::Ptr8AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(self.amode16(r))
                              + Wrapping(imm as u16)).0;
                let seg = self.segment(seg);
                self.mmu.read_u8(seg, offset) as usize
            }
            Parameter::Ptr8AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(self.amode16(r))
                              + Wrapping(imm as u16)).0;
                let seg = self.segment(seg);
                self.mmu.read_u8(seg, offset) as usize
            }
            Parameter::Ptr16Amode(seg, r) => {
                let seg = self.segment(seg);
                let offset = self.amode16(r);
                self.mmu.read_u16(seg, offset) as usize
            }
            Parameter::Ptr16AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(self.amode16(r))
                              + Wrapping(imm as u16)).0;
                let seg = self.segment(seg);
                self.mmu.read_u16(seg, offset) as usize
            }
            Parameter::Ptr16AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(self.amode16(r))
                              + Wrapping(imm as u16)).0;
                let seg = self.segment(seg);
                self.mmu.read_u16(seg, offset) as usize
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

    fn amode16(&self, idx: usize) -> u16 {
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
        let val = self.mmu.read_u8(self.sreg16[DS], self.r16[SI].val);
        let port = self.r16[DX].val;
        self.out_u8(port, val);

        self.r16[SI].val = if !self.flags.direction {
            (Wrapping(self.r16[SI].val) + Wrapping(1)).0
        } else {
            (Wrapping(self.r16[SI].val) - Wrapping(1)).0
        };
    }

    fn movsb(&mut self) {
        let b = self.mmu.read_u8(self.sreg16[DS], self.r16[SI].val);
        self.r16[SI].val = if !self.flags.direction {
            (Wrapping(self.r16[SI].val) + Wrapping(1)).0
        } else {
            (Wrapping(self.r16[SI].val) - Wrapping(1)).0
        };
        self.mmu.write_u8(self.sreg16[ES], self.r16[DI].val, b);
        self.r16[DI].val = if !self.flags.direction {
            (Wrapping(self.r16[DI].val) + Wrapping(1)).0
        } else {
            (Wrapping(self.r16[DI].val) - Wrapping(1)).0
        };
    }

    // used for MOVSW
    fn movsw(&mut self) {
        let b = self.mmu.read_u16(self.sreg16[DS], self.r16[SI].val);
        self.r16[SI].val = if !self.flags.direction {
            (Wrapping(self.r16[SI].val) + Wrapping(2)).0
        } else {
            (Wrapping(self.r16[SI].val) - Wrapping(2)).0
        };
        self.mmu.write_u16(self.sreg16[ES], self.r16[DI].val, b);
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
