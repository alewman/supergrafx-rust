// SPDX-License-Identifier: MIT
//! The lockstep gate's side of this add-on: how a traced SuperGrafx is built from a command
//! line, and the save-state checks. **Not a module of the reference**: its counterparts there
//! are `scripts/sgx_ours.py` (the modes and `--vpc-option`) and `scripts/sgx_state.py`.
//!
//! The lines themselves are the base board's (`pcengine_rust::trace::frame_line`: the CPU bus
//! log, the mixed picture, the audio and the CPU state of a frame), and so is the command line
//! (`pcengine_rust::trace::Setup`), to which this adds:
//!
//! - `--mode sgx|compat|base`: the add-on in native position (default), the back-panel switch
//!   in PC Engine position, or the **fallback**: the card on the plain base board with a row of
//!   32 KiB and one VDC (the reference's `sgx_ours.py --mode base`);
//! - `--vpc NAME=VALUE`: a named switch by the reference's attribute name
//!   (`PRIORITY_LABELS=sgxtech`, `WINDOW_ORIGIN=40,40,40` for the dividers 4, 3 and 2,
//!   `WINDOW_OFF_AT_OR_BELOW=N`, `ST_STEERS_IN_COMPAT=true`);
//! - `--compat-at FRAME`: the switch is moved to PC Engine position once FRAME frames have
//!   completed (U-SGX4 needs a `$0E` bit left over from native mode).
//!
//! `roundtrip` and `runahead` are the base board's `trace::roundtrip` and `runahead` over a
//! machine built here (the base's build its own boards from a `Setup`, so they could not be
//! called).

use pcengine_rust::board::{Board, BoardOptions};
use pcengine_rust::cards::make_card;
use pcengine_rust::machine::Machine;
use pcengine_rust::pads::{self, Pad2};
use pcengine_rust::sockets::ControllerPort;
use pcengine_rust::state::{load_snapshot, load_state, save_snapshot, save_state};
use pcengine_rust::system::System;
use pcengine_rust::trace::{frame_line, Setup};

use crate::board::{set_compat, SuperGrafx, SuperGrafxOptions};
use crate::machine::SUPERGRAFX;
use crate::vpc::{PriorityLabels, Vpc, WindowOrigin};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Sgx,
    Compat,
    Base,
}

/// The fallback's machine row: the SuperGrafx's RAM on a board with one VDC and no VPC.
pub const BASE_MODE: Machine = Machine {
    work_ram_bytes: SUPERGRAFX.work_ram_bytes,
    ..Machine::new("SuperGrafx (base mode)", true)
};

/// A named switch of the reference's `Vpc`, by its attribute name. `ST_STEERS_IN_COMPAT`
/// (the page's) is answered with `Ok(Some(value))`.
pub fn set_vpc_switch(vpc: &mut Vpc, name: &str, value: &str) -> Result<Option<bool>, String> {
    let number = |text: &str| {
        text.trim()
            .parse::<i64>()
            .map_err(|_| format!("{name}: a number, not {text}"))
    };
    match name {
        "PRIORITY_LABELS" => {
            vpc.priority_labels = match value {
                "detectors" => PriorityLabels::Detectors,
                "sgxtech" => PriorityLabels::Sgxtech,
                _ => return Err(format!("{name}: detectors or sgxtech, not {value}")),
            }
        }
        "WINDOW_ORIGIN" => {
            let parts: Vec<&str> = value.split(',').collect();
            if parts.len() != 3 {
                return Err(format!(
                    "{name}: three numbers (dividers 4, 3, 2), not {value}"
                ));
            }
            vpc.window_origin = WindowOrigin {
                divider4: number(parts[0])?,
                divider3: number(parts[1])?,
                divider2: number(parts[2])?,
            };
        }
        "WINDOW_OFF_AT_OR_BELOW" => vpc.window_off_at_or_below = number(value)?,
        "ST_STEERS_IN_COMPAT" => {
            return match value {
                "true" | "True" | "1" => Ok(Some(true)),
                "false" | "False" | "0" => Ok(Some(false)),
                _ => Err(format!("{name}: true or false, not {value}")),
            }
        }
        _ => return Err(format!("no VPC switch {name}")),
    }
    Ok(None)
}

/// How a traced machine is built: the base board's `Setup` and this add-on's own arguments.
#[derive(Clone, Debug)]
pub struct SgxSetup {
    pub base: Setup,
    pub mode: Mode,
    pub vpc: Vpc,
    pub st_steers_in_compat: bool,
    pub compat_at: Option<i64>,
    /// Arguments neither parser knows (a binary's own).
    pub rest: Vec<String>,
}

impl SgxSetup {
    /// The reference's defaults around a card image, in native position.
    pub fn for_image(image: Vec<u8>) -> SgxSetup {
        SgxSetup {
            base: Setup::for_image(image),
            mode: Mode::Sgx,
            vpc: Vpc::new(),
            st_steers_in_compat: false,
            compat_at: None,
            rest: Vec::new(),
        }
    }

    pub fn parse(args: &[String]) -> Result<SgxSetup, String> {
        let base = Setup::parse(args)?;
        let mut setup = SgxSetup::for_image(Vec::new());
        let mut iter = base.rest.iter();
        while let Some(arg) = iter.next() {
            let mut value = || iter.next().ok_or_else(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "--mode" => {
                    setup.mode = match value()?.as_str() {
                        "sgx" => Mode::Sgx,
                        "compat" => Mode::Compat,
                        "base" => Mode::Base,
                        other => return Err(format!("no mode {other}: sgx, compat or base")),
                    }
                }
                "--vpc" => {
                    let text = value()?;
                    let (name, to) = text
                        .split_once('=')
                        .ok_or_else(|| format!("--vpc NAME=VALUE, not {text}"))?;
                    if let Some(flag) = set_vpc_switch(&mut setup.vpc, name, to)? {
                        setup.st_steers_in_compat = flag;
                    }
                }
                "--compat-at" => {
                    let text = value()?;
                    setup.compat_at =
                        Some(text.parse().map_err(|_| format!("not a number: {text}"))?);
                }
                _ => setup.rest.push(arg.clone()),
            }
        }
        setup.base = base;
        Ok(setup)
    }

