
pub struct Disassembly {
    pub pc: usize,
    //rom: Vec<u8>,
    //memory: [u8; 0x10000],
}

pub struct Instruction {
    pub offset: usize,
    pub length: usize,
    pub text: String,
    pub bytes: Vec<u8>,
}

struct ModRegRm {
    md: u8, // "mod" is reserved in rust
    reg: u8,
    rm: u8,
}

struct Parameters {
    src: String,
    dst: String,
}


impl Disassembly {
    pub fn new() -> Disassembly {
        Disassembly {
            pc: 0,
            //memory: [0; 0x10000],
            //rom: vec![],
        }
    }

    // loads data into a 64k area starting at offset, then disassembles all of it
    pub fn disassemble(&mut self, data: &Vec<u8>, offset: usize) -> String {

        // TODO LATER: could use rust iter features


        //let data = disasm.read_u8_slice(op.offset as usize, op.length);
        //info!("{:04X}: {}   {}", op.offset, tools::to_hex_string(&data), op.text);


        let mut count = 0;
        let mut res = vec![];
        loop {
            let op = self.disasm_instruction(data);
            res.push(format!("{:04X}: {}   {}", op.offset, to_hex_string(&op.bytes), op.text));
            count += op.length as usize;
            if count >= data.len() {
                break;
            }
        }

        res.join("\n")
    }

