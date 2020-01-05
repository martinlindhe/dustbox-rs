use std::{mem, u8};
use std::num::Wrapping;
use std::fs::File;
use std::path::Path;
use std::error::Error;
use std::io::{BufWriter, Write};

use bincode::deserialize;

use crate::bios::BIOS;
use crate::cpu::{CPU, Op, Invalid, R, RegisterState};
use crate::cpu::{Instruction, RepeatMode, Exception};
use crate::cpu::{Parameter};
use crate::gpu::GFXMode;
use crate::hex::hex_bytes;
use crate::interrupt;
use crate::memory::{MMU, MemoryAddress};
use crate::ndisasm::ndisasm_first_instr;

use crate::storage::Storage as StorageComponent;
use crate::keyboard::Keyboard as KeyboardComponent;
use crate::mouse::Mouse as MouseComponent;
use crate::pic::PIC as PICComponent;
use crate::pit::PIT as PITComponent;
use crate::gpu::GPU as GPUComponent;

#[cfg(test)]
#[path = "./machine_test.rs"]
mod machine_test;

/// prints each instruction as they are executed
const DEBUG_EXEC: bool = false;

/// prints access to I/O ports
const DEBUG_IO: bool = false;

/// DEBUG FEATURE: adds a 16-bit stack marker in order to end execution if it is found
pub const DEBUG_MARK_STACK: bool = false;

/// value used to taint the stack, to notice on errors or small com apps just using "retn" to exit to DOS
pub const STACK_MARKER: u16 = 0xDEAD;

#[derive(Deserialize, Debug)]
struct ExeHeader {
    signature: u16,             // 0x5A4D == "MZ"
    bytes_in_last_block: u16,   // padding info for exact data size
    blocks_in_file: u16,        // data size in 512-byte blocks
    num_relocs: u16,            // number of relocation items
    header_paragraphs: u16,     // header size in 16-byte paragraphs
    min_extra_paragraphs: u16,
    max_extra_paragraphs: u16,
    ss: u16,
    sp: u16,
    checksum: u16,
    ip: u16,
    cs: u16,
    reloc_table_offset: u16,
    overlay_number: u16,
}

#[derive(Deserialize, Debug)]
struct ExeReloc {
    offset: u16,
    segment: u16,
}

struct Exe {
    header: ExeHeader,
    relocs: Vec<ExeReloc>,
}

pub enum MachineComponent {
    Storage(StorageComponent),
    Keyboard(KeyboardComponent),
    Mouse(MouseComponent),
    PIC(PICComponent),
    PIT(PITComponent),
    GPU(GPUComponent),
}

pub trait Component {
    /// returns Some<u8> if read was handled
    fn in_u8(&mut self, _port: u16) -> Option<u8> {
        None
    }

    /// returns true if write was handled
    fn out_u8(&mut self, _port: u16, _data: u8) -> bool {
        false
    }

    /// returns true if write was handled
    fn out_u16(&mut self, _port: u16, _data: u16) -> bool {
        false
    }

    /// returns true if interrupt was handled
    fn int(&mut self, _int: u8, _cpu: &mut CPU, _mmu: &mut MMU) -> bool {
        false
    }
}

pub struct Machine {
    pub mmu: MMU,
    pub bios: BIOS,
    pub cpu: CPU,

    /// base offset where rom was loaded
    pub rom_base: MemoryAddress,

    /// length of loaded rom in bytes (used by disassembler)
    pub rom_length: usize,

    /// if true, write opcode trace to trace_file
    tracing: bool,
    trace_file: Option<File>,

    /// handlers for i/o ports and interrupts
    components: Vec<MachineComponent>,
}

impl Machine {
     // returns a non-deterministic Machine instance
    pub fn default() -> Self {
        let mut m = Self::deterministic();
        m.pit_mut().unwrap().init();
        m
    }

    pub fn deterministic() -> Self {
        let mut mmu = MMU::default();
        let mut bios = BIOS::default();
        bios.init(&mut mmu);

        let mut m = Machine {
            cpu: CPU::deterministic(),
            mmu,
            bios,
            rom_base: MemoryAddress::default_real(),
            rom_length: 0,
            tracing: false,
            trace_file: None,
            components: Vec::new(),
        };

        m.register_components();
        m
    }

    /// Enables writing of opcode trace to file.
    /// The format tries to be similar to dosbox debugger "LOGS" format.
    pub fn write_trace_to(&mut self, filename: &str) {
        let trace_path = Path::new(filename);

        let file = match File::create(&trace_path) {
            Err(why) => panic!("couldn't create {:?}: {}", trace_path.display(), why.description()),
            Ok(file) => file,
        };

        self.trace_file = Some(file);
        self.tracing = true;
    }

    fn register_components(&mut self) {
        self.components.push(MachineComponent::PIC(PICComponent::new(0x0020)));
        self.components.push(MachineComponent::PIC(PICComponent::new(0x00A0)));
        self.components.push(MachineComponent::PIT(PITComponent::default()));
        self.components.push(MachineComponent::Keyboard(KeyboardComponent::default()));
        self.components.push(MachineComponent::Mouse(MouseComponent::default()));
        self.components.push(MachineComponent::Storage(StorageComponent::default()));

        let mut gpu = GPUComponent::default();
        gpu.init(&mut self.mmu);
        gpu.set_mode(&mut self.mmu, GFXMode::MODE_TEXT_80_25 as u8);
        self.components.push(MachineComponent::GPU(gpu));
    }

    /// returns a mutable reference to the PIT component
    pub fn pit_mut(&mut self) -> Option<&mut PITComponent> {
        for component in &mut self.components {
            if let MachineComponent::PIT(c) = component {
                return Some(c);
            }
        }
        None
    }

    /// returns a mutable reference to the Keyboard component
    pub fn keyboard_mut(&mut self) -> Option<&mut KeyboardComponent> {
        for component in &mut self.components {
            if let MachineComponent::Keyboard(c) = component {
                return Some(c);
            }
        }
        None
    }

    /// returns a mutable reference to the GPU component
    pub fn gpu_mut(&mut self) -> Option<&mut GPUComponent> {
        for component in &mut self.components {
            if let MachineComponent::GPU(c) = component {
                return Some(c);
            }
        }
        None
    }

    /// returns a reference to the GPU component
    pub fn gpu(&self) -> Option<&GPUComponent> {
        for component in &self.components {
            if let MachineComponent::GPU(c) = component {
                return Some(c);
            }
        }
        None
    }

    /// reset the CPU and memory
    pub fn hard_reset(&mut self) {
        self.cpu = CPU::default();
    }

    pub fn load_executable(&mut self, data: &[u8]) {
        if data[0] == b'M' && data[1] == b'Z' {
            self.load_exe(data);
        } else {
            self.load_com(data);
        }
    }

    /// loads an exe file (TODO finish impl)
    fn load_exe(&mut self, data: &[u8]) {
        let hdr: ExeHeader = deserialize(data).unwrap();
        println!("load_exe header: {:?}", hdr);

        let reloc_from = hdr.reloc_table_offset as usize;
        let reloc_to = hdr.reloc_table_offset as usize + (hdr.num_relocs as usize * 4);
        println!("load_exe read relocs from {:04X}-{:04X}", reloc_from, reloc_to);

        // let relocs: Vec<ExeReloc> = deserialize(&data[reloc_from..reloc_to]).unwrap();  // XXX crashes
        let relocs: ExeReloc = deserialize(&data[reloc_from..reloc_to]).unwrap(); // XXX only reads first reloc
        println!("XXX relocs: {:?}", relocs);

        let code_offset = hdr.header_paragraphs as usize * 16;
        let mut code_end = hdr.blocks_in_file as usize * 512;
        if hdr.bytes_in_last_block > 0 {
            code_end -= 512 - hdr.bytes_in_last_block as usize;
        }
        println!("load exe code from {:04X}:{:04X}", code_offset, code_end);

        self.load_com(&data[code_offset..code_end]);
        self.cpu.set_r16(R::SP, hdr.sp);
        self.cpu.set_r16(R::SS, hdr.ss); // XXX dosbox = 0923
        
        // at program start in dosbox-x:
        // BP = 091C (dustbox ok)
        // SP = 1000 (from hdr, dustbox ok)
        // CS = 0920
        // DS = 0910
        // ES = 0910
        // SS = 0923

        // XXX SI and DI values
    }

    /// load .com program into CS:0100 and set IP to program start
    fn load_com(&mut self, data: &[u8]) {

        // CS,DS,ES,SS = PSP segment
        let psp_segment = 0x0329;
        self.cpu.set_r16(R::CS, psp_segment);
        self.cpu.set_r16(R::DS, psp_segment);
        self.cpu.set_r16(R::ES, psp_segment);
        self.cpu.set_r16(R::SS, psp_segment);

        // offset of last word available in first 64k segment
        self.cpu.set_r16(R::SP, 0xFFFE);
        self.cpu.set_r16(R::BP, 0x091C); // is what dosbox used

        // This is what dosbox initializes the registers to
        // at program load
        self.cpu.set_r16(R::CX, 0x00FF);
        self.cpu.set_r16(R::DX, psp_segment);
        self.cpu.set_r16(R::SI, 0x0100);
        self.cpu.set_r16(R::DI, 0xFFFE);

        self.cpu.regs.flags.interrupt = true;

        self.cpu.regs.ip = 0x0100;
        self.rom_base = self.cpu.get_memory_address();
        self.rom_length = data.len();

        let cs = self.cpu.get_r16(R::CS);
        self.mmu.write(cs, self.cpu.regs.ip, data);

        self.mark_stack();
    }

    /// (for debugging): marks the stack with a magic value so we can detect when last "ret" exits the application
    fn mark_stack(&mut self) {
        if DEBUG_MARK_STACK {
            self.cpu.push16(&mut self.mmu, STACK_MARKER);
        }
    }

    /// returns a copy of register values at a given time
    pub fn register_snapshot(&self) -> RegisterState {
        self.cpu.regs.clone()
    }

    /// executes enough instructions that can run for 1 video frame
    pub fn execute_frame(&mut self) {
        let fps = 60;
        let cycles = self.cpu.clock_hz / fps;
        // println!("will execute {} cycles", cycles);

        loop {
            self.execute_instruction();
            if self.cpu.fatal_error {
                break;
            }
            if self.cpu.cycle_count > cycles {
                self.cpu.cycle_count = 0;
                break;
            }
        }
    }

    /// executes n instructions of the cpu
    pub fn execute_instructions(&mut self, count: usize) {
        for _ in 0..count {
            self.execute_instruction();
            if self.cpu.fatal_error {
                break;
            }
        }
    }

    /// returns first line of disassembly using nasm
    fn external_disasm_of_bytes(&self, cs: u16, ip: u16) -> String {
        let bytes = self.mmu.read(cs, ip, 16);
        ndisasm_first_instr(&bytes).unwrap()
    }

