// SPDX-License-Identifier: MIT
//! The HuC6202 video priority controller (VPC): the SuperGrafx's pixel path and its register
//! block. Transcribed from supergrafx-python's `vpc.py` (f04d648).
//!
//! [hw: sgxtech, "Video Priority Controller"] The VPC
//!
//! - mixes the 9-bit pixel buses of VDC 1 and VDC 2 and hands the result to the VCE
//!   (`Vpc::mix`, the board's `PixelPath` socket 7: one row per VDC, one row back);
//! - keeps two window widths and four 4-bit (enable, enable, priority) groups, one per window area;
//! - steers `ST0`-`ST2` to VDC 2 when `$0E` bit 0 is set (`Vpc::st_to_vdc2`, used by `VideoPage`);
//! - decodes the video chip area `$FF:0000-03FF` (`VideoPage`, with the compat switch).
//!
//! A VDC row value is 9 bits: bit 8 sprite, bits 7-4 palette, bits 3-0 colour; a colour of 0 is
//! transparent. "Priority between backgrounds and sprites is managed internally to the VDC": the
//! VPC sees only the bit-8 sprite flag and the transparency, so it cannot tell a high-priority
//! sprite from a low one.
//!
//! Everything the sources do not settle is a **named switch** with a provisional default and its
//! tier, so a later judge changes one line (the reference's `docs/m6-record.md` lists them as U/C
//! entries). The reference's class attributes are fields here: `Vpc::priority_labels`,
//! `Vpc::window_origin`, `Vpc::window_off_at_or_below`, `VideoPage::st_steers_in_compat`.
//!
//! **Where Rust forced a difference** (docs/rust-differences.md): the reference's `Vpc` and
//! `VideoPage` are two objects sharing state; here `VideoPage` is the board's `PixelPath`, owns
//! the `Vpc` and reaches both VDCs through the `BoardPins` handed in with every access, each VDC
//! access made with `pins.vdc_read` / `vdc_write` so that the base board's VDC wait can stop and
//! repeat it. `store_immediate` is `pins.opcode`. The mid-line dot's ceiling uses the base's
//! `py_floordiv`, and window edges are `i64`, so that a switch that makes one negative behaves as
//! in Python.

use std::any::Any;

use pcengine_rust::machine::{py_floordiv, HARDWARE_FIRST_PAGE, HSYNC_CLOCKS, LINE_CLOCKS};
use pcengine_rust::sockets::{BoardPins, DeviceState, PixelPath, ResetKind, VideoTiming};
use serde::{Deserialize, Serialize};

/// bit 8 of a VDC row value: the pixel came from a sprite
pub const SPRITE: u16 = 0x100;
/// bits 3-0: 0 is transparent
pub const COLOUR: u16 = 0x0F;

// Register offsets inside the VPC block ($FF:0008-000F, native mode).
/// bits 7-4 window 2 only, bits 3-0 windows 1 and 2 overlapping
pub const PRIORITY_A: u8 = 0x08;
/// bits 7-4 no window, bits 3-0 window 1 only
pub const PRIORITY_B: u8 = 0x09;
pub const WINDOW1_LSB: u8 = 0x0A;
pub const WINDOW1_MSB: u8 = 0x0B;
pub const WINDOW2_LSB: u8 = 0x0C;
pub const WINDOW2_MSB: u8 = 0x0D;
/// bit 0: ST0-ST2 go to VDC 2
pub const ST_STEER: u8 = 0x0E;

/// [hw: HuC6280 hardware manual] ST0, ST1, ST2 are opcodes $03, $13, $23.
pub const ST_OPCODES: [u8; 3] = [0x03, 0x13, 0x23];

/// C-SGX1: the two readings of the 2-bit priority codes (`Vpc::priority_labels`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriorityLabels {
    /// The default: codes 01b and 10b as the detectors (Mesen2, Beetle PCE) draw them
    /// [OBSERVED, closed box].
    Detectors,
    /// sgxtech's labels [DOC]: the two codes swapped.
    Sgxtech,
}

/// The window origin per dot divider (U-SGX1; the reference's `WINDOW_ORIGIN` dict, keyed by the
/// divider: 4 = 5.37 MHz, 3 = 7.16 MHz, 2 = 10.7 MHz).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowOrigin {
    /// divider 4, 5.37 MHz
    pub divider4: i64,
    /// divider 3, 7.16 MHz
    pub divider3: i64,
    /// divider 2, 10.7 MHz
    pub divider2: i64,
}

