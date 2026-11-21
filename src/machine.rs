// SPDX-License-Identifier: MIT
//! The SuperGrafx's row of the machine table (`machine.py`; the base board's
//! `machine::Machine`).
//!
//! [hw: sgxtech, "Overview"] a PC Engine with 32 KiB of work RAM instead of 8 KiB, a second VDC with
//! its own 64 KiB of VRAM and the HuC6202 video priority controller (VPC) between the VDC pixel
//! buses and the VCE. The CPU is a HuC6280A (U6: not distinguished from the HuC6280 here), the
//! master clock and the 263/262-line frame are the base board's. The I/O port's country bit (D6)
//! is 1: the SuperGrafx is a Japanese console [hw: sgxtech, "I/O port"].

use pcengine_rust::machine::Machine;

pub const SUPERGRAFX: Machine = Machine {
    work_ram_bytes: 32 * 1024,
    vdc_count: 2,
    ..Machine::new("SuperGrafx", true)
};
