#[derive(Clone, Default)]
pub struct VgaCRTC {
    horizontal_total: u8,
    horizontal_display_end: u8,
    start_horizontal_blanking: u8,
    end_horizontal_blanking: u8,
    start_horizontal_retrace: u8,
    end_horizontal_retrace: u8,
    vertical_total: u8,
    overflow: u8,
    preset_row_scan: u8,
    maximum_scan_line: u8,
    cursor_start: u8,
    cursor_end: u8,
    start_address_high: u8,
    start_address_low: u8,
    cursor_location_high: u8,
    cursor_location_low: u8,
    vertical_retrace_start: u8,
    vertical_retrace_end: u8,
    vertical_display_end: u8,
    offset: u8,
    underline_location: u8,
    start_vertical_blanking: u8,
    end_vertical_blanking: u8,
    mode_control: u8,
    line_compare: u8,

    index: u8,
    read_only: bool,
}

impl VgaCRTC {
    // 03D4  rW  CRT (6845) register index   (CGA/MCGA/color EGA/color VGA)
    // selects which register (0-11h) is to be accessed through 03D5
    // bit 7-6 =0: (VGA) reserved
    // bit 5   =0: (VGA) reserved for testage
    // bit 4-0   : selects which register is to be accessed through 03D5
    pub fn set_index(&mut self, data: u8) {
        self.index = data & 0xF;
    }

    // 03D5  -W  CRT (6845) data register   (CGA/MCGA/color EGA/color VGA) (see #P0708)
    // selected by PORT 03D4h. registers 0C-0F may be read (see also PORT 03B5h)
    // MCGA, native EGA and VGA use very different defaults from those
    // mentioned for the other adapters; for additional notes and
    // registers 00h-0Fh and EGA/VGA registers 10h-18h and ET4000
    // registers 32h-37h see PORT 03B5h (see #P0654)
    // registers 10h-11h on CGA, EGA, VGA and 12h-14h on EGA, VGA are conflictive with MCGA (see #P0710)
    pub fn write_current(&mut self, data: u8) {
        match self.index {
            0x00 => self.horizontal_total = data,
            0x01 => self.horizontal_display_end = data,
            0x02 => self.start_horizontal_blanking = data,
            0x03 => self.end_horizontal_blanking = data,
            0x04 => self.start_horizontal_retrace = data,
            0x05 => self.end_horizontal_retrace = data,
            0x06 => self.vertical_total = data,
            0x07 => self.overflow = data,
            0x08 => self.preset_row_scan = data,
            0x09 => self.maximum_scan_line = data,
            0x0A => self.cursor_start = data,
            0x0B => self.cursor_end = data,
            0x0C => self.start_address_high = data,
            0x0D => self.start_address_low = data,
            0x0E => self.cursor_location_high = data,
            0x0F => self.cursor_location_low = data,
            0x10 => self.vertical_retrace_start = data,
            0x11 => self.vertical_retrace_end = data,
            0x12 => self.vertical_display_end = data,
            0x13 => self.offset = data,
            0x14 => self.underline_location = data,
            0x15 => self.start_vertical_blanking = data,
            0x16 => self.end_vertical_blanking = data,
            0x17 => self.mode_control = data,
            0x18 => self.line_compare = data,
            _ => panic!(),
        }
    }
}