impl WindowOrigin {
    /// `WINDOW_ORIGIN[div]`; another divider is a `KeyError` in the reference, a panic here.
    pub fn of(&self, div: i64) -> i64 {
        match div {
            4 => self.divider4,
            3 => self.divider3,
            2 => self.divider2,
            _ => panic!("no window origin for dot divider {div}"),
        }
    }
}

/// Registers, ST steering and the mix. `mix(timing, rows)` is the PixelPath's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vpc {
    /// C-SGX1 (undecidable without hardware): which 2-bit priority code means what. [DOC sgxtech]
    /// 01b = "VDC #1 sprites behind the VDC #2 background" and 10b = "VDC #2 sprites in front of
    /// the VDC #1 background". [OBSERVED, closed box] Mesen2 and Beetle PCE draw Dai Makai Mura
    /// (Ghouls 'n Ghosts), which sets code 01b, as if the two were the other way round, and under
    /// sgxtech's labels the game's statues and score display go missing. A shipped game's picture
    /// outranks the document's labels, so the default follows the detectors; `Sgxtech` swaps the
    /// two codes. Tier: two detectors + a game's picture (5), against tier 2 text.
    /// Decided 2026-11-12 (Aubrey): this default stays. Past the title Dai Makai Mura's cutscene
    /// and stage 1 lose their statues and score display under `Sgxtech` and are row-identical to
    /// both detectors under `Detectors` (the reference's docs/m6-record.md); still no console run.
    pub priority_labels: PriorityLabels,

    /// U-SGX1: where the window edges fall. A window of width W covers the row's dots
    /// [0, W - $40 + origin), dots counted from the end of horizontal sync as `mix` counts them.
    /// [DOC sgxtech "Windows"] the windows start at the leftmost pixel of the physical screen,
    /// which is a width of $0040, and are set in single pixels. The origin per dot clock (divider
    /// 4 = 5.37 MHz, 3 = 7.16, 2 = 10.7) is a closed-box measurement (emulator tier: Beetle PCE
    /// b96c11e and Mesen2 b9fa69d on the reference's scripts/sgx_probe.py window cards), retaken
    /// 2026-11-19 on base board pcengine-python 3318027, whose display starts 8 dots earlier in
    /// the row: both 24 at 5.37 and 10.7 MHz; at 7.16 MHz Beetle PCE 38 and Mesen2 16. 38 is
    /// kept, provisional. (On base board 4c32f26 the same probes gave 32 / 46 (Beetle), 24
    /// (Mesen2) / 32: the detectors' edges stayed put against the picture, the picture moved in
    /// the row.) No library card brings a window edge into view. The measurement is in this
    /// board's row dots, so it is retaken whenever the base board's line geometry moves.
    pub window_origin: WindowOrigin,

    /// U-SGX2: a window width at or below this disables the window (sgxtech, tier 2).
    pub window_off_at_or_below: i64,

    // U-SGX3: the dot, counted from the end of the line's horizontal sync, at which a register
    // write takes effect is the first dot whose time is at or after the write (the same rule as
    // the VCE's palette). A choice of this model, not measured; not a switch.
    pub priority: [u8; 2],
    pub window: [u16; 2],
    pub st_to_vdc2: bool,
    shadow_priority: [u8; 2],
    shadow_window: [u16; 2],
    /// (mclk, register, value) of every write that changes the picture, drained by `mix`
    log: Vec<(i64, u8, u8)>,
}

impl Default for Vpc {
    fn default() -> Self {
        Vpc::new()
    }
}

impl Vpc {
    pub fn new() -> Vpc {
        Vpc {
            priority_labels: PriorityLabels::Detectors,
            window_origin: WindowOrigin {
                divider4: 24,
                divider3: 38,
                divider2: 24,
            },
            window_off_at_or_below: 0x40,
            priority: [0x11, 0x11],
            window: [0, 0],
            st_to_vdc2: false,
            shadow_priority: [0x11, 0x11],
            shadow_window: [0, 0],
            log: Vec::new(),
        }
    }

    /// [hw: sgxtech] power-up: `$11 $11 $00 $00 $00 $00 $00 $00` in `$08-$0F`: windows off,
    /// store-immediate to VDC 1, and only VDC 1 enabled with the default priority everywhere.
    pub fn reset(&mut self) {
        self.priority = [0x11, 0x11];
        self.window = [0, 0];
        self.st_to_vdc2 = false;
        self.shadow_priority = [0x11, 0x11];
        self.shadow_window = [0, 0];
        self.log.clear();
    }

