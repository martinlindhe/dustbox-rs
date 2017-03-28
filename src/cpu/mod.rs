#![allow(dead_code)]
#![allow(unused_variables)]

pub struct CPU {
    pub ip: u16,
    memory: Vec<u8>,
    // 8 low = r16, 8 hi = es,cs,ss,ds,fs,gs
    r16: [Register16; 8], // general purpose registers
    sreg16: [Register16; 6], // es,cs,ss,ds,fs,gs
    flags: Flags,
}

// https://en.wikipedia.org/wiki/FLAGS_register
struct Flags {
    carry: bool, // 0: carry flag
    reserved1: bool, // 1: Reserved, always 1 in EFLAGS
    parity: bool, // 2: parity flag
    reserved3: bool,
    adjust: bool, // 4: adjust flag
    reserved5: bool,
    zero: bool, // 6: zero flag
    sign: bool, // 7: sign flag
    trap: bool, // 8: trap flag (single step)
    interrupt_enable: bool, // 9: interrupt enable flag
    direction: bool, // 10: direction flag (control with cld, std)
    overflow: bool, // 11: overflow
    iopl12: bool, // 12: I/O privilege level (286+ only), always 1 on 8086 and 186
    iopl13: bool, // 13 --""---
    nested_task: bool, // 14: Nested task flag (286+ only), always 1 on 8086 and 186
    reserved15: bool, // 15: Reserved, always 1 on 8086 and 186, always 0 on later models
    resume: bool, // 16: Resume flag (386+ only)
    virtual_mode: bool, // 17: Virtual 8086 mode flag (386+ only)
    alignment_check: bool, // 18: Alignment check (486SX+ only)
    virtual_interrupt: bool, // 19: Virtual interrupt flag (Pentium+)
    virtual_interrupt_pending: bool, // 20: Virtual interrupt pending (Pentium+)
    cpuid: bool, // 21: Able to use CPUID instruction (Pentium+)
                 // 22-31: reserved
}


#[derive(Debug, Copy, Clone)] // XXX only need Copy ??
struct Register16 {
    val: u16,
}

impl Register16 {
    fn set_hi(&mut self, val: u8) {
        self.val = (self.val & 0xFF) + ((val as u16) << 8);
    }
    fn set_lo(&mut self, val: u8) {
        self.val = (self.val & 0xFF00) + val as u16;
    }
    fn set_u16(&mut self, val: u16) {
        self.val = val;
    }
    fn lo_u8(&mut self) -> u8 {
        (self.val & 0xFF) as u8
    }
    fn hi_u8(&mut self) -> u8 {
        (self.val >> 8) as u8
    }
    fn u16(&self) -> u16 {
        self.val
    }
}



struct ModRegRm {
    md: u8, // NOTE: "mod" is reserved in rust
    reg: u8,
    rm: u8,
}

#[derive(Debug)]
struct Parameters {
    command: Op,
    src: Parameter,
    dst: Parameter,
}

#[derive(Debug)]
enum Parameter {
    Imm8(u8),
    Imm16(u16),
    Reg(usize), // index into CPU.r16
    SReg16(usize), // index into cpu.sreg16
    Empty(),
}

#[derive(Debug)]
enum Op {
    Push16(),
    Unknown(),
}


// r16
const AX: usize = 0;
const CX: usize = 1;
const DX: usize = 2;
const BX: usize = 3;
const SP: usize = 4;
const BP: usize = 5;
const SI: usize = 6;
const DI: usize = 7;

// sreg16
const ES: usize = 0;
const CS: usize = 1;
const SS: usize = 2;
const DS: usize = 3;
const FS: usize = 4;
const GS: usize = 5;

