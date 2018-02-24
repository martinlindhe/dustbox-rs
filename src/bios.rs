// https://wiki.osdev.org/BIOS
// dosbox-x: src/hardware/bios.cpp

use cpu::CPU;
use cpu::flags::Flags;
use memory::mmu::{MMU, MemoryAddress};

#[derive(Clone)]
pub struct BIOS {
    pub flags_address: MemoryAddress, // the FLAGS register offset on stack while in interrupt
}

const BIOS_SEGMENT: u16 = 0xF000;
const EQUIPMENT_WORD: u16 = 0x21;
const EQUIPMENT_WORD_ADDR: u16 = 0x410;

impl BIOS {
    pub fn new() -> Self {
        // XXX see ROMBIOS_Init in dosbox-x
        BIOS {
            flags_address: MemoryAddress::Unset,
        }
    }

    // manipulates the FLAGS register on stack while in a interrupt
    pub fn set_flag(&mut self, mmu: &mut MMU, flag_mask: u16, flag_value: bool) {
        if self.flags_address == MemoryAddress::Unset {
            panic!("bios: set_flag with 0 flags_address");
        }
        let mut flags = mmu.memory.borrow().read_u16(self.flags_address.value());
        if flag_value {
            flags = flags | flag_mask;
        } else {
            flags = flags & !flag_mask;
        }
        mmu.memory.borrow_mut().write_u16(self.flags_address.value(), flags);
    }

    pub fn init(&mut self, mut mmu: &mut MMU) {
        self.init_ivt(&mut mmu);
        self.write_configuration_data_table(&mut mmu);
    }

    fn init_ivt(&mut self, mmu: &mut MMU) {
        const IRET: u8 = 0xCF;
        for irq in 0..0xFF {
            self.write_ivt_entry(mmu, irq, BIOS_SEGMENT, irq as u16);
            mmu.write_u8(BIOS_SEGMENT, irq as u16, IRET);
        }
    }

    fn write_ivt_entry(&self, mmu: &mut MMU, number: u8, seg: u16, offset: u16) {
        let _seg = 0;
        let _offset = (number as u16) * 4;
        mmu.write_u16(_seg, _offset, offset);
        mmu.write_u16(_seg, _offset + 2, seg);
    }

    // initializes the Configuration Data Table
    fn write_configuration_data_table(&self, mmu: &mut MMU) {
        const ADDR: u16 = 0xE6F5;
        mmu.write_u16(BIOS_SEGMENT, ADDR + 0, 8);         // table size
        mmu.write_u8(BIOS_SEGMENT, ADDR + 2, 0xFC);       // model: AT
        mmu.write_u8(BIOS_SEGMENT, ADDR + 3, 0);          // submodel
        mmu.write_u8(BIOS_SEGMENT, ADDR + 4, 0);          // BIOS revision
        mmu.write_u8(BIOS_SEGMENT, ADDR + 5, 0b00000000); // feature byte 1
        mmu.write_u8(BIOS_SEGMENT, ADDR + 6, 0b00000000); // feature byte 2
        mmu.write_u8(BIOS_SEGMENT, ADDR + 7, 0b00000000); // feature byte 3
        mmu.write_u8(BIOS_SEGMENT, ADDR + 8, 0b00000000); // feature byte 4
        mmu.write_u8(BIOS_SEGMENT, ADDR + 9, 0b00000000); // feature byte 5
        mmu.write_u16(BIOS_SEGMENT, EQUIPMENT_WORD_ADDR, EQUIPMENT_WORD);
    }
}
