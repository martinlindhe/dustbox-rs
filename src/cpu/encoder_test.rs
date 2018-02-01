use std::fs::File;
use std::io::{self, Write};
use std::process::Command;
use std::str;

use tempdir::TempDir;

use cpu::Encoder;
use cpu::CPU;
use cpu::RepeatMode;
use cpu::Segment;
use cpu::{Parameter, ParameterPair};
use cpu::instruction::{Instruction, InstructionInfo, Op};
use cpu::register::SR;
use memory::mmu::MMU;


#[test]
fn can_encode_instr() {
    let op = Instruction::new(Op::Int(), Parameter::Imm8(0x21));

    let encoder = Encoder::new();
    assert_eq!(vec!(0xCD, 0x21), encoder.encode(&op));

    assert_eq!("int 0x21".to_owned(), ndisasm(&op).unwrap());
}

/// disasm the encoded instruction with ndisasm
fn ndisasm(op: &Instruction) -> Result<String, io::Error> {
    let encoder = Encoder::new();
    let data = encoder.encode(op);

    let tmp_dir = TempDir::new("ndisasm")?;
    let file_path = tmp_dir.path().join("binary.bin");
    let file_str = file_path.to_owned();
    let mut tmp_file = File::create(file_path)?;

    tmp_file.write(&data)?;

    let output = Command::new("ndisasm")
            .args(&[file_str])
            .output()
            .expect("failed to execute process");

    drop(tmp_file);
    tmp_dir.close()?;

    let s = str::from_utf8(&output.stdout).unwrap().trim();

    // parse syntax "00000000  CD21              int 0x21", return third column
    let mut col = 0;
    let mut spacing = false;
    let mut res = String::new();
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
