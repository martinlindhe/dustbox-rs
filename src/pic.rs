// Programmable Interrupt Controller (8259A)
// https://wiki.osdev.org/8259_PIC

// The 8259 PIC controls the CPU's interrupt mechanism, by accepting several
// interrupt requests and feeding them to the processor in order.

use crate::cpu::CPU;
use crate::machine::Component;

#[cfg(test)]
#[path = "./pic_test.rs"]
mod pic_test;

#[derive(Clone, Debug)]
enum OperationMode {
    Clear,                              // 0 rotate in auto EOI mode (clear)
    NonspecificEOI,                     // 1 (WORD_A) nonspecific EOI
    NoOperation,                        // 2 (WORD_H) no operation
    SpecificEOI,                        // 3 (WORD_B) specific EOI
    Set,                                // 4 (WORD_F) rotate in auto EOI mode (set)
    RotateOnNonspecificEOICommand,      // 5 (WORD_C) rotate on nonspecific EOI command
    SetPriorityCommand,                 // 6 (WORD_E) set priority command
    RotateOnSpecificEOICommand,         // 7 (WORD_D) rotate on specific EOI command
}

#[derive(Clone)]
pub struct PIC {
    command: u8,
    data: u8,

    /// the base offset for I/O
    io_base: u16,

    operation: OperationMode,
}

impl Component for PIC {
    fn in_u8(&mut self, port: u16) -> Option<u8> {
        match port {
            _ if port < self.io_base => None,
            _ if port - self.io_base == 0x0000 => Some(self.get_register()),
            _ if port - self.io_base == 0x0001 => Some(self.get_ocw1()),
            _ => None
        }
    }

    fn out_u8(&mut self, port: u16, data: u8) -> bool {
        match port {
            _ if port < self.io_base => return false,
            _ if port - self.io_base == 0x0000 => self.set_command(data),
            _ if port - self.io_base == 0x0001 => self.set_data(data),
            _ => return false
        }
        true
    }
}

impl PIC {
    pub fn new(io_base: u16) -> Self {
        PIC {
            command: 0,
            data: 0,
            io_base,
            operation: OperationMode::NoOperation, // XXX default?
        }
    }

    /// io read of port 0021 (pic1) or 00A1 (pic2)
    fn get_ocw1(&self) -> u8 {
        // read: PIC master interrupt mask register OCW1
        0 // XXX
    }

    /// io read of port 0020 (pic1) or 00A0 (pic2)
    fn get_register(&self) -> u8 {
        /*
        0020  R-  PIC  interrupt request/in-service registers after OCW3
        request register:
            bit 7-0 = 0  no active request for the corresponding int. line
                = 1  active request for corresponding interrupt line
        in-service register:
            bit 7-0 = 0  corresponding line not currently being serviced
                = 1  corresponding int. line currently being serviced
        */
        0 // XXX
    }

    /// PIC - Command register, port 0x0020
    fn set_command(&mut self, val: u8) {
        self.command = val;
        println!("PIC COMMAND: {:02x} == {:08b}", val, val);
        // XXX 0x20 == 0b0010_0000 == EOI - End of interrrupt command code

        /*
        0020  -W  PIC initialization command word ICW1 (see #P0010)
        Bit(s)	Description	(Table P0010)
        7-5	0 (only used in 8080/8085 mode)
        4	ICW1 is being issued
        3	(LTIM)
            =0  edge triggered mode
            =1  level triggered mode
        2	interrupt vector size
            =0 successive interrupt vectors use 8 bytes (8080/8085)
            =1 successive interrupt vectors use 4 bytes (80x86)
        1	(SNGL)
            =0  cascade mode
            =1  single mode, no ICW3 needed
        0	ICW4 needed
        SeeAlso: #P0011,#P0012,#P0013
        */
        let kind = (val >> 3) & 0b11; // bits 4-3: reserved (00 - signals OCW2)
        match kind {
            0 => { // 0020  -W  PIC output control word OCW2
                // SeeAlso: #P0014,#P0016
                let operation = (val >> 5) & 0b111; // bits 7-5: operation
                self.operation = match operation {
                    0 => OperationMode::Clear,
                    1 => OperationMode::NonspecificEOI,
                    2 => OperationMode::NoOperation,
                    3 => OperationMode::SpecificEOI,
                    4 => OperationMode::Set,
                    5 => OperationMode::RotateOnNonspecificEOICommand,
                    6 => OperationMode::SetPriorityCommand,
                    7 => OperationMode::RotateOnSpecificEOICommand,
                    _ => unreachable!(),
                };

                let data = val & 0b11; // bits 0-2: interrupt request to which the command applies
                //     (only used by WORD_B, WORD_D, and WORD_E)
                println!("XXX: pic ocw2 operation {:?}, data {}", self.operation, data);
            }
            1 => { // 0020  -W  PIC output control word OCW3 (see #P0016)
                // Bit(s)	Description	(Table P0016)
                // 7	reserved (0)
                // 6-5	special mask
                //     0x  no operation
                //     10  reset special mask
                //     11  set special mask mode
                // 2	poll command
                // 1-0	function
                //     0x  no operation
                //     10  read interrupt request register on next read from PORT 0020h
                //     11  read interrupt in-service register on next read from PORT 0020h
                // Note:	the special mask mode permits all other interrupts (even those with
                //     lower priority) to be processed while an interrupt is already in
                //     service, but will not re-issue an interrupt for a particular IRQ
                //     while it remains in service
            }
            _ => panic!("unhandled kind {}", kind),
        }
    }

    /// Master PIC - Data register, port 0x0021
    fn set_data(&mut self, val: u8) {
        // XXX: one value if written immediately after value to 0020, another otherwise....
        self.data = val;

        // XXX impl, from https://wiki.osdev.org/8259_PIC#Disabling
        //If you are going to use the processor local APIC and the IOAPIC, you must first disable the PIC. This is done via:
        //mov al, 0xff
        //out 0xa1, al
        //out 0x21, al
    }

}
