use std::fs::File;
use std::io::{self, Read, Write};
use std::process::Command;
use std::str;
use std::collections::HashMap;

use tempdir::TempDir;
use tera::Context;

use cpu::CPU;
use cpu::encoder::Encoder;
use cpu::segment::Segment;
use cpu::parameter::Parameter;
use cpu::instruction::{Instruction, InstructionInfo, RepeatMode};
use cpu::op::Op;
use cpu::register::{R8, AMode};
use memory::mmu::MMU;


#[test]
fn can_encode_bitshift_instructions() {
    let encoder = Encoder::new();

    let op = Instruction::new2(Op::Shr8, Parameter::Reg8(R8::AH), Parameter::Imm8(0xFF));
    assert_eq!(vec!(0xC0, 0xEC, 0xFF), encoder.encode(&op));
    assert_eq!("shr ah,byte 0xff".to_owned(), ndisasm(&op).unwrap());

    let op = Instruction::new2(Op::Shl8, Parameter::Reg8(R8::AH), Parameter::Imm8(0xFF));
    assert_eq!(vec!(0xC0, 0xE4, 0xFF), encoder.encode(&op));
    assert_eq!("shl ah,byte 0xff".to_owned(), ndisasm(&op).unwrap());
}

#[test]
fn can_encode_int() {
    let encoder = Encoder::new();

    let op = Instruction::new1(Op::Int(), Parameter::Imm8(0x21));
    assert_eq!(vec!(0xCD, 0x21), encoder.encode(&op));
    assert_eq!("int 0x21".to_owned(), ndisasm(&op).unwrap());
}