    fn handle_interrupt(&mut self, int: u8) {
        // ask subsystems if they can handle the interrupt
        for component in &mut self.components {
            let handled = match component {
                MachineComponent::PIC(c) => c.int(int, &mut self.cpu, &mut self.mmu),
                MachineComponent::PIT(c) => c.int(int, &mut self.cpu, &mut self.mmu),
                MachineComponent::Keyboard(c) => c.int(int, &mut self.cpu, &mut self.mmu),
                MachineComponent::Mouse(c) => c.int(int, &mut self.cpu, &mut self.mmu),
                MachineComponent::Storage(c) => c.int(int, &mut self.cpu, &mut self.mmu),
                MachineComponent::GPU(c) => c.int(int, &mut self.cpu, &mut self.mmu),
            };
            if handled {
                return;
            }
        }

        match int {
            0x03 => {
                // debugger interrupt
                // http://www.ctyme.com/intr/int-03.htm
                println!("INT 3 - debugger interrupt. AX={:04X}", self.cpu.get_r16(R::AX));
                self.cpu.fatal_error = true; // stops execution
            }
            0x20 => {
                // DOS 1+ - TERMINATE PROGRAM
                // NOTE: Windows overloads INT 20
                println!("INT 20 - Terminating program");
                self.cpu.fatal_error = true; // stops execution
            }
            0x21 => interrupt::int21::handle(self),
            _ => {
                println!("int error: unknown interrupt {:02X}, AX={:04X}, BX={:04X}",
                        int,
                        self.cpu.get_r16(R::AX),
                        self.cpu.get_r16(R::BX));
            }
        }
    }

    pub fn execute_interrupt(&mut self, int: u8) {
        let flags = self.cpu.regs.flags.u16();
        self.cpu.push16(&mut self.mmu, flags);
        self.mmu.flags_address = MemoryAddress::RealSegmentOffset(self.cpu.get_r16(R::SS), self.cpu.get_r16(R::SP));

        self.cpu.regs.flags.interrupt = false;
        self.cpu.regs.flags.trap = false;
        let (cs, ip) = self.cpu.get_address_pair();
        self.cpu.push16(&mut self.mmu, cs);
        self.cpu.push16(&mut self.mmu, ip);
        let base = 0;
        let idx = u16::from(int) << 2;
        let ip = self.mmu.read_u16(base, idx);
        let cs = self.mmu.read_u16(base, idx + 2);
        // println!("int: jumping to interrupt handler for interrupt {:02X} pos at {:04X}:{:04X} = {:04X}:{:04X}", int, base, idx, cs, ip);
        self.cpu.regs.ip = ip;
        self.cpu.set_r16(R::CS, cs);
    }

    pub fn execute_instruction(&mut self) {
        let cs = self.cpu.get_r16(R::CS);
        let ip = self.cpu.regs.ip;
        if cs == 0xF000 {
            // we are in interrupt vector code, execute high-level interrupt.
            // the default interrupt vector table has a IRET
            self.handle_interrupt(ip as u8);
        }

        let op = self.cpu.decoder.get_instruction(&mut self.mmu, cs, ip);

        if self.tracing {
            let ax = self.cpu.get_r16(R::AX);
            let bx = self.cpu.get_r16(R::BX);
            let cx = self.cpu.get_r16(R::CX);
            let dx = self.cpu.get_r16(R::DX);

            let si = self.cpu.get_r16(R::SI);
            let di = self.cpu.get_r16(R::DI);
            let bp = self.cpu.get_r16(R::BP);
            let sp = self.cpu.get_r16(R::SP);

            let ds = self.cpu.get_r16(R::DS);
            let es = self.cpu.get_r16(R::ES);
            //let fs = self.cpu.get_r16(R::FS);
            //let gs = self.cpu.get_r16(R::GS);
            let ss = self.cpu.get_r16(R::SS);

            let cf = self.cpu.regs.flags.carry_numeric();
            let zf = self.cpu.regs.flags.zero_numeric();
            let sf = self.cpu.regs.flags.sign_numeric();
            let of = self.cpu.regs.flags.overflow_numeric();
            let iflag = self.cpu.regs.flags.interrupt_numeric();

            // format similar to dosbox LOGS output
            if let Some(file) = &self.trace_file {
                let disasm = &format!("{:30}", format!("{}", op))[..30];
                let mut writer = BufWriter::new(file);
                let _ = write!(&mut writer, "{:04X}:{:04X}  {}", cs, ip, disasm);
                let _ = write!(&mut writer, " EAX:{:08X} EBX:{:08X} ECX:{:08X} EDX:{:08X} ESI:{:08X} EDI:{:08X} EBP:{:08X} ESP:{:08X}", ax, bx, cx, dx, si, di, bp, sp);
                let _ = write!(&mut writer, " DS:{:04X} ES:{:04X}", ds, es);
                // let _ = write!(&mut writer, " FS:{:04X} GS:{:04X}", fs, g);
                let _ = write!(&mut writer, " SS:{:04X}", ss);
                let _ = write!(&mut writer, " C{} Z{} S{} O{} I{}\n", cf, zf, sf, of, iflag);
            }
        }

        match op.command {
            Op::Uninitialized => {
                self.cpu.fatal_error = true;
                println!("[{:04X}:{:04X}] ERROR: uninitialized op. {} instructions executed",
                         cs, ip, self.cpu.instruction_count);
            }
            Op::Invalid(bytes, reason) => {
                let hex = hex_bytes(&bytes);
                self.cpu.fatal_error = true;
                match reason {
                    Invalid::Op => {
                        println!("[{:04X}:{:04X}] {} ERROR: unhandled opcode", cs, ip, hex);
                        println!("ndisasm: {}", self.external_disasm_of_bytes(cs, ip));
                    }
                    Invalid::FPUOp => {
                        println!("[{:04X}:{:04X}] {} ERROR: unhandled FPU opcode", cs, ip, hex);
                        println!("ndisasm: {}", self.external_disasm_of_bytes(cs, ip));
                    }
                    Invalid::Reg(reg) => {
                        println!("[{:04X}:{:04X}] {} ERROR: unhandled reg value {:02X}", cs, ip, hex, reg);
                        println!("ndisasm: {}", self.external_disasm_of_bytes(cs, ip));
                    }
                }
                println!("{} Instructions executed", self.cpu.instruction_count);
            }
            _ => {
                if DEBUG_EXEC {
                    println!("[{:04X}:{:04X}] {}", cs, ip, op);
                }
                self.execute(&op);
            },
        }

        if self.cpu.cycle_count % 100 == 0 {
            // XXX need instruction timing to do this properly
            self.gpu_mut().unwrap().progress_scanline();
        }

        // HACK: pit should be updated regularry, but in a deterministic way
        if self.cpu.cycle_count % 100 == 0 {
            for component in &mut self.components {
                if let MachineComponent::PIT(pit) = component {
                    pit.update(&mut self.mmu);
                }
            }
        }

    }

    /// read byte from I/O port
    pub fn in_u8(&mut self, port: u16) -> u8 {
        if DEBUG_IO {
            println!("in_u8: read from {:04X}", port);
        }

        for component in &mut self.components {
            let handled = match component {
                MachineComponent::PIC(c) => c.in_u8(port),
                MachineComponent::PIT(c) => c.in_u8(port),
                MachineComponent::Keyboard(c) => c.in_u8(port),
                MachineComponent::Mouse(c) => c.in_u8(port),
                MachineComponent::Storage(c) => c.in_u8(port),
                MachineComponent::GPU(c) => c.in_u8(port),
            };
            if let Some(v) = handled {
                return v;
            }
        }

        match port {
            // PORT 0000-001F - DMA 1 - FIRST DIRECT MEMORY ACCESS CONTROLLER (8237)
            0x0002 => {
                // DMA channel 1	current address		byte  0, then byte 1
                println!("XXX fixme in_port read DMA channel 1 current address");
                0
            }

            0x0201 => {
                // read joystick position and status
                // Bit(s)	Description	(Table P0542)
                //  7	status B joystick button 2 / D paddle button
                //  6	status B joystick button 1 / C paddle button
                //  5	status A joystick button 2 / B paddle button
                //  4	status A joystick button 1 / A paddle button
                //  3	B joystick Y coordinate	   / D paddle coordinate
                //  2	B joystick X coordinate	   / C paddle coordinate
                //  1	A joystick Y coordinate	   / B paddle coordinate
                //  0	A joystick X coordinate	   / A paddle coordinate
                0 // XXX
            }
            _ => {
                println!("in_u8: unhandled port {:04X}", port);
                0
            }
        }
    }

    /// read word from I/O port
    pub fn in_u16(&mut self, port: u16) -> u16 {
        println!("in_u16: unhandled read from {:04X}", port);
        0
    }

    /// write byte to I/O port
    pub fn out_u8(&mut self, port: u16, data: u8) {
        if DEBUG_IO {
            println!("out_u8: write to {:04X} = {:02X}", port, data);
        }

        for component in &mut self.components {
            let b = match component {
                MachineComponent::PIC(c) => c.out_u8(port, data),
                MachineComponent::PIT(c) => c.out_u8(port, data),
                MachineComponent::Keyboard(c) => c.out_u8(port, data),
                MachineComponent::Mouse(c) => c.out_u8(port, data),
                MachineComponent::Storage(c) => c.out_u8(port, data),
                MachineComponent::GPU(c) => c.out_u8(port, data),
            };
            if b {
                return;
            }
        }

        match port {
            0x0201 => {
                // W  fire joystick's four one-shots
            }
            // PORT 03F0-03F7 - FDC 1	(1st Floppy Disk Controller)	second FDC at 0370
            0x03F2 => {
                // 03F2  -W  diskette controller DOR (Digital Output Register) (see #P0862)

                // ../dos-software-decoding/games-com/Galaxian (1983)(Atari Inc)/galaxian.com writes 0x0C
            }
            _ => println!("out_u8: unhandled port {:04X} = {:02X}", port, data),
        }
    }