impl CPU {
    pub fn new() -> CPU {
        let mut cpu = CPU {
            ip: 0,
            memory: vec![0u8; 0x10000 * 64],
            r16: [Register16 { val: 0 }; 8],
            sreg16: [Register16 { val: 0 }; 6],
            flags: Flags {
                carry: false, // 0: carry flag
                reserved1: false, // 1: Reserved, always 1 in EFLAGS
                parity: false, // 2: parity flag
                reserved3: false,
                adjust: false, // 4: adjust flag
                reserved5: false,
                zero: false, // 6: zero flag
                sign: false, // 7: sign flag
                trap: false, // 8: trap flag (single step)
                interrupt_enable: false, // 9: interrupt enable flag
                direction: false, // 10: direction flag (control with cld, std)
                overflow: false, // 11: overflow
                iopl12: false, // 12: I/O privilege level (286+ only), always 1 on 8086 and 186
                iopl13: false, // 13 --""---
                nested_task: false, // 14: Nested task flag (286+ only), always 1 on 8086 and 186
                reserved15: false, // 15: Reserved, always 1 on 8086 and 186, 0 on later models
                resume: false, // 16: Resume flag (386+ only)
                virtual_mode: false, // 17: Virtual 8086 mode flag (386+ only)
                alignment_check: false, // 18: Alignment check (486SX+ only)
                virtual_interrupt: false, // 19: Virtual interrupt flag (Pentium+)
                virtual_interrupt_pending: false, // 20: Virtual interrupt pending (Pentium+)
                cpuid: false, // 21: Able to use CPUID instruction (Pentium+)
            },
        };

        // intializes the cpu as if to run .com programs, info from
        // http://www.delorie.com/djgpp/doc/rbinter/id/51/29.html
        cpu.sreg16[SS].val = 0x0000;
        cpu.r16[SP].val = 0xFFF0; // XXX offset of last word available in first 64k segment

        cpu
    }

    pub fn reset(&mut self) {
        self.ip = 0;
        // XXX clear memory
    }

    pub fn load_rom(&mut self, data: &Vec<u8>, offset: u16) {
        self.ip = offset;

        // copy up to 64k of rom
        let mut max = (offset as usize) + data.len();
        if max > 0x10000 {
            max = 0x10000;
        }
        let min = offset as usize;
        info!("loading rom to {:04X}..{:04X}", min, max);

        for i in min..max {
            let rom_pos = i - (offset as usize);
            self.memory[i] = data[rom_pos];
        }
    }

    pub fn print_registers(&mut self) {
        print!("ip:{:04X}  ax:{:04X} bx:{:04X} cx:{:04X} dx:{:04X}",
               self.ip,
               self.r16[AX].u16(),
               self.r16[BX].u16(),
               self.r16[CX].u16(),
               self.r16[DX].u16());
        print!("  sp:{:04X} bp:{:04X} si:{:04X} di:{:04X}",
               self.r16[SP].u16(),
               self.r16[BP].u16(),
               self.r16[SI].u16(),
               self.r16[DI].u16());

        print!("   es:{:04X} cs:{:04X} ss:{:04X} ds:{:04X} fs:{:04X} gs:{:04X}",
               self.sreg16[ES].u16(),
               self.sreg16[CS].u16(),
               self.sreg16[SS].u16(),
               self.sreg16[DS].u16(),
               self.sreg16[FS].u16(),
               self.sreg16[GS].u16());

        println!("");
    }

    fn get_parameter_value(&self, p: &Parameter) -> usize {
        match *p {
            Parameter::SReg16(r) => self.sreg16[r].val as usize,
            _ => {
                error!("error unhandled parameter: {:?}", p);
                0
            }
        }
    }


    pub fn execute_instruction(&mut self) {
        let op = self.decode_instruction();
        self.execute(&op);
    }

    fn execute(&mut self, op: &Parameters) {
        match op.command {
            Op::Push16() => {
                // single parameter (dst)
                error!("YAY executing push, params {:?}, {:?}", op.dst, op.src);

                let data = self.get_parameter_value(&op.dst) as u16;

                self.r16[SP].val -= 2;
                let offset = (self.sreg16[SS].u16() as usize) * 16 + (self.r16[SP].u16() as usize);
                warn!("push16 {:04X}  to {:04X}:{:04X}  =>  {:06X}",
                      data,
                      self.sreg16[SS].u16(),
                      self.r16[SP].u16(),
                      offset);

                self.write_u16(offset, data);
            }
            _ => {
                error!("ERROR Op::execute {:?}", op.command);
            }
        }
    }


