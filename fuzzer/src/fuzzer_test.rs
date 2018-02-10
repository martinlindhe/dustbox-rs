use std::io::{self, Read, Write};

use dustbox::cpu::instruction::Instruction;
use dustbox::cpu::op::Op;
use dustbox::cpu::parameter::Parameter;
use dustbox::cpu::register::{R8, R16};

use fuzzer::{fuzz, AffectedFlags};

#[test] #[ignore] // expensive test
fn fuzz_instruction() {
    let affected_registers = vec!("ax");
    // verified register & flag operation with winXP:
    // Shr8, Rol8,
    // Cmp8, And8, Xor8, Or8, Add8, Adc8, Sub8, Sbb8
    // Test8, Not8, Mul8, Imul8
    // Aas, Aaa

    // XXX differs from winXP: Shl8 (OF), Sar8 (wrong result some times)
    //          - Ror8 (CF)
    // Neg8: mov ah,0; not ah =   overflow flag differs vs winxp
    // Idiv8: hard to fuzz due to input that triggers DIV0 exception

    for i in 0..65535 as usize {
        let op = Op::Div8;
        let affected_flags_mask = AffectedFlags::for_op(op.clone());

        let mut n1 = ((i + 1) & 0xFFFF) ^ 0xAAAA;
        let mut n2 = i & 0xFF;

        if op == Op::Div8 || op == Op::Idiv8 {
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
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::AX), Parameter::Imm16(n1 as u16)),
            Instruction::new2(Op::Mov8, Parameter::Reg8(R8::BL), Parameter::Imm8(n2 as u8)),
            Instruction::new1(op, Parameter::Reg8(R8::BL)), //  ax = ax / bl
        );

        print!("{:02x}, {:02x} ", n1, n2);
        io::stdout().flush().ok().expect("Could not flush stdout");
        fuzz(i, &ops, &affected_registers, affected_flags_mask);
    }
}
