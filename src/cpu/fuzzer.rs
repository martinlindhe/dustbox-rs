use std::collections::HashMap;
use std::process::Command;
use std::str;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::fs::File;
use std::io::{self, Read, Write};

use tera::Context;
use tempdir::TempDir;

use cpu::CPU;
use cpu::register::{R8, R16};
use cpu::instruction::Instruction;
use cpu::parameter::Parameter;
use cpu::encoder::Encoder;
use cpu::op::Op;

#[cfg(test)]
#[path = "./fuzzer_test.rs"]
mod fuzzer_test;

struct AffectedFlags {
    // ____ O___ SZ_A _P_C
    pub c: u8, // 0: carry flag
    pub p: u8, // 2: parity flag
    pub a: u8, // 4: auxiliary carry flag (AF)
    pub z: u8, // 6: zero flag
    pub s: u8, // 7: sign flag
    pub o: u8, // 11: overflow flag
}

impl AffectedFlags {
    pub fn mask(&self) -> u16 {
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

fn assemble_prober(ops: &[Instruction], prober_com: &str) {
    let mut tera = compile_templates!("utils/prober/*.tpl.asm");

    // disable autoescaping
    tera.autoescape_on(vec![]);

    let mut context = Context::new();
    context.add("snippet", &ops_as_db_bytes(ops));
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

// parse prober.com output into a map
fn prober_reg_map(stdout: &str) -> HashMap<String, u16> {
    let mut map = HashMap::new();
    let lines: Vec<String> = stdout.split('\n').map(|s| s.to_string()).collect();

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

fn stdout_from_dosbox(prober_com: &str) -> String {

    // copy prober_com to ~/dosbox-x
    use std::fs;
    fs::copy(prober_com, "/Users/m/dosbox-x/prober.com").unwrap();

    Command::new("dosbox-x")
        .args(&["-noautoexec", "-c", "prober.com > prober.out", "--exit"])
        .current_dir("/Users/m/dosbox-x")
        .output()
        .expect("failed to execute process");

    let cwd = Path::new("/Users/m/dosbox-x");
    let file_path = cwd.join("prober.out");

    read_text_file(&file_path)
}

// run .com with vmrun (vmware), parse result
fn stdout_from_vmx_vmrun(prober_com: &str) -> String {
    let vmx = "/Users/m/Documents/Virtual Machines.localized/Windows XP Professional.vmwarevm/Windows XP Professional.vmx";
    let vm_user = "vmware";
    let vm_password = "vmware";
    //let now = Instant::now();

    // copy file to guest
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "copyFileFromHostToGuest", vmx, prober_com, "C:\\prober.com"])
        .output()
        .expect("failed to execute process");

    //let elapsed = now.elapsed();
    //let upload_sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    //let now = Instant::now();

    // run prober.bat, where prober.bat is "c:\prober.com > c:\prober.out" (XXX create this file in vm once)
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "runProgramInGuest", vmx, "C:\\prober.bat"])
        .output()
        .expect("failed to execute process");

    //let elapsed = now.elapsed();
    //let run_sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);

    let tmp_dir = TempDir::new("vmware").unwrap();
    let file_path = tmp_dir.path().join("prober.out");
    let file_str = file_path.to_str().unwrap();

    //let now = Instant::now();
    // copy back result
    Command::new("vmrun")
        .args(&["-T", "ws", "-gu", vm_user, "-gp", vm_password,
            "copyFileFromGuestToHost", vmx, "C:\\prober.out", file_str])
        .output()
        .expect("failed to execute process");

    //let elapsed = now.elapsed();
    //let download_sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    //println!("vmrun: upload {}s, run {}s, download {}s", upload_sec, run_sec, download_sec);

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

/// disasm the encoded instruction with external ndisasm command
pub fn ndisasm(op: &Instruction) -> Result<String, io::Error> {
    let tmp_dir = TempDir::new("ndisasm")?;
    let file_path = tmp_dir.path().join("binary.bin");
    let file_str = file_path.to_str().unwrap();
    let mut tmp_file = File::create(&file_path)?;

    let encoder = Encoder::new();
    if let Ok(data) = encoder.encode(op) {
        tmp_file.write_all(&data)?;
    } else {
        panic!("invalid byte sequence");
    }

    let output = Command::new("ndisasm")
        .args(&["-b", "16", file_str])
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
