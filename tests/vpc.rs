// SPDX-License-Identifier: MIT
//! The VPC's registers and mix against sgxtech's text (tier 2), with synthetic VDC rows: the
//! reference's tests/test_vpc.py, test for test.
//!
//! A row value is 9 bits: bit 8 sprite, bits 7-4 palette, bits 3-0 colour (0 transparent).

use pcengine_rust::machine::HSYNC_CLOCKS;
use pcengine_rust::sockets::VideoTiming;
use supergrafx_rust::vpc::{PriorityLabels, Vpc, WindowOrigin};

const N: usize = 64; // dots in a synthetic row
const BG1: u16 = 0x011; // opaque background pixels of VDC 1 and VDC 2
const BG2: u16 = 0x022;
const SP1: u16 = 0x131; // opaque sprite pixels
const SP2: u16 = 0x142;
const CLEAR: u16 = 0x000;

fn timing(divider: i64) -> VideoTiming {
    VideoTiming {
        frame: 0,
        line: 20,
        start_mclk: 0,
        dot_divider: divider,
        frame_lines: 263,
        in_vsync: false,
    }
}

fn row(value: u16, n: usize) -> Vec<u16> {
    vec![value; n]
}

fn mix_at(vpc: &mut Vpc, a: &[u16], b: &[u16], divider: i64) -> Vec<u16> {
    vpc.mix(&timing(divider), &[a.to_vec(), b.to_vec()])
}

fn mix(vpc: &mut Vpc, a: &[u16], b: &[u16]) -> Vec<u16> {
    mix_at(vpc, a, b, 4)
}

/// The four nibbles: no window, window 1 only, window 2 only, both.
fn configure(vpc: &mut Vpc, none: u8, w1: u8, w2: u8, both: u8) {
    vpc.write(0x08, (w2 << 4) | both, 0);
    vpc.write(0x09, (none << 4) | w1, 0);
}

fn count(row: &[u16], value: u16) -> usize {
    row.iter().filter(|&&pixel| pixel == value).count()
}

// --- registers [hw: sgxtech "Video Priority Controller"] ---------------------------------------

#[test]
fn power_up_registers_are_11_11_then_zeros() {
    let vpc = Vpc::new();
    let read: Vec<u8> = (0x08..0x10).map(|r| vpc.read(r)).collect();
    assert_eq!(read, [0x11, 0x11, 0, 0, 0, 0, 0, 0]);
    assert!(!vpc.st_to_vdc2);
}

#[test]
fn registers_return_what_was_written_and_the_msbs_keep_two_bits() {
    let mut vpc = Vpc::new();
    for (reg, value, back) in [
        (0x08, 0xA5, 0xA5),
        (0x09, 0x5A, 0x5A),
        (0x0A, 0xCD, 0xCD),
        (0x0B, 0xFF, 0x03),
        (0x0C, 0x12, 0x12),
        (0x0D, 0xFE, 0x02),
    ] {
        vpc.write(reg, value, 0);
        assert_eq!(vpc.read(reg), back);
    }
    assert_eq!(vpc.window, [0x3CD, 0x212]);
}

#[test]
fn register_0e_steers_st_but_reads_zero_and_0f_is_unused() {
    let mut vpc = Vpc::new();
    vpc.write(0x0E, 0xFF, 0);
    assert!(vpc.st_to_vdc2 && vpc.read(0x0E) == 0);
    vpc.write(0x0E, 0xFE, 0);
    assert!(!vpc.st_to_vdc2);
    vpc.write(0x0F, 0xFF, 0);
    assert_eq!(vpc.read(0x0F), 0);
}

#[test]
fn reset_puts_the_power_up_values_back_and_keeps_the_switches() {
    let mut vpc = Vpc::new();
    vpc.priority_labels = PriorityLabels::Sgxtech;
    configure(&mut vpc, 0x2, 0x3, 0x7, 0xB);
    vpc.write(0x0A, 0x80, 0);
    vpc.write(0x0E, 0x01, 0);
    vpc.reset();
    let mut fresh = Vpc::new();
    fresh.priority_labels = PriorityLabels::Sgxtech;
    assert_eq!(vpc, fresh);
    // nothing of the writes before the reset is left for the next line
    assert_eq!(mix(&mut vpc, &row(BG1, N), &row(BG2, N)), row(BG1, N));
}

// --- priority and enables ----------------------------------------------------------------------

#[test]
fn default_order_vdc1_over_vdc2_where_vdc1_is_opaque() {
    let mut vpc = Vpc::new();
    configure(&mut vpc, 0x3, 0x3, 0x3, 0x3);
    let a = [BG1, CLEAR, SP1, CLEAR];
    let b = [BG2, BG2, BG2, SP2];
    assert_eq!(mix(&mut vpc, &a, &b), [BG1, BG2, SP1, SP2]);
}

