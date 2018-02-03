use std::fs::File;
use std::io::{self, Read, Write};
use std::process::Command;
use std::str;
use std::collections::HashMap;
use std::time::Instant;

use tempdir::TempDir;
use tera::Context;

use cpu::CPU;
use cpu::encoder::Encoder;
use cpu::segment::Segment;
use cpu::parameter::Parameter;
use cpu::instruction::{Instruction, InstructionInfo, RepeatMode};
use cpu::op::Op;
use cpu::register::{R8, AMode, R16};
use memory::mmu::MMU;

#[test]
fn can_encode_push() {
    let encoder = Encoder::new();

    let op = Instruction::new1(Op::Push16, Parameter::Imm16(0x8088));
    assert_eq!(vec!(0x68, 0x88, 0x80), encoder.encode(&op));
    assert_eq!("push word 0x8088".to_owned(), ndisasm(&op).unwrap());
}

#[test]
fn can_encode_pop() {
    let encoder = Encoder::new();

    let op = Instruction::new(Op::Popf);
    assert_eq!(vec!(0x9D), encoder.encode(&op));
    assert_eq!("popf".to_owned(), ndisasm(&op).unwrap());
}

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


#[test] #[ignore] // expensive test
fn can_fuzz_shr() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let encoder = Encoder::new();

    let mut tot_sec = 0.;

    for i in 1..65535 as usize {
        let n1 = ((i + 1) & 0xFF) ^ 0xFF;
        let n2 = i & 0xFF;
        let ops = vec!(
            // clear flags
            Instruction::new1(Op::Push16, Parameter::Imm16(0)),
            Instruction::new(Op::Popf),
            // clear ax,bx,cx,dx
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::AX), Parameter::Imm16(0)),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::BX), Parameter::Imm16(0)),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::CX), Parameter::Imm16(0)),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::DX), Parameter::Imm16(0)),
            // mutate parameters
            Instruction::new2(Op::Mov8, Parameter::Reg8(R8::AH), Parameter::Imm8(n1 as u8)),
            Instruction::new2(Op::Shr8, Parameter::Reg8(R8::AH), Parameter::Imm8(n2 as u8)),
        );
        let data = encoder.encode_vec(&ops);

        // execute the ops in dustbox
        cpu.load_com(&data);

        cpu.execute_instructions(ops.len());

        // run in vm, compare regs
        let prober_com = "/Users/m/dev/rs/dustbox-rs/utils/prober/prober.com"; // XXX expand relative path
        assemble_prober(&ops, prober_com);

        let now = Instant::now();
        //let output = stdout_from_vmx_vmrun(prober_com); // ~2.3 seconds per call
        let output = stdout_from_vm_http(prober_com); // ~0.05 seconds

        let elapsed = now.elapsed();
        let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
        tot_sec += sec;
        if i % 100 == 0 {
            println!("avg vm time after {} iterations: {:.*}s", i, 4, tot_sec / i as f64);
        }

        let vm_regs = prober_reg_map(&output);
        if compare_regs(&cpu, &vm_regs, vec!("ax")) {
            println!("shr 0x{:x}, 0x{:x}       (vm time {}s)", n1, n2, sec);
        }
    }
}

fn compare_regs(cpu: &CPU, vm_regs: &HashMap<String, u16>, reg_names: Vec<&str>) -> bool {
    let mut ret = false;
    for s in reg_names {
        if compare_reg(s, cpu, vm_regs[s]) {
            ret = true;
        }
    }
    ret
}

// returns true if registers dont match
fn compare_reg(reg_name: &str, cpu: &CPU, vm_val: u16) -> bool {
    let idx = reg_str_to_index(reg_name);
    let reg: R16 = Into::into(idx as u8);
    let dustbox_val = cpu.get_r16(&reg);
    if dustbox_val != vm_val {
        println!("{} differs. dustbox {:04x}, vm {:04x}", reg_name, dustbox_val, vm_val);
        true
    } else {
        false
    }
}