    // -- the CPU's port -----------------------------------------------------------------------

    /// `$08-$0F`. Both priority registers and the window LSBs return what was written; the
    /// window MSBs keep only bits 1-0; `$0E` and `$0F` read $00 [hw: sgxtech].
    pub fn read(&self, offset: u32) -> u8 {
        match (offset & 0x0F) as u8 {
            PRIORITY_A => self.priority[0],
            PRIORITY_B => self.priority[1],
            WINDOW1_LSB => (self.window[0] & 0xFF) as u8,
            WINDOW1_MSB => (self.window[0] >> 8) as u8,
            WINDOW2_LSB => (self.window[1] & 0xFF) as u8,
            WINDOW2_MSB => (self.window[1] >> 8) as u8,
            _ => 0x00,
        }
    }

    pub fn write(&mut self, offset: u32, value: u8, mclk: i64) {
        let reg = (offset & 0x0F) as u8;
        match reg {
            PRIORITY_A => self.priority[0] = value,
            PRIORITY_B => self.priority[1] = value,
            WINDOW1_LSB => self.window[0] = (self.window[0] & 0x300) | value as u16,
            WINDOW1_MSB => self.window[0] = (self.window[0] & 0x0FF) | ((value as u16 & 3) << 8),
            WINDOW2_LSB => self.window[1] = (self.window[1] & 0x300) | value as u16,
            WINDOW2_MSB => self.window[1] = (self.window[1] & 0x0FF) | ((value as u16 & 3) << 8),
            ST_STEER => {
                self.st_to_vdc2 = value & 1 != 0;
                return;
            }
            _ => return, // $0F is unused
        }
        self.log.push((mclk, reg, value));
    }

    // -- the mix ------------------------------------------------------------------------------

    /// The PixelPath: VDC 1's and VDC 2's rows for one line in, the row for the VCE out.
    ///
    /// A register written during the line takes effect at the first dot whose time is at or
    /// after the write, as the VCE's palette does (`Capture`): the line is mixed in pieces.
    pub fn mix(&mut self, timing: &VideoTiming, rows: &[Vec<u16>]) -> Vec<u16> {
        let (vdc1, vdc2) = (&rows[0][..], &rows[1][..]);
        let dots = vdc1.len() as i64;
        let div = timing.dot_divider;
        let first = timing.start_mclk + HSYNC_CLOCKS;
        let end = first + LINE_CLOCKS;
        let mut used = 0;
        while used < self.log.len() && self.log[used].0 < end {
            used += 1;
        }
        let mut out = Vec::with_capacity(vdc1.len());
        if used == 0 {
            self.span(&mut out, vdc1, vdc2, 0, dots, div);
            return out;
        }
        let mut x = 0;
        for index in 0..used {
            let (when, reg, value) = self.log[index];
            // ceil((when - first) / div)
            let stop = dots.min(x.max(-py_floordiv(first - when, div)));
            if stop > x {
                self.span(&mut out, vdc1, vdc2, x, stop, div);
                x = stop;
            }
            self.replay(reg, value);
        }
        self.span(&mut out, vdc1, vdc2, x, dots, div);
        self.log.drain(..used);
        out
    }

    fn replay(&mut self, reg: u8, value: u8) {
        match reg {
            PRIORITY_A => self.shadow_priority[0] = value,
            PRIORITY_B => self.shadow_priority[1] = value,
            WINDOW1_LSB => self.shadow_window[0] = (self.shadow_window[0] & 0x300) | value as u16,
            WINDOW1_MSB => {
                self.shadow_window[0] = (self.shadow_window[0] & 0x0FF) | ((value as u16 & 3) << 8)
            }
            WINDOW2_LSB => self.shadow_window[1] = (self.shadow_window[1] & 0x300) | value as u16,
            _ => {
                self.shadow_window[1] = (self.shadow_window[1] & 0x0FF) | ((value as u16 & 3) << 8)
            }
        }
    }

    /// The first row dot outside a window of `width` (U-SGX1); 0 when the window is off
    /// (U-SGX2: a width at or below $40 [DOC sgxtech]), at most `dots`.
    pub fn window_edge(&self, width: u16, div: i64, dots: i64) -> i64 {
        if width as i64 <= self.window_off_at_or_below {
            return 0;
        }
        dots.min(width as i64 - 0x40 + self.window_origin.of(div))
    }

