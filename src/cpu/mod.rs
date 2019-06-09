// these modules are re-exported as a single module

pub use self::decoder::*;
mod decoder;

pub use self::instruction::*;
mod instruction;

pub use self::segment::*;
mod segment;

pub use self::register::*;
mod register;

pub use self::flag::*;
mod flag;

pub use self::parameter::*;
mod parameter;

pub use self::op::*;
mod op;

pub use self::encoder::*;
mod encoder;

use std::{mem, u8};
use std::num::Wrapping;
use std::marker::PhantomData;

use crate::gpu::GPU;
use crate::interrupt;
use crate::machine::Machine;
use crate::memory::{MMU, MemoryAddress};

/// prints diagnostics if writes to memory close to SS:SP occurs
const DEBUG_PARAMS_TOUCHING_STACK: bool = false;

/// prints diagnostics of stack usage (push / pop)
const DEBUG_STACK: bool = false;

#[derive(Debug)]
pub enum Exception {
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

    /// general purpose registers, segment registers, ip
    pub regs: RegisterState,

    /// signals to debugger we hit an error (used by debugger)
    pub fatal_error: bool,

    /// toggles non-deterministic behaviour (used by tests)
    pub deterministic: bool,

    pub decoder: Decoder,
    pub clock_hz: usize,
}

impl CPU {
    pub fn default() -> Self {
        CPU {
            instruction_count: 0,
            cycle_count: 0,
            regs: RegisterState::default(),
            fatal_error: false,
            deterministic: false,
            decoder: Decoder::default(),
            clock_hz: 5_000_000, // Intel 8086: 0.330 MIPS at 5.000 MHz
        }
    }

