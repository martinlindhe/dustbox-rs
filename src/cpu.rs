#[allow(unused_imports)]

use test::Bencher;
use std::{mem, u8};
use std::num::Wrapping;

use register::Register16;
use flags::Flags;
use memory::Memory;
use segment::Segment;
use instruction::{Instruction, InstructionInfo, Parameter, ParameterPair, Op, ModRegRm};
use int10;
use int16;
use int21;
use gpu::GPU;
use register::{AX, BX, CX, DX, SI, DI, BP, SP, AL, CL, CS, DS, ES, FS, GS, SS};

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
}

impl CPU {
    pub fn new() -> Self {
        let mut cpu = CPU {
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
        };

        // intializes the cpu as if to run .com programs, info from
        // http://www.delorie.com/djgpp/doc/rbinter/id/51/29.html

        // offset of last word available in first 64k segment
        cpu.r16[SP].val = 0xFFFE;

        cpu
    }

    pub fn add_breakpoint(&mut self, bp: usize) {
        self.breakpoints.push(bp);
    }

    pub fn get_breakpoints(&self) -> Vec<usize> {
        self.breakpoints.clone()
    }

    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    pub fn reset(&mut self) {
        self.ip = 0;
        self.instruction_count = 0;
        // XXX clear memory
    }

    pub fn load_bios(&mut self, data: &[u8]) {
        self.sreg16[CS] = 0xF000;
        self.ip = 0x0000;
        let min = 0xF0000;
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

        self.ip = 0x100;
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

        res += format!("AX:{:04X}  SI:{:04X}  DS:{:04X}  IP:{:04X}\n",
                       self.r16[AX].val,
                       self.r16[SI].val,
                       self.sreg16[DS],
                       self.ip)
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
        res += format!("DX:{:04X}  SP:{:04X}  FS:{:04X}  SS:{:04X}",
                       self.r16[DX].val,
                       self.r16[SP].val,
                       self.sreg16[FS],
                       self.sreg16[SS])
                .as_ref();

        res
    }

    pub fn execute_instruction(&mut self) {
        let op = self.decode_instruction(Segment::CS());
        match op.command {
            Op::Unknown() => {
                self.fatal_error = true;
                println!("unknown op, {} instructions executed",
                         self.instruction_count);
            }
            _ => self.execute(&op),
        }

        // XXX need instruction timing to do this properly
        if self.instruction_count % 100 == 0 {
            self.gpu.progress_scanline();
        }
    }