    fn decode_instruction(&mut self) -> Parameters {
        let b = self.memory[self.ip as usize];
        self.ip += 1;
        let mut p = Parameters {
            command: Op::Unknown(),
            dst: Parameter::Empty(),
            src: Parameter::Empty(),
        };

        match b {
            0x1E => {
                // push ds
                p.command = Op::Push16();
                p.dst = Parameter::SReg16(DS);
                p
            }

            /*
            0x06 => {
                // push es
                let val = self.r16[ES].val;
                self.push16(val);
            }
            0x07 => {
                // pop es
                self.r16[ES].val = self.pop16();
            }
            0x31 => {
                // xor r16, r/m16
                let p = self.rm16_r16();
                self.xor_r16(&p);
            }
            0x40...0x47 => {
                // inc r16
                self.r16[(b & 7) as usize].val += 1;
                // XXX flags
            }
            //0x48...0x4F => format!("dec {}", r16(b & 7)),
            0x50...0x57 => {
                // push r16
                let val = self.r16[(b & 7) as usize].val;
                self.push16(val);
            }
            0x88 => {
                // mov r8, r/m8
                let p = self.r8_rm8();
                error!("XXX mov   {:?}, {:?}", p.dst, p.src);
                self.mov_r8(&p);
            }
            0x8B => {
                // mov r16, r/m16
                let p = self.r16_rm16();
                self.mov_r16(&p);
            }
            0x8E => {
                // mov sreg, r/m16
                let p = self.sreg_rm16();
                self.mov_r16(&p);
            }
            0xAA => {
                // stosb
                // For legacy mode, store AL at address ES:(E)DI;
                let offset = (self.r16[ES].val as usize) * 16 + (self.r16[DI].val as usize);
                let data = self.r16[AX].lo_u8(); // = AL
                self.write_u8(offset, data);
                if !self.flags.direction {
                    self.r16[DI].val += 1;
                } else {
                    self.r16[DI].val -= 1;
                }
            }
            0xB0...0xB7 => {
                // mov r8, u8
                let val = self.read_u8();
                self.mov_r8_u8((b & 7) as usize, val);
            }
            0xB8...0xBF => {
                // mov r16, u16
                let reg = (b & 7) as usize;
                self.r16[reg].val = self.read_u16();
            }
            0xCD => {
                // int u8
                // XXX jump to offset 0x21 in interrupt table (look up how hw does this)
                // http://wiki.osdev.org/Interrupt_Vector_Table
                error!("XXX IMPL: int {:02X}", self.read_u8());
            }
            0xE2 => {
                // loop rel8
                let dst = self.read_rel8();
                self.r16[CX].val -= 1;
                if self.r16[CX].val != 0 {
                    self.ip = dst;
                }
            }
            0xE8 => {
                // call s16
                let old_ip = self.ip;
                let temp_ip = self.read_rel16();
                self.push16(old_ip);
                self.ip = temp_ip;
            }
            */
            0xFA => {
                // cli
                error!("TODO - cli - clear intterrupts??");
                p
            }
            _ => {
                error!("cpu: unknown op {:02X} at {:04X}", b, self.ip - 1);
                p
            }
        }

    }

