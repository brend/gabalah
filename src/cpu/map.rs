use std::collections::HashMap;

use super::Instruction;
use super::ops::Mnemonic::*;
use super::ops::Location::*;

type I = Instruction;

/// Builds and returns a mapping of the 8-bit opcodes to instruction instances
pub fn build_opcode_map() -> HashMap<u8, Instruction> {
    let map: HashMap<u8, Instruction> = HashMap::from_iter([
        // no-op
        (0x00, I::new(Nop, 1, 4)),
        // load nn into BC
        (0x01, I::new(Ld(BC.imm(), Const16.imm()), 3, 12)),
        // load A into [BC]
        (0x02, I::new(Ld(BC.ind(), A.imm()), 1, 8)),
        // increase BC
        (0x03, I::new(Inc(BC.imm()), 1, 8)),
        // increase B
        (0x04, I::new(Inc(B.imm()), 1, 4)),
        // decrease B
        (0x05, I::new(Dec(B.imm()), 1, 4)),
        // load n into B
        (0x06, I::new(Ld(B.imm(), Const8.imm()), 2, 8)),
        // rotate A left; old bit 7 to Carry flag.
        (0x07, I::new(Rlca, 1, 4)),
        // load SP into [nn]
        (0x08, I::new(Ld(Const16.ind(), SP.imm()), 3, 20)),
        // add BC to HL
        (0x09, I::new(Add(HL.imm(), BC.imm()), 1, 8)),
        // load BC into A
        (0x0A, I::new(Ld(A.imm(), BC.ind()), 1, 8)),
        // decrease BC
        (0x0B, I::new(Dec(BC.imm()), 1, 8)),
        // increase C
        (0x0C, I::new(Inc(C.imm()), 1, 4)),
        // decrease C
        (0x0D, I::new(Dec(C.imm()), 1, 4)),
        // load n into C
        (0x0E, I::new(Ld(C.imm(), Const8.imm()), 2, 8)),
        // rotate A right; old bit 0 to Carry flag
        (0x0F, I::new(Rrca, 1, 4)),
        // stop
        (0x10, I::new(Stop(Const8.imm()), 2, 4)),
        // load nn into DE
        (0x11, I::new(Ld(DE.imm(), Const16.imm()), 3, 12)),
        // load A into [DE]
        (0x12, I::new(Ld(DE.ind(), A.imm()), 1, 8)),
        // increase DE
        (0x13, I::new(Inc(DE.imm()), 1, 8)),
        // increase D
        (0x14, I::new(Inc(D.imm()), 1, 4)),
        // decrease D
        (0x15, I::new(Dec(D.imm()), 1, 4)),
        // load n into D
        (0x16, I::new(Ld(D.imm(), Const8.imm()), 2, 6)),
        // rotate A left through Carry flag
        (0x17, I::new(Rla, 1, 4)),
        // jump relative by signed 8-bit offset
        (0x18, I::new(Jr(Const8.imm()), 2, 12)),
        // add DE to HL
        (0x19, I::new(Add(HL.imm(), DE.imm()), 1, 8)),
        // load [DE] into A
        (0x1A, I::new(Ld(A.imm(), DE.ind()), 1, 8)),
        // decrease DE
        (0x1B, I::new(Dec(DE.imm()), 1, 8)),
        // increase E
        (0x1C, I::new(Inc(E.imm()), 1, 4)),
        // decrease E
        (0x1D, I::new(Dec(E.imm()), 1, 4)),
        // load n into E
        (0x1E, I::new(Ld(E.imm(), Const8.imm()), 2, 8)),
        // rotate A right through Carry flag
        (0x1F, I::new(Rra, 1, 4)),
        // jump relative if nonzero
        (
            0x20,
            I::new_ex(Jrc(FlagNz.imm(), Const8.imm()), 2, vec![12, 8]),
        ),
        // load nn into HL
        (0x21, I::new(Ld(HL.imm(), Const16.imm()), 3, 12)),
        // load A into [HL]. Increment HL
        // 0x22
        // TODO: invent a way to implement this -- new type of operand maybe?

        // increase HL
        (0x23, I::new(Inc(HL.imm()), 1, 8)),
        // increase H
        (0x24, I::new(Inc(H.imm()), 1, 4)),
        // decrease H
        (0x25, I::new(Dec(H.imm()), 1, 4)),
        // load n into H
        (0x26, I::new(Ld(H.imm(), Const8.imm()), 2, 8)),
        // decimal adjust A
        (0x27, I::new(Daa, 1, 4)),
        // jump relative if zero
        (
            0x28,
            I::new_ex(Jrc(FlagZ.imm(), Const8.imm()), 2, vec![12, 8]),
        ),
        // add HL to HL
        (0x29, I::new(Add(HL.imm(), HL.imm()), 1, 8)),
        // load [HL] into A. Increment HL
        // 0x2A
        // TODO: invent a way to implement this
        // decrease HL
        (0x2B, I::new(Dec(HL.imm()), 1, 8)),
        // increase L
        (0x2C, I::new(Inc(L.imm()), 1, 4)),
        // decrease L
        (0x2D, I::new(Dec(L.imm()), 1, 4)),
        // load n into L
        (0x2E, I::new(Ld(L.imm(), Const8.imm()), 2, 8)),
        // complement A
        (0x2F, I::new(Cpl, 1, 4)),
        // jump relative if C flag is clear
        (
            0x30,
            I::new_ex(Jrc(FlagNc.imm(), Const8.imm()), 2, vec![12, 8]),
        ),
        // load nn into SP
        (0x31, I::new(Ld(SP.imm(), Const16.imm()), 3, 12)),
        // load A into [HL]. Decrement HL
        // 0x32
        // TODO: invent a way to implement this
        // increase SP
        (0x33, I::new(Inc(SP.imm()), 1, 8)),
        // increase (HL)
        (0x34, I::new(Inc(HL.ind()), 1, 12)),
        // decrease (HL)
        (0x35, I::new(Dec(HL.ind()), 1, 12)),
        // load n into (HL)
        (0x36, I::new(Ld(HL.ind(), Const8.imm()), 2, 12)),
        // set C flag
        (0x37, I::new(Scf, 1, 4)),
        // jump relative if C flag is set
        (
            0x38,
            I::new_ex(Jrc(FlagC.imm(), Const8.imm()), 2, vec![12, 8]),
        ),
        // add SP to HL
        (0x39, I::new(Add(HL.imm(), SP.imm()), 1, 8)),
        // load [HL] into A. Decrement HL
        // 0x3A
        // TODO: invent a way to implement this
        // decrease SP
        (0x3B, I::new(Dec(SP.imm()), 1, 8)),
        // increase A
        (0x3C, I::new(Inc(A.imm()), 1, 4)),
        // decrease A
        (0x3D, I::new(Dec(A.imm()), 1, 4)),
        // load n into A
        (0x3E, I::new(Ld(A.imm(), Const8.imm()), 2, 8)),
        // complement carry flag
        (0x3F, I::new(Ccf, 1, 4)),
        // load B into B
        (0x40, I::new(Ld(B.imm(), B.imm()), 1, 4)),
        // load C into B
        (0x41, I::new(Ld(B.imm(), C.imm()), 1, 4)),
        // load D into B
        (0x42, I::new(Ld(B.imm(), D.imm()), 1, 4)),
        // load E into B
        (0x43, I::new(Ld(B.imm(), E.imm()), 1, 4)),
        // load H into B
        (0x44, I::new(Ld(B.imm(), H.imm()), 1, 4)),
        // load L into B
        (0x45, I::new(Ld(B.imm(), L.imm()), 1, 4)),
        // load [HL] into B
        (0x46, I::new(Ld(B.imm(), HL.ind()), 1, 8)),
        // load A into B
        (0x47, I::new(Ld(B.imm(), A.imm()), 1, 4)),
        // load B into C
        (0x48, I::new(Ld(C.imm(), B.imm()), 1, 4)),
        // load C into C
        (0x49, I::new(Ld(C.imm(), C.imm()), 1, 4)),
        // load D into C
        (0x4A, I::new(Ld(C.imm(), D.imm()), 1, 4)),
        // load E into C
        (0x4B, I::new(Ld(C.imm(), E.imm()), 1, 4)),
        // load H into C
        (0x4C, I::new(Ld(C.imm(), H.imm()), 1, 4)),
        // load L into C
        (0x4D, I::new(Ld(C.imm(), L.imm()), 1, 4)),
        // load [HL] into C
        (0x4E, I::new(Ld(C.imm(), HL.ind()), 1, 8)),
        // load A into C
        (0x4F, I::new(Ld(C.imm(), A.imm()), 1, 4)),
        // load B into D
        (0x50, I::new(Ld(D.imm(), B.imm()), 1, 4)),
        // load C into D
        (0x51, I::new(Ld(D.imm(), C.imm()), 1, 4)),
        // load D into D
        (0x52, I::new(Ld(D.imm(), D.imm()), 1, 4)),
        // load E into D
        (0x53, I::new(Ld(D.imm(), E.imm()), 1, 4)),
        // load H into D
        (0x54, I::new(Ld(D.imm(), H.imm()), 1, 4)),
        // load L into D
        (0x55, I::new(Ld(D.imm(), L.imm()), 1, 4)),
        // load [HL] into D
        (0x56, I::new(Ld(D.imm(), HL.ind()), 1, 8)),
        // load A into D
        (0x57, I::new(Ld(D.imm(), A.imm()), 1, 4)),
        // load B into E
        (0x58, I::new(Ld(E.imm(), B.imm()), 1, 4)),
        // load C into E
        (0x59, I::new(Ld(E.imm(), C.imm()), 1, 4)),
        // load D into E
        (0x5A, I::new(Ld(E.imm(), D.imm()), 1, 4)),
        // load E into E
        (0x5B, I::new(Ld(E.imm(), E.imm()), 1, 4)),
        // load H into E
        (0x5C, I::new(Ld(E.imm(), H.imm()), 1, 4)),
        // load L into E
        (0x5D, I::new(Ld(E.imm(), L.imm()), 1, 4)),
        // load [HL] into E
        (0x5E, I::new(Ld(E.imm(), HL.ind()), 1, 8)),
        // load A into E
        (0x5F, I::new(Ld(E.imm(), A.imm()), 1, 4)),
        // load B into H
        (0x60, I::new(Ld(H.imm(), B.imm()), 1, 4)),
        // load C into H
        (0x61, I::new(Ld(H.imm(), C.imm()), 1, 4)),
        // load D into H
        (0x62, I::new(Ld(H.imm(), D.imm()), 1, 4)),
        // load E into H
        (0x63, I::new(Ld(H.imm(), E.imm()), 1, 4)),
        // load H into H
        (0x64, I::new(Ld(H.imm(), H.imm()), 1, 4)),
        // load L into H
        (0x65, I::new(Ld(H.imm(), L.imm()), 1, 4)),
        // load [HL] into H
        (0x66, I::new(Ld(H.imm(), HL.ind()), 1, 8)),
        // load A into H
        (0x67, I::new(Ld(H.imm(), A.imm()), 1, 4)),
        // load B into L
        (0x68, I::new(Ld(L.imm(), B.imm()), 1, 4)),
        // load C into L
        (0x69, I::new(Ld(L.imm(), C.imm()), 1, 4)),
        // load D into L
        (0x6A, I::new(Ld(L.imm(), D.imm()), 1, 4)),
        // load E into L
        (0x6B, I::new(Ld(L.imm(), E.imm()), 1, 4)),
        // load H into L
        (0x6C, I::new(Ld(L.imm(), H.imm()), 1, 4)),
        // load L into L
        (0x6D, I::new(Ld(L.imm(), L.imm()), 1, 4)),
        // load [HL] into L
        (0x6E, I::new(Ld(L.imm(), HL.ind()), 1, 8)),
        // load A into L
        (0x6F, I::new(Ld(L.imm(), A.imm()), 1, 4)),
        // load B into [HL]
        (0x70, I::new(Ld(HL.ind(), B.imm()), 1, 8)),
        // load C into [HL]
        (0x71, I::new(Ld(HL.ind(), C.imm()), 1, 8)),
        // load D into [HL]
        (0x72, I::new(Ld(HL.ind(), D.imm()), 1, 8)),
        // load E into [HL]
        (0x73, I::new(Ld(HL.ind(), E.imm()), 1, 8)),
        // load H into [HL]
        (0x74, I::new(Ld(HL.ind(), H.imm()), 1, 8)),
        // load L into [HL]
        (0x75, I::new(Ld(HL.ind(), L.imm()), 1, 8)),
        // halt
        (0x76, I::new(Halt, 1, 4)),
        // load A into [HL]
        (0x77, I::new(Ld(HL.ind(), A.imm()), 1, 8)),
        // load B into A
        (0x78, I::new(Ld(A.imm(), B.imm()), 1, 4)),
        // load C into A
        (0x79, I::new(Ld(A.imm(), C.imm()), 1, 4)),
        // load D into A
        (0x7A, I::new(Ld(A.imm(), D.imm()), 1, 4)),
        // load E into A
        (0x7B, I::new(Ld(A.imm(), E.imm()), 1, 4)),
        // load H into A
        (0x7C, I::new(Ld(A.imm(), H.imm()), 1, 4)),
        // load L into A
        (0x7D, I::new(Ld(A.imm(), L.imm()), 1, 4)),
        // load [HL] into A
        (0x7E, I::new(Ld(A.imm(), HL.ind()), 1, 8)),
        // load A into A
        (0x7F, I::new(Ld(A.imm(), A.imm()), 1, 4)),
        // add B to A
        (0x80, I::new(Add(A.imm(), B.imm()), 1, 4)),
        // add C to A
        (0x81, I::new(Add(A.imm(), C.imm()), 1, 4)),
        // add D to A
        (0x82, I::new(Add(A.imm(), D.imm()), 1, 4)),
        // add E to A
        (0x83, I::new(Add(A.imm(), E.imm()), 1, 4)),
        // add H to A
        (0x84, I::new(Add(A.imm(), H.imm()), 1, 4)),
        // add L to A
        (0x85, I::new(Add(A.imm(), L.imm()), 1, 4)),
        // add [HL] to A
        (0x86, I::new(Add(A.imm(), HL.ind()), 1, 8)),
        // add A to A
        (0x87, I::new(Add(A.imm(), A.imm()), 1, 4)),
        // add B to A with carry
        (0x88, I::new(Adc(A.imm(), B.imm()), 1, 4)),
        // add C to A with carry
        (0x89, I::new(Adc(A.imm(), C.imm()), 1, 4)),
        // add D to A with carry
        (0x8A, I::new(Adc(A.imm(), D.imm()), 1, 4)),
        // add E to A with carry
        (0x8B, I::new(Adc(A.imm(), E.imm()), 1, 4)),
        // add H to A with carry
        (0x8C, I::new(Adc(A.imm(), H.imm()), 1, 4)),
        // add L to A with carry
        (0x8D, I::new(Adc(A.imm(), L.imm()), 1, 4)),
        // add [HL] to A with carry
        (0x8E, I::new(Adc(A.imm(), HL.ind()), 1, 8)),
        // add A to A with carry
        (0x8F, I::new(Adc(A.imm(), A.imm()), 1, 4)),
        // subtract B from A
        (0x90, I::new(Sub(A.imm(), B.imm()), 1, 4)),
        // subtract C from A
        (0x91, I::new(Sub(A.imm(), C.imm()), 1, 4)),
        // subtract D from A
        (0x92, I::new(Sub(A.imm(), D.imm()), 1, 4)),
        // subtract E from A
        (0x93, I::new(Sub(A.imm(), E.imm()), 1, 4)),
        // subtract H from A
        (0x94, I::new(Sub(A.imm(), H.imm()), 1, 4)),
        // subtract L from A
        (0x95, I::new(Sub(A.imm(), L.imm()), 1, 4)),
        // subtract [HL] from A
        (0x96, I::new(Sub(A.imm(), HL.ind()), 1, 8)),
        // subtract A from A
        (0x97, I::new(Sub(A.imm(), A.imm()), 1, 4)),
        // subtract B from A with carry
        (0x98, I::new(Sbc(A.imm(), B.imm()), 1, 4)),
        // subtract C from A with carry
        (0x99, I::new(Sbc(A.imm(), C.imm()), 1, 4)),
        // subtract D from A with carry
        (0x9A, I::new(Sbc(A.imm(), D.imm()), 1, 4)),
        // subtract E from A with carry
        (0x9B, I::new(Sbc(A.imm(), E.imm()), 1, 4)),
        // subtract H from A with carry
        (0x9C, I::new(Sbc(A.imm(), H.imm()), 1, 4)),
        // subtract L from A with carry
        (0x9D, I::new(Sbc(A.imm(), L.imm()), 1, 4)),
        // subtract [HL] from A with carry
        (0x9E, I::new(Sbc(A.imm(), HL.ind()), 1, 8)),
        // subtract A from A with carry
        (0x9F, I::new(Sbc(A.imm(), A.imm()), 1, 4)),
        // and B with A
        (0xA0, I::new(And(A.imm(), B.imm()), 1, 4)),
        // and C with A
        (0xA1, I::new(And(A.imm(), C.imm()), 1, 4)),
        // and D with A
        (0xA2, I::new(And(A.imm(), D.imm()), 1, 4)),
        // and E with A
        (0xA3, I::new(And(A.imm(), E.imm()), 1, 4)),
        // and H with A
        (0xA4, I::new(And(A.imm(), H.imm()), 1, 4)),
        // and L with A
        (0xA5, I::new(And(A.imm(), L.imm()), 1, 4)),
        // and [HL] with A
        (0xA6, I::new(And(A.imm(), HL.ind()), 1, 8)),
        // and A with A
        (0xA7, I::new(And(A.imm(), A.imm()), 1, 4)),
        // xor B with A
        (0xA8, I::new(Xor(A.imm(), B.imm()), 1, 4)),
        // xor C with A
        (0xA9, I::new(Xor(A.imm(), C.imm()), 1, 4)),
        // xor D with A
        (0xAA, I::new(Xor(A.imm(), D.imm()), 1, 4)),
        // xor E with A
        (0xAB, I::new(Xor(A.imm(), E.imm()), 1, 4)),
        // xor H with A
        (0xAC, I::new(Xor(A.imm(), H.imm()), 1, 4)),
        // xor L with A
        (0xAD, I::new(Xor(A.imm(), L.imm()), 1, 4)),
        // xor [HL] with A
        (0xAE, I::new(Xor(A.imm(), HL.ind()), 1, 8)),
        // xor A with A
        (0xAF, I::new(Xor(A.imm(), A.imm()), 1, 4)),
        // or B with A
        (0xB0, I::new(Or(A.imm(), B.imm()), 1, 4)),
        // or C with A
        (0xB1, I::new(Or(A.imm(), C.imm()), 1, 4)),
        // or D with A
        (0xB2, I::new(Or(A.imm(), D.imm()), 1, 4)),
        // or E with A
        (0xB3, I::new(Or(A.imm(), E.imm()), 1, 4)),
        // or H with A
        (0xB4, I::new(Or(A.imm(), H.imm()), 1, 4)),
        // or L with A
        (0xB5, I::new(Or(A.imm(), L.imm()), 1, 4)),
        // or [HL] with A
        (0xB6, I::new(Or(A.imm(), HL.ind()), 1, 8)),
        // or A with A
        (0xB7, I::new(Or(A.imm(), A.imm()), 1, 4)),
        // compare B with A
        (0xB8, I::new(Cp(A.imm(), B.imm()), 1, 4)),
        // compare C with A
        (0xB9, I::new(Cp(A.imm(), C.imm()), 1, 4)),
        // compare D with A
        (0xBA, I::new(Cp(A.imm(), D.imm()), 1, 4)),
        // compare E with A
        (0xBB, I::new(Cp(A.imm(), E.imm()), 1, 4)),
        // compare H with A
        (0xBC, I::new(Cp(A.imm(), H.imm()), 1, 4)),
        // compare L with A
        (0xBD, I::new(Cp(A.imm(), L.imm()), 1, 4)),
        // compare [HL] with A
        (0xBE, I::new(Cp(A.imm(), L.imm()), 1, 8)),
        // compare A with A
        (0xBF, I::new(Cp(A.imm(), A.imm()), 1, 4)),
        // return if nonzero
        (0xC0, I::new_ex(Retc(FlagNz.imm()), 1, vec![20, 8])),
        // pop BC
        (0xC1, I::new(Pop(BC.imm()), 1, 12)),
        // jump to nn if nonzero
        (
            0xC2,
            I::new_ex(Jpc(FlagNz.imm(), Const16.imm()), 3, vec![16, 12]),
        ),
        // jump to nn
        (0xC3, I::new(Jp(Const16.imm()), 3, 16)),
        // call nn if nonzero
        (
            0xC4,
            I::new_ex(Callc(FlagNz.imm(), Const16.imm()), 3, vec![24, 12]),
        ),
        // push BC
        (0xC5, I::new(Push(BC.imm()), 1, 16)),
        // add n to A
        (0xC6, I::new(Ld(A.imm(), Const8.imm()), 2, 8)),
        // restart from 0x00
        (0xC7, I::new(Rst(0x00), 1, 32)),
        // return if zero
        (0xC8, I::new_ex(Retc(FlagZ.imm()), 1, vec![20, 8])),
        // return
        (0xC9, I::new(Ret, 1, 16)),
        // jump to nn if zero
        (
            0xCA,
            I::new_ex(Jpc(FlagZ.imm(), Const16.imm()), 3, vec![16, 12]),
        ),
        // extended operations
        // 0xCB
        (0xCB, I::new(Invalid("0xCB"), 1, 4)),
        // call nn if zero
        (
            0xCC,
            I::new_ex(Callc(FlagZ.imm(), Const16.imm()), 3, vec![24, 12]),
        ),
        // call nn
        (0xCD, I::new(Call(Const16.imm()), 3, 24)),
        // add n to A with carry
        (0xCE, I::new(Ld(A.imm(), Const8.imm()), 2, 8)),
        // restart from 0x08
        (0xCF, I::new(Rst(0x08), 1, 32)),
        // return if no carry
        (0xD0, I::new_ex(Retc(FlagNc.imm()), 1, vec![20, 8])),
        // pop DE
        (0xD1, I::new(Pop(DE.imm()), 1, 12)),
        // jump to nn if no carry
        (
            0xD2,
            I::new_ex(Jpc(FlagNc.imm(), Const16.imm()), 3, vec![16, 12]),
        ),
        // extended operations
        (0xD3, I::new(Invalid("0xD3"), 1, 4)),
        // call nn if no carry
        (
            0xD4,
            I::new_ex(Callc(FlagNc.imm(), Const16.imm()), 3, vec![24, 12]),
        ),
        // push DE
        (0xD5, I::new(Push(DE.imm()), 1, 16)),
        // subtract n from A
        (0xD6, I::new(Ld(A.imm(), Const8.imm()), 2, 8)),
        // restart from 0x10
        (0xD7, I::new(Rst(0x10), 1, 32)),
        // return if carry
        (0xD8, I::new_ex(Retc(FlagC.imm()), 1, vec![20, 8])),
        // return and enable interrupts
        (0xD9, I::new(Reti, 1, 16)),
        // jump to nn if carry
        (
            0xDA,
            I::new_ex(Jpc(FlagC.imm(), Const16.imm()), 3, vec![16, 12]),
        ),
        // extended operations
        (0xDB, I::new(Invalid("0xDB"), 1, 4)),
        // call nn if carry
        (
            0xDC,
            I::new_ex(Callc(FlagC.imm(), Const16.imm()), 3, vec![24, 12]),
        ),
        // extended operations
        (0xDD, I::new(Invalid("0xDD"), 1, 4)),
        // subtract n from A with carry
        (0xDE, I::new(Ld(A.imm(), Const8.imm()), 2, 8)),
        // restart from 0x18
        (0xDF, I::new(Rst(0x18), 1, 32)),
        // load A into [0xFF + n]
        (0xE0, I::new(Ld(Const8.high(), A.imm()), 2, 12)),
        // pop HL
        (0xE1, I::new(Pop(HL.imm()), 1, 12)),
        // load A into [0xFF + C]
        (0xE2, I::new(Ld(C.high(), A.imm()), 1, 8)),
        // extended operations
        (0xE3, I::new(Invalid("0xE3"), 1, 4)),
        // extended operations
        (0xE4, I::new(Invalid("0xE4"), 1, 4)),
        // push HL
        (0xE5, I::new(Push(HL.imm()), 1, 16)),
        // and n with A
        (0xE6, I::new(Ld(A.imm(), Const8.imm()), 2, 8)),
        // restart from 0x20
        (0xE7, I::new(Rst(0x20), 1, 32)),
        // add n to SP
        // (0xE8, I::new(Add(SP.imm(), Const8.imm()), 2, 16)),
        (0xE8, todo!()),
        // jump to HL
        (0xE9, I::new(Jp(HL.imm()), 1, 4)),
        // load A into [nn]
        (0xEA, I::new(Ld(Const16.ind(), A.imm()), 3, 16)),
        // extended operations
        (0xEB, I::new(Invalid("0xEB"), 1, 4)),
        // extended operations
        (0xEC, I::new(Invalid("0xEC"), 1, 4)),
        // extended operations
        (0xED, I::new(Invalid("0xED"), 1, 4)),
        // xor n with A
        (0xEE, I::new(Ld(A.imm(), Const8.imm()), 2, 8)),
        // restart from 0x28
        (0xEF, I::new(Rst(0x28), 1, 32)),
        // load [0xFF + n] into A
        (0xF0, I::new(Ld(A.imm(), Const8.high()), 2, 12)),
        // pop AF
        (0xF1, I::new(Pop(AF.imm()), 1, 12)),
        // load [0xFF + C] into A
        (0xF2, I::new(Ld(A.imm(), C.high()), 1, 8)),
        // disable interrupts
        (0xF3, I::new(Di, 1, 4)),
        // extended operations
        (0xF4, I::new(Invalid("0xF4"), 1, 4)),
        // push AF
        (0xF5, I::new(Push(AF.imm()), 1, 16)),
        // or n with A
        (0xF6, I::new(Ld(A.imm(), Const8.imm()), 2, 8)),
        // restart from 0x30
        (0xF7, I::new(Rst(0x30), 1, 32)),
        // load SP + n into HL
        (0xF8, I::new(Ldhl(Const8.imm()), 2, 12)),
        // load HL into SP
        (0xF9, I::new(Ld(SP.imm(), HL.imm()), 1, 8)),
        // load [nn] into A
        (0xFA, I::new(Ld(A.imm(), Const16.ind()), 3, 16)),
        // enable interrupts
        (0xFB, I::new(Ei, 1, 4)),
        // extended operations
        (0xFC, I::new(Invalid("0xFC"), 1, 4)),
        // extended operations
        (0xFD, I::new(Invalid("0xFD"), 1, 4)),
        // compare n with A
        (0xFE, I::new(Cp(A.imm(), Const8.imm()), 2, 8)),
        // restart from 0x38
        (0xFF, I::new(Rst(0x38), 1, 32)),
    ]);

    map
}