    pub fn disassemble_block(&mut self, origin: u16, count: usize) -> String {
        let old_ip = self.ip;
        self.ip = origin as u16;
        let mut res = String::new();

        for _ in 0..count {
            let op = self.disasm_instruction();
            res.push_str(&op.pretty_string());
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
        let offset = (self.sreg16[CS] as usize * 16) + old_ip as usize;

        InstructionInfo {
            segment: self.sreg16[CS] as usize,
            offset: old_ip as usize,
            length: length as usize,
            text: format!("{}", op),
            bytes: self.read_u8_slice(offset, length as usize),
            instruction: op,
        }
    }

    fn execute(&mut self, op: &Instruction) {
        self.instruction_count += 1;
        match op.command {
            Op::Aas() => {
                // ASCII Adjust AL After Subtraction
                println!("XXX impl aas");
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
                println!("XXX impl arpl");
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
                // XXX identical to Op::Sub16() except we dont use the result
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
                println!("XXX impl cwd");
            }
            Op::Daa() => {
                // Decimal Adjust AL after Addition
                println!("XXX impl daa");
                // XXX there is examples in manual that can be made into tests
            }
            Op::Das() => {
                // Decimal Adjust AL after Subtraction
                println!("XXX impl das");
                // XXX there is examples in manual that can be made into tests
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
                println!("XXX impl hlt");
            }
            Op::Idiv16() => {
                println!("XXX impl idiv16: {}", op);
            }
            Op::Imul8() => {
                // NOTE: only 1-parameter imul8 instruction exists
                // IMUL r/m8               : AX← AL ∗ r/m byte.
                let dst = self.read_parameter_value(&op.params.dst) as i8;
                let tmp = (self.r16[AX].lo_u8() as i8) as isize * dst as isize;
                self.r16[AX].val = tmp as u16;

                println!("XXX imul8 impl carry & overflow flag");
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
                    let dst = self.read_parameter_value(&op.params.dst) as i16;
                    let tmp = (self.r16[AX].val as i16) as isize * dst as isize;
                    self.r16[AX].val = tmp as u16;
                    self.r16[DX].val = (tmp >> 16) as u16;
                } else {
                    // IMUL r16, r/m16          : word register ← word register ∗ r/m16.
                    // IMUL r16, r/m16, imm8    : word register ← r/m16 ∗ sign-extended immediate byte.
                    // IMUL r16, r/m16, imm16   : word register ← r/m16 ∗ immediate word.
                    println!("XXX impl imul16 with multiple parameters: {}", op);
                }

                println!("XXX imul16 impl carry & overflow flag");
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
                println!("XXX impl insb");
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
            Op::Les() => {
                println!("XXX imp les");
            }
            Op::Lodsb() => {
                // no arguments
                // Load byte at address DS:(E)SI into AL.
                let offset = (self.sreg16[DS] as usize * 16) + (self.r16[SI].val as usize);
                let val = self.peek_u8_at(offset);
                self.r16[AX].set_lo(val); // AL =
                self.r16[SI].val = if !self.flags.direction {
                    (Wrapping(self.r16[SI].val) + Wrapping(1)).0
                } else {
                    (Wrapping(self.r16[SI].val) - Wrapping(1)).0
                };
            }
            Op::Lodsw() => {
                println!("XXX impl lodsw");
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
                println!("XXX impl movsb");
            }
            Op::Movsw() => {
                println!("XXX impl movsw");
            }
            Op::Movzx16() => {
                // Move with Zero-Extend
                // two arguments (dst=reg)
                let src = self.read_parameter_value(&op.params.src) as u8;
                let mut data = src as u16;
                if src & 0x80 != 0 {
                    data = 0xFF00 + data;
                }
                self.write_parameter_u16(op.segment, &op.params.dst, data);
            }
            Op::Mul8() => {
                // dst = AX
                let src = self.r16[AX].lo_u8() as usize; // AL
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) * Wrapping(src)).0;

                // The OF and CF flags are set to 0 if the upper half of the
                // result is 0; otherwise, they are set to 1.
                // The SF, ZF, AF, and PF flags are undefined.
                println!("XXX mul8 flags");

                self.r16[AX].val = (res & 0xFFFF) as u16;
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
                // two arguments (dst=DX or imm8)
                let data = self.read_parameter_value(&op.params.src) as u8;
                self.out_u8(&op.params.dst, data);
            }
            Op::Out16() => {
                println!("XXX impl out16");
            }
            Op::Outsb() => {
                // no arguments
                println!("XXX impl outs byte");
            }
            Op::Outsw() => {
                // no arguments
                println!("XXX impl outs word");
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
                println!("XXX push flags: {:04X}", data);
                self.push16(data);
            }
            Op::Rcl8() => {
                // two arguments
                // rotate 9 bits `src` times
                println!("XXX impl rcl8");
            }
            Op::Rcl16() => {
                // two arguments
                println!("XXX impl rcl16");
            }
            Op::Rcr8() => {
                // two arguments
                // rotate 9 bits right `src` times

                // The RCR instruction shifts the CF flag into the most-significant
                // bit and shifts the least-significant bit into the CF flag.
                // The OF flag is affected only for single-bit rotates; it is undefined
                // for multi-bit rotates. The SF, ZF, AF, and PF flags are not affected.
                let dst = self.read_parameter_value(&op.params.dst);
                let count = self.read_parameter_value(&op.params.src);

                // shift least-significant bit into the CF flag.
                let new_carry = (dst & 1) != 0;
                let msb = if self.flags.carry {
                    0x80
                } else {
                    0
                };
                let res = msb | (dst >> count);
                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
                self.flags.carry = new_carry;
                if count  == 1 {
                    println!("XXX rcr8 OF flag");
                    // IF (COUNT & COUNTMASK) = 1
                    // THEN OF ← MSB(DEST) XOR CF;
                    // ELSE OF is undefined;
                    // FI
                }
            }
            Op::Rcr16() => {
                println!("XXX impl rcr16");
            }
            Op::RepMovsb() => {
                // Move (E)CX bytes from DS:[(E)SI] to ES:[(E)DI].
                let mut src = (self.sreg16[DS] as usize * 16) + (self.r16[SI].val as usize);
                let mut dst = (self.sreg16[ES] as usize * 16) + (self.r16[DI].val as usize);
                let count = self.r16[CX].val as usize;
                println!("rep movsb   src = {:04X}, dst = {:04X}, count = {:04X}",
                         src,
                         dst,
                         count);
                loop {
                    let b = self.peek_u8_at(src);
                    src += 1;
                    // println!("rep movsb   write {:02X} to {:04X}", b, dst);
                    self.write_u8(dst, b);
                    dst += 1;
                    self.r16[CX].val -= 1;
                    if self.r16[CX].val == 0 {
                        break;
                    }
                }
            }
            Op::RepMovsw() => {
                // Move (E)CX bytes from DS:[(E)SI] to ES:[(E)DI].
                let mut src = (self.sreg16[DS] as usize * 16) + (self.r16[SI].val as usize);
                let mut dst = (self.sreg16[ES] as usize * 16) + (self.r16[DI].val as usize);
                println!("rep movsw   src = {:04X}, dst = {:04X}, count = {:04X}",
                         src,
                         dst,
                         self.r16[CX].val);
                loop {
                    let b = self.peek_u16_at(src);
                    src += 1;
                    // println!("rep movsb   write {:02X} to {:04X}", b, dst);
                    self.write_u16(dst, b);
                    dst += 1;

                    let res = (Wrapping(self.r16[CX].val) - Wrapping(1)).0;
                    self.r16[CX].val = res;
                    if res == 0 {
                        break;
                    }
                }
            }
            Op::RepOutsb() => {
                println!("XXX impl rep outsb");
            }
            Op::RepStosb() => {
                // Fill (E)CX bytes at ES:[(E)DI] with AL.

                let data = self.r16[AX].lo_u8(); // = AL

                /*
                println!("rep stosb   dst = {:04X}:{:04X}, count = {:04X}, data = {:02X}",
                         self.sreg16[ES] as usize,
                         self.r16[DI].val as usize,
                         self.r16[CX].val,
                         data);
                */
                loop {
                    let dst = (self.sreg16[ES] as usize * 16) + (self.r16[DI].val as usize);
                    self.write_u8(dst, data);
                    self.r16[DI].val = if !self.flags.direction {
                        (Wrapping(self.r16[DI].val) + Wrapping(1)).0
                    } else {
                        (Wrapping(self.r16[DI].val) - Wrapping(1)).0
                    };

                    let res = (Wrapping(self.r16[CX].val) - Wrapping(1)).0;
                    self.r16[CX].val = res;
                    if res == 0 {
                        break;
                    }
                }
            }
            Op::RepStosw() => {
                // Fill (E)CX words at ES:[(E)DI] with AX.
                // println!("XXX impl rep stos word");

                let data = self.r16[AX].val; // = AX

                println!("rep stosw   dst = {:04X}:{:04X}, count = {:04X}, data = {:04X}",
                         self.sreg16[ES] as usize,
                         self.r16[DI].val as usize,
                         self.r16[CX].val,
                         data);

                loop {
                    let dst = (self.sreg16[ES] as usize * 16) + (self.r16[DI].val as usize);
                    self.write_u16(dst, data);

                    self.r16[DI].val = if !self.flags.direction {
                        (Wrapping(self.r16[DI].val) + Wrapping(2)).0
                    } else {
                        (Wrapping(self.r16[DI].val) - Wrapping(2)).0
                    };

                    let res = (Wrapping(self.r16[CX].val) - Wrapping(1)).0;
                    self.r16[CX].val = res;
                    if res == 0 {
                        break;
                    }
                }
            }
            Op::RepneScasb() => {
                // Find AL, starting at ES:[(E)DI].
                println!("XXX impl repne scas byte");
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
            Op::Rol16() => {
                // two arguments
                println!("XXX impl rol16");
                // XXX flags
            }
            Op::Ror8() => {
                // two arguments
                println!("XXX impl ror8");
                // XXX flags
            }
            Op::Ror16() => {
                // two arguments
                println!("XXX impl ror16");
                // XXX flags
            }
            Op::Sahf() => {
                // Store AH into Flags
                println!("XXX impl sahf");
            }
            Op::Sar8() => {
                println!("XXX impl sar8");
            }
            Op::Sar16() => {
                // Signed divide* r/m8 by 2, imm8 times.
                // two arguments

                let dst = self.read_parameter_value(&op.params.dst);
                let count = self.read_parameter_value(&op.params.src);

                let res = dst >> count;
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
            Op::Shl8() => {
                // Multiply `dst` by 2, `src` times.
                // two arguments    (alias: sal)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst << src;
                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);

                // XXX flags
                println!("XXX shl8 flags");
                self.flags.carry = if res & 0x100 != 0 { true } else { false };
                //XXX overflow: OF ← MSB(DEST) XOR CF;
                // The OF flag is affected only for 1-bit shifts (see “Description” above);
                // otherwise, it is undefined. The SF, ZF, and PF flags are set according to
                // the result. If the count is 0, the flags are not affected. For a non-zero
                // count, the AF flag is undefined.
            }
            Op::Shl16() => {
                // Multiply `dst` by 2, `src` times.
                // two arguments    (alias: sal)

                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst << src;
                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);

                println!("XXX shl16 flags");
                // XXX flags
            }
            Op::Shr8() => {
                // Unsigned divide r/m8 by 2, `src` times.
                // two arguments
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);

                let res = dst >> src;
                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);

