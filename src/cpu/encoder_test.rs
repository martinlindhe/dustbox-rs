use std::fs::File;
use std::io::{self, Read, Write};
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

    // XXX TODO: assemble custom prober.com (with knowledge of "affected" registers???)

    let prober_com = "/Users/m/dev/rs/dustbox-rs/utils/prober/prober.com"; // XXX expand relative path
    let output = stdout_from_winxp_vmware(prober_com);

    println!("{}", output);
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
