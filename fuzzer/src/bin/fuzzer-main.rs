extern crate rand;
extern crate rand_xorshift;

#[macro_use]
extern crate clap;
use clap::{Arg, App};

extern crate dustbox;
extern crate dustbox_fuzzer;

use rand::prelude::*;
use rand_xorshift::XorShiftRng;

use dustbox::cpu::{Instruction, Op, Parameter, Segment, R, AMode, Encoder, instructions_to_str};
use dustbox_fuzzer::fuzzer::{fuzz, FuzzConfig, CodeRunner, AffectedFlags};

fn main() {
    let matches = App::new("dustbox-fuzzer")
        .version("0.1")
        .arg(Arg::with_name("mutations")
            .help("Number of mutations per instruction")
            .takes_value(true)
            .long("mutations"))
        .arg(Arg::with_name("ip")
            .help("Remote IP for supersafe client")
            .takes_value(true)
            .long("ip"))
        .get_matches();

    let affected_registers = vec!("ax", "dx");

    let ops_to_fuzz = vec!(
        Op::Cmp8,

        // ENCODING NOT IMPLEMENTED:
        //Op::Cmpsw,

        // ERROR - regs differ vs dosbox, regs match vs winxp! - overflow flag is wrong in both:
        // Op::Shld, Op::Shrd,

        /*
        // UNSURE: overflow is identical to bochs and dosbox, but differs in WinXP vm:
        Op::Rcl8, Op::Rcr8, Op::Ror8, Op::Shl8, Op::Rol8,

        // SEEMS ALL OK:
        Op::Shr8, Op::Sar8, // OK !
        Op::Div8, Op::Div16, Op::Idiv8, Op::Idiv16, // seems correct. NOTE that winxp crashes with "Divide overflow" on some input
        Op::Bt, Op::Bsf,
        Op::Aaa, Op::Aad, Op::Aam, Op::Aas, Op::Daa, Op::Das,
        
        Op::Cmp8, Op::Cmp16,
        Op::And8, Op::And16,
        Op::Xor8, Op::Xor16,
        Op::Or8, Op::Or16,
        Op::Add8, Op::Add16, Op::Adc8, Op::Adc16,
        Op::Sub8, Op::Sub16, Op::Sbb8, Op::Sbb16,
        Op::Test8, Op::Test16,
        Op::Not8, Op::Not16,
        Op::Neg8, Op::Neg16,
        Op::Xchg8, Op::Xchg16,
        Op::Mul8, Op::Mul16, Op::Imul8, Op::Imul16,
        Op::Lahf, Op::Sahf, Op::Salc,
        Op::Nop,
        Op::Clc, Op::Cld, Op::Cli, Op::Cmc, Op::Stc, Op::Std, Op::Sti,
        Op::Cbw, Op::Cwd16,
        Op::Lea16,
        Op::Inc8, Op::Inc16, Op::Inc32,
        Op::Dec8, Op::Dec16, Op::Dec32,
        */
    );

    let cfg = FuzzConfig{
        mutations_per_op: value_t!(matches, "mutations", usize).unwrap_or(50),
        remote_ip: matches.value_of("ip").unwrap_or("127.0.0.1").to_string(),
    };

    let mut rng = XorShiftRng::from_entropy();

    let runner = CodeRunner::SuperSafe;
    //let runner = CodeRunner::DosboxX;

    for op in ops_to_fuzz {
        println!("fuzzing {} forms of {:?} ...", cfg.mutations_per_op, op);
        let mut failures = 0;
        for _ in 0..cfg.mutations_per_op {
            let affected_flags_mask = AffectedFlags::for_op(&op);

            let snippet = get_mutator_snippet(&op, &mut rng);
            let mut ops = prober_setupcode();
            ops.extend(snippet.to_vec());

            let encoder = Encoder::new();
            let data = match encoder.encode_vec(&ops) {
                Ok(data) => data,
                Err(why) => panic!("{}", why),
            };

            if !fuzz(&runner, &data, ops.len(), &affected_registers, affected_flags_mask, &cfg) {
                println!("failed:");
                println!("{}", instructions_to_str(&snippet));
                println!("------");
                failures += 1;
            }
        }
        if failures > 0 {
            let successes = cfg.mutations_per_op - failures;
            println!("{}/{} successes", successes, cfg.mutations_per_op)
        }
        println!("-");
    }
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
fn get_mutator_snippet(op: &Op, rng: &mut XorShiftRng) -> Vec<Instruction> {
    match *op {
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
        Op::Bt | Op::Bsf | Op::Xchg16 => {vec!(
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