                // XXX flags
                println!("XXX shr8 flags");
            }
            Op::Shr16() => {
                // two arguments
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);

                let res = dst >> src;
                self.write_parameter_u16(op.segment, &op.params.dst, (res & 0xFFFF) as u16);

                println!("XXX shr16 flags");
                // XXX flags
            }
            Op::Shrd() => {
                println!("XXX impl shrd");
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
                let offset = (self.sreg16[ES] as usize * 16) + (self.r16[DI].val as usize);
                let data = self.r16[AX].lo_u8(); // = AL
                self.write_u8(offset, data);
                self.r16[DI].val = if !self.flags.direction {
                    (Wrapping(self.r16[DI].val) + Wrapping(1)).0
                } else {
                    (Wrapping(self.r16[DI].val) - Wrapping(1)).0
                };
            }
            Op::Stosw() => {
                // no parameters
                // store AX at address ES:(E)DI
                let offset = (self.sreg16[ES] as usize * 16) + (self.r16[DI].val as usize);
                let data = self.r16[AX].val;
                self.write_u16(offset, data);
                self.r16[DI].val = if !self.flags.direction {
                    (Wrapping(self.r16[DI].val) + Wrapping(2)).0
                } else {
                    (Wrapping(self.r16[DI].val) - Wrapping(2)).0
                };
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
                    _ => {
                        println!("op 0F error: unknown {:02X}", b);
                    }
                }
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
                // insb
                op.command = Op::Insb();
            }
            0x6E => {
                // outs byte
                op.command = Op::Outsb();
            }
            0x6F => {
                // outs word
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
                    /*
                    2 => {
                        op.command = Op::Adc16();
                    }
                    3 => {
                        op.command = Op::Sbb16();
                    }
                    */
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
                    _ => {
                        println!("op 81 error: unknown reg {}", x.reg);
                    }
                }
            }
            0x83 => {
                // arithmetic 16-bit with signed 8-bit value
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::ImmS8(self.read_s8());
                match x.reg {
                    0 => {
                        op.command = Op::Add16();
                    }
                    /*
                    1 => {
                        op.command = Op::Or16();
                    }
                    2 => {
                        op.command = Op::Adc16();
                    }
                    3 => {
                        op.command = Op::Sbb16();
                    }
                    */
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
                    _ => {
                        println!("op 83 error: unknown reg {}", x.reg);
                    }
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
                        println!("op 8F unknown reg = {}", x.reg);
                    }
                }
            }
            0x90 => {
                // nop
                op.command = Op::Nop();
            }
            0x91...0x97 => {
                // xchg AX, r16  | xchg r16, AX
                // NOTE:  ("xchg ax,ax" is an alias of "nop")
                op.command = Op::Xchg16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Reg16((b & 7) as usize);
            }
            0x98 => {
                // cbw
                op.command = Op::Cbw();
            }
            0x99 => {
                // cwd
                op.command = Op::Cwd();
            }
            0x9C => {
                // pushf
                op.command = Op::Pushf();
            }
            0x9D => {
                // popf
                op.command = Op::Popf();
            }
            0x9E => {
                // sahf
                op.command = Op::Sahf();
            }
            0xA0 => {
                // mov AL, moffs8
                op.command = Op::Mov8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Ptr8(op.segment, self.read_u16());
            }
            0xA1 => {
                // MOV AX, moffs16
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
                // movsb
                op.command = Op::Movsb();
            }
            0xA5 => {
                // movsw
                op.command = Op::Movsw();
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
                // stosb
                op.command = Op::Stosb();
            }
            0xAB => {
                // stosw
                op.command = Op::Stosw();
            }
            0xAC => {
                // lodsb
                op.command = Op::Lodsb();
            }
            0xAD => {
                // lodsw
                op.command = Op::Lodsw();
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
                    /*
                    0 => {
                        op.Cmd = "rol"
                    }
                    1 => {
                        op.Cmd = "ror"
                    }
                    2 => {
                        op.Cmd = "rcl"
                    }
                    3 => {
                        op.Cmd = "rcr"
                    }
                    */
                    4 => Op::Shl8(),
                    5 => Op::Shr8(),
                    7 => Op::Sar8(),
                    _ => {
                        println!("XXX 0xC0 unhandled reg = {}", x.reg);
                        Op::Unknown()
                    }
                };
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0xC1 => {
                // r16, byte imm8
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    /*
                    0 => {
                        op.Cmd = "rol"
                    }
                    */
                    1 => Op::Ror16(),
                    2 => Op::Rcl16(),
                    /*
                    3 => {
                        op.Cmd = "rcr"
                    }
                    */
                    4 => Op::Shl16(),
                    5 => Op::Shr16(),
                    7 => Op::Sar16(),
                    _ => {
                        println!("XXX 0xC1 unhandled reg = {}", x.reg);
                        Op::Unknown()
                    }
                };
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0xC3 => {
                // ret [near]
                op.command = Op::Retn();
            }
            0xC4 => {
                // les r16, m16
                op.command = Op::Les();
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
                        println!("op C6 unknown reg = {}", x.reg);
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
                        println!("op C7 unknown reg = {}", x.reg);
                    }
                }
            }
            0xCB => {
                // retf
                op.command = Op::Retf();
            }
            0xCD => {
                // int imm8
                op.command = Op::Int();
                op.params.dst = Parameter::Imm8(self.read_u8());
            }
            0xD0 => {
                // bit shift byte by 1
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    // 0 => Op::Rol8(),
                    1 => Op::Ror8(),
                    2 => Op::Rcl8(),
                    3 => Op::Rcr8(),
                    4 => Op::Shl8(),
                    5 => Op::Shr8(),
                    7 => Op::Sar8(),
                    _ => {
                        println!("XXX 0xD0 unhandled reg = {}", x.reg);
                        Op::Unknown()
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
                        println!("XXX 0xD1 unhandled reg = {}", x.reg);
                        Op::Unknown()
                    }
                };
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm16(1);
            }
            0xD2 => {
                // bit shift byte by CL
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    // 0 => Op::Rol8(),
                    //1 => Op::Ror8(),
                    //2 => Op::Rcl8(),
                    //3 => Op::Rcr8(),
                    4 => Op::Shl8(),
                    5 => Op::Shr8(),
                    // 7 => Op::Sar8(),
                    _ => {
                        println!("XXX 0xD2 unhandled reg = {}", x.reg);
                        Op::Unknown()
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
                    //2 => Op::Rcl16(),
                    //3 => Op::Rcr16(),
                    4 => Op::Shl16(),
                    5 => Op::Shr16(),
                    //7 => Op::Sar16(),
                    _ => {
                        println!("XXX 0xD3 unhandled reg = {}", x.reg);
                        Op::Unknown()
                    }
                };
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Reg8(CL);
            }
            0xE2 => {
                // loop rel8
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
            0xE6 => {
                // OUT imm8, AL
                op.command = Op::Out8();
                op.params.dst = Parameter::Imm8(self.read_u8());
                op.params.src = Parameter::Reg8(AL);
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
                        // XXXX intel dec-2016 manual says "REP OUTS DX, r/m8" which seems wrong?
                        // rep outs byte
                        op.command = Op::RepOutsb();
                    }
                    0xA4 => {
                        // rep movs byte
                        op.command = Op::RepMovsb();
                    }
                    0xA5 => {
                        // rep movs word
                        op.command = Op::RepMovsw();
                    }
                    0xAA => {
                        // rep stos byte
                        op.command = Op::RepStosb();
                    }
                    0xAB => {
                        // rep stos word
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
                    0 => {
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
                    // 7 => op.Cmd = "idiv"
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
                    0 => {
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
                        op.command = Op::Mul8(); // XXX mul16!?
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
                    0 => {
                        op.command = Op::Inc8();
                    }
                    1 => {
                        op.command = Op::Dec8();
                    }
                    _ => {
                        println!("op FE error: unknown reg {}", x.reg);
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
                        println!("op FF error: unknown reg {}", x.reg);
                    }
                }
            }
            _ => {
                println!("cpu: unknown op {:02X} at {:04X}:{:04X} ({} instructions executed)",
                         b,
                         self.sreg16[CS],
                         self.ip - 1,
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
        let offset = (self.sreg16[SS] as usize * 16) + (self.r16[SP].val as usize);
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
        let offset = (self.sreg16[SS] as usize * 16) + (self.r16[SP].val as usize);
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
        let offset = (self.sreg16[SS] as usize * 16) + (self.r16[SP].val as usize);
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
        (self.sreg16[CS] as usize * 16) + self.ip as usize
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
        (hi as u16) << 8 | lo as u16
    }

    fn read_s8(&mut self) -> i8 {
        self.read_u8() as i8
    }

    fn read_s16(&mut self) -> i16 {
        self.read_u16() as i16
    }

    fn read_rel8(&mut self) -> u16 {
        let val = self.read_u8() as i8;
        (self.ip as i16 + (val as i16)) as u16
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
        (hi as u16) << 8 | lo as u16
    }

    fn write_u16(&mut self, offset: usize, data: u16) {
        // println!("write_u16 [{:04X}] = {:04X}", offset, data);
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.write_u8(offset, lo);
        self.write_u8(offset + 1, hi);
    }

    // returns the offset part, excluding segment. used by LEA
    fn read_parameter_address(&mut self, p: &Parameter) -> usize {
        match *p {
            Parameter::Ptr16AmodeS8(_, r, imm) => self.amode16(r) + imm as usize,
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
                let offset = (self.segment(seg) as usize * 16) + imm as usize;
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr16(seg, imm) => {
                let offset = (self.segment(seg) as usize * 16) + imm as usize;
                self.peek_u16_at(offset) as usize
            }
            Parameter::Ptr8Amode(seg, r) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r);
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr8AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr8AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr16Amode(seg, r) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r);
                self.peek_u16_at(offset) as usize
            }
            Parameter::Ptr16AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.peek_u16_at(offset) as usize
            }
            Parameter::Ptr16AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
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
                let offset = (self.segment(seg) as usize * 16) + imm as usize;
                self.write_u8(offset, data);
            }
            Parameter::Ptr8Amode(seg, r) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r);
                self.write_u8(offset, data);
            }
            Parameter::Ptr8AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.write_u8(offset, data);
            }
            Parameter::Ptr8AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
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
                let offset = (self.segment(segment) as usize * 16) + imm as usize;
                self.write_u16(offset, data);
            }
            Parameter::Ptr16(seg, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) + Wrapping(imm as usize)).0;
                self.write_u16(offset, data);
            }
            Parameter::Ptr16Amode(seg, r) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r);
                self.write_u16(offset, data);
            }
            Parameter::Ptr16AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.write_u16(offset, data);
            }
            Parameter::Ptr16AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.write_u16(offset, data);
            }
            _ => {
                println!("write_u16_param unhandled type {:?} at {:06X}",
                         p,
                         self.get_offset());
            }
        }
    }

    fn write_u8(&mut self, offset: usize, data: u8) {
        // println!("debug: write_u8 to {:06X} = {:02X}", offset, data);
        self.memory.memory[offset] = data;
    }

    // used by disassembler
    pub fn read_u8_slice(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let mut res = vec![0u8; length];
        for i in offset..offset + length {
            res[i - offset] = self.memory.memory[i];
        }
        res
    }

    fn segment(&self, seg: Segment) -> u16 {
        match seg {
            Segment::CS() |
            Segment::Default() => self.sreg16[CS],
            Segment::DS() => self.sreg16[DS],
            Segment::ES() => self.sreg16[ES],
            Segment::FS() => self.sreg16[FS],
            Segment::GS() => self.sreg16[GS],
            Segment::SS() => self.sreg16[SS],
        }
    }

    fn amode16(&mut self, idx: usize) -> usize {
        match idx {
            0 => self.r16[BX].val as usize + self.r16[SI].val as usize,
            1 => self.r16[BX].val as usize + self.r16[DI].val as usize,
            2 => self.r16[BP].val as usize + self.r16[SI].val as usize,
            3 => self.r16[BP].val as usize + self.r16[DI].val as usize,
            4 => self.r16[SI].val as usize,
            5 => self.r16[DI].val as usize,
            6 => self.r16[BP].val as usize,
            7 => self.r16[BX].val as usize,
            _ => {
                println!("Impossible amode16, idx {}", idx);
                0
            }
        }
    }

    // output byte to I/O port
    fn out_u8(&mut self, p: &Parameter, data: u8) {
        let dst = match *p {
            Parameter::Reg16(r) => self.r16[r].val,
            Parameter::Imm8(imm) => imm as u16,
            _ => {
                println!("out_u8 unhandled type {:?}", p);
                0
            }
        };

        match dst {
            0x03C8 => {
                // (VGA,MCGA) PEL address register
                // Sets DAC in write mode and assign start of color register
                // index (0..255) for following write accesses to 3C9h.
                // Next access to 03C8h will stop pending mode immediatly.
                self.gpu.dac_index = data;
                println!("dac index = {}", data);
            }
            0x03C9 => {
                // (VGA,MCGA) PEL data register
                // Three consequtive writes in the order: red, green, blue.
                // The internal DAC index is incremented each 3rd access.
                if self.gpu.dac_color > 2 {
                    let i = self.gpu.dac_index as usize;
                    self.gpu.palette[i].r = self.gpu.dac_current_palette[0];
                    self.gpu.palette[i].g = self.gpu.dac_current_palette[1];
                    self.gpu.palette[i].b = self.gpu.dac_current_palette[2];

                    println!("DAC palette {} = {}, {}, {}",
                             self.gpu.dac_index,
                             self.gpu.palette[i].r,
                             self.gpu.palette[i].g,
                             self.gpu.palette[i].b);

                    self.gpu.dac_color = 0;
                    self.gpu.dac_index = (Wrapping(self.gpu.dac_index) + Wrapping(1)).0;
                }
                // map 6-bit color into 8 bits
                self.gpu.dac_current_palette[self.gpu.dac_color] = data << 2;
                self.gpu.dac_color += 1;
            }
            _ => {
                println!("XXX unhandled out_u8 to {:04X}, data {:02X}", dst, data);
            }
        }
    }

    // read byte from I/O port
    fn in_port(&mut self, port: u16) -> u8 {
        match port {
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

                // HACK: fake bit 0:
                if self.gpu.scanline == 0 {
                    flags |= 1; // set bit 0
                } else {
                    flags &= !(1 << 1); // clear bit 0
                }
                /*
                println!("XXX read io port CGA status register at {:06X} = {:02X}",
                         self.get_offset(),
                         flags);
                */
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
        // http://wiki.osdev.org/Interrupt_Vector_Table
        match int {
            0x10 => int10::handle(self),
            0x16 => int16::handle(self),
            0x20 => {
                // DOS 1+ - TERMINATE PROGRAM
                // NOTE: Windows overloads INT 20
                println!("INT 20 - Terminating program");
                self.fatal_error = true; // XXX just to stop debugger.run() function
            }
            0x21 => int21::handle(self),
            _ => {
                println!("int error: unknown interrupt {:02X}, AX={:04X}, BX={:04X}",
                         int,
                         self.r16[AX].val,
                         self.r16[BX].val);
            }
        }
    }
}