#[test]
fn codes_00_and_11_are_both_the_default_order() {
    for code in [0x0, 0xC] {
        let mut vpc = Vpc::new();
        configure(&mut vpc, 0x3 | code, 0x3, 0x3, 0x3);
        let a = [BG1, SP1, CLEAR, CLEAR];
        let b = [SP2, BG2, SP2, BG2];
        assert_eq!(mix(&mut vpc, &a, &b), [BG1, SP1, SP2, BG2]);
    }
}

#[test]
fn one_layer_enabled_shows_only_that_vdc_and_none_shows_colour_zero() {
    let (a, b) = ([BG1, SP1, CLEAR], [BG2, SP2, BG2]);
    let mut vpc = Vpc::new();
    configure(&mut vpc, 0x1, 0x1, 0x1, 0x1);
    assert_eq!(mix(&mut vpc, &a, &b), a);
    configure(&mut vpc, 0x2, 0x1, 0x1, 0x1);
    assert_eq!(mix(&mut vpc, &a, &b), b);
    configure(&mut vpc, 0x0, 0x1, 0x1, 0x1);
    assert_eq!(mix(&mut vpc, &a, &b), [0, 0, 0]);
}

#[test]
fn both_off_shows_colour_zero_in_the_overscan_too() {
    let mut vpc = Vpc::new();
    configure(&mut vpc, 0x0, 0x1, 0x1, 0x1);
    let overscan = [0x100; 4]; // the VDC's backdrop/overscan code
    assert_eq!(mix(&mut vpc, &overscan, &overscan), [0, 0, 0, 0]);
}

/// U-SGX5: where both pixels are transparent the result is VDC 2's (its backdrop or border
/// code), under every priority code. Added here before the reference had a test of its own
/// (`both_transparent_shows_vdc2s_pixel` below is that test, transcribed).
#[test]
fn two_transparent_pixels_show_vdc2s() {
    for code in [0x0, 0x4, 0x8, 0xC] {
        let mut vpc = Vpc::new();
        configure(&mut vpc, 0x3 | code, 0x1, 0x1, 0x1);
        let a = [0x100, CLEAR, 0x030];
        let b = [CLEAR, 0x100, 0x100];
        assert_eq!(mix(&mut vpc, &a, &b), b);
    }
}

#[test]
fn vdc2_sprites_in_front_of_vdc1_background_behind_vdc1_sprites() {
    let mut vpc = Vpc::new();
    configure(&mut vpc, 0x3 | 0x4, 0x1, 0x1, 0x1); // priority bits 3-2 = 01
    let a = [BG1, BG1, SP1, CLEAR];
    let b = [SP2, BG2, SP2, SP2];
    assert_eq!(mix(&mut vpc, &a, &b), [SP2, BG1, SP1, SP2]);
}

/// The detectors' code 10b; sgxtech's text calls it 01b.
#[test]
fn vdc1_sprites_behind_vdc2_background_but_in_front_of_vdc2_sprites() {
    let mut vpc = Vpc::new();
    configure(&mut vpc, 0x3 | 0x8, 0x1, 0x1, 0x1); // priority bits 3-2 = 10
    let a = [SP1, SP1, SP1, BG1];
    let b = [BG2, SP2, CLEAR, BG2];
    // VDC1 sprite hidden by a VDC2 background pixel, kept over a VDC2 sprite or transparency
    assert_eq!(mix(&mut vpc, &a, &b), [BG2, SP1, SP1, BG1]);
}

#[test]
fn sgxtech_labels_swap_the_two_codes_and_nothing_else() {
    let a = [SP1, BG1, CLEAR];
    let b = [BG2, SP2, SP2];
    let mut by_detectors = Vpc::new();
    configure(&mut by_detectors, 0x3 | 0x4, 0x1, 0x1, 0x1);
    let mut by_text = Vpc::new();
    by_text.priority_labels = PriorityLabels::Sgxtech;
    configure(&mut by_text, 0x3 | 0x8, 0x1, 0x1, 0x1);
    assert_eq!(mix(&mut by_text, &a, &b), mix(&mut by_detectors, &a, &b));
    // and the other code, which the two test rows above tell apart
    let mut other_detectors = Vpc::new();
    configure(&mut other_detectors, 0x3 | 0x8, 0x1, 0x1, 0x1);
    let mut other_text = Vpc::new();
    other_text.priority_labels = PriorityLabels::Sgxtech;
    configure(&mut other_text, 0x3 | 0x4, 0x1, 0x1, 0x1);
    let swapped = mix(&mut other_text, &a, &b);
    assert_eq!(swapped, mix(&mut other_detectors, &a, &b));
    assert_ne!(swapped, mix(&mut by_text, &a, &b));
    // the default order is untouched by the switch
    let mut by_text2 = Vpc::new();
    by_text2.priority_labels = PriorityLabels::Sgxtech;
    configure(&mut by_text2, 0x3, 0x1, 0x1, 0x1);
    assert_eq!(mix(&mut by_text2, &a, &b), [SP1, BG1, SP2]);
}

