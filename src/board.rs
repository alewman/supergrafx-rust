// SPDX-License-Identifier: MIT
//! The SuperGrafx: the base PC Engine board with a machine row, a VPC and a second VDC
//! (`board.py`).
//!
//! Nothing here is copied from pcengine-rust; this module only composes it through its sockets
//! (the reference's docs/start-here.md lists them):
//!
//! - socket 12 (instances) and the machine row: `Machine::vdc_count = 2` makes the board build,
//!   run, sync and read a second `Vdc` and OR its IRQ1;
//! - socket 3 (the decode): `VideoPage` claims `$FF:0000-03FF` over the VDC page;
//! - socket 7 (pixel path): `VideoPage::mix` (the `Vpc`'s) is the board's `PixelPath`;
//! - the work RAM is the machine row's 32 KiB, mirrored nowhere (banks `$F8-$FB`).
//!
//! Differences forced by Rust:
//!
//! - **The board owns the VPC and the CPU owns the board**, so `SuperGrafx` cannot keep `vpc`,
//!   `vdc1`, `vdc2` and `page` beside the board as the reference does. They are reached through
//!   the board: `board.vdcs[0]`, `board.vdcs[1]`, and `video_page(&mut board)` for the page and
//!   its `vpc` (the base board's `PixelPath::as_any_mut`). `build` consumes the `SuperGrafx` and
//!   gives the `System`; afterwards the same things are reached with `system.board_mut()`.
//! - **The constructor's keyword arguments** are `SuperGrafxOptions`: `compat`, `vpc` and the
//!   base board's `BoardOptions` (the reference's `**board_kwargs`), whose `machine` and
//!   `pixel_path` are the add-on's to set.
//! - **`_store_immediate`** is not here: the base board hands the opcode to the page with each
//!   access (vpc.rs). **`reset`** is `Board::reset`, which calls the page's (vpc.rs).

use pcengine_rust::board::{Board, BoardOptions};
use pcengine_rust::cards::make_card;
use pcengine_rust::pads::Pad2;
use pcengine_rust::sockets::ResetKind;
use pcengine_rust::system::System;

use crate::machine::SUPERGRAFX;
use crate::vpc::{VideoPage, Vpc};

/// The reference's `SuperGrafx(card, compat=..., vpc=..., **board_kwargs)`.
pub struct SuperGrafxOptions {
    /// `true` is the back-panel switch in PC Engine position.
    pub compat: bool,
    /// A VPC with other switches than the defaults (`None`: `Vpc::new()`).
    pub vpc: Option<Vpc>,
    /// U-SGX4 (`VideoPage::st_steers_in_compat`).
    pub st_steers_in_compat: bool,
    /// The base board's arguments (card, pad, port, power_on_seed, start_line, ...). `machine`
    /// is replaced by the `SUPERGRAFX` row and `pixel_path` must be `None`: the VPC is the path.
    pub board: BoardOptions,
}

impl Default for SuperGrafxOptions {
    fn default() -> Self {
        SuperGrafxOptions {
            compat: false,
            vpc: None,
            st_steers_in_compat: false,
            board: BoardOptions {
                machine: SUPERGRAFX,
                ..BoardOptions::default()
            },
        }
    }
}

/// A base board configured as a SuperGrafx. `board` is the plain `pcengine_rust::board::Board`.
pub struct SuperGrafx {
    pub board: Board,
}

impl SuperGrafx {
    pub fn new(options: SuperGrafxOptions) -> Result<SuperGrafx, String> {
        if options.board.pixel_path.is_some() {
            return Err("the SuperGrafx's pixel path is its VPC".to_string());
        }
        let mut page = VideoPage::new(options.vpc.unwrap_or_default(), options.compat);
        page.st_steers_in_compat = options.st_steers_in_compat;
        let board = Board::new(BoardOptions {
            machine: SUPERGRAFX,
            pixel_path: Some(Box::new(page)),
            ..options.board
        })?;
        Ok(SuperGrafx { board })
    }

    /// A SuperGrafx with this card image in the slot and a 2-button pad in the port (a host's
    /// usual machine: `make_card` strips a copier header; every SuperGrafx card is a plain ROM).
    pub fn with_card(image: &[u8], compat: bool) -> Result<SuperGrafx, String> {
        let mut options = SuperGrafxOptions {
            compat,
            ..SuperGrafxOptions::default()
        };
        options.board.card = Some(make_card(image)?);
        options.board.pad = Some(Box::new(Pad2::new()));
        SuperGrafx::new(options)
    }

    /// The video page (the decode, the compatibility switch) and its `vpc`.
    pub fn page(&mut self) -> &mut VideoPage {
        video_page(&mut self.board).expect("a SuperGrafx's pixel path is its video page")
    }

    pub fn compat(&mut self) -> bool {
        self.page().compat
    }

    pub fn set_compat(&mut self, value: bool) {
        self.page().compat = value;
    }

    /// The board's reset; a power-on reset also resets the VPC (`VideoPage`'s
    /// `PixelPath::reset`).
    pub fn reset(&mut self, kind: ResetKind) {
        self.board.reset(kind);
    }

    /// The real HuC6280 core on this board.
    pub fn build(self) -> System {
        System::new(self.board)
    }
}

/// The video page of a board built by `SuperGrafx` (`None` for any other board): after
/// `build`, `video_page(system.board_mut())`.
pub fn video_page(board: &mut Board) -> Option<&mut VideoPage> {
    board.pixel_path.as_any_mut().downcast_mut::<VideoPage>()
}

/// Move the back-panel switch of a running SuperGrafx (`true`: PC Engine position). `false`
/// if the system is not a SuperGrafx.
pub fn set_compat(system: &mut System, value: bool) -> bool {
    match video_page(system.board_mut()) {
        Some(page) => {
            page.compat = value;
            true
        }
        None => false,
    }
}