    /// The 4-bit group for a window area, from the registers as the line has replayed them.
    pub fn region_config(&self, in_window1: bool, in_window2: bool) -> u8 {
        let [a, b] = self.shadow_priority;
        match (in_window1, in_window2) {
            (true, true) => a & 0x0F,
            (false, true) => a >> 4,
            (true, false) => b & 0x0F,
            (false, false) => b >> 4,
        }
    }

    fn span(&self, out: &mut Vec<u16>, vdc1: &[u16], vdc2: &[u16], x0: i64, x1: i64, div: i64) {
        let dots = vdc1.len() as i64;
        let edge1 = self.window_edge(self.shadow_window[0], div, dots);
        let edge2 = self.window_edge(self.shadow_window[1], div, dots);
        // sorted({x0, x1, the edges strictly between them})
        let mut cuts = vec![x0, x1];
        for edge in [edge1, edge2] {
            if x0 < edge && edge < x1 {
                cuts.push(edge);
            }
        }
        cuts.sort_unstable();
        cuts.dedup();
        for pair in cuts.windows(2) {
            let (lo, hi) = (pair[0], pair[1]);
            let cfg = self.region_config(lo < edge1, lo < edge2);
            let (lo, hi) = (lo as usize, hi as usize);
            self.mix_run(out, &vdc1[lo..hi], &vdc2[lo..hi], cfg);
        }
    }

    /// Mix a run of dots of one window area: VDC 1's values `a`, VDC 2's `b`, the area's
    /// 4-bit group `cfg` (bit 0 VDC 1 on, bit 1 VDC 2 on, bits 3-2 the priority code), onto `out`.
    fn mix_run(&self, out: &mut Vec<u16>, a: &[u16], b: &[u16], cfg: u8) {
        let n = a.len();
        if cfg & 3 == 0 {
            // [DOC sgxtech] both layers off: colour 0, the overscan too
            out.resize(out.len() + n, 0);
            return;
        }
        if cfg & 2 == 0 {
            out.extend_from_slice(a); // VDC 1 alone, its transparent values included
            return;
        }
        if cfg & 1 == 0 {
            out.extend_from_slice(b);
            return;
        }
        let mut code = (cfg >> 2) & 3;
        if self.priority_labels == PriorityLabels::Sgxtech && (code == 1 || code == 2) {
            code ^= 3; // C-SGX1: the document's labels
        }
        assert_eq!(n, b.len()); // the reference's zip(strict=True)
        for (&p, &q) in a.iter().zip(b) {
            let mut front = p;
            if p & COLOUR == 0 {
                // U-SGX5: VDC 1 transparent, VDC 2's value whatever it is, so both transparent
                // shows VDC 2's backdrop code. Closed-box measurement (Mesen2 b9fa69d, Beetle PCE
                // b96c11e, the reference's scripts/sgx_probe.py clear-* cards, every dot clock);
                // MAME huc6202.cpp (BSD-3-Clause) the same. sgxtech silent.
                front = q;
            } else if code == 1 {
                // (detectors' 01b) VDC 2 sprites in front of VDC 1's background, behind its sprites
                if q & COLOUR != 0 && q & SPRITE != 0 && p & SPRITE == 0 {
                    front = q;
                }
            } else if code == 2 && p & SPRITE != 0 && q & COLOUR != 0 && q & SPRITE == 0 {
                // (detectors' 10b) VDC 1 sprites behind VDC 2's background, in front of its
                // sprites; [DOC sgxtech] VDC 1 shows only the sprite there, so its background
                // has a sprite-shaped hole
                front = q;
            }
            out.push(front);
        }
    }
}

/// What of the VPC and the page is in a save state (the reference's walk of both objects).
#[derive(Serialize, Deserialize)]
struct State {
    priority: [u8; 2],
    window: [u16; 2],
    st_to_vdc2: bool,
    shadow_priority: [u8; 2],
    shadow_window: [u16; 2],
    log: Vec<(i64, u8, u8)>,
    compat: bool,
}

