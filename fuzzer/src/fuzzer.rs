use std::collections::HashMap;
use std::process::Command;
use std::str;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

use colored::*;

use rand::Rng;

use tera::{Tera, Context};
use tempfile::tempdir;

use dustbox::cpu::{AMode, CPU, Encoder, Instruction, Op,  Parameter, R, Segment, instructions_to_str, r16};

use dustbox::machine::Machine;
use dustbox::ndisasm::ndisasm_bytes;

/// Runs program code in a specific environment
pub enum CodeRunner {
    /// uses https://github.com/martinlindhe/supersafe to connect to client over HTTP.
    /// Currently the fastest at about ~0.2 s
    SuperSafe,

    /// uses VMware `vmrun` command
    /// ~2.3 seconds
    Vmrun,

    /// uses `dosbox-x` command
    /// ~2.2 s
    DosboxX,
}

const DEBUG_ENCODER: bool = false;

pub struct FuzzConfig {
    /// general config
    pub mutations_per_op: usize,

    /// supersafe
    pub remote_host: String,

    /// vmrun
    pub vmx_path: String,

    /// username in the VM
    pub username: String,
    pub password: String,
}

impl FuzzConfig {
    fn counter_width(&self) -> usize {
        match self.mutations_per_op {
            0   ..=   9 => 1,
            10  ..=  99 => 2,
            100 ..= 999 => 3,
            1000..=9999 => 4,
            _           => 5,
        }
    }
}

pub fn fuzz_ops<RNG: Rng + ?Sized>(runner: &CodeRunner, ops_to_fuzz: Vec<Op>, cfg: &FuzzConfig, rng: &mut RNG) {
    for op in ops_to_fuzz {
        println!("fuzzing {} forms of {:?} ...", cfg.mutations_per_op, op);
        let mut failures = 0;
        let mut sum_duration = Duration::new(0, 0);
        for i in 0..cfg.mutations_per_op {
            let start = Instant::now();
            let snippet = get_mutator_snippet(&op, rng);
            let mut ops = prober_setupcode();
            ops.extend(snippet.to_vec());

            let encoder = Encoder::new();
            let data = match encoder.encode_vec(&ops) {
                Ok(data) => data,
                Err(why) => panic!("{}", why),
            };

            print!("MUT {:width$}/{} {:02X?}", i + 1, cfg.mutations_per_op, data, width = cfg.counter_width());

            if DEBUG_ENCODER {
                println!("{}", ndisasm_bytes(&data).unwrap().join("\n"));
            }
        
            if !fuzz(&runner, &data, ops.len(), AffectedFlags::for_op(&op), &cfg) {
                println!("failed:");
                println!("{}", instructions_to_str(&snippet));
                println!("------");
                failures += 1;
            }
            let elapsed = start.elapsed();
            sum_duration = sum_duration.checked_add(elapsed).unwrap();
            println!(" in {:.2} s", elapsed.as_secs_f64());
        }
        if failures > 0 {
            let successes = cfg.mutations_per_op - failures;
            println!("{}/{} successes", successes, cfg.mutations_per_op)
        }
        let secs = sum_duration.as_secs_f64();
        println!("done in {:.2} s. average {:.2} s", secs, secs / (cfg.mutations_per_op as f64));
        println!("-");
    }
}