#[test]
fn can_handle_stack() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x88, 0x88, // mov ax,0x8888
        0x8E, 0xD8,       // mov ds,ax
        0x1E,             // push ds
        0x07,             // pop es
    ];
    cpu.load_com(&code);

    cpu.execute_instruction(); // mov
    cpu.execute_instruction(); // mov

    assert_eq!(0xFFFE, cpu.r16[SP].val);
    cpu.execute_instruction(); // push
    assert_eq!(0xFFFC, cpu.r16[SP].val);
    cpu.execute_instruction(); // pop
    assert_eq!(0xFFFE, cpu.r16[SP].val);

    assert_eq!(0x107, cpu.ip);
    assert_eq!(0x8888, cpu.r16[AX].val);
    assert_eq!(0x8888, cpu.sreg16[DS]);
    assert_eq!(0x8888, cpu.sreg16[ES]);
}

#[test]
fn can_execute_mov_r8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB2, 0x13, // mov dl,0x13
        0x88, 0xD0, // mov al,dl
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x102, cpu.ip);
    assert_eq!(0x13, cpu.r16[DX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x104, cpu.ip);
    assert_eq!(0x13, cpu.r16[AX].lo_u8());
}

#[test]
fn can_execute_mov_r8_rm8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x05, 0x01, // mov bx,0x105
        0x8A, 0x27,       // mov ah,[bx]   | r8, r/m8
        0x99,             // db 0x99
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x105, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x99, cpu.r16[AX].hi_u8());
}