    /*

    fn pop16(&mut self) -> u16 {
        let offset = (self.r16[SS].u16() as usize) * 16 + (self.r16[SP].u16() as usize);
        let data = self.peek_u16_at(offset);
        self.r16[SP].val += 2;
        data
    }

    fn mov_r8_u8(&mut self, r: usize, imm: u8) {
        let lor = r & 3;
        if r & 4 == 0 {
            self.r16[lor].set_lo(imm);
        } else {
            self.r16[lor].set_hi(imm);
        }
    }

    // calculates imm from src parameter
    fn u8_value(&mut self, p: &Parameter) -> u8 {
        match p {
            &Parameter::Reg(r) => {
                error!("mov_u8 PARAM-ONE Reg PANIC");
                0
            }
            &Parameter::Imm8(imm) => imm,
            &Parameter::Imm16(imm2) => {
                error!("mov_u8 PARAM-ONE Imm16 PANIC");
                0
            }
        }
    }

    // calculates imm from src parameter
    fn u16_value(&mut self, p: &Parameter) -> u16 {
        match p {
            &Parameter::Reg(r_src) => self.r16[r_src].u16(),
            &Parameter::Imm8(imm) => {
                error!("!! XXX mov_r16 Imm8-SUB unhandled - PANIC {:?}", imm);
                0
            }
            &Parameter::Imm16(imm) => imm,
        }
    }

    fn mov_r8(&mut self, x: &Parameters) {
        error!("XXX impl mov_r8");
        /*
        match x.dst {
            Parameter::Reg(r) => {
                match x.src {
                    Parameter::Imm16(imm) => {
                        self.r16[r].set_u16(imm);
                    }
                    Parameter::Reg(r_src) => {
                        let val = self.r16[r_src].u16();
                        self.r16[r].set_u16(val);
                    }
                    Parameter::Imm8(imm) => {
                        error!("!! XXX mov_r16 Imm8-SUB unhandled - PANIC {:?}", imm);
                    }
                }
            }
            Parameter::Imm16(imm) => {
                error!("!! XXX mov_r16 Imm16 unhandled - PANIC {:?}", imm);
            }
            Parameter::Imm8(imm) => {
                error!("!! XXX mov_r16 Imm8 unhandled - PANIC {:?}", imm);
            }
        }
        */
    }

    fn mov_r16(&mut self, x: &Parameters) {
        match x.dst {
            Parameter::Reg(r) => {
                let imm = self.u16_value(&x.src);
                self.r16[r].set_u16(imm);
            }
            Parameter::Imm16(imm) => {
                error!("!! XXX mov_r16 Imm16 unhandled - PANIC {:?}", imm);
            }
            Parameter::Imm8(imm) => {
                error!("!! XXX mov_r16 Imm8 unhandled - PANIC {:?}", imm);
            }
        }
    }

    fn xor_r16(&mut self, x: &Parameters) {
        match x.dst {
            Parameter::Reg(r) => {
                match x.src {
                    Parameter::Imm16(imm) => {
                        error!("!! XXX xor_r16 Imm16-SUB unhandled - PANIC {:?}", imm);
                    }
                    Parameter::Reg(r_src) => {
                        // XXX should set flags
                        let val = self.r16[r].val ^ self.r16[r_src].val;
                        self.r16[r].set_u16(val);
                    }
                    Parameter::Imm8(imm) => {
                        error!("!! XXX xor_r16 Imm8-SUB unhandled - PANIC {:?}", imm);
                    }
                }
            }
            Parameter::Imm16(imm) => {
                error!("!! XXX xor_r16 Imm16 unhandled - PANIC {:?}", imm);
            }
            Parameter::Imm8(imm) => {
                error!("!! XXX xor_r16 Imm8 unhandled - PANIC {:?}", imm);
            }
        }
    }

    // decode Sreg, r/m16
    fn sreg_rm16(&mut self) -> Parameters {
        let mut res = self.rm16_sreg();
        let tmp = res.src;
        res.src = res.dst;
        res.dst = tmp;
        res
    }

    // decode r/m16, Sreg
    fn rm16_sreg(&mut self) -> Parameters {
        let x = self.read_mod_reg_rm();
        Parameters {
            src: Parameter::Reg(8 + (x.reg as usize)),
            dst: self.rm16(x.rm, x.md),
        }
    }

    // decode rm8
    fn rm8(&mut self, rm: u8, md: u8) -> Parameter {
        match md {
            0 => {
                // [reg]
                let mut pos = 0;
                if rm == 6 {
                    // [u16]
                    pos = self.read_u16();
                } else {
                    error!("XXX FIXME rm8 [u16] or [reg+u16] ??!?!?!");
                    // XXX read value of amode(x.rm) into pos
                    let pos = 0;
                }
                Parameter::Imm16(self.peek_u16_at(pos as usize))
            }
            1 => {
                // [reg+d8]
                // XXX read value of amode(x.rm) into pos
                error!("XXX FIXME rm8 [reg+d8]");
                let mut pos = 0;
                pos += self.read_s8() as u16; // XXX handle signed properly

                Parameter::Imm16(self.peek_u16_at(pos as usize))
            }
            2 => {
                // [reg+d16]
                // XXX read value of amode(x.rm) into pos
                error!("XXX FIXME rm8 [reg+d16]");
                let mut pos = 0;
                pos += self.read_s16() as u16; // XXX handle signed properly

                Parameter::Imm16(self.peek_u16_at(pos as usize))
            }
            _ => {
                // general purpose r16
                Parameter::Reg(rm as usize) // XXX r8
            }
        }
    }

    // decode rm16
    fn rm16(&mut self, rm: u8, md: u8) -> Parameter {
        match md {
            0 => {
                // [reg]
                let mut pos = 0;
                if rm == 6 {
                    // [u16]
                    pos = self.read_u16();
                } else {
                    error!("XXX FIXME [u16] or [reg+u16] ??!?!?!");
                    // XXX read value of amode(x.rm) into pos
                    let pos = 0;
                }
                Parameter::Imm16(self.peek_u16_at(pos as usize))
            }
            1 => {
                // [reg+d8]
                // XXX read value of amode(x.rm) into pos
                error!("XXX FIXME rm16 [reg+d8]");
                let mut pos = 0;
                pos += self.read_s8() as u16; // XXX handle signed properly

                Parameter::Imm16(self.peek_u16_at(pos as usize))
            }
            2 => {
                // [reg+d16]
                // XXX read value of amode(x.rm) into pos
                error!("XXX FIXME rm16 [reg+d16]");
                let mut pos = 0;
                pos += self.read_s16() as u16; // XXX handle signed properly

                Parameter::Imm16(self.peek_u16_at(pos as usize))
            }
            _ => {
                // general purpose r16
                Parameter::Reg(rm as usize)
            }
        }
    }


    // decode r8, r/m8
    fn r8_rm8(&mut self) -> Parameters {
        let mut res = self.rm8_r8();
        let tmp = res.src;
        res.src = res.dst;
        res.dst = tmp;
        res
    }

    // decode r/m8, r8
    fn rm8_r8(&mut self) -> Parameters {
        let x = self.read_mod_reg_rm();
        Parameters {
            src: Parameter::Reg(x.reg as usize), // XXX 8 bit reg
            dst: self.rm8(x.rm, x.md),
        }
    }

    // decode r16, r/m16
    fn r16_rm16(&mut self) -> Parameters {
        let mut res = self.rm16_r16();
        let tmp = res.src;
        res.src = res.dst;
        res.dst = tmp;
        res
    }

    // decode r/m16, r16
    fn rm16_r16(&mut self) -> Parameters {
        let x = self.read_mod_reg_rm();
        Parameters {
            src: Parameter::Reg(x.reg as usize),
            dst: self.rm16(x.rm, x.md),
        }
    }
