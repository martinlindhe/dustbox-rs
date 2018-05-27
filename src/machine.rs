use gpu::GPU;
use cpu::{CPU, Op, InvalidOp, R, RegisterSnapshot, Segment, OperandSize};
use memory::MMU;
use hardware::Hardware;
use ndisasm::ndisasm_bytes;

pub struct Machine {
    pub hw: Hardware,
    pub cpu: CPU,
}

impl Machine {
    pub fn default() -> Self {
        Machine {
            cpu: CPU::default(),
            hw: Hardware::default(),
        }
    }

   // reset the CPU and memory
    pub fn hard_reset(&mut self) {
        self.cpu = CPU::default();
    }

    // load .com program into CS:0100 and set IP to program start
    pub fn load_com(&mut self, data: &[u8]) {
        // CS,DS,ES,SS = PSP segment
        let psp_segment = 0x085F; // is what dosbox used
        self.cpu.set_r16(&R::CS, psp_segment);
        self.cpu.set_r16(&R::DS, psp_segment);
        self.cpu.set_r16(&R::ES, psp_segment);
        self.cpu.set_r16(&R::SS, psp_segment);

        // offset of last word available in first 64k segment
        self.cpu.set_r16(&R::SP, 0xFFFE);
        self.cpu.set_r16(&R::BP, 0x091C); // is what dosbox used

        // This is what dosbox initializes the registers to
        // at program load
        self.cpu.set_r16(&R::CX, 0x00FF);
        self.cpu.set_r16(&R::DX, psp_segment);
        self.cpu.set_r16(&R::SI, 0x0100);
        self.cpu.set_r16(&R::DI, 0xFFFE);

        self.cpu.regs.ip = 0x0100;
        let min = self.cpu.get_address();
        self.cpu.rom_base = min;

        let cs = self.cpu.get_r16(&R::CS);
        self.hw.mmu.write(cs, self.cpu.regs.ip, data);
    }

    // returns a copy of register values at a given time
    pub fn register_snapshot(&self) -> RegisterSnapshot {
        self.cpu.regs.clone()
    }

    // executes enough instructions that can run for 1 video frame
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

    // executes n instructions of the cpu. only used in tests
    pub fn execute_instructions(&mut self, count: usize) {
        for _ in 0..count {
            self.execute_instruction()
        }
    }

    pub fn execute_instruction(&mut self) {
        let cs = self.cpu.get_r16(&R::CS);
        let ip = self.cpu.regs.ip;
        if cs == 0xF000 {
            // we are in interrupt vector code, execute high-level interrupt.
            // the default interrupt vector table has a IRET
            self.cpu.handle_interrupt(&mut self.hw, ip as u8);
        }

        let (op, length) = self.cpu.decoder.get_instruction(&mut self.hw.mmu, cs, ip);

        match op.command {
            Op::Unknown => {
                self.cpu.fatal_error = true;
                println!("executed unknown op, stopping. {} instructions executed",
                         self.cpu.instruction_count);
            }
            Op::Invalid(reason) => {
                self.cpu.fatal_error = true;
                match reason {
                    InvalidOp::Op => {
                        let mut ops_str = Vec::new();
                        let mut bytes = Vec::new();
                        for i in 0..16 {
                            let b = self.hw.mmu.read_u8(cs, ip + i);
                            bytes.push(b);
                            ops_str.push(format!("0x{:02X}", b));
                        }
                        println!("Error unhandled OP {} at {:04X}:{:04X}", ops_str.join(", "), cs, ip);
                        let ndisasm_of_input = ndisasm_bytes(&bytes).unwrap();
                        println!("ndisasm: {}", ndisasm_of_input);
                    }
                    InvalidOp::Reg(reg) => {
                        println!("Error invalid register {:02X} at {:04X}:{:04X}", reg, cs, ip);
                    }
                }
                println!("{} Instructions executed", self.cpu.instruction_count);
            }
            _ => self.cpu.execute(&mut self.hw, &op, length),
        }

        // XXX need instruction timing to do this properly
        if self.cpu.cycle_count % 100 == 0 {
            self.hw.gpu.progress_scanline();
        }

        if self.cpu.cycle_count % 100 == 0 {
            // FIXME: counter should decrement ~18.2 times/sec
            self.hw.pit.counter0.dec();
        }
    }
}