// --- windows [hw: sgxtech "Windows"] -----------------------------------------------------------

/// The window register value whose right edge falls on row dot `edge` at 5.37 MHz.
fn width_for(edge: u8) -> u8 {
    0x40 + edge - Vpc::new().window_origin.divider4 as u8
}

/// $08 bits 7-4 window 2 only, bits 3-0 overlap; $09 bits 7-4 no window, bits 3-0 window 1.
#[test]
fn the_four_areas_take_their_own_nibbles() {
    let mut vpc = Vpc::new();
    // none: VDC1, window 1 only: VDC2, window 2 only: both off, overlap: VDC1 and VDC2 default
    configure(&mut vpc, 0x1, 0x2, 0x0, 0x3);
    vpc.write(0x0A, width_for(60), 0); // window 1 ends at dot 60
    vpc.write(0x0C, width_for(40), 0); // window 2 ends at dot 40
    let out = mix(&mut vpc, &row(BG1, N), &row(BG2, N));
    assert_eq!(out[..40], row(BG1, 40)); // both windows: the overlap nibble, default order, VDC1 opaque
    assert_eq!(out[40..60], row(BG2, 20)); // window 1 only
    assert_eq!(out[60..], row(BG1, N - 60)); // no window
                                             // and window 2 alone, with window 1 shorter
    vpc.write(0x0A, width_for(40), 0);
    vpc.write(0x0C, width_for(60), 0);
    let out = mix(&mut vpc, &row(BG1, N), &row(BG2, N));
    assert_eq!(out[..40], row(BG1, 40));
    assert_eq!(out[40..60], row(0, 20));
    assert_eq!(out[60..], row(BG1, N - 60));
}

#[test]
fn a_window_of_0x40_or_less_is_disabled() {
    for width in [0x00, 0x20, 0x3F, 0x40] {
        let mut vpc = Vpc::new();
        configure(&mut vpc, 0x1, 0x2, 0x1, 0x1);
        vpc.write(0x0A, width, 0);
        assert_eq!(mix(&mut vpc, &row(BG1, N), &row(BG2, N)), row(BG1, N));
    }
}

#[test]
fn the_width_that_disables_a_window_is_a_switch() {
    let mut vpc = Vpc::new();
    vpc.window_off_at_or_below = 0x60;
    configure(&mut vpc, 0x1, 0x2, 0x1, 0x1);
    vpc.write(0x0A, 0x60, 0);
    assert_eq!(mix(&mut vpc, &row(BG1, 128), &row(BG2, 128)), row(BG1, 128));
    vpc.write(0x0A, 0x61, 0);
    let out = mix(&mut vpc, &row(BG1, 128), &row(BG2, 128));
    assert_eq!(
        count(&out, BG2),
        0x21 + Vpc::new().window_origin.divider4 as usize
    );
}

#[test]
fn window_3ff_covers_the_whole_row() {
    let mut vpc = Vpc::new();
    configure(&mut vpc, 0x1, 0x2, 0x1, 0x1);
    vpc.write(0x0A, 0xFF, 0);
    vpc.write(0x0B, 0x03, 0);
    assert_eq!(mix(&mut vpc, &row(BG1, N), &row(BG2, N)), row(BG2, N));
}

#[test]
fn one_pixel_steps_move_the_edge_by_one_dot() {
    let mut out = Vec::new();
    for width in [0x60, 0x61, 0x62] {
        let mut vpc = Vpc::new();
        configure(&mut vpc, 0x1, 0x2, 0x1, 0x1);
        vpc.write(0x0A, width, 0);
        out.push(count(&mix(&mut vpc, &row(BG1, 128), &row(BG2, 128)), BG2));
    }
    let origin = Vpc::new().window_origin.divider4 as usize;
    assert_eq!(out, [0x20 + origin, 0x21 + origin, 0x22 + origin]); // width - 0x40 + origin
}

/// U-SGX1: closed-box measurement (emulator tier) on the reference's scripts/sgx_probe.py,
/// retaken 2026-11-19 on base board 3318027 (display origin 8 dots earlier in the row): Beetle PCE
/// b96c11e at all three clocks; Mesen2 b9fa69d agrees at 5.37 and 10.7 MHz, 16 at 7.16 MHz.
#[test]
fn the_edge_origin_follows_the_dot_clock() {
    for (divider, origin) in [(4, 24), (3, 38), (2, 24)] {
        let mut vpc = Vpc::new();
        configure(&mut vpc, 0x1, 0x2, 0x1, 0x1);
        vpc.write(0x0A, 0x50, 0);
        let out = mix_at(&mut vpc, &row(BG1, 128), &row(BG2, 128), divider);
        assert_eq!(count(&out, BG2), 0x10 + origin);
    }
}