/// The video chip area, `$FF:0000-03FF` (four 256-byte pages): what the VPC decodes.
///
/// Native mode [hw: sgxtech, "HuC6280 memory map"], a 32-byte block repeated through the 1 KiB:
///
/// ```text
///     $00-$03 VDC 1   $04-$07 VDC 1 (mirror)   $08-$0F VPC   $10-$13 VDC 2
///     $14-$17 VDC 2 (mirror)   $18-$1F unused, reads $FF
/// ```
///
/// PC Engine compatibility mode (the switch on the back): VDC 1 alone, mirrored every 4 bytes, as
/// on a PC Engine, for games that use those mirrors. Whether the CPU's current write is an
/// `ST0`/`ST1`/`ST2` is `pins.opcode` (the VPC sees the instruction, not the address).
///
/// This is the board's `PixelPath`: the mix is `vpc`'s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoPage {
    pub vpc: Vpc,
    /// The back-panel switch: `true` is the PC Engine position.
    pub compat: bool,
    /// U-SGX4: whether `ST0`-`ST2` still follow `$0E` bit 0 with the switch in compatibility mode.
    /// `$0E` cannot be written in that mode (it is a VDC 1 mirror), so the bit is only ever left
    /// over from native mode; nothing documents it. Default: no steering in compatibility mode.
    pub st_steers_in_compat: bool,
}

impl VideoPage {
    pub fn new(vpc: Vpc, compat: bool) -> VideoPage {
        VideoPage {
            vpc,
            compat,
            st_steers_in_compat: false,
        }
    }
}

impl DeviceState for VideoPage {
    fn save_state(&self) -> Vec<u8> {
        let vpc = &self.vpc;
        bincode::serialize(&State {
            priority: vpc.priority,
            window: vpc.window,
            st_to_vdc2: vpc.st_to_vdc2,
            shadow_priority: vpc.shadow_priority,
            shadow_window: vpc.shadow_window,
            log: vpc.log.clone(),
            compat: self.compat,
        })
        .expect("the VPC's state serialises")
    }

    fn load_state(&mut self, data: &[u8]) -> Result<(), String> {
        let state: State =
            bincode::deserialize(data).map_err(|error| format!("the VPC's state: {error}"))?;
        let vpc = &mut self.vpc;
        vpc.priority = state.priority;
        vpc.window = state.window;
        vpc.st_to_vdc2 = state.st_to_vdc2;
        vpc.shadow_priority = state.shadow_priority;
        vpc.shadow_window = state.shadow_window;
        vpc.log = state.log;
        self.compat = state.compat;
        Ok(())
    }

    fn state_identity(&self) -> String {
        "supergrafx_rust VideoPage (HuC6202 VPC)".to_string()
    }
}

impl PixelPath for VideoPage {
    fn mix(&mut self, timing: &VideoTiming, rows: Vec<Vec<u16>>) -> Vec<u16> {
        self.vpc.mix(timing, &rows)
    }

    /// The VPC decodes the whole 1 KiB video chip area: four pages of the hardware page.
    fn claims(&self) -> Vec<(usize, usize)> {
        vec![(HARDWARE_FIRST_PAGE, HARDWARE_FIRST_PAGE + 3)]
    }

    fn read(&mut self, address: u32, mclk: i64, pins: &mut BoardPins) -> u8 {
        if self.compat {
            return pins.vdc_read(0, address & 3, mclk);
        }
        let offset = address & 0x1F;
        if offset < 0x08 {
            pins.vdc_read(0, offset & 3, mclk)
        } else if offset < 0x10 {
            self.vpc.read(offset)
        } else if offset < 0x18 {
            pins.vdc_read(1, offset & 3, mclk)
        } else {
            0xFF
        }
    }

    fn write(&mut self, address: u32, value: u8, mclk: i64, pins: &mut BoardPins) {
        let steered = self.vpc.st_to_vdc2 && (self.st_steers_in_compat || !self.compat);
        if steered && ST_OPCODES.contains(&pins.opcode) {
            pins.vdc_write(1, address & 3, value, mclk);
            return;
        }
        if self.compat {
            pins.vdc_write(0, address & 3, value, mclk);
            return;
        }
        let offset = address & 0x1F;
        if offset < 0x08 {
            pins.vdc_write(0, offset & 3, value, mclk);
        } else if offset < 0x10 {
            self.vpc.write(offset, value, mclk);
        } else if offset < 0x18 {
            pins.vdc_write(1, offset & 3, value, mclk);
        }
    }

    /// The reference's `SuperGrafx.reset`, after the board's own: a power-on resets the VPC.
    fn reset(&mut self, kind: ResetKind, _pins: &mut BoardPins) {
        if kind == ResetKind::PowerOn {
            self.vpc.reset();
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
