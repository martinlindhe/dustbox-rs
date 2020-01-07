extern crate serde_derive;

use bincode::deserialize;
use clap::{Arg, App};

use dustbox::tools::read_binary;
use dustbox::format::{DosExeHeader, DosExeHeaderRelocation};

const VERSION: &str = "0.1";

const PARAGRAPH_SIZE: u16 = 16;

fn main() {
    let matches = App::new("dustbox-exeinfo")
            .version(VERSION)
            .arg(Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true)
                .index(1))
            .get_matches();

    let filename = matches.value_of("INPUT").unwrap();
    println!("dustbox-exeinfo {} - {}", VERSION, filename);

    let data = match read_binary(filename) {
        Ok(data) => data,
        Err(e) => panic!(e),
    };

    // map bytes into struct
    let h: DosExeHeader = deserialize(&data[..]).unwrap();

    if h.signature[0] != 0x4D || h.signature[1] != 0x5A {
        println!("ERROR: Does not look like MZ header: {:?}", h.signature);
        return;
    }

    println!("{:#?}", h);

    println!("pages: {}, and {} bytes in last page", h.pages, h.bytes_in_last_page);

    println!("header paragraphs: {}", h.header_paragraphs);

    println!("extra paragraphs: {} min, {} max", h.min_extra_paragraphs, h.max_extra_paragraphs);

    println!("ss:sp = {:04X}:{:04X}", h.ss, h.sp);
    println!("cs:ip = {:04X}:{:04X}", h.cs, h.ip);
    println!("checksum: {:04X}", h.checksum);
    if h.overlay_number != 0 {
        println!("overlay number: {}", h.overlay_number);
    }
    let code_start = h.header_paragraphs * PARAGRAPH_SIZE;
    println!("entry point: {:04X}", code_start);

    if h.reloc_table_offset >= 0x40 {
        println!("ERROR: unhandled new-format (NE,LE,LX,W3,PE,etc.) executable");
        return;
    }

    if h.relocations > 0 {
        println!("relocations ({}):", h.relocations);
        let mut offset = h.reloc_table_offset as usize;
        for i in 0..h.relocations {
            let reloc: DosExeHeaderRelocation = deserialize(&data[offset..offset+4]).unwrap();
            println!("  {}: {:?}", i, reloc);
            offset += 2;
        }
    }

}