    /// write word to I/O port
    pub fn out_u16(&mut self, port: u16, data: u16) {
        if DEBUG_IO {
            println!("out_u16: write to {:04X} = {:04X}", port, data);
        }
        for component in &mut self.components {
            let b = match component {
                MachineComponent::PIC(c) => c.out_u16(port, data),
                MachineComponent::PIT(c) => c.out_u16(port, data),
                MachineComponent::Keyboard(c) => c.out_u16(port, data),
                MachineComponent::Mouse(c) => c.out_u16(port, data),
                MachineComponent::Storage(c) => c.out_u16(port, data),
                MachineComponent::GPU(c) => c.out_u16(port, data),
            };
            if b {
                return;
            }
        }
        println!("out_u16: unhandled port {:04X} = {:04X}", port, data);
    }

    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cyclomatic_complexity))]
    fn execute(&mut self, op: &Instruction) {
        let start_ip = self.cpu.regs.ip;
        self.cpu.regs.ip = (Wrapping(self.cpu.regs.ip) + Wrapping(u16::from(op.length))).0;
        self.cpu.instruction_count += 1;
        self.cpu.cycle_count += 1; // XXX temp hack; we pretend each instruction takes 8 cycles due to lack of timing
        match op.command {
            Op::Aaa => {
                let v = if self.cpu.get_r8(R::AL) > 0xf9 {
                    2
                 } else {
                    1
                };
                self.cpu.adjb(6, v);
            }
            Op::Aad => {
                // one parameter
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                let mut ax = u16::from(self.cpu.get_r8(R::AH)) * op1;
                ax += u16::from(self.cpu.get_r8(R::AL));
                let al = ax as u8;
                self.cpu.set_r8(R::AL, al);
                self.cpu.set_r8(R::AH, 0);
                // modification of flags A,C,O is undocumented
                self.cpu.regs.flags.carry = false;
                self.cpu.regs.flags.overflow = false;
                self.cpu.regs.flags.adjust = false;
                // The SF, ZF, and PF flags are set according to the resulting binary value in the AL register
                self.cpu.regs.flags.sign = al >= 0x80;
                self.cpu.regs.flags.zero = al == 0;
                self.cpu.regs.flags.set_parity(al as usize);
            }
            Op::Aam => {
                // tempAL ← AL;
                // AH ← tempAL / imm8; (* imm8 is set to 0AH for the AAM mnemonic *)
                // AL ← tempAL MOD imm8;
                let imm8 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u8;
                if imm8 == 0 {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                let al = self.cpu.get_r8(R::AL);
                self.cpu.set_r8(R::AH, al / imm8);
                self.cpu.set_r8(R::AL, al % imm8);
                // modification of flags A,C,O is undocumented
                self.cpu.regs.flags.carry = false;
                self.cpu.regs.flags.overflow = false;
                self.cpu.regs.flags.adjust = false;
                // The SF, ZF, and PF flags are set according to the resulting binary value in the AL register
                let al = self.cpu.get_r8(R::AL);
                self.cpu.regs.flags.sign = al & 0x80 != 0;
                self.cpu.regs.flags.zero = al == 0;
                self.cpu.regs.flags.set_parity(al as usize);
            }
            Op::Aas => {
                let v = if self.cpu.get_r8(R::AL) < 6 {
                    -2
                } else {
                    -1
                };
                self.cpu.adjb(-6, v);
            }
            Op::Adc8 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let carry = if self.cpu.regs.flags.carry { 1 } else { 0 };
                let res = (Wrapping(dst) + Wrapping(src) + Wrapping(carry)).0;
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, (res & 0xFF) as u8);

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_add_u8(res, src + carry, dst);
                self.cpu.regs.flags.set_sign_u8(res);
                self.cpu.regs.flags.set_zero_u8(res);
                self.cpu.regs.flags.set_adjust(res, src + carry, dst);
                self.cpu.regs.flags.set_carry_u8(res);
                self.cpu.regs.flags.set_parity(res);
            }
            Op::Adc16 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let carry = if self.cpu.regs.flags.carry { 1 } else { 0 };
                let res = (Wrapping(dst) + Wrapping(src) + Wrapping(carry)).0;
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, (res & 0xFFFF) as u16);

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_add_u16(res, src + carry, dst);
                self.cpu.regs.flags.set_sign_u16(res);
                self.cpu.regs.flags.set_zero_u16(res);
                self.cpu.regs.flags.set_adjust(res, src + carry, dst);
                self.cpu.regs.flags.set_carry_u16(res);
                self.cpu.regs.flags.set_parity(res);
            }
            Op::Add8 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u8;
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u8;
                let res = src as usize + dst as usize;
                self.cpu.regs.flags.set_carry_u8(res);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.regs.flags.set_adjust(res, src as usize, dst as usize);
                self.cpu.regs.flags.set_zero_u8(res);
                self.cpu.regs.flags.set_sign_u8(res);
                self.cpu.regs.flags.set_overflow_add_u8(res, src as usize, dst as usize);
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res as u8);
            }
            Op::Add16 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u16;
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                let res = src as usize + dst as usize;
                self.cpu.regs.flags.set_carry_u16(res);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.regs.flags.set_adjust(res, src as usize, dst as usize);
                self.cpu.regs.flags.set_zero_u16(res);
                self.cpu.regs.flags.set_sign_u16(res);
                self.cpu.regs.flags.set_overflow_add_u16(res, src as usize, dst as usize);
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Add32 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u32;
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u32;
                let res = src as usize + dst as usize;
                self.cpu.regs.flags.set_carry_u32(res);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.regs.flags.set_adjust(res, src as usize, dst as usize);
                self.cpu.regs.flags.set_zero_u32(res);
                self.cpu.regs.flags.set_sign_u32(res);
                self.cpu.regs.flags.set_overflow_add_u32(res, src as usize, dst as usize);
                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u32);
            }
            Op::And8 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = dst & src;

                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.cpu.regs.flags.overflow = false;
                self.cpu.regs.flags.carry = false;
                self.cpu.regs.flags.set_sign_u8(res);
                self.cpu.regs.flags.set_zero_u8(res);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res as u8);
            }
            Op::And16 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = dst & src;

                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.cpu.regs.flags.overflow = false;
                self.cpu.regs.flags.carry = false;
                self.cpu.regs.flags.set_sign_u16(res);
                self.cpu.regs.flags.set_zero_u16(res);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Arpl => {
                println!("XXX impl {}", op);
                /*
                // NOTE: RPL is the low two bits of the address
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let mut dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                if dst & 3 < src & 3 {
                    self.cpu.regs.flags.zero = true;
                    dst = (dst & 0xFFFC) + (src & 3);
                    self.cpu.write_parameter_u16(&mut self.mmu, op.segment, &op.params.dst, (dst & 0xFFFF) as u16);
                } else {
                    self.cpu.regs.flags.zero = false;
                }
                */
            }
            Op::Bsf => {
                let mut src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                if src == 0 {
                    self.cpu.regs.flags.zero = true;
                } else {
                    let mut count = 0;
                    while src & 1 == 0 {
                        count += 1;
                        src >>= 1;
                    }
                    self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, count);
                    self.cpu.regs.flags.zero = false;
                }
            }
            Op::Bt => {
                let bit_base = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let bit_offset = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                self.cpu.regs.flags.carry = bit_base & (1 << (bit_offset & 15)) != 0;
            }
            Op::Bound => {
                // XXX throw BR exception if out of bounds
                println!("XXX impl {}", op);
            }
            Op::CallNear => {
                let old_ip = self.cpu.regs.ip;
                let temp_ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                self.cpu.push16(&mut self.mmu, old_ip);
                self.cpu.regs.ip = temp_ip as u16;
            }
            Op::CallFar => {
                let old_seg = self.cpu.regs.get_r16(R::CS);
                let old_ip = self.cpu.regs.ip;
                self.cpu.push16(&mut self.mmu, old_seg);
                self.cpu.push16(&mut self.mmu, old_ip);
                match op.params.dst {
                    Parameter::Ptr16Imm(seg, offs) => {
                        self.cpu.regs.ip = offs;
                        self.cpu.regs.set_r16(R::CS, seg);
                    }
                    Parameter::Ptr16(seg, offs) => {
                        let seg = self.cpu.segment(seg);
                        self.cpu.set_r16(R::CS, seg);
                        self.cpu.regs.ip = offs;
                    }
                    Parameter::Ptr16AmodeS8(seg, ref amode, imm) => {
                        let seg = self.cpu.segment(seg);
                        self.cpu.set_r16(R::CS, seg);
                        self.cpu.regs.ip = (self.cpu.amode(amode) as isize + imm as isize) as u16;
                    }
                    _ => panic!("CallFar unhandled type {:?}", op.params.dst),
                }
            }
            Op::Cbw => {
                let ah = if self.cpu.get_r8(R::AL) & 0x80 != 0 {
                    0xFF
                } else {
                    0x00
                };
                self.cpu.set_r8(R::AH, ah);
            }
            Op::Clc => {
                self.cpu.regs.flags.carry = false;
            }
            Op::Cld => {
                self.cpu.regs.flags.direction = false;
            }
            Op::Cli => {
                self.cpu.regs.flags.interrupt = false;
            }
            Op::Cmc => {
                self.cpu.regs.flags.carry = !self.cpu.regs.flags.carry;
            }
            Op::Cmp8 => {
                // two parameters
                // Modify status flags in the same manner as the SUB instruction
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                self.cpu.cmp8(dst, src);
            }
            Op::Cmp16 => {
                // two parameters
                // Modify status flags in the same manner as the SUB instruction
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                self.cpu.cmp16(dst, src);
            }
            Op::Cmp32 => {
                // two parameters
                // Modify status flags in the same manner as the SUB instruction
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                self.cpu.cmp32(dst, src);
            }
            Op::Cmpsw => {
                // no parameters
                // Compare word at address DS:(E)SI with word at address ES:(E)DI
                // The DS segment may be overridden with a segment override prefix, but the ES segment cannot be overridden.
                let src = self.mmu.read_u16(self.cpu.segment(op.segment_prefix), self.cpu.get_r16(R::SI)) as usize;
                let dst = self.mmu.read_u16(self.cpu.get_r16(R::ES), self.cpu.get_r16(R::DI)) as usize;
                self.cpu.cmp16(dst, src);

                let si = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::SI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::SI)) - Wrapping(2)).0
                };
                self.cpu.set_r16(R::SI, si);
                let di = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::DI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::DI)) - Wrapping(2)).0
                };
                self.cpu.set_r16(R::DI, di);
            }
            Op::Cwd16 => {
                // DX:AX ← sign-extend of AX.
                let dx = if self.cpu.get_r16(R::AX) & 0x8000 != 0 {
                    0xFFFF
                } else {
                    0
                };
                self.cpu.set_r16(R::DX, dx);
            }
            Op::Cwde32 => {
                // EAX ← sign-extend of AX.
                let ax = self.cpu.get_r16(R::AX) as i16;
                self.cpu.set_r32(R::EAX, ax as u32);
            }
            Op::Daa => {
                self.cpu.adj4(6, 0x60);
            }
            Op::Das => {
                self.cpu.adj4(-6, -0x60);
            }
            Op::Dec8 => {
                // single parameter (dst)
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_sub_u8(res, src, dst);
                self.cpu.regs.flags.set_sign_u8(res);
                self.cpu.regs.flags.set_zero_u8(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);

                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res as u8);
            }
            Op::Dec16 => {
                // single parameter (dst)
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_sub_u16(res, src, dst);
                self.cpu.regs.flags.set_sign_u16(res);
                self.cpu.regs.flags.set_zero_u16(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);

                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Dec32 => {
                // single parameter (dst)
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_sub_u32(res, src, dst);
                self.cpu.regs.flags.set_sign_u32(res);
                self.cpu.regs.flags.set_zero_u32(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);

                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u32);
            }
            Op::Div8 => {
                // Unsigned divide AX by r/m8, with result stored in AL ← Quotient, AH ← Remainder.
                let ax = self.cpu.get_r16(R::AX) as u16;
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                if op1 == 0 {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                let quotient = ax / op1;
                let remainder = (ax % op1) as u8;
                let quo8 = (quotient & 0xFF) as u8;
                if quotient > 0xFF {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                self.cpu.set_r8(R::AH, remainder);
                self.cpu.set_r8(R::AL, quo8);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Div16 => {
                // Unsigned divide DX:AX by r/m16, with result stored in AX ← Quotient, DX ← Remainder.
                let num = (u32::from(self.cpu.get_r16(R::DX)) << 16) + u32::from(self.cpu.get_r16(R::AX)); // DX:AX
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u32;
                if op1 == 0 {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                let remainder = (num % op1) as u16;
                let quotient = num / op1;
                let quo16 = (quotient & 0xFFFF) as u16;
                if quotient != u32::from(quo16) {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                self.cpu.set_r16(R::DX, remainder);
                self.cpu.set_r16(R::AX, quo16);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Div32 => {
                // Unsigned divide EDX:EAX by r/m32, with result stored in EAX ← Quotient, EDX ← Remainder.
                let num = (u64::from(self.cpu.get_r32(R::EDX)) << 32) + u64::from(self.cpu.get_r32(R::EAX)); // EDX:EAX
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u64;
                if op1 == 0 {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                let remainder = (num % op1) as u32;
                let quotient = num / op1;
                let quo32 = (quotient & 0xFFFF) as u32;
                if quotient != u64::from(quo32) {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                self.cpu.set_r32(R::EDX, remainder);
                self.cpu.set_r32(R::EAX, quo32);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Enter => {
                // Make Stack Frame for Procedure Parameters
                // Create a stack frame with optional nested pointers for a procedure.
                // XXX test this
                let alloc_size = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                let mut nesting_level = self.cpu.read_parameter_value(&self.mmu, &op.params.src);

                nesting_level &= 0x1F; // XXX "mod 32" says docs
                let bp = self.cpu.get_r16(R::BP);
                self.cpu.push16(&mut self.mmu, bp);
                let frame_temp = self.cpu.get_r16(R::SP);

                if nesting_level != 0 {
                    for i in 0..nesting_level {
                        let bp = self.cpu.get_r16(R::BP) - 2;
                        self.cpu.set_r16(R::BP, bp);
                        let val = self.mmu.read_u16(self.cpu.get_r16(R::SS), self.cpu.get_r16(R::BP));
                        println!("XXX ENTER: pushing {} = {:04X}", i, val);
                        self.cpu.push16(&mut self.mmu, val);
                    }
                    self.cpu.push16(&mut self.mmu, frame_temp);
                }

                self.cpu.set_r16(R::BP, frame_temp);
                let sp = self.cpu.get_r16(R::SP) - alloc_size;
                self.cpu.set_r16(R::SP, sp);
            }
            Op::Hlt => {
                // println!("XXX impl {}", op);
                // self.fatal_error = true;
            }
            Op::Idiv8 => {
                let ax = self.cpu.get_r16(R::AX) as i16; // dividend
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as i8;
                if op1 == 0 {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                let rem = (ax % i16::from(op1)) as i8;
                let quo = ax / i16::from(op1);
                let quo8s = (quo & 0xFF) as i8;
                if quo != i16::from(quo8s) {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                self.cpu.set_r8(R::AL, quo as u8);
                self.cpu.set_r8(R::AH, rem as u8);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Idiv16 => {
                let dividend = ((u32::from(self.cpu.get_r16(R::DX)) << 16) | u32::from(self.cpu.get_r16(R::AX))) as i32; // DX:AX
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as i16;
                if op1 == 0 {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                let quo = dividend / i32::from(op1);
                let rem = (dividend % i32::from(op1)) as i16;
                let quo16s = quo as i16;
	            if quo != i32::from(quo16s) {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                self.cpu.set_r16(R::AX, quo16s as u16);
                self.cpu.set_r16(R::DX, rem as u16);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Idiv32 => {
                let dividend = ((u64::from(self.cpu.get_r32(R::EDX)) << 32) | u64::from(self.cpu.get_r32(R::EAX))) as i64; // EDX:EAX
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as i32;
                if op1 == 0 {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                let quo = dividend / i64::from(op1);
                let rem = (dividend % i64::from(op1)) as i32;
                let quo32s = quo as i32;
	            if quo != i64::from(quo32s) {
                    return self.cpu.exception(&Exception::DIV0, 0);
                }
                self.cpu.set_r32(R::EAX, quo32s as u32);
                self.cpu.set_r32(R::EDX, rem as u32);
                // The CF, OF, SF, ZF, AF, and PF flags are undefined.
            }
            Op::Imul8 => {
                // NOTE: only 1-parameter imul8 instruction exists
                // IMUL r/m8               : AX← AL ∗ r/m byte.
                let f1 = self.cpu.get_r8(R::AL) as i8;
                let f2 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as i8;
                let ax = (i16::from(f1) * i16::from(f2)) as u16; // product
                self.cpu.set_r16(R::AX, ax);

                // For the one operand form of the instruction, the CF and OF flags are set when significant
                // bits are carried into the upper half of the result and cleared when the result fits
                // exactly in the lower half of the result.
                if (ax & 0xFF80) == 0xFF80 || (ax & 0xFF80) == 0x0000 {
                    self.cpu.regs.flags.carry = false;
                    self.cpu.regs.flags.overflow = false;
                } else {
                    self.cpu.regs.flags.carry = true;
                    self.cpu.regs.flags.overflow = true;
                }
            }
            Op::Imul16 => {
                match op.params.count() {
                    1 => {
                        // IMUL r/m16               : DX:AX ← AX ∗ r/m word.
                        let a = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as i16;
                        let tmp = (self.cpu.get_r16(R::AX) as i16) as isize * a as isize;
                        self.cpu.set_r16(R::AX, tmp as u16);
                        self.cpu.set_r16(R::DX, (tmp >> 16) as u16);
                    }
                    2 => {
                        // IMUL r16, r/m16          : word register ← word register ∗ r/m16.
                        let a = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                        let b = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                        let tmp = a as isize * b as isize;
                        self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, tmp as u16);
                    }
                    3 => {
                        // IMUL r16, r/m16, imm8    : word register ← r/m16 ∗ sign-extended immediate byte.
                        // IMUL r16, r/m16, imm16   : word register ← r/m16 ∗ immediate word.
                        let a = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                        let b = self.cpu.read_parameter_value(&self.mmu, &op.params.src2);
                        let tmp = b as isize * a as isize;
                        self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, tmp as u16);
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
                        let a = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as i32;
                        let tmp = (self.cpu.get_r32(R::EAX) as i32) as isize * a as isize;
                        self.cpu.set_r32(R::EAX, tmp as u32);
                        self.cpu.set_r32(R::EDX, (tmp >> 32) as u32);
                    }
                    2 => {
                        // IMUL r32, r/m32          : doubleword register ← doubleword register ∗ r/m32.
                        let a = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                        let b = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                        let tmp = a as isize * b as isize;
                        self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, tmp as u32);
                    }
                    3 => {
                        // IMUL r32, r/m32, imm8     : doubleword register ← r/m32 ∗ sign- extended immediate byte.
                        // IMUL r32, r/m32, imm32    : doubleword register ← r/m32 ∗ immediate doubleword.
                        let a = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                        let b = self.cpu.read_parameter_value(&self.mmu, &op.params.src2);
                        let tmp = b as isize * a as isize;
                        self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, tmp as u32);
                    }
                    _ => unreachable!(),
                }
                // XXX flags
            }
            Op::In8 => {
                // two parameters (dst=AL)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let data = self.in_u8(src as u16);
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, data);
            }
            Op::In16 => {
                // two parameters (dst=AX)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let data = self.in_u16(src as u16);
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Inc8 => {
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_add_u8(res, src, dst);
                self.cpu.regs.flags.set_sign_u8(res);
                self.cpu.regs.flags.set_zero_u8(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);

                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res as u8);
            }
            Op::Inc16 => {
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_add_u16(res, src, dst);
                self.cpu.regs.flags.set_sign_u16(res);
                self.cpu.regs.flags.set_zero_u16(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);

                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Inc32 => {
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_add_u32(res, src, dst);
                self.cpu.regs.flags.set_sign_u32(res);
                self.cpu.regs.flags.set_zero_u32(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);

                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u32);
            }
            Op::Insb => {
                // Input byte from I/O port specified in DX into memory location specified in ES:DI.
                // The ES segment cannot be overridden with a segment override prefix.
                let dx = self.cpu.get_r16(R::DX);
                let data = self.in_u8(dx);
                self.mmu.write_u8(self.cpu.get_r16(R::ES), self.cpu.get_r16(R::DI), data);
                let di = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::DI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::DI)) - Wrapping(1)).0
                };
                self.cpu.set_r16(R::DI, di);
            }
            Op::Int => {
                let int = self.cpu.read_parameter_imm(&op.params.dst);
                self.execute_interrupt(int as u8);
            }
            Op::Ja => {
                if !self.cpu.regs.flags.carry & !self.cpu.regs.flags.zero {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jc => {
                if self.cpu.regs.flags.carry {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jcxz => {
                if self.cpu.get_r16(R::CX) == 0 {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jg => {
                if !self.cpu.regs.flags.zero & self.cpu.regs.flags.sign == self.cpu.regs.flags.overflow {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jl => {
                if self.cpu.regs.flags.sign != self.cpu.regs.flags.overflow {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::JmpFar => {
                match op.params.dst {
                    Parameter::Ptr16Imm(seg, imm) => {
                        self.cpu.set_r16(R::CS, seg);
                        self.cpu.regs.ip = imm;
                    }
                    Parameter::Ptr16Amode(seg, ref amode) => {
                        let seg = self.cpu.segment(seg);
                        self.cpu.set_r16(R::CS, seg);
                        self.cpu.regs.ip = self.cpu.amode(amode) as u16;
                    }
                    Parameter::Ptr16AmodeS8(seg, ref amode, imm) => {
                        let seg = self.cpu.segment(seg);
                        self.cpu.set_r16(R::CS, seg);
                        self.cpu.regs.ip = (self.cpu.amode(amode) as isize + imm as isize) as u16;
                    }
                    _ => panic!("jmp far with unexpected type {:?}", op.params.dst),
                }
            }
            Op::JmpNear | Op::JmpShort => {
                self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
            }
            Op::Jna => {
                if self.cpu.regs.flags.carry | self.cpu.regs.flags.zero {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jnc => {
                if !self.cpu.regs.flags.carry {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jng => {
                if self.cpu.regs.flags.zero | self.cpu.regs.flags.sign != self.cpu.regs.flags.overflow {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jnl => {
                if self.cpu.regs.flags.sign == self.cpu.regs.flags.overflow {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jno => {
                if !self.cpu.regs.flags.overflow {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jns => {
                if !self.cpu.regs.flags.sign {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jnz => {
                if !self.cpu.regs.flags.zero {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jo => {
                if self.cpu.regs.flags.overflow {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jpe => {
                if self.cpu.regs.flags.parity {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jpo => {
                 if !self.cpu.regs.flags.parity {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Js => {
                if self.cpu.regs.flags.sign {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Jz => {
                if self.cpu.regs.flags.zero {
                    self.cpu.regs.ip = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                }
            }
            Op::Lahf => {
                // Load: AH ← EFLAGS(SF:ZF:0:AF:0:PF:1:CF).
                let mut val = 0 as u8;
                if self.cpu.regs.flags.carry {
                    val |= 1;
                }
                val |= 1 << 1;
                if self.cpu.regs.flags.parity {
                    val |= 1 << 2;
                }
                if self.cpu.regs.flags.adjust {
                    val |= 1 << 4;
                }
                if self.cpu.regs.flags.zero {
                    val |= 1 << 6;
                }
                if self.cpu.regs.flags.sign {
                    val |= 1 << 7;
                }
                self.cpu.set_r8(R::AH, val);
            }
            Op::Lds => {
                // Load DS:r16 with far pointer from memory.
                let (segment, offset) = self.cpu.read_segment_selector(&self.mmu, &op.params.src);
                self.cpu.set_r16(R::DS, segment);
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, offset);
            }
            Op::Lea16 => {
                let src = self.cpu.read_parameter_address(&op.params.src) as u16;
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, src);
            }
            Op::Leave => {
                // High Level Procedure Exit
                // Set SP to BP, then pop BP.
                // XXX test this
                let bp = self.cpu.get_r16(R::BP);
                self.cpu.set_r16(R::SP, bp);
                let bp = self.cpu.pop16(&mut self.mmu);
                self.cpu.set_r16(R::BP, bp);
            }
            Op::Les => {
                // Load ES:r16 with far pointer from memory.
                let (segment, offset) = self.cpu.read_segment_selector(&self.mmu, &op.params.src);
                self.cpu.set_r16(R::ES, segment);
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, offset);
            }
            Op::Lodsb => {
                // no arguments
                // Load byte at address DS:(E)SI into AL.
                // The DS segment may be over-ridden with a segment override prefix.
                let val = self.mmu.read_u8(self.cpu.segment(op.segment_prefix), self.cpu.get_r16(R::SI));

                self.cpu.set_r8(R::AL, val);
                let si = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::SI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::SI)) - Wrapping(1)).0
                };
                self.cpu.set_r16(R::SI, si);
            }
            Op::Lodsw => {
                // no arguments
                // Load word at address DS:(E)SI into AX.
                // The DS segment may be over-ridden with a segment override prefix.
                let val = self.mmu.read_u16(self.cpu.segment(op.segment_prefix), self.cpu.get_r16(R::SI));

                self.cpu.set_r16(R::AX, val);
                let si = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::SI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::SI)) - Wrapping(2)).0
                };
                self.cpu.set_r16(R::SI, si);
            }
            Op::Lodsd => {
                // no arguments
                // Load dword at address DS:(E)SI into EAX.
                // The DS segment may be over-ridden with a segment override prefix.
                let val = self.mmu.read_u32(self.cpu.segment(op.segment_prefix), self.cpu.get_r16(R::SI));

                self.cpu.set_r32(R::EAX, val);
                let si = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::SI)) + Wrapping(4)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::SI)) - Wrapping(4)).0
                };
                self.cpu.set_r16(R::SI, si);
            }
            Op::Loop => {
                // Decrement count; jump short if count ≠ 0.
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                let cx = (Wrapping(self.cpu.get_r16(R::CX)) - Wrapping(1)).0;
                self.cpu.set_r16(R::CX, cx);
                if cx != 0 {
                    self.cpu.regs.ip = dst;
                }
            }
            Op::Loope => {
                // Decrement count; jump short if count ≠ 0 and ZF = 1.
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                let cx = (Wrapping(self.cpu.get_r16(R::CX)) - Wrapping(1)).0;
                self.cpu.set_r16(R::CX, cx);
                if cx != 0 && self.cpu.regs.flags.zero {
                    self.cpu.regs.ip = dst;
                }
            }
            Op::Loopne => {
                // Decrement count; jump short if count ≠ 0 and ZF = 0.
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                let cx = (Wrapping(self.cpu.get_r16(R::CX)) - Wrapping(1)).0;
                self.cpu.set_r16(R::CX, cx);
                if cx != 0 && !self.cpu.regs.flags.zero {
                    self.cpu.regs.ip = dst;
                }
            } 
            Op::Mov8 => {
                // two arguments (dst=reg)
                let data = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u8;
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, data);
            }
            Op::Mov16 => {
                // two arguments (dst=reg)
                let data = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u16;
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Mov32 => {
                // two arguments (dst=reg)
                let data = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u32;
                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Movsb => {
                // move byte from address DS:(E)SI to ES:(E)DI.
                // The DS segment may be overridden with a segment override prefix, but the ES segment cannot be overridden.
                let val = self.mmu.read_u8(self.cpu.segment(op.segment_prefix), self.cpu.get_r16(R::SI));
                let si = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::SI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::SI)) - Wrapping(1)).0
                };
                self.cpu.set_r16(R::SI, si);
                let es = self.cpu.get_r16(R::ES);
                let di = self.cpu.get_r16(R::DI);
                self.mmu.write_u8(es, di, val);
                let di = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::DI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::DI)) - Wrapping(1)).0
                };
                self.cpu.set_r16(R::DI, di);
            }
            Op::Movsw => {
                // move word from address DS:(E)SI to ES:(E)DI.
                // The DS segment may be overridden with a segment override prefix, but the ES segment cannot be overridden.
                let val = self.mmu.read_u16(self.cpu.segment(op.segment_prefix), self.cpu.get_r16(R::SI));
                let si = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::SI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::SI)) - Wrapping(2)).0
                };
                self.cpu.set_r16(R::SI, si);
                let es = self.cpu.get_r16(R::ES);
                let di = self.cpu.get_r16(R::DI);
                self.mmu.write_u16(es, di, val);
                let di = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::DI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::DI)) - Wrapping(2)).0
                };
                self.cpu.set_r16(R::DI, di);
            }
            Op::Movsd => {
                // move dword from address DS:(E)SI to ES:(E)DI
                // The DS segment may be overridden with a segment override prefix, but the ES segment cannot be overridden.
                let val = self.mmu.read_u32(self.cpu.segment(op.segment_prefix), self.cpu.get_r16(R::SI));
                let si = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::SI)) + Wrapping(4)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::SI)) - Wrapping(4)).0
                };
                self.cpu.set_r16(R::SI, si);
                let es = self.cpu.get_r16(R::ES);
                let di = self.cpu.get_r16(R::DI);
                self.mmu.write_u32(es, di, val);
                let di = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::DI)) + Wrapping(4)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::DI)) - Wrapping(4)).0
                };
                self.cpu.set_r16(R::DI, di);
            }
            Op::Movsx16 => {
                // 80386+
                // moves a signed value into a register and sign-extends it with 1.
                // two arguments (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u8;

                let mut data = u16::from(src);
                // XXX should not work identical as Movzx16
                if src & 0x80 != 0 {
                    data += 0xFF00;
                }
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Movsx32 => {
                // 80386+
                // moves a signed value into a register and sign-extends it with 1.
                // two arguments (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u8;

                let mut data = u32::from(src);
                // XXX should not work identical as Movzx16
                if src & 0x80 != 0 {
                    data += 0xFFFF_FF00;
                }
                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Movzx16 => {
                // 80386+
                // moves an unsigned value into a register and zero-extends it with zero.
                // two arguments (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u8;
                let mut data = u16::from(src);
                if src & 0x80 != 0 {
                    data += 0xFF00;
                }
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Movzx32 => {
                // 80386+
                // moves an unsigned value into a register and zero-extends it with zero.
                // two arguments (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u8;
                let mut data = u32::from(src);
                if src & 0x80 != 0 {
                    data += 0xFFFF_FF00;
                }
                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Mul8 => {
                // Unsigned multiply (AX ← AL ∗ r/m8).
                let al = self.cpu.get_r8(R::AL) as usize;
                let arg1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let ax = (Wrapping(al) * Wrapping(arg1)).0 as u16;
                self.cpu.set_r16(R::AX, ax);
                // The OF and CF flags are set to 0 if the upper half of the
                // result is 0; otherwise, they are set to 1.
                // The SF, ZF, AF, and PF flags are undefined.
                if ax & 0xFF00 != 0 {
                    self.cpu.regs.flags.carry = true;
                    self.cpu.regs.flags.overflow = true;
                } else {
                    self.cpu.regs.flags.carry = false;
                    self.cpu.regs.flags.overflow = false;
                }
            }
            Op::Mul16 => {
                // Unsigned multiply (DX:AX ← AX ∗ r/m16).
                let src = self.cpu.get_r16(R::AX) as usize;
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = (Wrapping(dst) * Wrapping(src)).0;

                self.cpu.set_r16(R::AX, res as u16);
                let dx = (res >> 16) as u16;
                self.cpu.set_r16(R::DX, dx);

                self.cpu.regs.flags.carry = dx != 0;
                self.cpu.regs.flags.overflow = dx != 0;
            }
            Op::Mul32 => {
                // Unsigned multiply (EDX:EAX ← EAX ∗ r/m32)
                let src = self.cpu.get_r32(R::EAX) as usize;
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = (Wrapping(dst) * Wrapping(src)).0;

                self.cpu.set_r32(R::EAX, res as u32);
                let edx = (res >> 32) as u32;
                self.cpu.set_r32(R::EDX, edx);

                self.cpu.regs.flags.carry = edx != 0;
                self.cpu.regs.flags.overflow = edx != 0;
            }
            Op::Neg8 => {
                // Two's Complement Negation
                // one argument
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let src = 0;
                let res = (Wrapping(src) - Wrapping(dst)).0;
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res as u8);

                self.cpu.regs.flags.carry = dst != 0;
                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.cpu.regs.flags.overflow = res == 0x80;
                self.cpu.regs.flags.set_sign_u8(res);
                self.cpu.regs.flags.set_zero_u8(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);
            }
            Op::Neg16 => {
                // one argument
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let src = 0;
                let res = (Wrapping(src) - Wrapping(dst)).0;
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);

                self.cpu.regs.flags.carry = dst != 0;
                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.cpu.regs.flags.overflow = res == 0x8000;
                self.cpu.regs.flags.set_sign_u16(res);
                self.cpu.regs.flags.set_zero_u16(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);
            }
            Op::Neg32 => {
                // one argument
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let src = 0;
                let res = (Wrapping(src) - Wrapping(dst)).0;
                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u32);

                self.cpu.regs.flags.carry = dst != 0;
                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.cpu.regs.flags.overflow = res == 0x8000_0000;
                self.cpu.regs.flags.set_sign_u32(res);
                self.cpu.regs.flags.set_zero_u32(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);
            }
            Op::Nop => {}
            Op::Not8 => {
                // one arguments (dst)
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = !dst;
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, (res & 0xFF) as u8);
                // Flags Affected: None
            }
            Op::Not16 => {
                // one arguments (dst)
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = !dst;
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, (res & 0xFFFF) as u16);
                // Flags Affected: None
            }
            Op::Or8 => {
                // two arguments (dst=AL)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = dst | src;
                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.cpu.regs.flags.overflow = false;
                self.cpu.regs.flags.carry = false;
                self.cpu.regs.flags.set_sign_u8(res);
                self.cpu.regs.flags.set_zero_u8(res);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, (res & 0xFF) as u8);
            }
            Op::Or16 => {
                // two arguments (dst=AX)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = dst | src;
                // The OF and CF flags are cleared; the SF, ZF, and PF flags
                // are set according to the result.
                self.cpu.regs.flags.overflow = false;
                self.cpu.regs.flags.carry = false;
                self.cpu.regs.flags.set_sign_u16(res);
                self.cpu.regs.flags.set_zero_u16(res);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, (res & 0xFFFF) as u16);
            }
            Op::Out8 => {
                // two arguments
                let addr = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                let val = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u8;
                self.out_u8(addr, val);
            }
            Op::Out16 => {
                // two arguments
                let addr = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                let val = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u16;
                self.out_u16(addr, val);
            }
            Op::Outsb => {
                // Output byte from memory location specified in DS:(E)SI or RSI to I/O port specified in DX.
                // no arguments
                let val = self.mmu.read_u8(self.cpu.segment(op.segment_prefix), self.cpu.get_r16(R::SI));
                let port = self.cpu.get_r16(R::DX);
                self.out_u8(port, val);
                let si = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::SI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::SI)) - Wrapping(1)).0
                };
                self.cpu.set_r16(R::SI, si);
            }
            Op::Outsw => {
                // Output word from memory location specified in DS:(E)SI or RSI to I/O port specified in DX**.
                // no arguments
                let val = self.mmu.read_u16(self.cpu.segment(op.segment_prefix), self.cpu.get_r16(R::SI));
                let port = self.cpu.get_r16(R::DX);
                self.out_u16(port, val);
                let si = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::SI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::SI)) - Wrapping(2)).0
                };
                self.cpu.set_r16(R::SI, si);
            }
            Op::Pop16 => {
                // one arguments (dst)
                let data = self.cpu.pop16(&mut self.mmu);
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Pop32 => {
                // one arguments (dst)
                let data = self.cpu.pop32(&mut self.mmu);
                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, data);
            }
            Op::Popa16 => {
                let di = self.cpu.pop16(&mut self.mmu);
                self.cpu.set_r16(R::DI, di);
                let si = self.cpu.pop16(&mut self.mmu);
                self.cpu.set_r16(R::SI, si);
                let bp = self.cpu.pop16(&mut self.mmu);
                self.cpu.set_r16(R::BP, bp);
                let sp = self.cpu.get_r16(R::SP) + 2; // skip next word of stack
                self.cpu.set_r16(R::SP, sp);
                let bx = self.cpu.pop16(&mut self.mmu);
                self.cpu.set_r16(R::BX, bx);
                let dx = self.cpu.pop16(&mut self.mmu);
                self.cpu.set_r16(R::DX, dx);
                let cx = self.cpu.pop16(&mut self.mmu);
                self.cpu.set_r16(R::CX, cx);
                let ax = self.cpu.pop16(&mut self.mmu);
                self.cpu.set_r16(R::AX, ax);
            }
            Op::Popad32 => {
                let edi = self.cpu.pop32(&mut self.mmu);
                self.cpu.set_r32(R::EDI, edi);
                let esi = self.cpu.pop32(&mut self.mmu);
                self.cpu.set_r32(R::ESI, esi);
                let ebp = self.cpu.pop32(&mut self.mmu);
                self.cpu.set_r32(R::EBP, ebp);
                let esp = self.cpu.get_r32(R::ESP) + 4; // skip next dword of stack
                self.cpu.set_r32(R::ESP, esp);
                let ebx = self.cpu.pop32(&mut self.mmu);
                self.cpu.set_r32(R::EBX, ebx);
                let edx = self.cpu.pop32(&mut self.mmu);
                self.cpu.set_r32(R::EDX, edx);
                let ecx = self.cpu.pop32(&mut self.mmu);
                self.cpu.set_r32(R::ECX, ecx);
                let eax = self.cpu.pop32(&mut self.mmu);
                self.cpu.set_r32(R::EAX, eax);
            }
            Op::Popf => {
                let data = self.cpu.pop16(&mut self.mmu);
                self.cpu.regs.flags.set_u16(data);
            }
            Op::Push16 => {
                // single parameter (dst)
                let data = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                self.cpu.push16(&mut self.mmu, data);
            }
            Op::Push32 => {
                // single parameter (dst)
                let data = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u32;
                self.cpu.push32(&mut self.mmu, data);
            }
            Op::Pusha16 => {
                let ax = self.cpu.get_r16(R::AX);
                let cx = self.cpu.get_r16(R::CX);
                let dx = self.cpu.get_r16(R::DX);
                let bx = self.cpu.get_r16(R::BX);
                let sp = self.cpu.get_r16(R::SP);
                let bp = self.cpu.get_r16(R::BP);
                let si = self.cpu.get_r16(R::SI);
                let di = self.cpu.get_r16(R::DI);
                self.cpu.push16(&mut self.mmu, ax);
                self.cpu.push16(&mut self.mmu, cx);
                self.cpu.push16(&mut self.mmu, dx);
                self.cpu.push16(&mut self.mmu, bx);
                self.cpu.push16(&mut self.mmu, sp);
                self.cpu.push16(&mut self.mmu, bp);
                self.cpu.push16(&mut self.mmu, si);
                self.cpu.push16(&mut self.mmu, di);
            }
            Op::Pushad32 => {
                let eax = self.cpu.get_r32(R::EAX);
                let ecx = self.cpu.get_r32(R::ECX);
                let edx = self.cpu.get_r32(R::EDX);
                let ebx = self.cpu.get_r32(R::EBX);
                let esp = self.cpu.get_r32(R::ESP);
                let ebp = self.cpu.get_r32(R::EBP);
                let esi = self.cpu.get_r32(R::ESI);
                let edi = self.cpu.get_r32(R::EDI);
                self.cpu.push32(&mut self.mmu, eax);
                self.cpu.push32(&mut self.mmu, ecx);
                self.cpu.push32(&mut self.mmu, edx);
                self.cpu.push32(&mut self.mmu, ebx);
                self.cpu.push32(&mut self.mmu, esp);
                self.cpu.push32(&mut self.mmu, ebp);
                self.cpu.push32(&mut self.mmu, esi);
                self.cpu.push32(&mut self.mmu, edi);
            }
            Op::Pushf => {
                let data = self.cpu.regs.flags.u16();
                self.cpu.push16(&mut self.mmu, data);
            }
            Op::Rcl8 => {
                // Rotate 9 bits (CF, r/m8) left imm8 times.
                // two arguments
                let count = (self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0x1F) % 9;
                if count > 0 {
                    let cf = self.cpu.regs.flags.carry_val() as u16;
                    let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                    let res = if count == 1 {
                        ((op1 << 1) | cf)
                    } else {
                        ((op1 << count) | (cf << (count - 1)) | (op1 >> (9 - count)))
                    } as u8;
                    self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res);
                    let cf = (op1 >> (8 - count)) & 1;
                    self.cpu.regs.flags.carry = cf != 0;
                    // For left rotates, the OF flag is set to the exclusive OR of the CF bit
                    // (after the rotate) and the most-significant bit of the result.
                    self.cpu.regs.flags.overflow = cf ^ (u16::from(res) >> 7) != 0;
                }
            }
            Op::Rcl16 => {
                // Rotate 9 bits (CF, r/m8) left imm8 times.
                // two arguments
                let count = (self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0x1F) % 17;
                if count > 0 {
                    let cf = self.cpu.regs.flags.carry_val() as u16;
                    let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                    let res = if count == 1 {
                        (op1 << 1) | cf
                    } else if count == 16 {
                        (cf << 15) | (op1 >> 1)
                    } else {
                        (op1 << count) | (cf << (count - 1)) | (op1 >> (17 - count))
                    };
                    self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
                    self.cpu.regs.flags.carry = (op1 >> (16 - count)) & 1 != 0;
                    self.cpu.regs.flags.overflow = self.cpu.regs.flags.carry_val() as u16 ^ (op1 >> 15) != 0;
                }
            }
            Op::Rcr8 => {
                // two arguments
                // rotate 9 bits right `op1` times
                let mut count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u16/* & 0x1F*/;
                if count % 9 != 0 {
                    count %= 9;
                    let cf = self.cpu.regs.flags.carry_val() as u16;
                    let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                    let res = (op1 >> count | (cf << (8 - count)) | (op1 << (9 - count))) as u8;
                    self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res);
                    self.cpu.regs.flags.carry = (op1 >> (count - 1)) & 1 != 0;
                    // The OF flag is set to the exclusive OR of the two most-significant bits of the result.
                    self.cpu.regs.flags.overflow = (res ^ (res << 1)) & 0x80 != 0; // dosbox
                    //self.cpu.regs.flags.overflow = (((res << 1) ^ res) >> 7) & 0x1 != 0; // bochs. of = result6 ^ result7
                }
            }
            Op::Rcr16 => {
                // two arguments
                // rotate 9 bits right `op1` times
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let count = (self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u32 & 0x1F) % 17;
                if count > 0 {
                    let cf = self.cpu.regs.flags.carry_val();
                    let res = (op1 >> count) | (cf << (16 - count)) | (op1 << (17 - count));
                    self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
                    self.cpu.regs.flags.carry = (op1 >> (count - 1)) & 1 != 0;
                    let bit15 = (res >> 15) & 1;
                    let bit14 = (res >> 14) & 1;
                    self.cpu.regs.flags.overflow = bit15 ^ bit14 != 0;
                }
            }
            Op::Rcr32 => {
                // two arguments
                // rotate 9 bits right `op1` times
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let count = (self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u32 & 0x1F) % 17;    // XXX
                if count > 0 {
                    let cf = self.cpu.regs.flags.carry_val();
                    let res = (op1 >> count) | (cf << (32 - count)) | (op1 << (33 - count));
                    self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u32);
                    self.cpu.regs.flags.carry = (op1 >> (count - 1)) & 1 != 0;
                    let bit15 = (res >> 15) & 1; // XXX
                    let bit14 = (res >> 14) & 1;
                    self.cpu.regs.flags.overflow = bit15 ^ bit14 != 0;
                }
            }
            Op::Iret => {
                self.cpu.regs.ip = self.cpu.pop16(&mut self.mmu);
                let cs = self.cpu.pop16(&mut self.mmu);
                self.cpu.set_r16(R::CS, cs);
                let flags = self.cpu.pop16(&mut self.mmu);
                self.cpu.regs.flags.set_u16(flags);
                self.mmu.flags_address = MemoryAddress::Unset;
            }
            Op::Retf => {
                if op.params.count() == 1 {
                    // 1 argument: pop imm16 bytes from stack
                    let imm16 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                    let sp = self.cpu.get_r16(R::SP) + imm16;
                    self.cpu.set_r16(R::SP, sp);
                }
                self.cpu.regs.ip = self.cpu.pop16(&mut self.mmu);
                let cs = self.cpu.pop16(&mut self.mmu);
                self.cpu.set_r16(R::CS, cs);
            }
            Op::Retn => {
                let val = self.cpu.pop16(&mut self.mmu);
                if DEBUG_MARK_STACK && val == STACK_MARKER {
                    println!("[{}] WARNING: stack marker was popped after {} instr. execution ended. (can be valid where small app just return to DOS with a 'ret', but can also indicate memory corruption)",
                        self.cpu.get_memory_address(), self.cpu.instruction_count);
                    self.cpu.fatal_error = true;
                }
                // println!("Retn, ip from {:04X} to {:04X}", self.cpu.regs.ip, val);
                self.cpu.regs.ip = val;
                if op.params.count() == 1 {
                    // 1 argument: pop imm16 bytes from stack
                    let imm16 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                    let sp = self.cpu.get_r16(R::SP) + imm16;
                    self.cpu.set_r16(R::SP, sp);
                }
            }
            Op::Rol8 => {
                // Rotate 8 bits of 'dst' left for 'src' times.
                // two arguments: op1, count
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u8;
                let mut count = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                if count & 0b0_0111 == 0 {
                    if count & 0b1_1000 != 0 {
                        let bit0 = op1 & 1;
                        let bit7 = op1 >> 7;
                        self.cpu.regs.flags.overflow = bit0 ^ bit7 != 0;
                        self.cpu.regs.flags.carry = bit0 != 0;
                    }
                    // no-op if count is 0
                    return;
                }
                count &= 0x7;
                let res = (op1 << count) | (op1 >> (8 - count));
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res);
                let bit0 = res & 1;
                let bit7 = res >> 7;
                self.cpu.regs.flags.overflow = bit0 ^ bit7 != 0;
                self.cpu.regs.flags.carry = bit0 != 0;
            }
            Op::Rol16 => {
                // Rotate 16 bits of 'dst' left for 'src' times.
                // two arguments
                let mut res = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0x1F;
                res = res.rotate_left(count as u32);
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res);
                let bit0 = res & 1;
                let bit15 = (res >> 15) & 1;
                if count == 1 {
                    self.cpu.regs.flags.overflow = bit0 ^ bit15 != 0;
                }
                self.cpu.regs.flags.carry = bit0 != 0;
            }
            Op::Ror8 => {
                // Rotate 8 bits of 'dst' right for 'src' times.
                // two arguments
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u8;
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0x1F;

                if count & 0b0_0111 == 0 {
                    if count & 0b1_1000 != 0 {
                        let bit6 = (op1 >> 6) & 1;
                        let bit7 = op1 >> 7;
                        self.cpu.regs.flags.overflow = bit6 ^ bit7 != 0;
                        self.cpu.regs.flags.carry = bit7 != 0;
                    }
                    return;
                }

                let res = op1.rotate_right(count as u32);
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res);
                let bit6 = (res >> 6) & 1;
                let bit7 = res >> 7;
                self.cpu.regs.flags.overflow = bit6 ^ bit7 != 0;
                self.cpu.regs.flags.carry = bit7 != 0;
            }
            Op::Ror16 => {
                // Rotate 16 bits of 'dst' right for 'src' times.
                // two arguments
                let mut res = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0x1F;
                res = res.rotate_right(count as u32);
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res);
                let bit14 = (res >> 14) & 1;
                let bit15 = (res >> 15) & 1;
                if count == 1 {
                    self.cpu.regs.flags.overflow = bit14 ^ bit15 != 0;
                }
                self.cpu.regs.flags.carry = bit15 != 0;
            }
            Op::Sahf => {
                // Loads the SF, ZF, AF, PF, and CF flags of the EFLAGS register with values
                // from the corresponding bits in the AH register (bits 7, 6, 4, 2, and 0, respectively).
                let ah = self.cpu.get_r8(R::AH);
                self.cpu.regs.flags.carry = ah & 0x1 != 0; // bit 0
                self.cpu.regs.flags.parity = ah & 0x4 != 0; // bit 2
                self.cpu.regs.flags.adjust = ah & 0x10 != 0; // bit 4
                self.cpu.regs.flags.zero = ah & 0x40 != 0; // bit 6
                self.cpu.regs.flags.sign = ah & 0x80 != 0; // bit 7
            }
            Op::Salc => {
                let al = if self.cpu.regs.flags.carry {
                    0xFF
                } else {
                    0
                };
                self.cpu.set_r8(R::AL, al);
            }
            Op::Sar8 => {
                // Signed divide r/m8 by 2, imm8 times.
                // two arguments
                let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u8;
                let mut count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0x1F;
                if count > 0 {
                    if count > 8 {
                        count = 8;
                    }

                    let res = if op1 & 0x80 != 0 {
                        ((op1 as usize) >> count) | (0xFF << (8 - count))
                    } else {
                        ((op1 as usize) >> count)
                    };
                    
                    self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res as u8);
                    self.cpu.regs.flags.carry = (op1 as isize >> (count - 1)) & 0x1 != 0;
                    self.cpu.regs.flags.overflow = false;
                    self.cpu.regs.flags.set_sign_u8(res as usize);
                    self.cpu.regs.flags.set_zero_u8(res as usize);
                    self.cpu.regs.flags.set_parity(res as usize);
                }
            }
            Op::Sar16 => {
                // Signed divide r/m8 by 2, imm8 times.
                // two arguments
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0xF;
                if count > 0 {
                    let res = if dst & 0x8000 != 0 {
                        let x = 0xFFFF as usize;
                        dst.rotate_right(count as u32) | x.rotate_left(16 - count as u32)
                    } else {
                        dst.rotate_right(count as u32)
                    };
                    self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
                    self.cpu.regs.flags.carry = (dst as u16 >> (count - 1)) & 0x1 != 0;
                    if count == 1 {
                        self.cpu.regs.flags.overflow = false;
                    }
                    self.cpu.regs.flags.set_sign_u16(res);
                    self.cpu.regs.flags.set_zero_u16(res);
                    self.cpu.regs.flags.set_parity(res);
                }
            }
            Op::Sar32 => {
                // Signed divide r/m8 by 2, imm8 times.
                // two arguments
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0xF; // XXX
                if count > 0 {
                    let res = if dst & 0x8000_0000 != 0 {
                        let x = 0xFFFF_FFFF as usize;
                        dst.rotate_right(count as u32) | x.rotate_left(32 - count as u32)
                    } else {
                        dst.rotate_right(count as u32)
                    };
                    self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u32);
                    self.cpu.regs.flags.carry = (dst as u32 >> (count - 1)) & 0x1 != 0; // XXX
                    if count == 1 {
                        self.cpu.regs.flags.overflow = false;
                    }
                    self.cpu.regs.flags.set_sign_u32(res);
                    self.cpu.regs.flags.set_zero_u32(res);
                    self.cpu.regs.flags.set_parity(res);
                }
            }
            Op::Sbb8 => {
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let cf = if self.cpu.regs.flags.carry { 1 } else { 0 };
                let res = (Wrapping(dst) - (Wrapping(src) + Wrapping(cf))).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_sub_u8(res, src, dst);
                self.cpu.regs.flags.set_sign_u8(res);
                self.cpu.regs.flags.set_zero_u8(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.regs.flags.set_carry_u8(res);

                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res as u8);
            }
            Op::Sbb16 => {
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let cf = if self.cpu.regs.flags.carry { 1 } else { 0 };
                let res = (Wrapping(dst) - (Wrapping(src) + Wrapping(cf))).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_sub_u16(res, src, dst);
                self.cpu.regs.flags.set_sign_u16(res);
                self.cpu.regs.flags.set_zero_u16(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.regs.flags.set_carry_u16(res);

                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Scasb => {
                // Compare AL with byte at ES:(E)DI then set status flags.
                // ES cannot be overridden with a segment override prefix.
                let src = self.cpu.get_r8(R::AL);
                let dst = self.mmu.read_u8(self.cpu.get_r16(R::ES), self.cpu.get_r16(R::DI));
                self.cpu.cmp8(dst as usize, src as usize);
                let di = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::DI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::DI)) - Wrapping(1)).0
                };
                self.cpu.set_r16(R::DI, di);
            }
            Op::Scasw => {
                // Compare AX with word at ES:(E)DI or RDI then set status flags.
                // ES cannot be overridden with a segment override prefix.
                let src = self.cpu.get_r16(R::AX);
                let dst = self.mmu.read_u16(self.cpu.get_r16(R::ES), self.cpu.get_r16(R::DI));
                self.cpu.cmp16(dst as usize, src as usize);
                let di = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::DI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::DI)) - Wrapping(2)).0
                };
                self.cpu.set_r16(R::DI, di);
            }
            Op::Setc => {
                let val = if self.cpu.regs.flags.carry {
                    1
                } else {
                    0
                };
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, val);
            }
            Op::Setnz => {
                let val = if !self.cpu.regs.flags.zero {
                    1
                } else {
                    0
                };
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, val);
            }
            Op::Shl8 => {
                // Multiply `dst` by 2, `src` times.
                // two arguments    (alias: sal)
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0b1_1111;
                // XXX differs from dosbox & winxp
                //if count > 0 {
                    let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u16;
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
                    self.cpu.regs.flags.carry = cf != 0;
                    //self.cpu.regs.flags.overflow = cf ^ (res >> 7) != 0; // bochs
                    //self.cpu.regs.flags.overflow = (op1 ^ res) & 0x80 != 0; // dosbox buggy
                    self.cpu.regs.flags.overflow = res >> 7 ^ cf as u16 != 0; // MSB of result XOR CF. WARNING: This only works because FLAGS_CF == 1
                    //self.cpu.regs.flags.overflow = ((op1 ^ res) >> (12 - 8)) & 0x800 != 0; // qemu
                    //self.cpu.regs.flags.adjust = count & 0x1F != 0; // XXX dosbox. AF not set in winxp
                    self.cpu.regs.flags.set_sign_u8(res as usize);
                    self.cpu.regs.flags.set_zero_u8(res as usize);
                    self.cpu.regs.flags.set_parity(res as usize);
                    self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res as u8);
                //}
            }
            Op::Shl16 => {
                // Multiply `dst` by 2, `src` times.
                // two arguments    (alias: sal)
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0x1F;
                if count > 0 {
                    let res = dst.wrapping_shl(count as u32);
                    self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
                    self.cpu.regs.flags.carry = (res & 0x8000) != 0;
                    if count == 1 {
                        self.cpu.regs.flags.overflow = self.cpu.regs.flags.carry_val() ^ ((res & 0x8000) >> 15) != 0;
                    }
                    self.cpu.regs.flags.set_sign_u16(res);
                    self.cpu.regs.flags.set_zero_u16(res);
                    self.cpu.regs.flags.set_parity(res);
                }
            }
            Op::Shl32 => {
                // Multiply `dst` by 2, `src` times.
                // two arguments    (alias: sal)
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0x1F; // XXX
                if count > 0 {
                    let res = dst.wrapping_shl(count as u32);
                    self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u32);
                    self.cpu.regs.flags.carry = (res & 0x8000_0000) != 0;
                    if count == 1 {
                        self.cpu.regs.flags.overflow = self.cpu.regs.flags.carry_val() ^ ((res & 0x8000) >> 15) != 0; // XXX
                    }
                    self.cpu.regs.flags.set_sign_u32(res);
                    self.cpu.regs.flags.set_zero_u32(res);
                    self.cpu.regs.flags.set_parity(res);
                }
            }
            Op::Shld => {
                // 3 arguments
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src2) & 0x1F; // use 5 lsb
                if count > 0 {
                    let op1 = self.cpu.read_parameter_value(&self.mmu, &op.params.dst) as u32;
                    let op2 = self.cpu.read_parameter_value(&self.mmu, &op.params.src) as u32;

                    // count < 32, since only lower 5 bits used
                    let temp_32 = (op1 << 16) | op2; // double formed by op1:op2
                    let mut result_32 = temp_32 << count;

                    // hack to act like x86 SHLD when count > 16
                    if count > 16 {
                        // for Pentium processor, when count > 16, actually shifting op1:op2:op2 << count,
                        // it is the same as shifting op2:op2 by count-16
                        // For P6 and later (CPU_LEVEL >= 6), when count > 16, actually shifting op1:op2:op1 << count,
                        // which is the same as shifting op2:op1 by count-16
                        // The behavior is undefined so both ways are correct, we prefer P6 way of implementation
                        result_32 |= op1 << (count - 16);
                     }

                    let res16 = (result_32 >> 16) as u16;
                    self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res16);

                    let cf = (temp_32 >> (32 - count)) & 0x1;
                    self.cpu.regs.flags.carry = cf != 0;

                    let of = cf ^ (u32::from(res16 >> 15));
                    self.cpu.regs.flags.overflow = of != 0;

                    self.cpu.regs.flags.set_zero_u16(res16 as usize);
                    self.cpu.regs.flags.set_sign_u16(res16 as usize);
                    self.cpu.regs.flags.set_parity(res16 as usize);
                }
            }
            Op::Shr8 => {
                // Unsigned divide r/m8 by 2, `src` times.
                // two arguments
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0x1F;
                if count > 0 {
                    let res = dst.wrapping_shr(count as u32);
                    self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res as u8);
                    self.cpu.regs.flags.carry = (dst.wrapping_shr((count - 1) as u32) & 0x1) != 0;
                    self.cpu.regs.flags.overflow = dst & 0x80 != 0;
                    self.cpu.regs.flags.set_sign_u8(res);
                    self.cpu.regs.flags.set_zero_u8(res);
                    self.cpu.regs.flags.set_parity(res);
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
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0x1F;
                if count > 0 {
                    let res = dst.wrapping_shr(count as u32);
                    self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
                    self.cpu.regs.flags.carry = (dst.wrapping_shr((count - 1) as u32) & 0x1) != 0;
                    self.cpu.regs.flags.overflow = dst & 0x8000 != 0;
                    self.cpu.regs.flags.set_sign_u16(res);
                    self.cpu.regs.flags.set_zero_u16(res);
                    self.cpu.regs.flags.set_parity(res);
                }
            }
            Op::Shr32 => {
                // two arguments
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src) & 0x1F; // XXX
                if count > 0 {
                    let res = dst.wrapping_shr(count as u32);
                    self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u32);
                    self.cpu.regs.flags.carry = (dst.wrapping_shr((count - 1) as u32) & 0x1) != 0; // XXX
                    self.cpu.regs.flags.overflow = dst & 0x8000_0000 != 0;
                    self.cpu.regs.flags.set_sign_u32(res);
                    self.cpu.regs.flags.set_zero_u32(res);
                    self.cpu.regs.flags.set_parity(res);
                }
            }
            Op::Shrd => {
                // 3 arguments

                let count = self.cpu.read_parameter_value(&self.mmu, &op.params.src2) & 0x1F; // use 5 lsb
                if count == 0 {
                    return;
                }
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);

                // count < 32, since only lower 5 bits used
                let temp_32 = (src << 16) | dst;
                let mut result_32 = temp_32 >> count;

                // hack to act like x86 SHRD when count > 16
                if count > 16 {
                    // for Pentium processor, when count > 16, actually shifting op2:op2:op1 >> count,
                    // it is the same as shifting op2:op2 by count-16
                    // For P6 and later (CPU_LEVEL >= 6), when count > 16, actually shifting op1:op2:op1 >> count,
                    // which is the same as shifting op1:op2 by count-16
                    // The behavior is undefined so both ways are correct, we prefer P6 way of implementation
                    result_32 |= dst << (32 - count);
                }

                let result_16 = result_32 as u16;

                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, result_16);

                // SF, ZF, and PF flags are set according to the value of the result.
                self.cpu.regs.flags.set_sign_u16(result_16 as usize);
                self.cpu.regs.flags.set_zero_u16(result_16 as usize);
                self.cpu.regs.flags.set_parity(result_16 as usize);

                let mut cf = (dst >> (count - 1)) & 0x1;
                let of = (((result_16 << 1) ^ result_16) >> 15) & 0x1; // of = result14 ^ result15
                if count > 16 {
                    // undefined flags behavior matching real HW
                    cf = (src >> (count - 17)) & 0x1;
                }
                self.cpu.regs.flags.carry = cf != 0;
                self.cpu.regs.flags.overflow = of != 0;
            }
            Op::Sldt => {
                println!("XXX impl {:?}", op);
            }
            Op::Stc => {
                self.cpu.regs.flags.carry = true;
            }
            Op::Std => {
                self.cpu.regs.flags.direction = true;
            }
            Op::Sti => {
                self.cpu.regs.flags.interrupt = true;
            }
            Op::Stosb => {
                // no parameters
                // store AL at ES:(E)DI
                // The ES segment cannot be overridden with a segment override prefix.
                let al = self.cpu.get_r8(R::AL);
                let es = self.cpu.get_r16(R::ES);
                let di = self.cpu.get_r16(R::DI);
                self.mmu.write_u8(es, di, al);
                let di = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::DI)) + Wrapping(1)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::DI)) - Wrapping(1)).0
                };
                self.cpu.set_r16(R::DI, di);
            }
            Op::Stosw => {
                // no parameters
                // store AX at address ES:(E)DI
                // The ES segment cannot be overridden with a segment override prefix.
                let ax = self.cpu.get_r16(R::AX);
                let es = self.cpu.get_r16(R::ES);
                let di = self.cpu.get_r16(R::DI);
                self.mmu.write_u16(es, di, ax);
                let di = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::DI)) + Wrapping(2)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::DI)) - Wrapping(2)).0
                };
                self.cpu.set_r16(R::DI, di);
            }
            Op::Stosd => {
                // no parameters
                // store EAX at address ES:(E)DI
                // The ES segment cannot be overridden with a segment override prefix.
                let eax = self.cpu.get_r32(R::EAX);
                let es = self.cpu.get_r16(R::ES);
                let di = self.cpu.get_r16(R::DI);
                self.mmu.write_u32(es, di, eax);
                // XXX adjust DI or EDI ?
                let di = if !self.cpu.regs.flags.direction {
                    (Wrapping(self.cpu.get_r16(R::DI)) + Wrapping(4)).0
                } else {
                    (Wrapping(self.cpu.get_r16(R::DI)) - Wrapping(4)).0
                };
                self.cpu.set_r16(R::DI, di);
            }
            Op::Sub8 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_sub_u8(res, src, dst);
                self.cpu.regs.flags.set_sign_u8(res);
                self.cpu.regs.flags.set_zero_u8(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.regs.flags.set_carry_u8(res);

                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res as u8);
            }
            Op::Sub16 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_sub_u16(res, src, dst);
                self.cpu.regs.flags.set_sign_u16(res);
                self.cpu.regs.flags.set_zero_u16(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.regs.flags.set_carry_u16(res);

                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Sub32 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.cpu.regs.flags.set_overflow_sub_u32(res, src, dst);
                self.cpu.regs.flags.set_sign_u32(res);
                self.cpu.regs.flags.set_zero_u32(res);
                self.cpu.regs.flags.set_adjust(res, src, dst);
                self.cpu.regs.flags.set_parity(res);
                self.cpu.regs.flags.set_carry_u32(res);

                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u32);
            }
            Op::Test8 => {
                // two parameters
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = dst & src;
                // set SF, ZF, PF according to result.
                self.cpu.regs.flags.set_sign_u8(res);
                self.cpu.regs.flags.set_zero_u8(res);
                self.cpu.regs.flags.set_parity(res);
            }
            Op::Test16 => {
                // two parameters
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = dst & src;
                // set SF, ZF, PF according to result.
                self.cpu.regs.flags.set_sign_u16(res);
                self.cpu.regs.flags.set_zero_u16(res);
                self.cpu.regs.flags.set_parity(res);
            }
            Op::Xchg8 => {
                // two parameters (registers)
                let mut src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let mut dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                mem::swap(&mut src, &mut dst);
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, dst as u8);
                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.src, src as u8);
            }
            Op::Xchg16 => {
                // two parameters (registers)
                let mut src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let mut dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                mem::swap(&mut src, &mut dst);
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, dst as u16);
                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.src, src as u16);
            }
            Op::Xchg32 => {
                // two parameters (registers)
                let mut src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let mut dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                mem::swap(&mut src, &mut dst);
                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, dst as u32);
                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.src, src as u32);
            }
            Op::Xlatb => {
                // no parameters
                // Set AL to memory byte DS:[(E)BX + unsigned AL].
                // The DS segment may be overridden with a segment override prefix.
                let al = self.mmu.read_u8(self.cpu.segment(op.segment_prefix), self.cpu.get_r16(R::BX) + u16::from(self.cpu.get_r8(R::AL)));
                self.cpu.set_r8(R::AL, al);
            }
            Op::Xor8 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = dst ^ src;

                // The OF and CF flags are cleared; the SF, ZF,
                // and PF flags are set according to the result.
                self.cpu.regs.flags.overflow = false;
                self.cpu.regs.flags.carry = false;
                self.cpu.regs.flags.set_sign_u8(res);
                self.cpu.regs.flags.set_zero_u8(res);
                self.cpu.regs.flags.set_parity(res);

                self.cpu.write_parameter_u8(&mut self.mmu, &op.params.dst, res as u8);
            }
            Op::Xor16 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = dst ^ src;

                // The OF and CF flags are cleared; the SF, ZF,
                // and PF flags are set according to the result.
                self.cpu.regs.flags.overflow = false;
                self.cpu.regs.flags.carry = false;
                self.cpu.regs.flags.set_sign_u16(res);
                self.cpu.regs.flags.set_zero_u16(res);
                self.cpu.regs.flags.set_parity(res);

                self.cpu.write_parameter_u16(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u16);
            }
            Op::Xor32 => {
                // two parameters (dst=reg)
                let src = self.cpu.read_parameter_value(&self.mmu, &op.params.src);
                let dst = self.cpu.read_parameter_value(&self.mmu, &op.params.dst);
                let res = dst ^ src;

                // The OF and CF flags are cleared; the SF, ZF,
                // and PF flags are set according to the result.
                self.cpu.regs.flags.overflow = false;
                self.cpu.regs.flags.carry = false;
                self.cpu.regs.flags.set_sign_u32(res);
                self.cpu.regs.flags.set_zero_u32(res);
                self.cpu.regs.flags.set_parity(res);

                self.cpu.write_parameter_u32(&mut self.mmu, op.segment_prefix, &op.params.dst, res as u32);
            }
            _ => {
                let (seg, off) = self.cpu.get_address_pair();
                println!("execute error: unhandled '{}' at {:04X}:{:04X} (flat {:06X})",
                         op,
                         seg,
                         off,
                         self.cpu.get_address());
            }
        }

        match op.repeat {
            RepeatMode::Rep => {
                let cx = (Wrapping(self.cpu.get_r16(R::CX)) - Wrapping(1)).0;
                self.cpu.set_r16(R::CX, cx);
                if cx != 0 {
                    self.cpu.regs.ip = start_ip;
                }
            }
            RepeatMode::Repe => {
                let cx = (Wrapping(self.cpu.get_r16(R::CX)) - Wrapping(1)).0;
                self.cpu.set_r16(R::CX, cx);
                if cx != 0 && self.cpu.regs.flags.zero {
                    self.cpu.regs.ip = start_ip;
                }
            }
            RepeatMode::Repne => {
                let cx = (Wrapping(self.cpu.get_r16(R::CX)) - Wrapping(1)).0;
                self.cpu.set_r16(R::CX, cx);
                if cx != 0 && !self.cpu.regs.flags.zero {
                    self.cpu.regs.ip = start_ip;
                }
            }
            RepeatMode::None => {}
        }

        if op.lock {
            // TODO implement lock
            // println!("XXX FIXME: instruction has LOCK prefix: {}", op);
        }
    }
}
