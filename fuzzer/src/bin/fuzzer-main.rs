#[macro_use]
extern crate clap;
use clap::{Arg, App};

use rand::prelude::*;
use rand_xorshift::XorShiftRng;

use dustbox::cpu::Op;
use fuzzer::fuzzer::{fuzz_ops, FuzzConfig, CodeRunner};

fn main() {
    let matches = App::new("dustbox-fuzzer")
        .version("0.1")
        .arg(Arg::with_name("RUNNER")
            .help("Code runner to use (supersafe, vmrun, dosbox-x)")
            .required(true)
            .index(1)
            .long("runner"))
        .arg(Arg::with_name("MUTATIONS")
            .help("Number of mutations per instruction")
            .takes_value(true)
            .long("mutations"))
        .arg(Arg::with_name("HOST")
            .help("Remote host (supersafe)")
            .takes_value(true)
            .long("host"))
        .arg(Arg::with_name("USERNAME")
            .help("VM username (vmrun)")
            .takes_value(true)
            .long("username"))
        .arg(Arg::with_name("PASSWORD")
            .help("VM password (vmrun)")
            .takes_value(true)
            .long("password"))
        .arg(Arg::with_name("SEED")
            .help("Specify PRNG seed for reproducibility")
            .takes_value(true)
            .long("seed"))
        .arg(Arg::with_name("VMX")
            .help("Specify VMX image (vmrun)")
            .takes_value(true)
            .long("vmx"))
            .get_matches();

    let ops_to_fuzz = vec!(
        Op::Rol32, // Op::Rcl32,  // XXX not implemented in dustbox
        //Op::Ror32, // XXX carry flag diff vs WinXP
        //Op::Shl32, // XXX carry & overflow differs

        //Op::Ror16, Op::Rol16,  // XXX carry flag diff vs WinXP
        //Op::Rcr32,  // XXX overflow flag diff vs WinXP

        //Op::Div32,  // XXX MAJOR REG DIFF

        // Op::Loop, // XXX need to keep relative offsets in decoder in order to encode back

        // TODO - EMULATION NOT IMPLEMENTED:
        //Op::Adc32, Op::And32, Op::Or32, Op::Sbb32, Op::Test32, Op::Not32

        // TODO - ENCODING NOT IMPLEMENTED:
        //Op::Test32, Op::Cmpsw,

        // TODO FUZZ:
        // movsb/w, stosb/w

        // Op::Shld, Op::Shrd,      // ERROR - regs differ vs dosbox, regs match vs winxp! - overflow flag is wrong in both:
        // Op::Shl16, Op::Rcl16,    // ERROR - overflow flag diff vs both dosbox & winxp. algo from bochs
        // Op::Shr16, Op::Shr32,    // ERROR? - identical to winxp, but overflow flag differs vs dosbox

        // Op::Sar32, // reg diff if shift == 1 in WinXP

        /*
        // UNSURE: overflow is identical to bochs and dosbox, but differs in WinXP vm:
        Op::Rcl8, Op::Rcr8, Op::Rcr16, Op::Ror8, Op::Shl8, Op::Rol8,

        // SEEMS ALL OK:
        Op::Movsx16, Op::Movsx32, Op::Movzx16, Op::Movzx32,
        Op::Shr8, Op::Sar8, Op::Sar16, // OK !
        //Op::Div8, Op::Div16, Op::Idiv8, Op::Idiv16, Op::Idiv32, // seems correct. NOTE that winxp crashes with "Divide overflow" on some input
        Op::Bt, Op::Bsf,
        Op::Aaa, Op::Aad, Op::Aam, Op::Aas, Op::Daa, Op::Das,

        Op::Push16, // NOTE: also tests Op::Pop16
        Op::Mov8, Op::Mov16, Op::Mov32,
        Op::Cmp8, Op::Cmp16, Op::Cmp32,
        Op::And8, Op::And16,
        Op::Xor8, Op::Xor16, Op::Xor32,
        Op::Or8, Op::Or16,
        Op::Add8, Op::Add16, Op::Add32, Op::Adc8, Op::Adc16,
        Op::Sub8, Op::Sub16, Op::Sub32, Op::Sbb8, Op::Sbb16,
        Op::Test8, Op::Test16,
        Op::Not8, Op::Not16,
        Op::Neg8, Op::Neg16, Op::Neg32,
        Op::Xchg8, Op::Xchg16,
        Op::Mul8, Op::Mul16, Op::Mul32, Op::Imul8, Op::Imul16, Op::Imul32,
        Op::Lahf, Op::Sahf, Op::Salc,
        Op::Nop, Op::Lea16,
        Op::Clc, Op::Cld, Op::Cli, Op::Cmc, Op::Stc, Op::Std, Op::Sti,
        Op::Cbw, Op::Cwd16,
        Op::Inc8, Op::Inc16, Op::Inc32,
        Op::Dec8, Op::Dec16, Op::Dec32,
        */
    );

    let cfg = FuzzConfig{
        mutations_per_op: value_t!(matches, "MUTATIONS", usize).unwrap_or(50),
        remote_host: matches.value_of("HOST").unwrap_or("127.0.0.1").to_string(),
        vmx_path: matches.value_of("VMX").unwrap_or("").to_string(),

        username: matches.value_of("USERNAME").unwrap_or("vmware").to_string(),
        password: matches.value_of("PASSWORD").unwrap_or("vmware").to_string(),
    };

    let runner = match matches.value_of("RUNNER").unwrap() {
        "supersafe" => CodeRunner::SuperSafe,
        "dosbox-x"  => CodeRunner::DosboxX,
        "vmrun"     => CodeRunner::Vmrun,
        _ => panic!("unrecognized runner"),
    };

    // seed prng if argument was given
    let mut rng: XorShiftRng;
    let seed_value = if matches.is_present("SEED") {
        value_t!(matches, "SEED", u64).unwrap()
    } else {
        XorShiftRng::from_entropy().gen()
    };

    rng = XorShiftRng::seed_from_u64(seed_value);
    println!("rng seed = {}", seed_value);

    fuzz_ops(&runner, ops_to_fuzz, &cfg, &mut rng);
}
