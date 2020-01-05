use std::collections::HashMap;
use std::process::Command;
use std::str;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Read, Write};

use tera::{Tera, Context};
use tempfile::tempdir;

use dustbox::cpu::{CPU, Op, r16};
use dustbox::machine::Machine;
use dustbox::ndisasm::ndisasm_bytes;

pub enum VmRunner {
    VmHttp,
    VmxVmrun,
    DosboxX,
}

const DEBUG_ENCODER: bool = false;

/// return false on failure
pub fn fuzz(runner: &VmRunner, data: &[u8], op_count: usize, affected_registers: &[&str], affected_flag_mask: u16) -> bool {
    let mut machine = Machine::deterministic();
    println!("EXECUTING {:X?}", data);

    if DEBUG_ENCODER {
        println!("{}", ndisasm_bytes(&data).unwrap().join("\n"));
    }


    machine.load_executable(data);
    machine.execute_instructions(op_count);

    // run in vm, compare regs

    let prober_com = Path::new("utils/prober/prober.com");
    assemble_prober(data, prober_com);

    let output = match *runner {
        VmRunner::VmHttp => stdout_from_vm_http(prober_com), // ~0.05 seconds per call
        VmRunner::VmxVmrun => stdout_from_vmx_vmrun(prober_com), // ~2.3 seconds
        VmRunner::DosboxX => stdout_from_dosbox(prober_com), // ~2.3 seconds
    };

    let vm_regs = prober_reg_map(&output);
    if vm_regs.is_empty() {
        println!("FATAL: no vm regs from vm output: {}", output);
        return false;
    }

    if compare_regs(&machine.cpu, &vm_regs, affected_registers) {
        println!("\nMAJOR: regs differ");
        return false;
    }

    let vm_flags = vm_regs["flag"];
    let vm_masked_flags = vm_flags & affected_flag_mask;
    let dustbox_flags = machine.cpu.regs.flags.u16();
    let dustbox_masked_flags = dustbox_flags & affected_flag_mask;
    if vm_masked_flags != dustbox_masked_flags {
        let xored = vm_masked_flags ^ dustbox_masked_flags;
        print!("\nflag diff: vm {:04x} {:8} vs dustbox {:04x} {:8} = diff {:8}\n",
            vm_masked_flags, bitflags_str(vm_masked_flags), dustbox_masked_flags, bitflags_str(dustbox_masked_flags), bitflags_str(xored));
        return false;
    }
    true
}

// return 8 char string
fn bitflags_str(f: u16) -> String {
    let mut s = String::new();
    if f & 0x0000_0001 != 0 {
        s.push_str("C");
    } else {
        s.push_str(" ");
    }
    if f & 0x0000_0004 != 0 {
        s.push_str("P");
    } else {
        s.push_str(" ");
    }
    if f & 0x0000_0010 != 0 {
        s.push_str("A");
    } else {
        s.push_str(" ");
    }
    if f & 0x0000_0040 != 0 {
        s.push_str("Z");
    } else {
        s.push_str(" ");
    }
    if f & 0x0000_0080 != 0 {
        s.push_str("S");
    } else {
        s.push_str(" ");
    }
    if f & 0x0000_0200 != 0 {
        s.push_str("I");
    } else {
        s.push_str(" ");
    }
    if f & 0x0000_0400 != 0 {
        s.push_str("D");
    } else {
        s.push_str(" ");
    }
    if f & 0x0000_0800 != 0 {
        s.push_str("O");
    } else {
        s.push_str(" ");
    }
    s
}

pub struct AffectedFlags {
    // ____ O___ SZ_A _P_C
    pub c: u8, // 0: carry flag
    pub p: u8, // 2: parity flag
    pub a: u8, // 4: adjust flag
    pub z: u8, // 6: zero flag
    pub s: u8, // 7: sign flag
    pub i: u8, // 9: interrupt flag
    pub d: u8, // 10 direction flag
    pub o: u8, // 11: overflow flag
}

