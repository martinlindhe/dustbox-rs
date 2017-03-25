#![allow(dead_code)]
#![allow(unused_variables)]

pub struct CPU {
    pub pc: u16,
    memory: Vec<u8>,
    // 8 low = r16, 8 hi = es,cs,ss,ds,fs,gs
    r16: [Register16; 16],
}

#[derive(Debug, Copy, Clone)] // XXX only need Copy ??
struct Register16 {
    hi: u8,
    lo: u8,
}

impl Register16 {
    fn set_u16(&mut self, val: u16) {
        self.hi = (val >> 8) as u8;
        self.lo = (val & 0xff) as u8;
    }
    fn u16(&self) -> u16 {
        (self.hi as u16) << 8 | self.lo as u16
    }
}



struct ModRegRm {
    md: u8, // NOTE: "mod" is reserved in rust
    reg: u8,
    rm: u8,
}

#[derive(Debug)]
struct Parameters {
    src: Parameter,
    dst: Parameter,
}

#[derive(Debug)]
enum Parameter {
    Imm8(u8),
    Imm16(u16),
    Reg(usize), // index into CPU.r16
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
const ES: usize = 8;
const CS: usize = 9;
const SS: usize = 10;
const DS: usize = 11;
const FS: usize = 12;
const GS: usize = 13;

impl CPU {
    pub fn new() -> CPU {
        CPU {
            pc: 0,
            memory: vec![0u8; 0x10000 * 64], // = 4 MB. maybe shoudl allocate .?1
            r16: [Register16 { hi: 0, lo: 0 }; 16],
        }
    }

    pub fn reset(&mut self) {
        self.pc = 0;
    }

    pub fn load_rom(&mut self, data: &Vec<u8>, offset: u16) {
        self.pc = offset;

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

    pub fn execute_instruction(&mut self) {
        let b = self.memory[self.pc as usize];
        self.pc += 1;
        match b {
            //0x48...0x4F => format!("dec {}", r16(b & 7)),
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
            0xB0...0xB7 => {
                // mov r8, u8
                let p = Parameters {
                    dst: Parameter::Reg((b & 7) as usize),
                    src: Parameter::Imm8(self.read_u8()),
                };
                self.mov_u8(&p);
            }
            0xB8...0xBF => {
                // mov r16, u16
                let reg = (b & 7) as usize;
                self.r16[reg].lo = self.read_u8();
                self.r16[reg].hi = self.read_u8();
            }
            0xCD => {
                // int u8
                // XXX jump to offset 0x21 in interrupt table (look up how hw does this)
                // http://wiki.osdev.org/Interrupt_Vector_Table
                error!("XXX IMPL: int {:02X}", self.read_u8());
            }
            0xE8 => {
                // call s16
                let temp_ip = (self.read_s16() + (self.pc as i16)) as u16;
                let old_ip = self.pc;
                self.push16(old_ip);
                self.pc = temp_ip;
            }
            _ => error!("UNHANDLED OP {:02X} AT {:04X}", b, self.pc - 1),
        };
    }

    fn push16(&mut self, data: u16) {
        // self.sp -= 2; // XXX should be really 16-bit internally with setter for hi/lo
        error!("XXX also decrease SP");

        let offset = (self.r16[SS].u16() as usize) * 16 + (self.r16[SP].u16() as usize);
        warn!("push16 to {:04X}:{:04X}  =>  {:06X}",
              self.r16[SS].u16(),
              self.r16[SP].u16(),
              offset);

        self.write_u16(offset, data);
    }

    fn mov_u8(&mut self, p: &Parameters) {
        match p.src {
            Parameter::Imm8(imm) => {
                match p.dst {
                    Parameter::Reg(dst_r) => {
                        let lor = dst_r & 3;
                        if dst_r & 4 == 0 {
                            self.r16[lor].lo = imm;
                        } else {
                            self.r16[lor].hi = imm;
                        }
                    }
                    Parameter::Imm8(imm2) => {
                        error!("mov_u8 PARAM Imm8 PANIC");
                    }
                    Parameter::Imm16(imm2) => {
                        error!("mov_u8 PARAM Imm16 PANIC");
                    }
                }
            }
            Parameter::Reg(r) => {
                error!("mov_u8 PARAM-ONE Reg PANIC");
            }
            Parameter::Imm16(imm2) => {
                error!("mov_u8 PARAM-ONE Imm16 PANIC");
            }
        }
    }

    fn mov_r16(&mut self, x: &Parameters) {
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
                        error!("!! XXX Imm8-SUB unhandled - PANIC {:?}", imm);
                    }
                }
            }
            Parameter::Imm16(imm) => {
                error!("!! XXX Imm16 unhandled - PANIC {:?}", imm);
            }
            Parameter::Imm8(imm) => {
                error!("!! XXX Imm8 unhandled - PANIC {:?}", imm);
            }
        }
    }

    pub fn print_registers(&mut self) {
        print!("pc:{:04X}  ax:{:04X} bx:{:04X} cx:{:04X} dx:{:04X}",
               self.pc,
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
               self.r16[ES].u16(),
               self.r16[CS].u16(),
               self.r16[SS].u16(),
               self.r16[DS].u16(),
               self.r16[FS].u16(),
               self.r16[GS].u16());

        println!("");
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
                Parameter::Imm16(self.peek_u16_at(pos))
            }
            1 => {
                // [reg+d8]
                // XXX read value of amode(x.rm) into pos
                error!("XXX FIXME rm16 [reg+d8]");
                let mut pos = 0;
                pos += self.read_s8() as u16; // XXX handle signed properly

                Parameter::Imm16(self.peek_u16_at(pos))
            }
            2 => {
                // [reg+d16]
                // XXX read value of amode(x.rm) into pos
                error!("XXX FIXME rm16 [reg+d16]");
                let mut pos = 0;
                pos += self.read_s16() as u16; // XXX handle signed properly

                Parameter::Imm16(self.peek_u16_at(pos))
            }
            _ => {
                // general purpose r16
                Parameter::Reg(rm as usize)
            }
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

    fn read_mod_reg_rm(&mut self) -> ModRegRm {
        let b = self.read_u8();
        ModRegRm {
            md: b >> 6,
            reg: (b >> 3) & 7,
            rm: b & 7,
        }
    }

    fn read_u8(&mut self) -> u8 {
        let offset = (self.r16[CS].u16() as usize) + self.pc as usize;
        let b = self.memory[offset];
        self.pc += 1;
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

    fn peek_u16_at(&mut self, pos: u16) -> u16 {
        error!("XXX implement peek_u16_at");
        0
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