/// Runs given binary data in dustbox and in a CodeRunner, comparing the resulting regs and flags
/// returns false on failure
fn fuzz(runner: &CodeRunner, data: &[u8], op_count: usize, affected_flag_mask: u16, cfg: &FuzzConfig) -> bool {
    let affected_registers = vec!("ax", "dx");
    let mut machine = Machine::deterministic();

    machine.load_executable(data, 0x085F);
    machine.execute_instructions(op_count);

    let prober_com = Path::new("utils/prober/prober.com");
    assemble_prober(data, prober_com);

    let output = match *runner {
        CodeRunner::SuperSafe => stdout_from_supersafe(prober_com, &cfg.remote_host),
        CodeRunner::Vmrun => stdout_from_vmrun(prober_com, &cfg.vmx_path, &cfg.username, &cfg.password),
        CodeRunner::DosboxX => stdout_from_dosbox(prober_com),
    };

    let vm_regs = prober_reg_map(&output);
    if vm_regs.is_empty() {
        println!("FATAL: no vm regs from vm output: {}", output);
        return false;
    }

    if compare_regs(&machine.cpu, &vm_regs, &affected_registers) {
        println!("\n{}", "MAJOR: regs differ".red());
        return false;
    }

    let vm_flags = vm_regs["flag"];
    let vm_masked_flags = vm_flags & affected_flag_mask;
    let dustbox_flags = machine.cpu.regs.flags.u16();
    let dustbox_masked_flags = dustbox_flags & affected_flag_mask;
    if vm_masked_flags != dustbox_masked_flags {
        let xored = vm_masked_flags ^ dustbox_masked_flags;
        print!("\nflag diff: vm {:04x} {:8} vs dustbox {:04x} {:8} = diff {:8}\n",
            vm_masked_flags, bitflags_str(vm_masked_flags).green(), dustbox_masked_flags, bitflags_str(dustbox_masked_flags).red(), bitflags_str(xored).red());
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
    // ____ O_I_ SZ_A _P_C
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
            Op::Nop | Op::Mov8 | Op::Mov16 | Op::Mov32 | Op::Not8 | Op::Not16 |
            Op::Div8 | Op::Div16 | Op::Idiv8 | Op::Idiv16 | Op::Xchg8 | Op::Xchg16 |
            Op::Salc | Op::Cbw | Op::Cwd16 | Op::Lahf | Op::Lea16 | Op::Xlatb =>
                AffectedFlags{s:0, z:0, p:0, c:0, a:0, o:0, d:0, i:0}.mask(), // none

            Op::Sahf =>
                AffectedFlags{o:1, s:1, z:1, a:1, p:1, c:1, d:1, i:1}.mask(), // all

            Op::Bt | Op::Clc | Op::Cmc | Op::Stc =>
                AffectedFlags{c:1, a:0, o:0, s:0, z:0, p:0, d:0, i:0}.mask(), // C

            Op::Cld | Op::Std =>
                AffectedFlags{d:1, c:0, a:0, o:0, s:0, z:0, p:0, i:0}.mask(), // D

            Op::Cli | Op::Sti =>
                AffectedFlags{i:1, d:0, c:0, a:0, o:0, s:0, z:0, p:0}.mask(), // I

            Op::Bsf =>
                AffectedFlags{s:0, z:1, p:0, c:0, a:0, o:0, d:0, i:0}.mask(), // Z

            Op::Aaa | Op::Aas =>
                AffectedFlags{c:1, a:1, o:0, s:0, z:0, p:0, d:0, i:0}.mask(), // C A

            Op::Rol8 | Op::Rcl8 | Op::Ror8 | Op::Rcr8 | Op::Mul8 | Op::Mul16 | Op::Imul8 | Op::Imul16 =>
                AffectedFlags{c:1, o:1, z:0, s:0, p:0, a:0, d:0, i:0}.mask(), // C O

            Op::Add8 | Op::Add16 | Op::Adc8 | Op::Adc16 |
            Op::Sub8 | Op::Sub16 | Op::Sbb8 | Op::Sbb16 |
            Op::Cmp8 | Op::Cmp16 | Op::Neg8 | Op::Neg16 | Op::Shrd | Op::Cmpsw =>
                AffectedFlags{c:1, s:1, z:1, a:1, p:1, o:1, d:0, i:0}.mask(), // C A S Z P O

            Op::Aad | Op::Aam | Op::Xor8 | Op::Xor16 | Op::Test8 | Op::Test16 |
            Op::And8 | Op::And16 | Op::Or8 | Op::Or16 |
            Op::Shl8 | Op::Shl16 | Op::Shr8 | Op::Shr16 | Op::Sar8 =>
                AffectedFlags{c:1, o:1, s:1, z:1, a:0, p:1, d:0, i:0}.mask(), // C O S Z P

            Op::Daa | Op::Das =>
                AffectedFlags{c:1, s:1, z:1, a:1, p:1, o:0, d:0, i:0}.mask(), // C A S Z P

            Op::Inc8 | Op::Inc16 | Op::Inc32 | Op::Dec8 | Op::Dec16 | Op::Dec32 | Op::Shld =>
                AffectedFlags{s:1, z:1, a:1, p:1, o:1, c:0, d:0, i:0}.mask(), // S Z P O A

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
fn stdout_from_supersafe(path: &Path, remote_ip: &str) -> String {
    use curl::easy::{Easy, Form};
    let mut dst = Vec::new();
    let mut easy = Easy::new();
    let timeout = Duration::from_millis(1000);
    easy.timeout(timeout).unwrap();
    easy.url(&format!("http://{}:28111/run", remote_ip)).unwrap();

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

/// run .com with vmrun (vmware) in given vmx, parse result
fn stdout_from_vmrun(path: &Path, vmx_path: &str, username: &str, password: &str) -> String {
    let vmrun_path = if cfg!(windows) {
        "C:\\Program Files (x86)\\VMware\\VMware Workstation\\vmrun.exe"
    } else {
        "vmrun"
    };

    // copy file to guest
    Command::new(vmrun_path)
        .args(&["-T", "ws", "-gu", username, "-gp", password,
            "copyFileFromHostToGuest", vmx_path, path.to_str().unwrap(), "C:\\prober.com"])
        .output()
        .expect("failed to execute process");

    // XXX create C:\prober.bat in vm

    // run prober.bat, where prober.bat is "c:\prober.com > c:\prober.out"
    Command::new(vmrun_path)
        .args(&["-T", "ws", "-gu", username, "-gp", password,
            "runProgramInGuest", vmx_path, "C:\\prober.bat"])
        .output()
        .expect("failed to execute process");

    let tmp_dir = tempdir().unwrap();
    let file_path = tmp_dir.path().join("prober.out");
    let file_str = file_path.to_str().unwrap();

    // copy back result
    Command::new(vmrun_path)
        .args(&["-T", "ws", "-gu", username, "-gp", password,
            "copyFileFromGuestToHost", vmx_path, "C:\\prober.out", file_str])
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

// returns the setup code (clear registers and flags)
fn prober_setupcode() -> Vec<Instruction> {
    vec!(
        // clear ax,dx
        Instruction::new2(Op::Xor16, Parameter::Reg16(R::AX), Parameter::Reg16(R::AX)),
        Instruction::new2(Op::Xor16, Parameter::Reg16(R::DX), Parameter::Reg16(R::DX)),

        // clear flags
        Instruction::new1(Op::Push16, Parameter::Imm16(0)),
        Instruction::new(Op::Popf),
    )
}

// returns a snippet used to mutate state for op
fn get_mutator_snippet<RNG: Rng + ?Sized>(op: &Op, rng: &mut RNG) -> Vec<Instruction> {
    match *op {
        Op::Mov8 => { vec!(
            Instruction::new2(op.clone(), Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
        )}
        Op::Mov16 => { vec!(
            Instruction::new2(op.clone(), Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
        )}
        Op::Cmpsw => { vec!(
            // compare word at address DS:(E)SI with byte at address ES:(E)DI;
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::SI), Parameter::Imm16(0x3030)),
            Instruction::new2(Op::Mov16, Parameter::Ptr16Amode(Segment::Default, AMode::SI), Parameter::Imm16(rng.gen())),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::DI), Parameter::Imm16(0x3040)),
            Instruction::new2(Op::Mov16, Parameter::Ptr16Amode(Segment::Default, AMode::DI), Parameter::Imm16(rng.gen())),
            Instruction::new(op.clone()),
        )}
        Op::Shld | Op::Shrd => { vec!(
            // mutate ax, dx, imm8
            // shld ax, dx, imm8
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::DX), Parameter::Imm16(rng.gen())),
            Instruction::new3(op.clone(), Parameter::Reg16(R::AX), Parameter::Reg16(R::DX), Parameter::Imm8(rng.gen())),
        )}
        Op::Shl8 | Op::Shr8 | Op::Sar8 | Op::Rol8 | Op::Ror8 | Op::Rcl8 | Op::Rcr8 |
        Op::Cmp8 | Op::And8 | Op::Xor8 | Op::Or8 | Op::Add8 | Op::Adc8 | Op::Sub8 | Op::Sbb8 | Op::Test8 => { vec!(
            // test r/m8, imm8
            Instruction::new1(Op::Push16, Parameter::Imm16(rng.gen())),
            Instruction::new(Op::Popf),
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
            Instruction::new2(op.clone(), Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
        )}
        Op::Shl16 | Op::Shr16 => { vec!(
            Instruction::new1(Op::Push16, Parameter::Imm16(rng.gen())),
            Instruction::new(Op::Popf),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new2(op.clone(), Parameter::Reg16(R::AX), Parameter::Imm8(rng.gen())),
        )}
        Op::Bt | Op::Bsf | Op::Xchg16 => { vec!(
            // bsf r16, r/m16
            // bt r/m16, r16
            // xchg r/m16, r16
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::BX), Parameter::Imm16(rng.gen())),
            Instruction::new2(op.clone(), Parameter::Reg16(R::AX), Parameter::Reg16(R::BX)),
        )}
        Op::Mul8 | Op::Imul8 => { vec!(
            // mul r/m8      ax = al * r/m
            // imul r/m8     ax = al * r/m
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::DL), Parameter::Imm8(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg8(R::DL)),
        )}
        Op::Div8 | Op::Idiv8 => { vec!(
            // divide AX by r/m8, store in AL, AH
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::DL), Parameter::Imm8(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg8(R::DL)),
        )}
        Op::Div16 | Op::Idiv16 => { vec!(
            // div r/m16        divide DX:AX by r/m16, with result stored in AX ← Quotient, DX ← Remainde
            // idiv r/m16       Signed divide DX:AX by r/m16, with result stored in AX ← Quotient, DX ← Remainder.
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::DX), Parameter::Imm16(rng.gen())),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::BX), Parameter::Imm16(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg16(R::BX)),
        )}
        Op::Mul16 => { vec!(
            // mul r/m16        DX:AX ← AX ∗ r/m16
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::BX), Parameter::Imm16(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg16(R::BX)),
        )}
        Op::Imul16 => { vec!(
            // imul r/m16        DX:AX = AX ∗ r/m16
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::BX), Parameter::Imm16(rng.gen())),

            // Instruction::new1(op.clone(), Parameter::Reg16(R::BX)), // 1-operand form
            // Instruction::new2(op.clone(), Parameter::Reg16(R::AX), Parameter::Reg16(R::BX)), // 2-operand form
            Instruction::new3(op.clone(), Parameter::Reg16(R::AX), Parameter::Reg16(R::BX), Parameter::ImmS8(rng.gen())), // 3-operand form
        )}
        Op::Xchg8 => { vec!(
            // xchg r/m8, r8
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::DL), Parameter::Imm8(rng.gen())),
            Instruction::new2(op.clone(), Parameter::Reg8(R::DL), Parameter::Reg8(R::BL)),
        )}
        Op::Lahf | Op::Salc | Op::Clc | Op::Cld | Op::Cli | Op::Cmc | Op::Stc | Op::Std | Op::Sti => { vec!(
            // mutate flags
            Instruction::new1(Op::Push16, Parameter::Imm16(rng.gen())),
            Instruction::new(Op::Popf),
            Instruction::new(op.clone()),
        )}
        Op::Aas | Op::Aaa | Op::Daa | Op::Das | Op::Cbw => { vec!(
            // mutate al: no args
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
            Instruction::new(op.clone()),
        )}
        Op::Not8 | Op::Neg8 | Op::Inc8 | Op::Dec8 => { vec!(
            // mutate al: r/m8
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg8(R::AL)),
        )}
        Op::Sahf => { vec!(
            // mutate ah: no args
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AH), Parameter::Imm8(rng.gen())),
            Instruction::new(op.clone()),
        )}
        Op::Cwd16 => { vec!(
            // mutate ax: no args
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new(op.clone()),
        )}
        Op::Add16 | Op::Adc16 | Op::And16 | Op::Cmp16 | Op::Sub16 | Op::Or16 | Op::Sbb16 | Op::Test16 | Op::Xor16 => { vec!(
            // TEST AX, imm16
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new2(op.clone(), Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
        )}
        Op::Inc16 | Op::Dec16 | Op::Not16 | Op::Neg16 => { vec!(
            // mutate ax: r/m16
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg16(R::AX)),
        )}
        Op::Aad | Op::Aam => { vec!(
            // mutate ax: imm8
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Imm8(rng.gen())),
        )}
        Op::Lea16 => { vec!(
            // lea r16, m
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::BX), Parameter::Imm16(rng.gen())),
            Instruction::new2(op.clone(), Parameter::Reg16(R::AX), Parameter::Ptr16Amode(Segment::Default, AMode::BX)),
        )}
        Op::Inc32 | Op::Dec32 => { vec!(
            // mutate eax: r/m16
            Instruction::new2(Op::Mov32, Parameter::Reg32(R::EAX), Parameter::Imm32(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg32(R::EAX)),
        )}
        Op::Nop => vec!(Instruction::new(op.clone())),
        _ => panic!("get_mutator_snippet: unhandled op {:?}", op),
    }
}