fn reg_str_to_index(s: &str) -> usize {
    match s {
        "al" => 0,
        "cl" => 1,
        "dl" => 2,
        "bl" => 3,
        "ah" => 4,
        "ch" => 5,
        "dh" => 6,
        "bh" => 7,

        "ax" => 0,
        "cx" => 1,
        "dx" => 2,
        "bx" => 3,
        "sp" => 4,
        "bp" => 5,
        "si" => 6,
        "di" => 7,
        _ => panic!("{}", s),
    }
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
    let op = Instruction::new2(Op::Mov8, Parameter::Reg8(R8::BH), Parameter::Imm8(0xFF));
    assert_eq!("mov bh,0xff".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0xB7, 0xFF), encoder.encode(&op));

    // r16, imm8
    let op = Instruction::new2(Op::Mov16, Parameter::Reg16(R16::BX), Parameter::Imm16(0x8844));
    assert_eq!("mov bx,0x8844".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0xBB, 0x44, 0x88), encoder.encode(&op));

    // r/m8, r8  (dst is r8)
    let op = Instruction::new2(Op::Mov8, Parameter::Reg8(R8::BH), Parameter::Reg8(R8::DL));
    assert_eq!("mov bh,dl".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0xD7), encoder.encode(&op));

    // r/m8, r8  (dst is AMode::BP + imm8)
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8AmodeS8(Segment::Default, AMode::BP, 0x10), Parameter::Reg8(R8::BH));
    assert_eq!("mov [bp+0x10],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0x7E, 0x10), encoder.encode(&op));

    // r/m8, r8  (dst is AMode::BP + imm8)    - reversed
    let op = Instruction::new2(Op::Mov8, Parameter::Reg8(R8::BH), Parameter::Ptr8AmodeS8(Segment::Default, AMode::BP, 0x10));
    assert_eq!(vec!(0x8A, 0x7E, 0x10), encoder.encode(&op));
    assert_eq!("mov bh,[bp+0x10]".to_owned(), ndisasm(&op).unwrap());

    // r/m8, r8  (dst is AMode::BP + imm8)
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8AmodeS16(Segment::Default, AMode::BP, -0x800), Parameter::Reg8(R8::BH));
    assert_eq!("mov [bp-0x800],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0xBE, 0x00, 0xF8), encoder.encode(&op));

    // r/m8, r8  (dst is [imm16]) // XXX no direct amode mapping in resulting Instruction. can we implement a "Instruction.AMode() -> AMode" ?
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8(Segment::Default, 0x8000), Parameter::Reg8(R8::BH));
    assert_eq!("mov [0x8000],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0x3E, 0x00, 0x80), encoder.encode(&op));

    // r/m8, r8  (dst is [bx])
    let op = Instruction::new2(Op::Mov8, Parameter::Ptr8Amode(Segment::Default, AMode::BX), Parameter::Reg8(R8::BH));
    assert_eq!("mov [bx],bh".to_owned(), ndisasm(&op).unwrap());
    assert_eq!(vec!(0x88, 0x3F), encoder.encode(&op));
}

fn assemble_prober(ops: &Vec<Instruction>, prober_com: &str) {
    let mut tera = compile_templates!("utils/prober/*.tpl.asm");

    // disable autoescaping
    tera.autoescape_on(vec![]);

    let mut context = Context::new();
    context.add("snippet", &ops_as_db_bytes(&ops));
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

// creates a "db 0x1,0x2..." representation of the encoded instructions
fn ops_as_db_bytes(ops: &Vec<Instruction>) -> String {
    let encoder = Encoder::new();
    let enc = encoder.encode_vec(&ops);

    let mut v = Vec::new();
    for c in enc {
        v.push(format!("0x{:02X}", c));
    }
    let s = v.join(",");
    format!("db {}", s)
}

// parse prober.com output into a map
fn prober_reg_map(stdout: &str) -> HashMap<String, u16> {
    let mut map = HashMap::new();
    let lines: Vec<String> = stdout.split("\n").map(|s| s.to_string()).collect();

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

// upload data as http post to supersafe http server running in VM
fn stdout_from_vm_http(prober_com: &str) -> String {
    use curl::easy::{Easy, Form};

    let mut dst = Vec::new();
    let mut easy = Easy::new();
    easy.url("http://10.10.30.63:28111/run").unwrap();

    let mut form = Form::new();
    form.part("com").file(prober_com).add().unwrap();
    easy.httppost(form).unwrap();

    {
        let mut transfer = easy.transfer();
        transfer.write_function(|data| {
            dst.extend_from_slice(data);
            Ok(data.len())
        }).unwrap();
        transfer.perform().unwrap();
    }

    str::from_utf8(&dst).unwrap().to_owned()
}

// run .com with vmrun (vmware), parse result
fn stdout_from_vmx_vmrun(prober_com: &str) -> String {
    let vmx = "/Users/m/Documents/Virtual Machines.localized/Windows XP Professional.vmwarevm/Windows XP Professional.vmx";
    let vm_user = "vmware";
    let vm_password = "vmware";

    let now = Instant::now();

    // copy file to guest
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "copyFileFromHostToGuest", vmx, prober_com, "C:\\prober.com"])
        .output()
        .expect("failed to execute process");

    let elapsed = now.elapsed();
    let upload_sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);

    let now = Instant::now();
    // run prober.bat, where prober.bat is "c:\prober.com > c:\prober.out" (XXX create this file in vm once)
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "runProgramInGuest", vmx, "C:\\prober.bat"])
        .output()
        .expect("failed to execute process");

    let elapsed = now.elapsed();
    let run_sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);

    let tmp_dir = TempDir::new("vmware").unwrap();
    let file_path = tmp_dir.path().join("prober.out");
    let file_str = file_path.to_str().unwrap();

    let now = Instant::now();
    // copy back result
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "copyFileFromGuestToHost", vmx, "C:\\prober.out", file_str])
        .output()
        .expect("failed to execute process");

    let elapsed = now.elapsed();
    let download_sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);

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

    println!("vmrun: upload {}s, run {}s, download {}s", upload_sec, run_sec, download_sec);

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