#[test]
fn can_execute_mv_r16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x23, 0x01, // mov ax,0x123
        0x8B, 0xE0,       // mov sp,ax   | r16, r16
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x123, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x123, cpu.r16[SP].val);
}

#[test]
fn can_execute_mov_r16_rm16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB9, 0x23, 0x01, // mov cx,0x123
        0x8E, 0xC1,       // mov es,cx   | r/m16, r16
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x123, cpu.r16[CX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x123, cpu.sreg16[ES]);
}

#[test]
fn can_execute_mov_rm16_sreg() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x34, 0x12,       // mov bx,0x1234
        0x8E, 0xC3,             // mov es,bx
        0x8C, 0x06, 0x09, 0x01, // mov [0x109],es  | r/m16, sreg
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x1234, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x1234, cpu.sreg16[ES]);

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);
    let cs = cpu.sreg16[CS] as usize;
    assert_eq!(0x1234, cpu.peek_u16_at((cs * 16) + 0x0109));
}

#[test]
fn can_execute_mov_data() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xC6, 0x06, 0x31, 0x10, 0x38,       // mov byte [0x1031],0x38
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    let cs = cpu.sreg16[CS] as usize;
    assert_eq!(0x38, cpu.peek_u8_at((cs * 16) + 0x1031));
}