impl AffectedFlags {
    /// returns a flag mask for affected flag registers by op
    pub fn for_op(op: &Op) -> u16 {
        match *op {
            Op::Nop | Op::Salc | Op::Not8 | Op::Not16 | Op::Div8 | Op::Div16 | Op::Idiv8 | Op::Idiv16 |
            Op::Cbw | Op::Cwd16 | Op::Lahf | Op::Lea16 | Op::Xchg8 | Op::Xchg16 | Op::Xlatb =>
                AffectedFlags{s:0, z:0, p:0, c:0, a:0, o:0, d:0, i:0}.mask(), // none

            Op::Sahf => AffectedFlags{o:1, s:1, z:1, a:1, p:1, c:1, d:1, i:1}.mask(), // all

            Op::Sub8 | Op::Sbb8 |
            Op::Add8 | Op::Adc8 | Op::Cmp8 | Op::Cmp16 | Op::Neg8 | Op::Neg16 |
            Op::Shrd | Op::Cmpsw =>
                AffectedFlags{c:1, s:1, z:1, a:1, p:1, o:1, d:0, i:0}.mask(), // C A S Z P O

            Op::Shl8 | Op::Shr8 | Op::Sar8 | Op::And8 | Op::Or8 =>
                AffectedFlags{c:1, o:1, s:1, z:1, a:0, p:1, d:0, i:0}.mask(), // C O S Z P

            Op::Daa | Op::Das => AffectedFlags{c:1, s:1, z:1, a:1, p:1, o:0, d:0, i:0}.mask(), // C A S Z P
            Op::Inc8 | Op::Inc16 | Op::Inc32 | Op::Dec8 | Op::Dec16 | Op::Dec32 | Op::Shld => AffectedFlags{s:1, z:1, a:1, p:1, o:1, c:0, d:0, i:0}.mask(), // S Z P O A
            Op::Aaa | Op::Aas => AffectedFlags{c:1, a:1, o:0, s:0, z:0, p:0, d:0, i:0}.mask(),  // C A
            Op::Rol8 | Op::Rcl8 | Op::Ror8 | Op::Rcr8 | Op::Mul8 | Op::Mul16 | Op::Imul8 | Op::Imul16 => AffectedFlags{c:1, o:1, z:0, s:0, p:0, a:0, d:0, i:0}.mask(), // C O
            Op::Aad | Op::Aam | Op::Xor8 | Op::Test8 | Op::Test16 => AffectedFlags{s:1, z:1, p:1, c:1, a:0, o:1, d:0, i:0}.mask(),        // O C S Z P
            Op::Bt | Op::Clc | Op::Cmc | Op::Stc => AffectedFlags{c:1, a:0, o:0, s:0, z:0, p:0, d:0, i:0}.mask(),  // C
            Op::Cld | Op::Std => AffectedFlags{d:1, c:0, a:0, o:0, s:0, z:0, p:0, i:0}.mask(),  // D
            Op::Cli | Op::Sti => AffectedFlags{i:1, d:0, c:0, a:0, o:0, s:0, z:0, p:0}.mask(),  // I
            Op::Bsf => AffectedFlags{s:0, z:1, p:0, c:0, a:0, o:0, d:0, i:0}.mask(),        // Z
            _ => panic!("AffectedFlags: unhandled op {:?}", op),
        }
    }

    fn mask(&self) -> u16 {
        let mut out = 0;
        if self.c != 0 {
            out |= 0x0000_0001;
        }
        if self.p != 0 {
            out |= 0x0000_0004;
        }
        if self.a != 0 {
            out |= 0x0000_0010;
        }
        if self.z != 0 {
            out |= 0x0000_0040;
        }
        if self.s != 0 {
            out |= 0x0000_0080;
        }
        if self.i != 0 {
            out |= 0x0000_0200;
        }
        if self.d != 0 {
            out |= 0x0000_0400;
        }
        if self.o != 0 {
            out |= 0x0000_0800;
        }
        out
    }
}

fn compare_regs<'a>(cpu: &CPU, vm_regs: &HashMap<String, u16>, reg_names: &[&'a str]) -> bool {
    let mut ret = false;
    for s in reg_names {
        let s = s.to_owned();
        if compare_reg(s, cpu, vm_regs[s]) {
            ret = true;
        }
    }
    ret
}

/// returns true if registers dont match
fn compare_reg(reg_name: &str, cpu: &CPU, vm_val: u16) -> bool {
    let idx = reg_str_to_index(reg_name);
    let reg = r16(idx as u8);
    let dustbox_val = cpu.get_r16(reg);
    if dustbox_val != vm_val {
        println!("{} differs. dustbox {:04x}, vm {:04x}", reg_name, dustbox_val, vm_val);
        true
    } else {
        false
    }
}

fn reg_str_to_index(s: &str) -> usize {
    match s {
        "al" | "ax" => 0,
        "cl" | "cx" => 1,
        "dl" | "dx" => 2,
        "bl" | "bx" => 3,
        "ah" | "sp" => 4,
        "ch" | "bp" => 5,
        "dh" | "si" => 6,
        "bh" | "di" => 7,
        _ => panic!("{}", s),
    }
}