/// The reference's `WINDOW_ORIGIN` is a class attribute a subclass replaces; here a field.
#[test]
fn the_window_origin_is_a_switch() {
    let mut vpc = Vpc::new();
    vpc.window_origin = WindowOrigin {
        divider4: 40,
        divider3: 40,
        divider2: 40,
    };
    configure(&mut vpc, 0x1, 0x2, 0x1, 0x1);
    vpc.write(0x0A, 0x50, 0);
    let out = mix(&mut vpc, &row(BG1, 128), &row(BG2, 128));
    assert_eq!(count(&out, BG2), 0x10 + 40);
}

// --- both pixels transparent (U-SGX5) ----------------------------------------------------------

/// U-SGX5: closed-box measurement (emulator tier): on the reference's scripts/sgx_probe.py clear-*
/// cards Mesen2 b9fa69d and Beetle PCE b96c11e show VDC 2's value where VDC 1 is in its border
/// ($100) over VDC 2's transparent display ($000), and where VDC 1's display is transparent over
/// VDC 2's border, at every dot clock. sgxtech is silent.
#[test]
fn both_transparent_shows_vdc2s_pixel() {
    let (border, clear1, clear2) = (0x100, 0x020, 0x0A0); // transparent values, different codes
    let a = [border, clear1, clear1, border];
    let b = [clear2, border, clear2, border];
    for code in [0x0, 0x4, 0x8, 0xC] {
        for labels in [PriorityLabels::Detectors, PriorityLabels::Sgxtech] {
            let mut vpc = Vpc::new();
            vpc.priority_labels = labels;
            configure(&mut vpc, 0x3 | code, 0x1, 0x1, 0x1);
            assert_eq!(mix(&mut vpc, &a, &b), b);
        }
    }
}

// --- writes during the line --------------------------------------------------------------------

#[test]
fn a_register_written_mid_line_takes_effect_at_that_dot() {
    let mut vpc = Vpc::new();
    configure(&mut vpc, 0x1, 0x1, 0x1, 0x1);
    let when = HSYNC_CLOCKS + 10 * 4; // the write lands as dot 10 starts at 5 MHz
    vpc.write(0x09, 0x20, when); // no-window nibble -> VDC 2 only
    let out = mix(&mut vpc, &row(BG1, N), &row(BG2, N));
    assert_eq!(out[..10], row(BG1, 10));
    assert_eq!(out[10..], row(BG2, N - 10));
    // and it stays: the next line starts from the new state with nothing left in the log
    assert_eq!(mix(&mut vpc, &row(BG1, N), &row(BG2, N)), row(BG2, N));
}

/// A write a master clock after a dot's start shows from the next dot (the ceiling).
#[test]
fn a_write_inside_a_dot_takes_effect_at_the_next() {
    let mut vpc = Vpc::new();
    configure(&mut vpc, 0x1, 0x1, 0x1, 0x1);
    vpc.write(0x09, 0x20, HSYNC_CLOCKS + 10 * 4 + 1);
    let out = mix(&mut vpc, &row(BG1, N), &row(BG2, N));
    assert_eq!(out[..11], row(BG1, 11));
    assert_eq!(out[11..], row(BG2, N - 11));
}

/// A window register written mid-line moves the edge for the rest of the line only.
#[test]
fn a_window_written_mid_line_cuts_the_rest_of_the_line() {
    let mut vpc = Vpc::new();
    configure(&mut vpc, 0x1, 0x2, 0x1, 0x1);
    vpc.write(0x0A, width_for(50), HSYNC_CLOCKS + 20 * 4);
    let out = mix(&mut vpc, &row(BG1, N), &row(BG2, N));
    assert_eq!(out[..20], row(BG1, 20)); // before the write: no window
    assert_eq!(out[20..50], row(BG2, 30)); // window 1, from the write to its edge
    assert_eq!(out[50..], row(BG1, N - 50));
}

#[test]
fn a_write_in_the_next_line_does_not_leak_into_this_one() {
    let mut vpc = Vpc::new();
    configure(&mut vpc, 0x1, 0x1, 0x1, 0x1);
    vpc.write(0x09, 0x20, 1365 + HSYNC_CLOCKS);
    assert_eq!(mix(&mut vpc, &row(BG1, N), &row(BG2, N)), row(BG1, N));
}