#[test]
fn can_execute_segment_prefixed() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x34, 0x12, // mov bx,0x1234
        0x8E, 0xC3,       // mov es,bx
        0xB4, 0x88,       // mov ah,0x88
        0x26, 0x88, 0x25, // mov [es:di],ah
        0x26, 0x8A, 0x05, // mov al,[es:di]
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x1234, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x1234, cpu.sreg16[ES]);

    cpu.execute_instruction();
    assert_eq!(0x107, cpu.ip);
    assert_eq!(0x88, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0x10A, cpu.ip);
    let offset = (cpu.segment(Segment::ES()) as usize * 16) + cpu.amode16(5); // 5=amode DI
    assert_eq!(0x88, cpu.peek_u8_at(offset));

    cpu.execute_instruction();
    assert_eq!(0x10D, cpu.ip);
    assert_eq!(0x88, cpu.r16[AX].lo_u8());
}

#[test]
fn can_execute_imms8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBF, 0x00, 0x01, // mov di,0x100
        0x83, 0xC7, 0x3A, // add di,byte +0x3a
        0x83, 0xC7, 0xC6, // add di,byte -0x3a
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x0100, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x106, cpu.ip);
    assert_eq!(0x013A, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);
    assert_eq!(0x0100, cpu.r16[DI].val);
}

#[test]
fn can_execute_with_flags() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB4, 0xFE,       // mov ah,0xfe
        0x80, 0xC4, 0x02, // add ah,0x2   - OF and ZF should be set
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x102, cpu.ip);
    assert_eq!(0xFE, cpu.r16[AX].hi_u8());
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(false, cpu.flags.auxiliary_carry);
    assert_eq!(false, cpu.flags.parity);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x00, cpu.r16[AX].hi_u8());
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.parity);
}

#[test]
fn can_execute_cmp() {
    // make sure we dont overflow (0 - 0x2000 = overflow)
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x00,       // mov bx,0x0
        0x89, 0xDF,             // mov di,bx
        0x81, 0xFF, 0x00, 0x20, // cmp di,0x2000
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);

    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(false, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.parity);
}

#[test]
fn can_execute_xchg() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x91, // xchg ax,cx
    ];

    cpu.load_com(&code);

    cpu.r16[AX].val = 0x1234;
    cpu.r16[CX].val = 0xFFFF;

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.r16[AX].val);
    assert_eq!(0x1234, cpu.r16[CX].val);
}

#[test]
fn can_execute_rep() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        // copy first 5 bytes into 0x200
        0x8D, 0x36, 0x00, 0x01, // lea si,[0x100]
        0x8D, 0x3E, 0x00, 0x02, // lea di,[0x200]
        0xB9, 0x05, 0x00,       // mov cx,0x5
        0xF3, 0xA4,             // rep movsb
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x100, cpu.r16[SI].val);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x5, cpu.r16[CX].val);

    cpu.execute_instruction(); // rep movsb
    assert_eq!(0x0, cpu.r16[CX].val);
    let min = (cpu.sreg16[CS] as usize * 16) + 0x100;
    let max = min + 5;
    for i in min..max {
        assert_eq!(cpu.memory.memory[i], cpu.memory.memory[i + 0x100]);
    }
}

#[test]
fn can_execute_addressing() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x02,             // mov bx,0x200
        0xC6, 0x47, 0x2C, 0xFF,       // mov byte [bx+0x2c],0xff  | rm8 [amode+s8]
        0x8D, 0x36, 0x00, 0x01,       // lea si,[0x100]
        0x8B, 0x14,                   // mov dx,[si]  | rm16 [reg]
        0x8B, 0x47, 0x2C,             // mov ax,[bx+0x2c]  | rm16 [amode+s8]
        0x89, 0x87, 0x30, 0x00,       // mov [bx+0x0030],ax  | rm [amode+s16]
        0x89, 0x05,                   // mov [di],ax  | rm16 [amode]
        0xC6, 0x85, 0xAE, 0x06, 0xFE, // mov byte [di+0x6ae],0xfe  | rm8 [amode+s16]
        0x8A, 0x85, 0xAE, 0x06,       // mov al,[di+0x6ae]
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 9);
    assert_eq!("[085F:0100] BB0002     Mov16    bx, 0x0200
[085F:0103] C6472CFF   Mov8     byte [bx+0x2C], 0xFF
[085F:0107] 8D360001   Lea16    si, word [0x0100]
[085F:010B] 8B14       Mov16    dx, word [si]
[085F:010D] 8B472C     Mov16    ax, word [bx+0x2C]
[085F:0110] 89873000   Mov16    word [bx+0x0030], ax
[085F:0114] 8905       Mov16    word [di], ax
[085F:0116] C685AE06FE Mov8     byte [di+0x06AE], 0xFE
[085F:011B] 8A85AE06   Mov8     al, byte [di+0x06AE]
",
               res);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.r16[BX].val);

    cpu.execute_instruction();
    let cs = cpu.sreg16[CS] as usize;
    assert_eq!(0xFF, cpu.peek_u8_at((cs * 16) + 0x22C));

    cpu.execute_instruction();
    assert_eq!(0x100, cpu.r16[SI].val);

    cpu.execute_instruction();
    // should have read word at [0x100]
    assert_eq!(0x00BB, cpu.r16[DX].val);

    cpu.execute_instruction();
    // should have read word at [0x22C]
    assert_eq!(0x00FF, cpu.r16[AX].val);

    cpu.execute_instruction();
    // should have written word to [0x230]
    assert_eq!(0x00FF, cpu.peek_u16_at((cs * 16) + 0x230));

    cpu.execute_instruction();
    // should have written ax to [di]
    let di = cpu.r16[DI].val as usize;
    assert_eq!(0x00FF, cpu.peek_u16_at((cs * 16) + di));

    cpu.execute_instruction();
    // should have written byte to [di+0x06AE]
    assert_eq!(0xFE, cpu.peek_u8_at((cs * 16) + di + 0x06AE));

    cpu.execute_instruction();
    // should have read byte from [di+0x06AE] to al
    assert_eq!(0xFE, cpu.r16[AX].lo_u8());
}

