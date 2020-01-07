/// http://www.delorie.com/djgpp/doc/exe/
#[derive(Deserialize, Debug)]
pub struct DosExeHeader {
    /// magic number "MZ"
    pub signature: [u8; 2],

    /// number of bytes in last 512-byte page of executable
    pub bytes_in_last_page: u16,

    /// Total number of 512-byte pages in executable (includes any partial last page)
    /// If `bytes_in_last_block` is non-zero, only that much of the last block is used.
    pub pages: u16,

    /// Number of relocation entries.
    pub relocations: u16,

    /// Header size in paragraphs.
    pub header_paragraphs: u16,

    /// Minimum paragraphs of memory required to allocate in addition to executable's size.
    pub min_extra_paragraphs: u16,

    /// Maximum paragraphs to allocate in addition to executable's size.
    pub max_extra_paragraphs: u16,

    /// Initial SS relative to start of executable. This value is added to the segment the
    /// program was loaded at, and the result is used to initialize the SS register.
    pub ss: i16,

    /// Initial SP.
    pub sp: u16,

    /// Checksum (usually unset).
    pub checksum: u16,

    /// Initial value of the IP register.
    pub ip: u16,

    /// Initial value of the CS register, relative to the segment the program was loaded at.
    pub cs: i16,

    /// Offset within header of relocation table.
    /// 40h or greater for new-format (NE,LE,LX,W3,PE,etc.) executable.
    pub reloc_table_offset: u16,

    /// Overlay number (normally 0000h = main program).
    pub overlay_number: u16,
}

#[derive(Deserialize, Debug)]
pub struct DosExeHeaderRelocation {
    pub offset: u16,
    pub segment: u16,
}
