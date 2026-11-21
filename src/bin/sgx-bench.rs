// SPDX-License-Identifier: MIT
//! Frames per second of this machine on a card, nothing logged or digested.
//!
//!     sgx-bench IMAGE [--frames N] [options of sgx-trace]
//!
//! Speed is not this crate's goal (a fast fork's); `scripts/bench.py` times the reference the
//! same way.

use std::time::Instant;

use supergrafx_rust::trace::SgxSetup;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let setup = match SgxSetup::parse(&args) {
        Ok(setup) => setup,
        Err(error) => {
            eprintln!("sgx-bench: {error}");
            std::process::exit(1);
        }
    };
    let mut system = setup.build().expect("a board");
    let began = Instant::now();
    let mut samples = 0;
    for _ in 0..setup.base.frames {
        setup.before_frame(&mut system);
        system.run_frame();
        samples += system.board_mut().take_audio().len();
    }
    let seconds = began.elapsed().as_secs_f64();
    println!(
        "{} frames in {:.3} s: {:.1} frames per second ({} audio samples, mclk {})",
        setup.base.frames,
        seconds,
        setup.base.frames as f64 / seconds,
        samples,
        system.board().mclk
    );
    if let Some(page) = supergrafx_rust::video_page(system.board_mut()) {
        let vpc = &page.vpc;
        println!(
            "VPC ${:02X} ${:02X}, windows ${:03X} ${:03X}, ST0-ST2 to VDC {}",
            vpc.priority[0],
            vpc.priority[1],
            vpc.window[0],
            vpc.window[1],
            if vpc.st_to_vdc2 { 2 } else { 1 }
        );
    }
}