#[test]
fn can_execute_math() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xF6, 0x06, 0x2C, 0x12, 0xFF, // test byte [0x122c],0xff
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 1);
    assert_eq!("[085F:0100] F6062C12FF Test8    byte [0x122C], 0xFF
",
               res);

    // XXX also execute
}

#[test]
fn can_execute_and() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB0, 0xF0, // mov al,0xF0
        0xB4, 0x1F, // mov ah,0x1F
        0x20, 0xC4, // and ah,al
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 3);

    assert_eq!("[085F:0100] B0F0       Mov8     al, 0xF0
[085F:0102] B41F       Mov8     ah, 0x1F
[085F:0104] 20C4       And8     ah, al
",
               res);

    cpu.execute_instruction();
    assert_eq!(0xF0, cpu.r16[AX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x1F, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0x10, cpu.r16[AX].hi_u8());
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.parity);
}

#[test]
fn can_execute_mul() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB0, 0x40, // mov al,0x40
        0xB3, 0x10, // mov bl,0x10
        0xF6, 0xE3, // mul bl
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 3);

    assert_eq!("[085F:0100] B040       Mov8     al, 0x40
[085F:0102] B310       Mov8     bl, 0x10
[085F:0104] F6E3       Mul8     bl
",
               res);

    cpu.execute_instruction();
    assert_eq!(0x40, cpu.r16[AX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x10, cpu.r16[BX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x400, cpu.r16[AX].val);
    // XXX flags
}

#[test]
fn can_execute_div8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x40, 0x00, // mov ax,0x40
        0xB3, 0x10,       // mov bl,0x10
        0xF6, 0xF3,       // div bl
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 3);

    assert_eq!("[085F:0100] B84000     Mov16    ax, 0x0040
[085F:0103] B310       Mov8     bl, 0x10
[085F:0105] F6F3       Div8     bl
",
               res);

    cpu.execute_instruction();
    assert_eq!(0x40, cpu.r16[AX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x10, cpu.r16[BX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x04, cpu.r16[AX].lo_u8()); // quotient
    assert_eq!(0x00, cpu.r16[AX].hi_u8()); // remainder
}

#[test]
fn can_execute_div16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBA, 0x10, 0x00, // mov dx,0x10
        0xB8, 0x00, 0x40, // mov ax,0x4000
        0xBB, 0x00, 0x01, // mov bx,0x100
        0xF7, 0xF3,       // div bx
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 4);

    assert_eq!("[085F:0100] BA1000     Mov16    dx, 0x0010
[085F:0103] B80040     Mov16    ax, 0x4000
[085F:0106] BB0001     Mov16    bx, 0x0100
[085F:0109] F7F3       Div16    bx
",
               res);

    cpu.execute_instruction();
    assert_eq!(0x10, cpu.r16[DX].val);

    cpu.execute_instruction();
    assert_eq!(0x4000, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0x100, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x1040, cpu.r16[AX].val); // quotient
    assert_eq!(0x0000, cpu.r16[DX].val); // remainder
}

#[test]
fn can_execute_shr() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB6,0xF0, // mov dh,0xf0
        0xB1,0x05, // mov cl,0x5
        0xD2,0xEE, // shr dh,cl
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 3);

    assert_eq!("[085F:0100] B6F0       Mov8     dh, 0xF0
[085F:0102] B105       Mov8     cl, 0x05
[085F:0104] D2EE       Shr8     dh, cl
",
               res);

    cpu.execute_instruction();
    assert_eq!(0xF0, cpu.r16[DX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0x05, cpu.r16[CX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x07, cpu.r16[DX].hi_u8()); // == 7.5
}

#[test]
fn can_execute_dec() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBD, 0x00, 0x02, // mov bp,0x200
        0x4D,             // dec bp
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.r16[BP].val);

    cpu.execute_instruction();
    assert_eq!(0x1FF, cpu.r16[BP].val);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(true, cpu.flags.parity);
}

#[test]
fn can_execute_neg() {

    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x23, 0x01, // mov bx,0x123
        0xF7, 0xDB,       // neg bx
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 2);

    assert_eq!("[085F:0100] BB2301     Mov16    bx, 0x0123
[085F:0103] F7DB       Neg16    bx
",
               res);

    cpu.execute_instruction();
    assert_eq!(0x0123, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0xFEDD, cpu.r16[BX].val);
    // assert_eq!(true, cpu.flags.carry);  // XXX dosbox = TRUE
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.parity);
}

#[test]
fn can_execute_jmp_far() {

    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xEA, 0x00, 0x06, 0x00, 0x00, // jmp word 0x0:0x600
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 1);

    assert_eq!("[085F:0100] EA00060000 JmpFar   0000:0600
",
               res);

    cpu.execute_instruction();
    assert_eq!(0x0000, cpu.sreg16[CS]);
    assert_eq!(0x0600, cpu.ip);
}