    pub fn deterministic() -> Self {
        let mut res = Self::default();
        res.deterministic = true;
        res
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

    pub fn exception(&mut self, which: &Exception, error: usize) {
        /*
        #define CPU_INT_SOFTWARE    0x1
        #define CPU_INT_EXCEPTION   0x2
        #define CPU_INT_HAS_ERROR   0x4
        #define CPU_INT_NOIOPLCHECK 0x8
        */
        println!("Exception {:?}, error {}", which, error);

        // CPU_Interrupt(which,CPU_INT_EXCEPTION | ((which>=8) ? CPU_INT_HAS_ERROR : 0),reg_eip);
    }

    pub fn cmp8(&mut self, dst: usize, src: usize) {
        let res = (Wrapping(dst) - Wrapping(src)).0;

        // The CF, OF, SF, ZF, AF, and PF flags are set according to the result.
        self.regs.flags.set_carry_u8(res);
        self.regs.flags.set_overflow_sub_u8(res, src, dst);
        self.regs.flags.set_sign_u8(res);
        self.regs.flags.set_zero_u8(res);
        self.regs.flags.set_adjust(res, src, dst);
        self.regs.flags.set_parity(res);
    }

    pub fn cmp16(&mut self, dst: usize, src: usize) {
        let res = (Wrapping(dst) - Wrapping(src)).0;

        // The CF, OF, SF, ZF, AF, and PF flags are set according to the result.
        self.regs.flags.set_carry_u16(res);
        self.regs.flags.set_overflow_sub_u16(res, src, dst);
        self.regs.flags.set_sign_u16(res);
        self.regs.flags.set_zero_u16(res);
        self.regs.flags.set_adjust(res, src, dst);
        self.regs.flags.set_parity(res);
    }

    pub fn cmp32(&mut self, dst: usize, src: usize) {
        let res = (Wrapping(dst) - Wrapping(src)).0;

        // The CF, OF, SF, ZF, AF, and PF flags are set according to the result.
        self.regs.flags.set_carry_u32(res);
        self.regs.flags.set_overflow_sub_u32(res, src, dst);
        self.regs.flags.set_sign_u32(res);
        self.regs.flags.set_zero_u32(res);
        self.regs.flags.set_adjust(res, src, dst);
        self.regs.flags.set_parity(res);
    }

    pub fn push16(&mut self, mmu: &mut MMU, data: u16) {
        let sp = (Wrapping(self.get_r16(R::SP)) - Wrapping(2)).0;
        self.set_r16(R::SP, sp);
        let ss = self.get_r16(R::SS);
        if DEBUG_STACK {
            println!("[{}] push16 {:04X} to {:04X}:{:04X}", self.get_memory_address(), data, ss, sp);
        }
        mmu.write_u16(ss, sp, data);
    }

    pub fn push32(&mut self, mmu: &mut MMU, data: u32) {
        let sp = (Wrapping(self.get_r16(R::SP)) - Wrapping(4)).0;
        self.set_r16(R::SP, sp);
        let ss = self.get_r16(R::SS);
        if DEBUG_STACK {
            println!("[{}] push32 {:04X} to {:04X}:{:04X}", self.get_memory_address(), data, ss, sp);
        }
        mmu.write_u32(ss, sp, data);
    }

    pub fn pop16(&mut self, mmu: &mut MMU) -> u16 {
        let ss = self.get_r16(R::SS);
        let sp = self.get_r16(R::SP);
        let data = mmu.read_u16(ss, self.get_r16(R::SP));
        if DEBUG_STACK {
            println!("[{}] pop16 {:04X} from {:04X}:{:04X}", self.get_memory_address(), data, ss, sp);
        }
        let sp = (Wrapping(sp) + Wrapping(2)).0;
        self.set_r16(R::SP, sp);
        data
    }

    pub fn pop32(&mut self, mmu: &mut MMU) -> u32 {
        let ss = self.get_r16(R::SS);
        let sp = self.get_r16(R::SP);
        let data = mmu.read_u32(ss, sp);
        if DEBUG_STACK {
            println!("[{}] pop32 {:04X} from {:04X}:{:04X}", self.get_memory_address(), data, ss, sp);
        }
        let sp = (Wrapping(sp) + Wrapping(4)).0;
        self.set_r16(R::SP, sp);
        data
    }

    /// returns the absoute address of CS:IP
    pub fn get_address(&self) -> u32 {
        self.get_memory_address().value()
    }

    /// returns cs, ip
    pub fn get_address_pair(&self) -> (u16, u16) {
        (self.get_r16(R::CS), self.regs.ip)
    }

    /// returns the address of CS:IP as a MemoryAddress::RealSegmentOffset
    pub fn get_memory_address(&self) -> MemoryAddress {
        MemoryAddress::RealSegmentOffset(self.get_r16(R::CS), self.regs.ip)
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
    pub fn read_segment_selector(&self, mmu: &MMU, p: &Parameter) -> (u16, u16) {
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
    pub fn read_parameter_address(&mut self, p: &Parameter) -> usize {
        match *p {
            Parameter::Ptr16Amode(_, ref amode) => self.amode(amode),
            Parameter::Ptr16AmodeS8(_, ref amode, imm) => (Wrapping(self.amode(amode)) + Wrapping(imm as usize)).0,
            Parameter::Ptr16AmodeS16(_, ref amode, imm) => (Wrapping(self.amode(amode)) + Wrapping(imm as usize)).0,
            Parameter::Ptr16(_, imm) => imm as usize,
            _ => panic!("unhandled parameter: {:?} at {:06X}", p, self.get_address()),
        }
    }

    pub fn read_parameter_imm(&self, p: &Parameter) -> usize {
        match *p {
            Parameter::Imm8(imm) => imm as usize,
            Parameter::Imm16(imm) => imm as usize,
            Parameter::ImmS8(imm) => imm as usize,
            _ => panic!("read_parameter_imm only allows imm-type params: {:?}", p),
        }
    }

    pub fn read_parameter_value(&mut self, mmu: &MMU, p: &Parameter) -> usize {
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
            Parameter::Ptr32(seg, offset) => mmu.read_u32(self.segment(seg), offset) as usize,
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

    pub fn write_parameter_u8(&mut self, mmu: &mut MMU, p: &Parameter, data: u8) {
        match *p {
            Parameter::Reg8(r) => self.set_r8(r, data),
            Parameter::Ptr8(seg, offset) => {
                let seg = self.segment(seg);
                self.debug_write_u8(seg, offset, data);
                mmu.write_u8(seg, offset, data);
            }
            Parameter::Ptr8Amode(seg, ref amode) => {
                let seg = self.segment(seg);
                let offset = self.amode(amode) as u16;
                self.debug_write_u8(seg, offset, data);
                mmu.write_u8(seg, offset, data);
            }
            Parameter::Ptr8AmodeS8(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = (Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16)).0;
                self.debug_write_u8(seg, offset, data);
                mmu.write_u8(seg, offset, data);
            }
            Parameter::Ptr8AmodeS16(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = (Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16)).0;
                self.debug_write_u8(seg, offset, data);
                mmu.write_u8(seg, offset, data);
            }
            _ => panic!("write_parameter_u8 unhandled type {:?} at {:06X}", p, self.get_address()),
        }
    }

    pub fn write_parameter_u16(&mut self, mmu: &mut MMU, segment: Segment, p: &Parameter, data: u16) {
        match *p {
            Parameter::Reg16(r) |
            Parameter::SReg16(r) => self.set_r16(r, data),
            Parameter::Imm16(imm) => {
                let seg = self.segment(segment);
                self.debug_write_u16(seg, imm, data);
                mmu.write_u16(seg, imm, data);
            }
            Parameter::Ptr16(seg, offset) => {
                let seg = self.segment(seg);
                self.debug_write_u16(seg, offset, data);
                mmu.write_u16(seg, offset, data);
            }
            Parameter::Ptr16Amode(seg, ref amode) => {
                let seg = self.segment(seg);
                let offset = self.amode(amode) as u16;
                self.debug_write_u16(seg, offset, data);
                mmu.write_u16(seg, offset, data);
            }
            Parameter::Ptr16AmodeS8(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = (Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16)).0;
                self.debug_write_u16(seg, offset, data);
                mmu.write_u16(seg, offset, data);
            }
            Parameter::Ptr16AmodeS16(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = (Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16)).0;
                self.debug_write_u16(seg, offset, data);
                mmu.write_u16(seg, offset, data);
            }
            _ => panic!("unhandled type {:?} at {:06X}", p, self.get_address()),
        }
    }

