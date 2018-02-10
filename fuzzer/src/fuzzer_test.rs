use std::io::{self, Write};

use dustbox::cpu::instruction::Instruction;
use dustbox::cpu::op::Op;
use dustbox::cpu::parameter::Parameter;
use dustbox::cpu::segment::Segment;
use dustbox::cpu::register::{R8, R16, AMode};

use fuzzer::{fuzz, VmRunner, AffectedFlags};

#[test] #[ignore] // expensive test
fn fuzz_instruction() {
    let affected_registers = vec!("ax", "bx");
    // verified register & flag operation with winXP:
    // Shl8, Shr8, Rol8, Ror8, Sar8, Rcl8, Rcr8
    // Cmp8, And8, Xor8, Or8, Add8, Adc8, Sub8, Sbb8
    // Test8, Not8, Mul8, Imul8, Xchg8
    // Nop
    // Aas, Aaa, Daa, Das
    // Clc, Cld, Cli, Cmc, Stc, Std, Sti, Lahf, Sahf, Salc
    // Cbw, Cwd
    // Lea16

    // differs from winXP:
    // Neg8: mov ah,0; not ah =   OVERFLOW flag differs vs winxp
    // Idiv8: hard to fuzz due to input that triggers DIV0 exception

    // SPECIAL NOTES:
    // Rol8, Ror8, Rcl8, Rcr8 - OVERFLOW flag differ from winxp
    // XXX: Aam - P Z S flags differ from winxp & dosbox-x
    // XXX: Rcl8 register values dont match with dosbox-x, but with bochs & winxp
    // dustbox tries to be consistent with dosbox-x where behavior differs

    for i in 0..65535 as usize {
        let op = Op::Xlatb;
        let runner = VmRunner::VmHttp;
        let affected_flags_mask = AffectedFlags::for_op(&op);

        let mut n1 = ((i + 1) & 0xFFFF) ^ 0xAAAA;
        let mut n2 = i & 0xFF;

        if op == Op::Div8 || op == Op::Idiv8 || op == Op::Aam {
            // avoid divide by 0 because it crashes the app run inside the vm
            if n1 == 0 {
                n1 += 1
            }
            if n2 == 0 {
                n2 += 1
            }
        }
        /*
        if op == Op::Idiv8 {
            let quo = ((n1 as i16) / n2 as i16) as i16;
            let quo8s = (quo & 0xFF) as i8;
            if quo != quo8s as i16 {
                println!("avoidin idiv crash: {}, {}", n1, n2);
                continue;
            } else {
                println!("OK idiv: {}, {}", n1, n2);
            }
        }
        */
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

            // <op> al, imm8
            //Instruction::new2(Op::Mov8, Parameter::Reg8(R8::AH), Parameter::Imm8(n1 as u8)),
            //Instruction::new2(Op::Mov8, Parameter::Reg8(R8::BH), Parameter::Imm8(n2 as u8)),
            //Instruction::new2(op, Parameter::Reg8(R8::AH), Parameter::Reg8(R8::BH)),

            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::BX), Parameter::Imm16(n1 as u16)),
            Instruction::new2(op, Parameter::Reg16(R16::DI), Parameter::Ptr16Amode(Segment::Default, AMode::BX)),
        );

        print!("{:02x}, {:02x} ", n1, n2);
        io::stdout().flush().ok().expect("Could not flush stdout");
        fuzz(&runner, i, &ops, &affected_registers, affected_flags_mask);
    }
}
