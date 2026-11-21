// SPDX-License-Identifier: MIT
//! The add-on with VDC 2 idle is the base board: same rows as the fallback (base mode). The
//! reference's tests/test_gate.py.
//!
//! Power-up leaves the VPC at `$11 $11 ...` (windows off, VDC 1 only), so a program that never
//! touches VDC 2 or the VPC must draw what the one-VDC board draws. Uses Covell's CPU test (the
//! base board's fetched test card, read in place: `$COVELL`); skipped when absent, which
//! scripts/rung4.sh does not allow.

use pcengine_rust::board::{Board, BoardOptions};
use pcengine_rust::cards::make_card;
use pcengine_rust::pads::Pad2;
use pcengine_rust::system::System;
use supergrafx_rust::trace::BASE_MODE;
use supergrafx_rust::SuperGrafx;

const FRAMES: usize = 4;

#[test]
fn idle_vdc2_and_default_vpc_draw_what_the_base_mode_draws() {
    let path = std::env::var("COVELL").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../pcengine-python/tests/test_roms/CPU_Test/CPU_Test_10.pce"
        )
        .to_string()
    });
    let Ok(image) = std::fs::read(&path) else {
        eprintln!("skipped: Covell's CPU test is not at {path}");
        return;
    };
    let mut base = System::new(
        Board::new(BoardOptions {
            machine: BASE_MODE,
            card: Some(make_card(&image).unwrap()),
            pad: Some(Box::new(Pad2::new())),
            ..BoardOptions::default()
        })
        .unwrap(),
    );
    let mut system = SuperGrafx::with_card(&image, false).unwrap().build();
    for _ in 0..FRAMES {
        base.run_frame();
        system.run_frame();
        let ours = system.board().capture.last_frame.as_ref().unwrap();
        let theirs = base.board().capture.last_frame.as_ref().unwrap();
        assert_eq!(ours.rows.len(), 242);
        assert!(ours.rows == theirs.rows);
    }
    assert_eq!(system.board().mclk, base.board().mclk);
    println!("compared: {FRAMES} frames of {path}");
}
