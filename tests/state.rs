// SPDX-License-Identifier: MIT
//! Save states: the base board's `state` over the SuperGrafx (the VPC, the page, both VDCs).
//! The reference's tests/test_state.py.

use pcengine_rust::board::{Board, BoardOptions};
use pcengine_rust::cards::RomCard;
use pcengine_rust::state::{load_snapshot, load_state, save_snapshot, save_state};
use pcengine_rust::system::System;
use supergrafx_rust::trace::{roundtrip, runahead, Mode, SgxSetup, BASE_MODE};
use supergrafx_rust::{set_compat, video_page, SuperGrafx, SuperGrafxOptions, VideoPage};

/// A card that is only NOPs and its reset vector (to $E000).
fn image() -> Vec<u8> {
    let mut data = vec![0xEA; 0x2000];
    data[0x1FFE] = 0x00;
    data[0x1FFF] = 0xE0;
    data
}

fn running(compat: bool) -> System {
    let mut options = SuperGrafxOptions {
        compat,
        ..SuperGrafxOptions::default()
    };
    options.board.card = Some(Box::new(RomCard::new(&image()).unwrap()));
    SuperGrafx::new(options).unwrap().build()
}

fn page(system: &mut System) -> &mut VideoPage {
    video_page(system.board_mut()).unwrap()
}

#[test]
fn the_vpc_the_page_and_the_second_vdc_come_back() {
    let mut system = running(false);
    page(&mut system).vpc.priority = [0x75, 0x56];
    page(&mut system).vpc.window = [0x123, 0x3FF];
    page(&mut system).vpc.write(0x0E, 0x01, 0);
    page(&mut system).vpc.write(0x09, 0x56, i64::MAX / 2); // a write no line has drawn yet
    system.board_mut().vdcs[1].vram[0x1234] = 0xBEEF;
    system.board_mut().vdcs[1].regs.select = 0x09;
    set_compat(&mut system, true);
    for _ in 0..2000 {
        system.step();
    }
    let mut loaded = running(false);
    page(&mut loaded).st_steers_in_compat = true; // a switch: the loaded system keeps its own
    load_state(&mut loaded, &save_state(&system)).unwrap();
    assert_eq!(page(&mut loaded).vpc.priority, [0x75, 0x56]);
    assert_eq!(page(&mut loaded).vpc.window, [0x123, 0x3FF]);
    assert!(page(&mut loaded).vpc.st_to_vdc2);
    assert_eq!(page(&mut loaded).vpc, page(&mut system).vpc); // the shadows and the log too
    assert_eq!(loaded.board().vdcs[1].vram[0x1234], 0xBEEF);
    assert_eq!(loaded.board().vdcs[1].regs.select, 0x09);
    assert!(page(&mut loaded).compat);
    assert!(page(&mut loaded).st_steers_in_compat && !page(&mut system).st_steers_in_compat);
    assert!(save_state(&loaded) == save_state(&system));
    for _ in 0..2000 {
        system.step();
        loaded.step();
    }
    assert!(save_state(&loaded) == save_state(&system));
}

#[test]
fn a_plain_board_refuses_a_state_of_this_machine_and_this_machine_one_of_a_plain_board() {
    let mut system = running(false);
    let state = save_state(&system);
    let plain = |machine| {
        System::new(
            Board::new(BoardOptions {
                machine,
                card: Some(Box::new(RomCard::new(&image()).unwrap())),
                ..BoardOptions::default()
            })
            .unwrap(),
        )
    };
    let mut pce = plain(pcengine_rust::machine::PC_ENGINE);
    let before = save_state(&pce);
    let error = load_state(&mut pce, &state).unwrap_err();
    assert!(
        error.contains("of the SuperGrafx, not the PC Engine"),
        "{error}"
    );
    assert!(save_state(&pce) == before); // refused with the system untouched
    let error = load_state(&mut system, &before).unwrap_err();
    assert!(
        error.contains("of the PC Engine, not the SuperGrafx"),
        "{error}"
    );
    // the fallback (one VDC, no VPC, 32 KiB) is another machine too
    let mut base = plain(BASE_MODE);
    assert!(load_state(&mut base, &state).is_err());
    assert!(load_state(&mut system, &save_state(&base)).is_err());
}

#[test]
fn a_state_that_is_not_the_vpcs_is_refused_by_the_page() {
    use pcengine_rust::sockets::DeviceState;
    let mut system = running(false);
    assert!(page(&mut system).load_state(&[1, 2, 3]).is_err());
    let saved = page(&mut system).save_state();
    assert!(!saved.is_empty() && page(&mut system).load_state(&saved).is_ok());
}

/// A card that drives the VPC and both VDCs for ever: the counter in `$2000` goes to a
/// priority register, a window, VDC 2's BXR (through a steered ST0-ST2) and the colour table.
fn program() -> Vec<u8> {
    #[rustfmt::skip]
    let code: &[u8] = &[
        0x78, 0xD4,  // SEI, CSH
        0xA9, 0xFF, 0x53, 0x01,  // LDA #$FF; TAM #$01: the hardware page at $0000
        0xA9, 0xF8, 0x53, 0x02,  // LDA #$F8; TAM #$02: work RAM at $2000
        0xA9, 0x05, 0x8D, 0x10, 0x00,  // LDA #5; STA $0010: VDC 2 select CR
        0xA9, 0xC0, 0x8D, 0x12, 0x00,  // LDA #$C0; STA $0012: background and sprites on
        0xEE, 0x00, 0x20,  // loop ($E014): INC $2000
        0xAD, 0x00, 0x20,  // LDA $2000
        0x8D, 0x09, 0x00,  // STA $0009 (priority B)
        0x8D, 0x0A, 0x00,  // STA $000A (window 1)
        0x8D, 0x0E, 0x00,  // STA $000E (steering: on and off in turn)
        0x03, 0x07, 0x13, 0x21, 0x23, 0x00,  // ST0 #7; ST1 #$21; ST2 #0
        0x8D, 0x02, 0x04,  // STA $0402 (VCE colour address, low)
        0x8D, 0x04, 0x04,  // STA $0404 (VCE colour data, low)
        0x4C, 0x14, 0xE0,  // JMP loop
    ];
    let mut data = image();
    data[..code.len()].copy_from_slice(code);
    data
}

#[test]
fn a_loaded_run_continues_bus_cycle_for_bus_cycle() {
    for mode in [Mode::Sgx, Mode::Compat] {
        let mut setup = SgxSetup::for_image(program());
        setup.mode = mode;
        let what = roundtrip(&setup, 4, 8, 777).unwrap();
        assert!(
            what.contains("4 frames after the save equal, twice"),
            "{what}"
        );
    }
    // the switch moved during the run, before the save and after it
    for compat_at in [2, 6] {
        let mut setup = SgxSetup::for_image(program());
        setup.compat_at = Some(compat_at);
        roundtrip(&setup, 4, 8, 333).unwrap();
    }
}

#[test]
fn a_snapshot_put_back_leaves_the_run_as_it_was() {
    let setup = SgxSetup::for_image(program());
    runahead(&setup, 6, 2).unwrap();
    let mut system = setup.build().unwrap();
    system.run_frame();
    let snapshot = save_snapshot(&system);
    let state = save_state(&system);
    page(&mut system).vpc.write(0x08, 0x00, 0);
    set_compat(&mut system, true);
    system.run_frame();
    load_snapshot(&mut system, &snapshot).unwrap();
    assert!(save_state(&system) == state);
    assert!(!page(&mut system).compat);
}
