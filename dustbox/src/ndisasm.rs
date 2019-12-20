use std::io::{self, Read, Write};
use std::fs::File;
use std::process::Command;
use std::str;

use tempfile::tempdir;

use crate::cpu::{Encoder, Instruction};

pub fn ndisasm_first_instr(bytes: &[u8]) -> Result<String, io::Error> {
    let rows = ndisasm_bytes(bytes).unwrap();
    // parse syntax "00000000  CD21              int 0x21", return third column
    let mut col = 0;
    let mut spacing = false;
    let mut res = String::new();

    let s = rows.first().unwrap();
    for c in s.chars() {
        if c == ' ' {
            if !spacing && col < 2 {
                col += 1;
                spacing = true;
            }
        } else {
            spacing = false;
        }
        if col == 2 {
            res.push(c);
        }
    }

    Ok(res.trim().to_owned())
}

pub fn ndisasm_bytes(bytes: &[u8]) -> Result<Vec<String>, io::Error> {
    let tmp_dir = tempdir()?;
    let file_path = tmp_dir.path().join("binary.bin");
    let file_str = file_path.to_str().unwrap();
    let mut tmp_file = File::create(&file_path)?;

    tmp_file.write_all(bytes)?;

    let output = Command::new("ndisasm")
        .args(&["-b", "16", file_str])
        .output()
        .expect("failed to execute process");

    drop(tmp_file);
    tmp_dir.close()?;

    let stdout = output.stdout;
    let s = str::from_utf8(&stdout).unwrap();

    let res: Vec<String> = s.trim().lines().map(|s| s.to_string()).collect();
    Ok(res)
}

/// encodes an instruction and then disasms the resulting byte sequence with external ndisasm command
fn ndisasm_instruction(op: &Instruction) -> Result<Vec<String>, io::Error> {
    let encoder = Encoder::new();
    if let Ok(data) = encoder.encode(op) {
        ndisasm_bytes(&data)
    } else {
        panic!("invalid byte sequence");
    }
}

#[test]
pub fn can_ndisasm_first_instr() {
    let data = vec!(0x66, 0x0F, 0xBF, 0xC0, 0x66, 0x50);
    assert_eq!("movsx eax,ax", ndisasm_first_instr(&data).unwrap());
}

#[test]
pub fn can_ndisasm_bytes() {
    let data = vec!(0x66, 0x0F, 0xBF, 0xC0, 0x66, 0x50);
    assert_eq!("\
00000000  660FBFC0          movsx eax,ax
00000004  6650              push eax", ndisasm_bytes(&data).unwrap().join("\n"));
}
