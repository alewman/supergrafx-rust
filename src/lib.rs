// SPDX-License-Identifier: MIT
//! A Rust NEC SuperGrafx transcribed from
//! [supergrafx-python](https://github.com/alewman/supergrafx-python) at `v0.1.0` (`eee789a`)
//! and proven equivalent to it, as an add-on to pcengine-rust through its socket traits: the
//! `SUPERGRAFX` machine row (two VDCs, 32 KiB), the HuC6202 `Vpc` (registers, windows, the
//! priority mix), the `VideoPage` decode with the compatibility switch (the board's
//! `PixelPath`), and the `SuperGrafx` builder.
//!
//! Each module is the transcription of the `supergrafx_python` module of the same name; where
//! Rust forced a difference, the module header says so. `cards` and `trace` are not from the
//! reference's package.
//!
//! ```
//! use supergrafx_rust::{is_supergrafx_card, SuperGrafx};
//! use pcengine_rust::pads::RUN;
//!
//! let image = vec![0u8; 0x2000]; // a HuCard image (bring your own)
//! let _ = is_supergrafx_card(&image, Some("Daimakaimura (Japan).sgx"));
//! let mut system = SuperGrafx::with_card(&image, false).unwrap().build();
//! system.board_mut().set_pressed(0, RUN);
//! system.run_frame();
//! let _samples = system.board_mut().take_audio(); // 44.1 kHz stereo
//! let _picture = system.board().capture.last_frame.as_ref(); // 242 rows of 9-bit values
//! ```

pub mod board;
pub mod cards;
pub mod machine;
pub mod trace;
pub mod vpc;

pub use board::{set_compat, video_page, SuperGrafx, SuperGrafxOptions};
pub use cards::{is_supergrafx_card, supergrafx_title};
pub use machine::SUPERGRAFX;
pub use pcengine_rust;
pub use vpc::{PriorityLabels, VideoPage, Vpc, WindowOrigin};

/// The README's example, compiled with the documentation tests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeExample;