    /// One more machine, built the same way each time.
    pub fn build(&self) -> Result<System, String> {
        let base = &self.base;
        let pad: Option<Box<dyn ControllerPort>> = match base.device.as_str() {
            "none" => None,
            _ => Some(Box::new(Pad2::new())),
        };
        let board = BoardOptions {
            machine: BASE_MODE,
            card: Some(make_card(&base.image)?),
            port: None,
            pad,
            pixel_path: None,
            power_on_seed: base.seed,
            start_line: base.start_line,
            start_offset: base.start_offset,
            vblank_dots: base.vblank_dots,
            dma_resumes: base.dma_resumes,
            psg: base.psg,
            timer_underflow_window_mclk: base.timer_window,
        };
        if self.mode == Mode::Base {
            return Ok(System::new(Board::new(board)?));
        }
        let machine = SuperGrafx::new(SuperGrafxOptions {
            compat: self.mode == Mode::Compat,
            vpc: Some(self.vpc.clone()),
            st_steers_in_compat: self.st_steers_in_compat,
            board,
        })?;
        Ok(machine.build())
    }

    /// Before a frame: hold the buttons of the presses, and move the switch if it is time.
    pub fn before_frame(&self, system: &mut System) {
        if self.compat_at == Some(system.frame()) {
            set_compat(system, true);
        }
        self.base.apply_presses(system);
    }

    /// Run `frames` frames; a line a frame.
    pub fn run_frames(&self, system: &mut System, frames: i64) -> Vec<String> {
        (0..frames)
            .map(|_| {
                self.before_frame(system);
                system.run_frame();
                frame_line(system)
            })
            .collect()
    }

    fn started(&self) -> Result<System, String> {
        let mut system = self.build()?;
        system.board_mut().bus_log = Some(Vec::new());
        Ok(system)
    }
}

fn settle(system: &mut System) {
    if let Some(log) = system.board_mut().bus_log.as_mut() {
        log.clear();
    }
    system.board_mut().take_audio();
}

/// Save states, proven complete (the base board's `trace::roundtrip`, the reference's
/// `scripts/sgx_state.py`): power on, run to frame `at` plus `extra` instructions, save, then
/// load the state into a fresh system and into one that had run elsewhere, and run each to frame
/// `to`: every frame after the save must be the same, by the gate's own lines, and the loaded
/// system saved at once must give the same bytes.
pub fn roundtrip(setup: &SgxSetup, at: i64, to: i64, extra: i64) -> Result<String, String> {
    let mut first = setup.started()?;
    setup.run_frames(&mut first, at);
    for _ in 0..extra {
        first.step();
    }
    let state = save_state(&first);
    settle(&mut first);
    let expected = setup.run_frames(&mut first, to - at);
    for (label, warm) in [
        ("a fresh system", 0),
        ("a system that had run", 1.max(at / 2)),
    ] {
        let mut second = setup.started()?;
        setup.run_frames(&mut second, warm);
        load_state(&mut second, &state).map_err(|error| format!("{label}: {error}"))?;
        if save_state(&second) != state {
            return Err(format!(
                "{label}: saved again at once, the state is not the same bytes"
            ));
        }
        settle(&mut second);
        let got = setup.run_frames(&mut second, to - at);
        for (n, (a, b)) in expected.iter().zip(&got).enumerate() {
            if a != b {
                return Err(format!(
                    "{label}: differs in frame {} after the save\n  expected {a}\n  got      {b}",
                    n + 1
                ));
            }
        }
    }
    Ok(format!(
        "{} bytes; {} frames after the save equal, twice",
        state.len(),
        to - at
    ))
}

/// The snapshot pair in a run-ahead pass (the base board's `trace::runahead`): every frame is
/// run, a snapshot taken, `ahead` more frames run with other buttons held, and the snapshot put
/// back. Each frame's line, and the state at the end, must equal a run's that never looked ahead.
pub fn runahead(setup: &SgxSetup, frames: i64, ahead: i64) -> Result<String, String> {
    let mut plain = setup.started()?;
    let expected = setup.run_frames(&mut plain, frames);
    let mut system = setup.started()?;
    let mut bytes = 0;
    for (n, wanted) in expected.iter().enumerate() {
        setup.before_frame(&mut system);
        system.run_frame();
        let line = frame_line(&mut system);
        if &line != wanted {
            return Err(format!(
                "frame {} differs after a snapshot was put back\n  expected {wanted}\n  got      {line}",
                n + 1
            ));
        }
        let snapshot = save_snapshot(&system);
        bytes = snapshot.len();
        for _ in 0..ahead {
            system
                .board_mut()
                .set_pressed(0, pads::RUN | pads::I | pads::RIGHT);
            system.run_frame();
        }
        load_snapshot(&mut system, &snapshot)?;
        if let Some(log) = system.board_mut().bus_log.as_mut() {
            log.clear();
        }
    }
    if save_state(&system) != save_state(&plain) {
        return Err("the end state is not the plain run's".to_string());
    }
    Ok(format!(
        "{frames} frames, {ahead} ahead of each, equal to the plain run; a snapshot is {bytes} bytes"
    ))
}