fn assemble_prober(data: &[u8], path: &Path) {
    let mut tera = match Tera::new("utils/prober/*.tpl.asm") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    // disable autoescaping
    tera.autoescape_on(vec![]);

    let mut context = Context::new();
    context.insert("snippet", &vec_as_db_bytes(data));
    // add stuff to context
    match tera.render("prober.tpl.asm", &context) {
        Ok(res) => {
            let mut f = File::create("utils/prober/prober.asm").expect("Unable to create file");
            f.write_all(res.as_bytes()).expect("Unable to write data");
        }
        Err(why) => panic!("fatal tera error: {}", why),
    }

    let dir = path.parent().unwrap();

    // assemble generated prober.asm
    Command::new("nasm")
        .current_dir(dir)
        .args(&["-f", "bin", "-o", "prober.com", "prober.asm"])
        .output()
        .expect("failed to execute process");
}

/*
// creates a "db 0x1,0x2..." representation of the encoded instructions
fn ops_as_db_bytes(ops: &[Instruction]) -> String {
    let encoder = Encoder::new();
    if let Ok(data) = encoder.encode_vec(ops) {
        let mut v = Vec::new();
        for c in data {
            v.push(format!("0x{:02X}", c));
        }
        let s = v.join(",");
        format!("db {}", s)
    } else {
        panic!("invalid byte sequence");
    }
}
*/

/// creates a "db 0x1,0x2..." representation of a &[u8]
fn vec_as_db_bytes(data: &[u8]) -> String {
    let mut v = Vec::new();
    for c in data {
        v.push(format!("0x{:02X}", c));
    }
    let s = v.join(",");
    format!("db {}", s)
}

/// parse prober.com output into a map
fn prober_reg_map(stdout: &str) -> HashMap<String, u16> {
    let mut map = HashMap::new();
    let lines: Vec<&str> = stdout.split('\n').collect();

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

/// upload data as http post to supersafe http server running in VM
fn stdout_from_vm_http(path: &Path) -> String {
    use curl::easy::{Easy, Form};
    use std::time::Duration;
    let mut dst = Vec::new();
    let mut easy = Easy::new();
    let timeout = Duration::from_millis(1000);
    easy.timeout(timeout).unwrap();
    easy.url("http://172.16.72.129:28111/run").unwrap();

    let mut form = Form::new();
    form.part("com").file(path).add().unwrap();
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

fn stdout_from_dosbox(path: &Path) -> String {

    // copy prober.com to ~/dosbox-x
    use std::fs;
    fs::copy(path, "/Users/m/dosbox-x/prober.com").unwrap();

    Command::new("dosbox-x")
        .args(&["-c", "prober.com > PROBER.OUT", "-fastbioslogo", "--exit"])
        .current_dir("/Users/m/dosbox-x")
        .output()
        .expect("failed to execute process");

    let cwd = Path::new("/Users/m/dosbox-x");
    let file_path = cwd.join("PROBER.OUT");

    read_text_file(&file_path)
}

/// run .com with vmrun (vmware), parse result
fn stdout_from_vmx_vmrun(path: &Path) -> String {
    let vmx = "/Users/m/Documents/Virtual Machines.localized/Windows XP Professional.vmwarevm/Windows XP Professional.vmx";
    let vm_user = "vmware";
    let vm_password = "vmware";

    // copy file to guest
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "copyFileFromHostToGuest", vmx, path.to_str().unwrap(), "C:\\prober.com"])
        .output()
        .expect("failed to execute process");

    // run prober.bat, where prober.bat is "c:\prober.com > c:\prober.out" (XXX create this file in vm once)
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "runProgramInGuest", vmx, "C:\\prober.bat"])
        .output()
        .expect("failed to execute process");

    let tmp_dir = tempdir().unwrap();
    let file_path = tmp_dir.path().join("prober.out");
    let file_str = file_path.to_str().unwrap();

    // copy back result
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "copyFileFromGuestToHost", vmx, "C:\\prober.out", file_str])
        .output()
        .expect("failed to execute process");

    let buffer = read_text_file(&file_path);

    let f = File::open(&file_path);
    drop(f);
    tmp_dir.close().unwrap();

    buffer
}

fn read_text_file(filename: &PathBuf) -> String {
    let mut buffer = String::new();
    let mut f = match File::open(&filename) {
        Ok(x) => x,
        Err(why) => {
            panic!("Could not open file {:?}: {}", filename, why);
        }
    };
    match f.read_to_string(&mut buffer) {
        Ok(x) => x,
        Err(why) => {
            panic!("could not read contents of file: {}", why);
        }
    };
    buffer
}