*/

    fn read_mod_reg_rm(&mut self) -> ModRegRm {
        let b = self.read_u8();
        ModRegRm {
            md: b >> 6,
            reg: (b >> 3) & 7,
            rm: b & 7,
        }
    }

    fn read_u8(&mut self) -> u8 {
        let offset = (self.r16[CS].u16() as usize) + self.ip as usize;
        let b = self.memory[offset];
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

    fn peek_u8_at(&mut self, pos: usize) -> u8 {
        self.memory[pos]
    }

    fn peek_u16_at(&mut self, pos: usize) -> u16 {
        let lo = self.peek_u8_at(pos);
        let hi = self.peek_u8_at(pos + 1);
        (hi as u16) << 8 | lo as u16
    }

    fn write_u16(&mut self, offset: usize, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.write_u8(offset, lo);
        self.write_u8(offset + 1, hi);
    }

    fn write_u8(&mut self, offset: usize, data: u8) {
        self.memory[offset] = data;
    }

    // used by disassembler
    pub fn read_u8_slice(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let mut res = vec![0u8; length];
        for i in offset..offset + length {
            res[i - offset] = self.memory[i];
        }
        res
    }
}



fn sreg(reg: u8) -> &'static str {
    match reg {
        0 => "es",
        1 => "cs",
        2 => "ss",
        3 => "ds",
        4 => "fs",
        5 => "gs",
        _ => "?",
    }
}

// 16 bit addressing modes
fn amode(reg: u8) -> &'static str {
    match reg {
        0 => "bx+si",
        1 => "bx+di",
        2 => "bp+si",
        3 => "bp+di",
        4 => "si",
        5 => "di",
        6 => "bp",
        7 => "bx",
        _ => "?",
    }
}

#[test]
fn can_execute_sr_r16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB9, 0x23, 0x01, // mov cx,0x123
        0x8E, 0xC1,       // mov es,cx   | sr, r16
    ];
    cpu.load_rom(&code, 0x100);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.pc);
    assert_eq!(0x123, cpu.r16[CX].u16());

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.pc);
    assert_eq!(0x123, cpu.r16[ES].u16());
}

#[test]
fn can_execute_r16_r16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x23, 0x01, // mov ax,0x123
        0x8B, 0xE0,       // mov sp,ax   | r16, r16
    ];
    cpu.load_rom(&code, 0x100);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.pc);
    assert_eq!(0x123, cpu.r16[AX].u16());

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.pc);
    assert_eq!(0x123, cpu.r16[SP].u16());
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
    cpu.load_rom(&code, 0x100);

    cpu.execute_instruction(); // mov
    cpu.execute_instruction(); // mov

    assert_eq!(0xFFF0, cpu.r16[SP].u16());
    cpu.execute_instruction(); // push
    assert_eq!(0xFFEE, cpu.r16[SP].u16());
    cpu.execute_instruction(); // pop
    assert_eq!(0xFFF0, cpu.r16[SP].u16());

    assert_eq!(0x107, cpu.pc);
    assert_eq!(0x8888, cpu.r16[AX].u16());
    assert_eq!(0x8888, cpu.r16[DS].u16());
    assert_eq!(0x8888, cpu.r16[ES].u16());
}



#[test]
fn can_execute_mov() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB2, 0x13, // mov dl,0x13
        0x88, 0xD0, // mov al,dl
    ];
    cpu.load_rom(&code, 0x100);

    cpu.execute_instruction(); // mov dl,0x13
    assert_eq!(0x102, cpu.pc);
    assert_eq!(0x13, cpu.r16[DX].lo_u8());

    cpu.execute_instruction(); // mov al,dl
    assert_eq!(0x104, cpu.pc);
    assert_eq!(0x13, cpu.r16[AX].lo_u8());
}