    pub fn write_parameter_u32(&mut self, mmu: &mut MMU, _segment: Segment, p: &Parameter, data: u32) {
        match *p {
            Parameter::Reg32(r) => self.set_r32(r, data),
            Parameter::Ptr32(seg, offset) => {
                let seg = self.segment(seg);
                self.debug_write_u32(seg, offset, data);
                mmu.write_u32(seg, offset, data);
            }
            Parameter::Ptr32Amode(seg, ref amode) => {
                let seg = self.segment(seg);
                let offset = self.amode(amode);
                self.debug_write_u32(seg, offset as u16, data);
                mmu.write_u32(seg, offset as u16, data);
            }
            Parameter::Ptr32AmodeS8(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = (Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16)).0;
                self.debug_write_u32(seg, offset as u16, data);
                mmu.write_u32(seg, offset, data);
            }
            Parameter::Ptr32AmodeS16(seg, ref amode, imm) => {
                let seg = self.segment(seg);
                let offset = (Wrapping(self.amode(amode) as u16) + Wrapping(imm as u16)).0;
                self.debug_write_u32(seg, offset as u16, data);
                mmu.write_u32(seg, offset, data);
            }
            _ => panic!("unhandled type {:?} at {:06X}", p, self.get_address()),
        }
    }

    fn debug_write_u8(&self, seg: u16, off: u16, data: u8) {
        if !DEBUG_PARAMS_TOUCHING_STACK {
            return;
        }
        let pos = MemoryAddress::RealSegmentOffset(seg, off).value() as isize;
        let stack = MemoryAddress::RealSegmentOffset(self.get_r16(R::SS), self.get_r16(R::SP));
        let code = MemoryAddress::RealSegmentOffset(self.get_r16(R::CS), self.get_r16(R::IP));
        let dist = (pos - stack.value() as isize).abs();
        if dist < 256 {
            // XXX points to the instruction AFTER the one to blame
            println!("[{}] debug_write_u8 {:04X}:{:04X} = {:02X} ... stack {} (dist {})", code, seg, off, data, stack, dist);
        }
    }

    fn debug_write_u16(&self, seg: u16, off: u16, data: u16) {
        if !DEBUG_PARAMS_TOUCHING_STACK {
            return;
        }
        let pos = MemoryAddress::RealSegmentOffset(seg, off).value() as isize;
        let stack = MemoryAddress::RealSegmentOffset(self.get_r16(R::SS), self.get_r16(R::SP));
        let code = MemoryAddress::RealSegmentOffset(self.get_r16(R::CS), self.get_r16(R::IP));
        let dist = (pos - stack.value() as isize).abs();
        if dist < 256 {
            // XXX points to the instruction AFTER the one to blame
            println!("[{}] debug_write_u16 {:04X}:{:04X} = {:04X} ... stack {} (dist {})", code, seg, off, data, stack, dist);
        }
    }

    fn debug_write_u32(&self, seg: u16, off: u16, data: u32) {
        if !DEBUG_PARAMS_TOUCHING_STACK {
            return;
        }
        let pos = MemoryAddress::RealSegmentOffset(seg, off).value() as isize;
        let stack = MemoryAddress::RealSegmentOffset(self.get_r16(R::SS), self.get_r16(R::SP));
        let code = MemoryAddress::RealSegmentOffset(self.get_r16(R::CS), self.get_r16(R::IP));
        let dist = (pos - stack.value() as isize).abs();
        if dist < 256 {
             // XXX points to the instruction AFTER the one to blame
            println!("[{}] debug_write_u32 {:04X}:{:04X} = {:08X} ... stack {} (dist {})", code, seg, off, data, stack, dist);
        }
    }

    /// returns the value of the given segment register
    pub fn segment(&self, seg: Segment) -> u16 {
        self.get_r16(seg.as_register())
    }

    pub fn amode(&self, amode: &AMode) -> usize {
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
    pub fn adjb(&mut self, param1: i8, param2: i8) {
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
    pub fn adj4(&mut self, param1: i16, param2: i16) {
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
}
