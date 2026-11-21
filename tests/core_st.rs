// SPDX-License-Identifier: MIT
//! ST0/ST1/ST2 through the real HuC6280 core: the VPC sees the instruction, not the address.
//! The reference's tests/test_core_st.py.

use pcengine_rust::cards::RomCard;
use pcengine_rust::system::System;
use supergrafx_rust::{set_compat, video_page, SuperGrafx, SuperGrafxOptions};

const BANK: usize = 0x2000;

/// A one-bank HuCard: code at $E000 (MPR7 = bank 0 at reset), reset vector $FFFE -> $E000.
fn program(code: &[u8]) -> System {
    let mut rom = vec![0xEA; BANK];
    rom[..code.len()].copy_from_slice(code);
    rom[0x1FFE] = 0x00;
    rom[0x1FFF] = 0xE0;
    let mut options = SuperGrafxOptions::default();
    options.board.card = Some(Box::new(RomCard::new(&rom).unwrap()));
    SuperGrafx::new(options).unwrap().build()
}

#[rustfmt::skip]
const CODE: &[u8] = &[
    0xA9, 0xFF,  // LDA #$FF
    0x53, 0x04,  // TAM #$04: MPR2 -> bank $FF, so $4000 is the hardware page
    0xA9, 0x01,  // LDA #1
    0x8D, 0x0E, 0x40,  // STA $400E : VPC ST steering -> VDC 2
    0x03, 0x07,  // ST0 #$07 : VDC 2 register select (BXR)
    0x13, 0x12,  // ST1 #$12
    0x23, 0x01,  // ST2 #$01  : VDC 2 BXR = $0112
    0xA9, 0x07,  // LDA #7
    0x8D, 0x00, 0x40,  // STA $4000 : VDC 1 register select, a plain store
    0x9C, 0x0E, 0x40,  // STZ $400E : steering off
    0x03, 0x08,  // ST0 #$08 : VDC 1 register select (BYR)
    0x13, 0x34,  // ST1 #$34
    0x23, 0x02,  // ST2 #$02  : VDC 1 BYR = $0234
    0xDB,  // STP
];

#[test]
fn the_core_st_instructions_follow_the_vpc_and_plain_stores_do_not() {
    let mut system = program(CODE);
    for _ in 0..40 {
        system.step();
    }
    let vdcs = &system.board().vdcs;
    assert_eq!(
        (vdcs[1].regs.value[0x07], vdcs[0].regs.value[0x07]),
        (0x0112, 0)
    );
    assert_eq!(
        (vdcs[0].regs.select, vdcs[0].regs.value[0x08]),
        (0x08, 0x0234)
    );
    assert_eq!(vdcs[1].regs.value[0x08], 0);
}

#[rustfmt::skip]
const LEFT_OVER: &[u8] = &[
    0xA9, 0xFF, 0x53, 0x04,  // LDA #$FF, TAM #$04
    0xA9, 0x01, 0x8D, 0x0E, 0x40,  // LDA #1, STA $400E : steering on, in native position
    0x4C, 0x09, 0xE0,  // $E009: JMP $E009 ... the switch is moved here ... then, at $E00C:
    0x03, 0x09,  // ST0 #$09
    0x4C, 0x0E, 0xE0,  // $E00E: JMP $E00E
];

/// U-SGX4 through the core: the `$0E` bit left from native mode, the switch then moved.
#[test]
fn a_steering_bit_left_from_native_mode_follows_the_named_switch() {
    for (switch, selects) in [(false, (0x09, 0x00)), (true, (0x00, 0x09))] {
        let mut system = program(LEFT_OVER);
        for _ in 0..12 {
            system.step();
        }
        assert_eq!(system.cpu.pc, 0xE009);
        assert!(video_page(system.board_mut()).unwrap().vpc.st_to_vdc2);
        assert!(set_compat(&mut system, true));
        video_page(system.board_mut()).unwrap().st_steers_in_compat = switch;
        system.cpu.pc = 0xE00C;
        system.step();
        let vdcs = &system.board().vdcs;
        assert_eq!((vdcs[0].regs.select, vdcs[1].regs.select), selects);
    }
}