#[test]
fn can_encode_mov_addressing_modes() {
    let encoder = Encoder::new();

    // r8, imm8
    let op = Instruction::new2(Op::Mov8(), Parameter::Reg8(R8::BH), Parameter::Imm8(0xFF));
    assert_eq!("mov bh,0xff".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0xB7, 0xFF), encoder.encode(&op));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Mov8(), Parameter::Reg8(R8::BH), Parameter::Reg8(R8::DL));
    assert_eq!("mov bh,dl".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0xD7), encoder.encode(&op));

    // r/m8, r8  (dst is AMode::BP + imm8)
    let op = Instruction::new2(Op::Mov8(), Parameter::Ptr8AmodeS8(Segment::Default, AMode::BP, 0x10), Parameter::Reg8(R8::BH));
    assert_eq!("mov [bp+0x10],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0x7E, 0x10), encoder.encode(&op));

    // r/m8, r8  (dst is AMode::BP + imm8)    - reversed
    let op = Instruction::new2(Op::Mov8(), Parameter::Reg8(R8::BH), Parameter::Ptr8AmodeS8(Segment::Default, AMode::BP, 0x10));
    assert_eq!(vec!(0x8A, 0x7E, 0x10), encoder.encode(&op));
    assert_eq!("mov bh,[bp+0x10]".to_owned(), ndisasm(&op).unwrap());

    // r/m8, r8  (dst is AMode::BP + imm8)
    let op = Instruction::new2(Op::Mov8(), Parameter::Ptr8AmodeS16(Segment::Default, AMode::BP, -0x800), Parameter::Reg8(R8::BH));
    assert_eq!("mov [bp-0x800],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0xBE, 0x00, 0xF8), encoder.encode(&op));

    // r/m8, r8  (dst is [imm16]) // XXX no direct amode mapping in resulting Instruction. can we implement a "Instruction.AMode() -> AMode" ?
    let op = Instruction::new2(Op::Mov8(), Parameter::Ptr8(Segment::Default, 0x8000), Parameter::Reg8(R8::BH));
    assert_eq!("mov [0x8000],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0x3E, 0x00, 0x80), encoder.encode(&op));

    // r/m8, r8  (dst is [bx])
    let op = Instruction::new2(Op::Mov8(), Parameter::Ptr8Amode(Segment::Default, AMode::BX), Parameter::Reg8(R8::BH));
    assert_eq!("mov [bx],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0x3F), encoder.encode(&op));
}

#[test] #[ignore] // expensive test
fn vmware_fuzz() {
    let op = Instruction::new2(Op::Mov8(), Parameter::Reg8(R8::BH), Parameter::Imm8(0xFF));

    let prober_com = "/Users/m/dev/rs/dustbox-rs/utils/prober/prober.com"; // XXX expand relative path

    assemble_prober(&op, prober_com);
    let output = stdout_from_winxp_vmware(prober_com);

    let m = prober_reg_map(&output);
    println!("vmware result: {:?}", m);

    // TODO: run the program in dustbox too, capture stdout and compare results
}

fn assemble_prober(op: &Instruction, prober_com: &str) {
    let mut tera = compile_templates!("utils/prober/*.tpl.asm");

    // disable autoescaping
    tera.autoescape_on(vec![]);

    let mut context = Context::new();
    context.add("snippet", &op_as_db_bytes(&op));
    // add stuff to context
    match tera.render("prober.tpl.asm", &context) {
        Ok(res) => {
            use std::fs::File;
            use std::io::Write;
            let mut f = File::create("utils/prober/prober.asm").expect("Unable to create file");
            f.write_all(res.as_bytes()).expect("Unable to write data");
        }
        Err(why) => println!("ERROR = {}", why),
    }

    // assemble generated prober.asm
    Command::new("nasm")
        .current_dir("/Users/m/dev/rs/dustbox-rs/utils/prober") // XXX get path name from prober_com
        .args(&["-f", "bin", "-o", "prober.com", "prober.asm"])
        .output()
        .expect("failed to execute process");
}

// creates a "db 0x1,0x2..." representation of the encoded instruction
fn op_as_db_bytes(op: &Instruction) -> String {
    let encoder = Encoder::new();
    let enc = encoder.encode(&op);

    let mut v = Vec::new();
    for c in enc {
        v.push(format!("0x{:02X}", c));
    }
    let s = v.join(",");
    format!("db {}", s)
}

// parse prober.com output into a map
fn prober_reg_map(stdout: &str) -> HashMap<String, u16>{
    let mut map = HashMap::new();
    let lines: Vec<String> = stdout.split("\r\n").map(|s| s.to_string()).collect();

    for line in lines {
        if let Some(pos) = line.find('=') {
            let p1 = &line[0..pos];
            let p2 = &line[pos+1..];
            let val = u16::from_str_radix(p2, 16).unwrap();
            map.insert(p1.to_owned(), val);
        }
    }

    map
}

// run .com in vm, parse result
fn stdout_from_winxp_vmware(prober_com: &str) -> String {
    let vmx = "/Users/m/Documents/Virtual Machines.localized/Windows XP Professional.vmwarevm/Windows XP Professional.vmx";
    let vm_user = "vmware";
    let vm_password = "vmware";

    // copy file to guest
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "copyFileFromHostToGuest", vmx, prober_com, "C:\\prober.com"])
        .output()
        .expect("failed to execute process");

    // run prober.bat, where prober.bat is "c:\prober.com > c:\prober.out" (XXX create this file in vm once)
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "runProgramInGuest", vmx, "C:\\prober.bat"])
        .output()
        .expect("failed to execute process");

    let tmp_dir = TempDir::new("vmware").unwrap();
    let file_path = tmp_dir.path().join("prober.out");
    let file_str = file_path.to_str().unwrap();

    // copy back result
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "copyFileFromGuestToHost", vmx, "C:\\prober.out", file_str])
        .output()
        .expect("failed to execute process");

    let mut buffer = String::new();
    let mut f = match File::open(&file_path) {
        Ok(x) => x,
        Err(why) => {
            panic!("Could not open file {:?}: {}", file_path, why);
        }
    };
    match f.read_to_string(&mut buffer) {
        Ok(x) => x,
        Err(why) => {
            panic!("could not read contents of file: {}", why);
        }
    };

    drop(f);
    tmp_dir.close().unwrap();

    buffer
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