#[test]
fn can_execute_movzx() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB4, 0xFF,       // mov ah,0xff
        0x0F, 0xB6, 0xDC, // movzx bx,ah

    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 2);

    assert_eq!("[085F:0100] B4FF       Mov8     ah, 0xFF
[085F:0102] 0FB6DC     Movzx16  bx, ah
",
               res);

    cpu.execute_instruction();
    assert_eq!(0xFF, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.r16[BX].val);
}

#[test]
fn can_execute_rcr() {
let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB1, 0x3E, // mov cl,0x3e     ; 0x3e     = 0b00111110
        0xD0, 0xD9, // rcr cl,1        ; cl = 0x1f, 0b00011111
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x3E, cpu.r16[CX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x1F,  cpu.r16[CX].lo_u8());
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.overflow); // XXX unsure
}


#[test]
fn can_execute_sar() {
let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0xF5, 0x05, // mov ax,0x5f5
        0xC1, 0xF8, 0x09, // sar ax,byte 0x9
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x05F5, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0x0002,  cpu.r16[AX].val);
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    // assert_eq!(true, cpu.flags.auxiliary_carry); // is true in dosbox, undefined in intel docs for non-zero count
    assert_eq!(false, cpu.flags.parity);
}

#[test]
fn can_execute_imul8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB0, 0xF0, // mov al,0xf0
        0xB7, 0xD0, // mov bh,0xd0
        0xF6, 0xEF, // imul bh
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 3);

    assert_eq!("[085F:0100] B0F0       Mov8     al, 0xF0
[085F:0102] B7D0       Mov8     bh, 0xD0
[085F:0104] F6EF       Imul8    bh
",
               res);

    cpu.execute_instruction();
    assert_eq!(0xF0, cpu.r16[AX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0xD0, cpu.r16[BX].hi_u8());

    cpu.execute_instruction();
    // AX = AL ∗ r/m byte.
    assert_eq!(0x0300, cpu.r16[AX].val);
    // XXX Carry & overflow is true in dosbox
}

#[test]
fn can_execute_imul16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x8F, 0x79, // mov bx,0x798f
        0xB8, 0xD9, 0xFF, // mov ax,0xffd9
        0xF7, 0xEB,       // imul bx
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 3);

    assert_eq!("[085F:0100] BB8F79     Mov16    bx, 0x798F
[085F:0103] B8D9FF     Mov16    ax, 0xFFD9
[085F:0106] F7EB       Imul16   bx
",
               res);

    cpu.execute_instruction();
    assert_eq!(0x798F, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0xFFD9, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0xFFED, cpu.r16[DX].val);
    assert_eq!(0x7B37, cpu.r16[AX].val);
}

#[test]
fn can_disassemble_basic() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xE8, 0x05, 0x00, // call l_0x108   ; call a later offset
        0xBA, 0x0B, 0x01, // mov dx,0x10b
        0xB4, 0x09,       // mov ah,0x9
        0xCD, 0x21,       // l_0x108: int 0x21
        0xE8, 0xFB, 0xFF, // call l_0x108   ; call an earlier offset
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 5);

    assert_eq!("[085F:0100] E80500     CallNear 0x0108
[085F:0103] BA0B01     Mov16    dx, 0x010B
[085F:0106] B409       Mov8     ah, 0x09
[085F:0108] CD21       Int      0x21
[085F:010A] E8FBFF     CallNear 0x0108
",
               res);
}

#[test]
fn can_disassemble_lea() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x8D, 0x47, 0x80, // lea ax,[bx-0x80]
 ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 1);

    assert_eq!("[085F:0100] 8D4780     Lea16    ax, word [bx-0x80]
",
               res);
}

#[test]
fn can_disassemble_segment_prefixed() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x26, 0x88, 0x25, // mov [es:di],ah
        0x26, 0x8A, 0x25, // mov ah,[es:di]
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 2);

    assert_eq!("[085F:0100] 268825     Mov8     byte [es:di], ah
[085F:0103] 268A25     Mov8     ah, byte [es:di]
",
               res);
}

#[test]
fn can_disassemble_arithmetic() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x81, 0xC7, 0xC0, 0x00,       // add di,0xc0
        0x83, 0xC7, 0x3A,             // add di,byte +0x3a
        0x83, 0xC7, 0xC6,             // add di,byte -0x3a
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 4);

    assert_eq!("[085F:0100] 803E311000 Cmp8     byte [0x1031], 0x00
[085F:0105] 81C7C000   Add16    di, 0x00C0
[085F:0109] 83C73A     Add16    di, byte +0x3A
[085F:010C] 83C7C6     Add16    di, byte -0x3A
",
               res);
}

#[test]
fn can_disassemble_shr() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xC1, 0xE8, 0x02, // shr ax,byte 0x2
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 1);

    assert_eq!("[085F:0100] C1E802     Shr16    ax, 0x02
",
               res);
}

#[test]
fn can_disassemble_shrd() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x0F, 0xAC, 0xD0, 0x08, // shrd ax,dx,0x8
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 1);

    assert_eq!("[085F:0100] 0FACD008   Shrd     ax, dx, 0x08
",
               res);
}

#[test]
fn can_disassemble_jz_rel() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x74, 0x04, // jz 0x106
        0x74, 0xFE, // jz 0x102
        0x74, 0x00, // jz 0x106
        0x74, 0xFA, // jz 0x102
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 4);

    assert_eq!("[085F:0100] 7404       Jz       0x0106
[085F:0102] 74FE       Jz       0x0102
[085F:0104] 7400       Jz       0x0106
[085F:0106] 74FA       Jz       0x0102
",
               res);
}


#[bench]
fn exec_simple_loop(b: &mut Bencher) {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB9, 0xFF, 0xFF, // mov cx,0xffff
        0x49,             // dec cx
        0xEB, 0xFA,       // jmp short 0x100
    ];

    cpu.load_com(&code);

    b.iter(|| cpu.execute_instruction())
}

#[bench]
fn disasm_block(b: &mut Bencher) {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB9, 0xFF, 0xFF, // mov cx,0xffff
        0x49,             // dec cx
        0xEB, 0xFA,       // jmp short 0x100
    ];

    cpu.load_com(&code);

    b.iter(|| cpu.disassemble_block(0x100, 3))
}