    pub fn disasm_instruction(&mut self, data: &Vec<u8>) -> Instruction {
        let b = data[self.pc];
        let offset = self.pc;
        self.pc += 1;
        let s = match b {
            0x06 => format!("push  es"),
            0x07 => format!("pop   es"),
            0x1E => format!("push  ds"),
            0x31 => {
                let p = self.rm16_r16();
                format!("xor   {}, {}", p.dst, p.src)
            }
            0x40...0x47 => format!("inc   {}", r16(b & 7)),
            0x48...0x4F => format!("dec   {}", r16(b & 7)),
            0x50...0x57 => format!("push  {}", r16(b & 7)),
            0x88 => {
                // mov r8, r/m8
                let p = self.r8_rm8();
                format!("mov   {}, {}", p.dst, p.src)

            }
            0x8B => {
                let p = self.r16_rm16();
                format!("mov   {}, {}", p.dst, p.src)
            }
            0x8E => {
                let p = self.sreg_rm16();
                format!("mov   {}, {}", p.dst, p.src)
            }
            0xAA => format!("stosb"),
            0xAB => format!("stosw"),
            0xAC => format!("lodsb"),
            0xAD => format!("lodsw"),
            0xAE => format!("scasb"),
            0xAF => format!("scasw"),
            0xB0...0xB7 => format!("mov   {}, {:02X}", r8(b & 7), self.read_u8()),
            0xB8...0xBF => format!("mov   {}, {:04X}", r16(b & 7), self.read_u16()),
            0xCD => format!("int   {:02X}", self.read_u8()),
            0xE2 => format!("loop  {:04X}", self.read_rel8()),
            0xE8 => format!("call  {:04X}", self.read_rel16()),
            0xFA => format!("cli"),
            _ => {
                error!("disasm: unknown op {:02X} at {:04X}", b, offset);
                format!("db {:02X}", b)
            }
        };

        Instruction {
            offset: offset,
            length: self.pc - offset,
            text: s,
            bytes: self.read_u8_slice(offset, self.pc - offset),
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
            src: sreg(x.reg).to_string(),
            dst: self.decode_rm16(x.rm, x.md),
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

    fn r8_rm8(&mut self) -> Parameters {
        let x = self.read_mod_reg_rm();
        Parameters {
            src: r8(x.reg).to_string(),
            dst: self.decode_rm8(x.rm, x.md),
        }
    }

    // decode r/m16, r16
    fn rm16_r16(&mut self) -> Parameters {
        let x = self.read_mod_reg_rm();
        Parameters {
            src: r16(x.reg).to_string(),
            dst: self.decode_rm16(x.rm, x.md),
        }
    }

    fn decode_rm16(&mut self, rm: u8, md: u8) -> String {
        match md {
            0 => {
                // [reg]
                if rm == 6 {
                    format!("[{:04X}]", self.read_u16())
                } else {
                    format!("[{}]", amode(rm))
                }
            }
            1 => {
                // [reg+d8]
                error!("XXX [reg+d8] signed value formatting!?=!?1§1");
                format!("[{}{:02X}]", amode(rm), self.read_s8())
            }
            2 => {
                // [reg+d16]
                error!("XXX [reg+d16] signed value formatting!?=!?1§1");
                format!("[{}{:04X}]", amode(rm), self.read_s16())
            }
            _ => r16(rm).to_string(),
        }
    }

    fn decode_rm8(&mut self, rm: u8, md: u8) -> String {
        match md {
            0 => {
                // [reg]
                if rm == 6 {
                    format!("[{:04X}]", self.read_u16())
                } else {
                    format!("[{}]", amode(rm))
                }
            }
            1 => {
                // [reg+d8]
                error!("XXX [reg+d8] signed value formatting!?=!?1§1");
                format!("[{}{:02X}]", amode(rm), self.read_s8())
            }
            2 => {
                // [reg+d16]
                error!("XXX [reg+d16] signed value formatting!?=!?1§1");
                format!("[{}{:04X}]", amode(rm), self.read_s16())
            }
            _ => r8(rm).to_string(),
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
        let b = self.memory[self.pc as usize];
        self.pc += 1;
        b
    }

    fn read_u16(&mut self) -> u16 {
        let lo = self.read_u8();
        let hi = self.read_u8();
        (hi as u16) << 8 | lo as u16
    }

    fn read_rel8(&mut self) -> u16 {
        let val = self.read_u8() as i8;
        (self.pc as i16 + (val as i16)) as u16
    }

    fn read_rel16(&mut self) -> u16 {
        let val = self.read_u16() as i16;
        (self.pc as i16 + val) as u16
    }

    fn read_s8(&mut self) -> i8 {
        self.read_u8() as i8
    }

    fn read_s16(&mut self) -> i16 {
        self.read_u16() as i16
    }
}

fn r8(reg: u8) -> &'static str {
    match reg {
        0 => "al",
        1 => "cl",
        2 => "dl",
        3 => "bl",
        4 => "ah",
        5 => "ch",
        6 => "dh",
        7 => "bh",
        _ => "?",
    }
}
fn r16(reg: u8) -> &'static str {
    match reg {
        0 => "ax",
        1 => "cx",
        2 => "dx",
        3 => "bx",
        4 => "sp",
        5 => "bp",
        6 => "si",
        7 => "di",
        _ => "?",
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

pub fn to_hex_string(bytes: &Vec<u8>) -> String {
    let strs: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    strs.join(" ")
}

#[test]
fn can_disassemble_basic_instructions() {
    let mut disasm = Disassembly::new();
    let code: Vec<u8> = vec![
        0xE8, 0x05, 0x00, // call l_0x108   ; call a later offset
        0xBA, 0x0B, 0x01, // mov dx,0x10b
        0xB4, 0x09,       // mov ah,0x9
        0xCD, 0x21,       // l_0x108: int 0x21
        0xE8, 0xFB, 0xFF, // call l_0x108   ; call an earlier offset
        /*0x26,*/  //0x8B, 0x05, // mov ax,[es:di]  - XXX 0x26 means next instr uses segment ES
    ];
    let res = disasm.disassemble(&code, 0x100);

    assert_eq!("0100: call  0108
0103: mov   dx, 010B
0106: mov   ah, 09
0108: int   21
010A: call  0108",
               //010D: mov ax,[es:di]",
               res);
    /*
    assert_diff!("0100: call 0108
0103: mov dx, 010B
0106: mov ah, 09
0108: int 21
010A: call 0108",
                 &res,
                 "\n",
                 0);
*/
}

#[test]
fn can_disassemble_xor() {
    let mut disasm = Disassembly::new();
    let code: Vec<u8> = vec![
        0x31, 0xC1, // xor cx,ax
        0x31, 0xC8, // xor ax,cx
    ];
    let res = disasm.disassemble(&code, 0x100);

    assert_eq!("0100: xor   cx, ax
0102: xor   ax, cx",
               res);
}


#[test]
fn can_disassemble_mov() {
    let mut disasm = Disassembly::new();
    let code: Vec<u8> = vec![
        0x88, 0xC8, // mov al, cl
    ];
    let res = disasm.disassemble(&code, 0x100);

    assert_eq!("0100: mov   al, cl", res);
}
