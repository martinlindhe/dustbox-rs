/// http://www.delorie.com/djgpp/doc/exe/
/// http://www.delorie.com/djgpp/doc/rbinter/id/51/29.html

use std::fmt;

use bincode::deserialize;

pub struct ExeFile {
    pub header: ExeHeader,
    pub relocs: Vec<ExeRelocation>,
    pub program_data: Vec<u8>,

    /// total .EXE file size
    exe_size: usize,
}

pub enum ParseError {
    WrongMagic,
}

const DEBUG_PARSER: bool = false;

/// Header pages is 512 bytes
/// also sometimes called blocks
const PAGE_SIZE: u16 = 512;

impl ExeFile {
    pub fn from_data(data: &[u8]) -> Result<Self, ParseError> {
        let header = match ExeHeader::from_data(data) {
            Ok(hdr) => hdr,
            Err(e) => panic!(e),
        };

        if header.exe_data_end_offset() > data.len() {
            println!("WARNING: program end = {:04X} but data len = {:04X}", header.exe_data_end_offset(), data.len());
        }
        let program_data = data[header.exe_data_start_offset()..data.len()].to_vec();
        let relocs = header.parse_relocations(data);

        Ok(ExeFile {
            header: header,
            relocs: relocs,
            program_data: program_data,
            exe_size: data.len(),
        })
    }

    pub fn print_details(&self) {
        println!("exe file size: {} bytes", self.exe_size);
        self.header.print_details();

        if self.header.relocations > 0 {
            println!("relocations:");
            let mut i = 0;
            for reloc in &self.relocs {
                i += 1;
                println!("  {}: {}", i, reloc);
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ExeHeader {
    /// magic number "MZ"
    pub signature: [u8; 2],

    /// number of bytes in last 512-byte page of executable
    pub bytes_in_last_page: u16,

    /// Total number of 512-byte pages in executable (includes any partial last page)
    /// If `bytes_in_last_page` is non-zero, only that much of the last block is used.
    pub pages: u16,

    /// Number of relocation entries.
    pub relocations: u16,

    /// Header size in 16-byte paragraphs.
    pub header_paragraphs: u16,

    /// Minimum paragraphs of memory required to allocate in addition to executable's size.
    pub min_extra_paragraphs: u16,

    /// Maximum paragraphs to allocate in addition to executable's size.
    pub max_extra_paragraphs: u16,

    /// Initial (relative) SS.
    pub ss: i16,

    /// Initial SP.
    pub sp: u16,

    /// Checksum (usually unset).
    pub checksum: u16,

    /// Initial IP.
    pub ip: u16,

    /// Initial (relative) CS.
    pub cs: i16,

    /// Offset within header of relocation table.
    /// 40h or greater for new-format (NE,LE,LX,W3,PE,etc.) executable.
    pub reloc_table_offset: u16,

    /// Overlay number (normally 0000h = main program).
    pub overlay_number: u16,
}

impl ExeHeader {
    pub fn from_data(data: &[u8]) -> Result<Self, ParseError> {
        let h: ExeHeader = deserialize(data).unwrap();
        if h.signature[0] != 0x4D || h.signature[1] != 0x5A {
            return Err(ParseError::WrongMagic);
        }

        Ok(h)
    }

    /// Returns the header size in bytes.
    fn header_size(&self) -> usize {
        // a header paragraph is 16 bytes wide
        (self.header_paragraphs as usize) * 16
    }

    /// Returns the starting offset of the program code inside the EXE file.
    fn exe_data_start_offset(&self) -> usize {
        // XXX note this is not the start of CODE!
        self.header_size()
    }

    /// Returns the end offset of the program code inside the EXE file.
    fn exe_data_end_offset(&self) -> usize {
        let mut code_end = self.pages as usize * 512;
        if self.bytes_in_last_page > 0 {
            code_end -= 512 - self.bytes_in_last_page as usize;
        }
        code_end
    }

    /// parses the exe header relocation table
    fn parse_relocations(&self, data: &[u8]) -> Vec<ExeRelocation> {
        let mut relocs = Vec::new();

        if self.relocations > 0 {
            if DEBUG_PARSER {
                println!("relocations ({}):", self.relocations);
            }
            let mut offset = self.reloc_table_offset as usize;
            for i in 0..self.relocations {
                let reloc: ExeRelocation = deserialize(&data[offset..offset+4]).unwrap();
                if DEBUG_PARSER {
                    println!("  {}: {:?}", i, reloc);
                }
                relocs.push(reloc);
                offset += 4;
            }
        }
        relocs
    }

    fn print_details(&self) {
        println!("ExeHeader::print_details {:#?}", self);
        let pages_in_bytes = self.pages as usize * PAGE_SIZE as usize;
        println!("pages: {}, and {} bytes in last page, pages in bytes = {}", self.pages, self.bytes_in_last_page, pages_in_bytes);
        println!("header size: {} paragraphs / {} bytes (0x{:04X})", self.header_paragraphs, self.header_size(), self.header_size());
        println!("extra paragraphs: min {}, max {}", self.min_extra_paragraphs, self.max_extra_paragraphs);
        println!("ss:sp = {:04X}:{:04X}", self.ss, self.sp);
        println!("cs:ip = {:04X}:{:04X}", self.cs, self.ip);
        println!("checksum: {:04X}", self.checksum);
        if self.overlay_number != 0 {
            println!("overlay number: {}", self.overlay_number);
        }

        if self.reloc_table_offset >= 0x40 {
            println!("ERROR: unhandled new-format (NE,LE,LX,W3,PE,etc.) executable");
        }

        if self.relocations > 0 {
            let reloc_start = self.reloc_table_offset as usize;
            let reloc_end   = (reloc_start) + (self.relocations as usize * 4);
            let reloc_size  = reloc_end - reloc_start;
            println!("- relocations ({}) from {:04X} to {:04X} ({} bytes)", self.relocations, reloc_start, reloc_end, reloc_size);
        }

        let code_start = self.exe_data_start_offset();
        let code_end   = self.exe_data_end_offset();
        let code_size  = code_end - code_start;
        println!("- exe data from {:04X} to {:04X} ({} bytes)", code_start, code_end, code_size);
    }
}

#[derive(Deserialize, Debug)]
pub struct ExeRelocation {
    pub offset: u16,
    pub segment: u16,
}

impl fmt::Display for ExeRelocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04X}:{:04X}", self.segment, self.offset)
    }
}
