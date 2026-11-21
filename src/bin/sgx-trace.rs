// SPDX-License-Identifier: MIT
//! The lockstep gate's Rust side: run a card on the SuperGrafx and write one line a frame, the
//! same lines `scripts/sgx_trace.py` writes for the reference.
//!
//!     sgx-trace IMAGE --frames N [--mode sgx|compat|base] [--vpc NAME=VALUE]... [--compat-at N]
//!               [options of pcengine_rust::trace::Setup] [--detail FRAME] --out FILE
//!
//! With `--detail FRAME` only the bus cycles of the frame whose line would start with FRAME are
//! written, one a line (`r ADDRESS VALUE MCLK`, `w ...`, `i MCLK`).

use std::io::Write;

use pcengine_rust::trace::{detail_lines, frame_line};
use supergrafx_rust::trace::SgxSetup;

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let setup = SgxSetup::parse(&args)?;
    if let Some(unknown) = setup.rest.first() {
        return Err(format!("unknown argument {unknown}"));
    }
    let mut system = setup.build()?;
    system.board_mut().bus_log = Some(Vec::new());
    let mut out: Box<dyn Write> = match &setup.base.out {
        Some(path) => Box::new(std::io::BufWriter::new(
            std::fs::File::create(path).map_err(|error| format!("{path}: {error}"))?,
        )),
        None => Box::new(std::io::stdout().lock()),
    };
    let io = |error: std::io::Error| error.to_string();
    for _ in 0..setup.base.frames {
        setup.before_frame(&mut system);
        let wanted = setup.base.detail.map(|frame| frame == system.frame() + 1);
        if wanted == Some(true) {
            system.board_mut().bus_detail = Some(Vec::new());
        }
        system.run_frame();
        let line = frame_line(&mut system);
        match wanted {
            None => writeln!(out, "{line}").map_err(io)?,
            Some(true) => {
                for cycle in detail_lines(&mut system) {
                    writeln!(out, "{cycle}").map_err(io)?;
                }
                break;
            }
            Some(false) => {}
        }
    }
    out.flush().map_err(io)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("sgx-trace: {error}");
        std::process::exit(1);
    }
}
