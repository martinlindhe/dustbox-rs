// https://wiki.osdev.org/BIOS
// dosbox-x: src/hardware/bios.cpp

use crate::memory::{MMU, MemoryAddress};

#[derive(Clone)]
pub struct BIOS {
}

impl BIOS {
    pub const DATA_SEG: u16           = 0x0040; // bios data segment, 256 byte at 000400 to 0004FF
    pub const ROM_SEG: u16            = 0xF000; // bios rom segment, 64k at F_0000 to F_FFFF

    // offsets
    pub const DATA_INITIAL_MODE: u32  = 0x0010;
    pub const DATA_CURRENT_MODE: u32  = 0x0049;
    pub const DATA_NB_COLS: u32       = 0x004A;
    pub const DATA_PAGE_SIZE: u32     = 0x004C;
    pub const DATA_CURRENT_START: u32 = 0x004E;
    pub const DATA_CURSOR_POS: u32    = 0x0050;
    pub const DATA_CURSOR_TYPE: u32   = 0x0060;
    pub const DATA_CURRENT_PAGE: u32  = 0x0062;
    pub const DATA_CRTC_ADDRESS: u32  = 0x0063;
    pub const DATA_CURRENT_MSR: u32   = 0x0065;
    pub const DATA_CURRENT_PAL: u32   = 0x0066;
    pub const DATA_NB_ROWS: u32       = 0x0084;
    pub const DATA_CHAR_HEIGHT: u32   = 0x0085;
    pub const DATA_VIDEO_CTL: u32     = 0x0087;
    pub const DATA_SWITCHES: u32      = 0x0088;
    pub const DATA_MODESET_CTL: u32   = 0x0089;
    pub const DATA_DCC_INDEX: u32     = 0x008A;
    pub const DATA_CRTCPU_PAGE: u32   = 0x008A;
    pub const DATA_VS_POINTER: u32    = 0x00A8;

    pub const ROM_EQUIPMENT_WORD: u32 = 0x0410;

    pub fn default() -> Self {
        BIOS {
        }
    }

    pub fn init(&mut self, mut mmu: &mut MMU) {
        self.init_ivt(&mut mmu);
        self.write_configuration_data_table(&mut mmu);
    }

    fn init_ivt(&mut self, mmu: &mut MMU) {
        const IRET: u8 = 0xCF;
        for irq in 0..0xFF {
            self.write_ivt_entry(mmu, irq, BIOS::ROM_SEG, irq as u16);
            mmu.write_u8(BIOS::ROM_SEG, irq as u32, IRET);
        }
    }

    fn write_ivt_entry(&self, mmu: &mut MMU, number: u8, seg: u16, offset: u16) {
        let _seg = 0;
        let _offset = (number as u32) * 4;
        mmu.write_u16(_seg, _offset, offset);
        mmu.write_u16(_seg, _offset + 2, seg);
    }

    /// initializes the Configuration Data Table
    fn write_configuration_data_table(&self, mmu: &mut MMU) {
        let mut addr = MemoryAddress::RealSegmentOffset(BIOS::ROM_SEG, 0xE6F5);
        mmu.write_u16_inc(&mut addr, 8);          // table size
        mmu.write_u8_inc(&mut addr, 0xFC);        // model: AT
        mmu.write_u8_inc(&mut addr, 0);           // submodel
        mmu.write_u8_inc(&mut addr, 0);           // BIOS revision
        mmu.write_u8_inc(&mut addr, 0b0000_0000); // feature byte 1
        mmu.write_u8_inc(&mut addr, 0b0000_0000); // feature byte 2
        mmu.write_u8_inc(&mut addr, 0b0000_0000); // feature byte 3
        mmu.write_u8_inc(&mut addr, 0b0000_0000); // feature byte 4
        mmu.write_u8_inc(&mut addr, 0b0000_0000); // feature byte 5
        mmu.write_u16(BIOS::ROM_SEG, BIOS::ROM_EQUIPMENT_WORD, 0x0021);
    }
}
