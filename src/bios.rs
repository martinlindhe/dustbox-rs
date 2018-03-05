// https://wiki.osdev.org/BIOS
// dosbox-x: src/hardware/bios.cpp

use cpu::CPU;
use cpu::flags::Flags;
use memory::{MMU, MemoryAddress};
use gpu::modes::{GFXMode, VideoModeBlock};

#[derive(Clone)]
pub struct BIOS {
    pub flags_address: MemoryAddress, // the FLAGS register offset on stack while in interrupt
}

impl BIOS {
    pub const DATA_SEG: u16           = 0x0040; // bios data segment, 256 byte at 000400 to 0004FF

    pub const DATA_INITIAL_MODE: u16  = 0x0010;
    pub const DATA_CURRENT_MODE: u16  = 0x0049;
    pub const DATA_NB_COLS: u16       = 0x004A;
    pub const DATA_PAGE_SIZE: u16     = 0x004C;
    pub const DATA_CURRENT_START: u16 = 0x004E;
    pub const DATA_CURSOR_POS: u16    = 0x0050;
    pub const DATA_CURSOR_TYPE: u16   = 0x0060;
    pub const DATA_CURRENT_PAGE: u16  = 0x0062;
    pub const DATA_CRTC_ADDRESS: u16  = 0x0063;
    pub const DATA_CURRENT_MSR: u16   = 0x0065;
    pub const DATA_CURRENT_PAL: u16   = 0x0066;
    pub const DATA_NB_ROWS: u16       = 0x0084;
    pub const DATA_CHAR_HEIGHT: u16   = 0x0085;
    pub const DATA_VIDEO_CTL: u16     = 0x0087;
    pub const DATA_SWITCHES: u16      = 0x0088;
    pub const DATA_MODESET_CTL: u16   = 0x0089;
    pub const DATA_DCC_INDEX: u16     = 0x008A;
    pub const DATA_CRTCPU_PAGE: u16   = 0x008A;
    pub const DATA_VS_POINTER: u16    = 0x00A8;

    const ROM_SEG: u16                = 0xF000; // bios rom segment, 64k at F0000 to FFFFF
    const ROM_EQUIPMENT_WORD: u16     = 0x0410;

    pub fn new() -> Self {
        // XXX see ROMBIOS_Init in dosbox-x
        BIOS {
            flags_address: MemoryAddress::Unset,
        }
    }

    pub fn set_video_mode(&mut self, mmu: &mut MMU, mode: &VideoModeBlock, clear_mem: bool) {
        if mode.mode < 128 {
            mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_CURRENT_MODE, mode.mode as u8);
        } else {
            mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_CURRENT_MODE, (mode.mode - 0x98) as u8); // Looks like the s3 bios
        }
        mmu.write_u16(BIOS::DATA_SEG, BIOS::DATA_NB_COLS, mode.twidth as u16);
        mmu.write_u16(BIOS::DATA_SEG, BIOS::DATA_PAGE_SIZE, mode.plength as u16);
        mmu.write_u16(BIOS::DATA_SEG, BIOS::DATA_CRTC_ADDRESS, mode.crtc_address());
        mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_NB_ROWS, (mode.theight - 1) as u8);
        mmu.write_u16(BIOS::DATA_SEG, BIOS::DATA_CHAR_HEIGHT, mode.cheight as u16);
        let video_ctl = 0x60 | if clear_mem {
            0
        } else {
            0x80
        };
        mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_VIDEO_CTL, video_ctl);
        mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_SWITCHES, 0x09);

        // this is an index into the dcc table
        if mode.kind == GFXMode::VGA {
            mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_DCC_INDEX, 0x0B);
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
            self.write_ivt_entry(mmu, irq, BIOS::ROM_SEG, irq as u16);
            mmu.write_u8(BIOS::ROM_SEG, irq as u16, IRET);
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
        let base: u16 = 0xE6F5;
        mmu.write_u16(BIOS::ROM_SEG, base + 0, 8);         // table size
        mmu.write_u8(BIOS::ROM_SEG, base + 2, 0xFC);       // model: AT
        mmu.write_u8(BIOS::ROM_SEG, base + 3, 0);          // submodel
        mmu.write_u8(BIOS::ROM_SEG, base + 4, 0);          // BIOS revision
        mmu.write_u8(BIOS::ROM_SEG, base + 5, 0b00000000); // feature byte 1
        mmu.write_u8(BIOS::ROM_SEG, base + 6, 0b00000000); // feature byte 2
        mmu.write_u8(BIOS::ROM_SEG, base + 7, 0b00000000); // feature byte 3
        mmu.write_u8(BIOS::ROM_SEG, base + 8, 0b00000000); // feature byte 4
        mmu.write_u8(BIOS::ROM_SEG, base + 9, 0b00000000); // feature byte 5
        mmu.write_u16(BIOS::ROM_SEG, BIOS::ROM_EQUIPMENT_WORD, 0x0021);
    }
}

/// get the cursor x position
pub fn cursor_pos_col(mmu: &MMU, page: u8) -> u8 {
    return mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_CURSOR_POS + (page as u16 * 2));
}

/// get the cursor y position
pub fn cursor_pos_row(mmu: &MMU, page: u8) -> u8 {
    return mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_CURSOR_POS + (page as u16 * 2) + 1);
}
