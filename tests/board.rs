// SPDX-License-Identifier: MIT
//! The SuperGrafx through the base board's sockets: decode, RAM, IRQ1, ST steering, reset.
//! The reference's tests/test_board.py.
//!
//! Differences forced by Rust: the reference's `ScriptedCpu` sets the opcode it wants the board
//! to see with each write. The base Rust board takes the opcode from the core's `Pins`, which a
//! test cannot set, so the ST tests drive the page itself with `sockets::DetachedPins` (two
//! VDCs and an opcode, no board) and, through the board, the real core (tests/core_st.rs). The
//! reference's test of its fallback from `cpu.opcode` to `cpu._opcode` has no counterpart: there
//! is one `pins.opcode`.

use pcengine_rust::board::{Board, BoardOptions};
use pcengine_rust::fake_devices::ScriptedCpu;
use pcengine_rust::machine::{HARDWARE_BASE, LINE_CLOCKS};
use pcengine_rust::sockets::{DetachedPins, PixelPath, ResetKind};
use pcengine_rust::vdc::Vdc;
use supergrafx_rust::{video_page, SuperGrafx, SuperGrafxOptions, VideoPage, Vpc};

const VIDEO: u32 = HARDWARE_BASE; // $FF:0000
const VDC1: u32 = VIDEO;
const VPC: u32 = VIDEO + 0x08;
const VDC2: u32 = VIDEO + 0x10;

fn machine(compat: bool) -> ScriptedCpu {
    let sg = SuperGrafx::new(SuperGrafxOptions {
        compat,
        ..SuperGrafxOptions::default()
    })
    .unwrap();
    ScriptedCpu::new(sg.board, false)
}

fn page(cpu: &mut ScriptedCpu) -> &mut VideoPage {
    video_page(&mut cpu.board).unwrap()
}

/// Select a VDC register and write its 16 bits through the bus at `base` (a VDC's port 0).
fn vdc_reg(cpu: &mut ScriptedCpu, base: u32, register: u8, value: u16) {
    cpu.write(base, register);
    cpu.write(base + 2, (value & 0xFF) as u8);
    cpu.write(base + 3, (value >> 8) as u8);
}

/// The 256x224 mode of pcetech 7 (see the base board's tests).
fn standard_display(cpu: &mut ScriptedCpu, base: u32) {
    vdc_reg(cpu, base, 0x0A, 0x0202); // HSR
    vdc_reg(cpu, base, 0x0B, 0x041F); // HDR
    vdc_reg(cpu, base, 0x0C, 0x1702); // VPR
    vdc_reg(cpu, base, 0x0D, 0x00DF); // VDW
    vdc_reg(cpu, base, 0x0E, 0x000C); // VCR
    vdc_reg(cpu, base, 0x05, 0x00C8); // CR
}

// --- the decode [hw: sgxtech "HuC6280 memory map"] ---------------------------------------------

#[test]
fn native_decode_maps_two_vdcs_and_the_vpc_in_a_32_byte_block() {
    let mut cpu = machine(false);
    cpu.write(VDC1, 0x07); // VDC 1 register select
    cpu.write(VDC2, 0x09); // VDC 2 register select
    assert_eq!(cpu.board.vdcs[0].regs.select, 0x07);
    assert_eq!(cpu.board.vdcs[1].regs.select, 0x09);
    cpu.write(VIDEO + 0x04, 0x0A); // $04-$07: VDC 1 mirror
    cpu.write(VIDEO + 0x14, 0x0B); // $14-$17: VDC 2 mirror
    assert_eq!(cpu.board.vdcs[0].regs.select, 0x0A);
    assert_eq!(cpu.board.vdcs[1].regs.select, 0x0B);
    cpu.write(VPC, 0x5A);
    assert_eq!(cpu.read(VPC), 0x5A);
    assert_eq!(page(&mut cpu).vpc.priority[0], 0x5A);
}

#[test]
fn the_block_repeats_every_32_bytes_through_the_1k_area() {
    let mut cpu = machine(false);
    for k in 0..32u32 {
        cpu.write(VIDEO + 32 * k + 0x10, (k & 0x1F) as u8);
        assert_eq!(cpu.board.vdcs[1].regs.select, (k & 0x1F) as u8);
    }
    cpu.write(VIDEO + 0x3E8, 0x99); // $3E8 = 31*32 + 8: the VPC priority register again
    assert_eq!(page(&mut cpu).vpc.priority[0], 0x99);
}

#[test]
fn the_gap_18_to_1f_reads_ff_and_ignores_writes() {
    let mut cpu = machine(false);
    for offset in 0x18..0x20 {
        assert_eq!(cpu.read(VIDEO + offset), 0xFF);
        cpu.write(VIDEO + offset, 0x00);
    }
    assert_eq!(cpu.board.vdcs[0].regs.select, 0);
    assert_eq!(cpu.board.vdcs[1].regs.select, 0);
}

#[test]
fn every_vpc_register_reads_back_through_the_bus() {
    let mut cpu = machine(false);
    let written = [0xA5, 0x5A, 0xCD, 0xFF, 0x12, 0xFE, 0xFF, 0xFF];
    for (offset, value) in written.into_iter().enumerate() {
        cpu.write(VPC + offset as u32, value);
    }
    let read: Vec<u8> = (0..8).map(|offset| cpu.read(VPC + offset)).collect();
    assert_eq!(read, [0xA5, 0x5A, 0xCD, 0x03, 0x12, 0x02, 0x00, 0x00]);
    assert!(page(&mut cpu).vpc.st_to_vdc2);
}

#[test]
fn compat_switch_decodes_vdc1_alone_mirrored_every_four_bytes() {
    let mut cpu = machine(true);
    for offset in [0x08, 0x10, 0x14, 0x1C, 0x3FC] {
        cpu.write(VIDEO + offset, 0x06);
        assert_eq!(cpu.board.vdcs[0].regs.select, 0x06);
        cpu.write(VIDEO + offset, 0x00);
    }
    assert_eq!(cpu.board.vdcs[1].regs.select, 0);
    assert_eq!(page(&mut cpu).vpc.read(0x08), 0x11);
    page(&mut cpu).compat = false; // flipping the switch re-decodes
    cpu.write(VPC, 0x77);
    assert_eq!(page(&mut cpu).vpc.priority[0], 0x77);
}

#[test]
fn compat_reads_are_vdc1s_everywhere() {
    let mut cpu = machine(true);
    cpu.board.vdcs[0].status |= 0x20; // VD
                                      // the gap and the VPC's block are VDC 1's status register in PC Engine position
    assert_eq!(cpu.read(VIDEO + 0x18) & 0x20, 0x20);
    cpu.board.vdcs[0].status |= 0x20;
    assert_eq!(cpu.read(VIDEO + 0x08) & 0x20, 0x20);
    assert_eq!(cpu.board.vdcs[0].status & 0x20, 0); // and the read acknowledged it
}

// --- RAM and the bank map ----------------------------------------------------------------------

#[test]
fn work_ram_is_32k_in_banks_f8_to_fb_without_the_8k_mirror() {
    let mut cpu = machine(false);
    let banks = [0x1F0000, 0x1F2000, 0x1F4000, 0x1F6000];
    for (i, base) in banks.into_iter().enumerate() {
        cpu.write(base + 0x123, 0x10 + i as u8);
    }
    let read: Vec<u8> = banks.iter().map(|base| cpu.read(base + 0x123)).collect();
    assert_eq!(read, [0x10, 0x11, 0x12, 0x13]);
    assert_eq!(cpu.board.ram.data.len(), 32 * 1024);
}

#[test]
fn a_pc_engine_has_the_mirror_the_supergrafx_does_not() {
    let pce = Board::new(BoardOptions::default()).unwrap();
    let mut cpu = ScriptedCpu::new(pce, false);
    cpu.write(0x1F0123, 0x42);
    assert_eq!(cpu.read(0x1F2123), 0x42); // banks F8 and F9 are one 8 KiB
}

#[test]
fn unused_banks_read_ff() {
    let mut cpu = machine(false);
    // [hw: sgxtech] 80-87 and 88-F6, F7 and FC-FE return $FF on a stock machine
    for bank in [0x80u32, 0x87, 0x88, 0xF6, 0xF7, 0xFC, 0xFD, 0xFE] {
        assert_eq!(cpu.read(bank << 13), 0xFF);
    }
}

// --- the second VDC ----------------------------------------------------------------------------

#[test]
fn vdc2_has_its_own_vram() {
    let mut cpu = machine(false);
    vdc_reg(&mut cpu, VDC2, 0x00, 0x0100); // MAWR
    vdc_reg(&mut cpu, VDC2, 0x02, 0xBEEF); // VWR
                                           // the write is held until a CPU slot; run a few cycles so it commits
    cpu.idle(40);
    assert_eq!(cpu.board.vdcs[1].vram[0x0100], 0xBEEF);
    assert_eq!(cpu.board.vdcs[0].vram[0x0100], 0);
}

#[test]
fn irq1_is_the_or_of_both_vdcs_and_each_is_acknowledged_by_its_own_status_read() {
    let mut cpu = machine(false);
    standard_display(&mut cpu, VDC2); // only VDC 2 raises the vertical-blank interrupt
    for _ in 0..2 * 263 * LINE_CLOCKS / 12 {
        cpu.idle(1);
        if cpu.pins.irq_lines().0 {
            break;
        }
    }
    assert!(cpu.pins.irq_lines().0 && cpu.board.vdcs[1].irq1() && !cpu.board.vdcs[0].irq1());
    cpu.read(VDC1); // reading VDC 1's status does not clear VDC 2's request
    assert!(cpu.pins.irq_lines().0);
    let status = cpu.read(VDC2);
    assert!(status & 0x20 != 0 && !cpu.pins.irq_lines().0); // VD, now acknowledged
}

#[test]
fn both_vdcs_requesting_need_both_reads() {
    let mut cpu = machine(false);
    standard_display(&mut cpu, VDC1);
    standard_display(&mut cpu, VDC2);
    for _ in 0..2 * 263 * LINE_CLOCKS / 12 {
        cpu.idle(1);
        if cpu.board.vdcs[0].irq1() && cpu.board.vdcs[1].irq1() {
            break;
        }
    }
    assert!(cpu.pins.irq_lines().0);
    cpu.read(VDC1);
    assert!(cpu.pins.irq_lines().0); // still VDC 2
    cpu.read(VDC2);
    assert!(!cpu.pins.irq_lines().0);
}

// --- ST0/ST1/ST2 [hw: sgxtech "Store Immediate Control"] ---------------------------------------

const ST0: u8 = 0x03;
const ST1: u8 = 0x13;
const ST2: u8 = 0x23;
const STA: u8 = 0x8D;

/// The page with two VDCs and no board: a write made while the CPU runs `opcode`.
struct Bench {
    page: VideoPage,
    pins: DetachedPins,
}

impl Bench {
    fn new(compat: bool) -> Bench {
        Bench {
            page: VideoPage::new(Vpc::new(), compat),
            pins: DetachedPins {
                vdcs: vec![
                    Vdc::new(None, Some(-16), true),
                    Vdc::new(None, Some(-16), true),
                ],
                ..DetachedPins::default()
            },
        }
    }

    fn write(&mut self, address: u32, value: u8, opcode: u8) {
        self.pins.opcode = opcode;
        self.page.write(address, value, 0, &mut self.pins.pins());
    }

    fn select(&self, vdc: usize) -> u8 {
        self.pins.vdcs[vdc].regs.select
    }

    fn value(&self, vdc: usize, register: usize) -> u16 {
        self.pins.vdcs[vdc].regs.value[register]
    }
}

#[test]
fn st_goes_to_vdc2_only_while_0e_bit_0_is_set_and_only_for_st_instructions() {
    let mut bench = Bench::new(false);
    bench.write(0x1FE000, 0x05, ST0);
    assert_eq!(bench.select(0), 0x05);
    bench.write(VPC + 6, 0x01, STA); // $0E: ST to VDC 2
    bench.write(0x1FE000, 0x06, ST0);
    assert_eq!((bench.select(1), bench.select(0)), (0x06, 0x05));
    // a plain store to the same address still reaches VDC 1 ("regular access functions normally")
    bench.write(0x1FE000, 0x07, STA);
    assert_eq!((bench.select(0), bench.select(1)), (0x07, 0x06));
    // data ports: ST1 and ST2 (physical $1FE002 and $1FE003)
    bench.write(0x1FE000, 0x08, ST0); // VDC 2 register 8 is BYR
    bench.write(0x1FE002, 0x34, ST1);
    bench.write(0x1FE003, 0x01, ST2);
    assert_eq!((bench.value(1, 0x08), bench.value(0, 0x08)), (0x0134, 0));
    bench.write(VPC + 6, 0x00, STA);
    bench.write(0x1FE000, 0x08, ST0);
    bench.write(0x1FE002, 0x56, ST1);
    bench.write(0x1FE003, 0x00, ST2);
    assert_eq!(
        (bench.value(0, 0x08), bench.value(1, 0x08)),
        (0x0056, 0x0134)
    );
}

/// The VPC sees the instruction, not the address: a steered ST to the VPC's own block or to
/// VDC 2's goes to VDC 2's port by the low two address bits.
#[test]
fn a_steered_st_goes_to_vdc2_whatever_the_address_decodes_to() {
    let mut bench = Bench::new(false);
    bench.write(VPC + 6, 0x01, STA);
    bench.write(VPC, 0x0C, ST0); // $08: would be a priority register
    assert_eq!(bench.select(1), 0x0C);
    assert_eq!(bench.page.vpc.priority[0], 0x11);
    bench.write(VIDEO + 0x1C, 0x0D, ST0); // the gap
    assert_eq!(bench.select(1), 0x0D);
}

#[test]
fn st_steering_is_off_with_the_switch_in_compat_position() {
    let mut bench = Bench::new(false);
    bench.write(VPC + 6, 0x01, STA);
    bench.page.compat = true;
    bench.write(0x1FE000, 0x09, ST0);
    assert_eq!((bench.select(0), bench.select(1)), (0x09, 0));
    bench.page.st_steers_in_compat = true; // the named switch
    bench.write(0x1FE000, 0x0A, ST0);
    assert_eq!(bench.select(1), 0x0A);
}

/// The reference's `_store_immediate` answers False with no CPU; here the board hands opcode 0.
#[test]
fn with_no_cpu_nothing_is_steered() {
    let mut cpu = machine(false);
    cpu.write(VPC + 6, 0x01);
    cpu.write(0x1FE000, 0x09);
    assert_eq!(cpu.board.vdcs[0].regs.select, 0x09);
    assert_eq!(cpu.board.vdcs[1].regs.select, 0);
}

#[test]
fn the_machine_row_is_a_japanese_console_with_two_vdcs_and_32k() {
    let mut cpu = machine(false);
    assert!(cpu.board.machine.country_pc_engine);
    assert_eq!(cpu.board.machine, supergrafx_rust::SUPERGRAFX);
    assert_eq!(cpu.board.vdcs.len(), 2);
    assert!(cpu.read(0x1FE000 + 0x1000) & 0x40 != 0); // I/O port D6 = 1 (PC Engine compatible)
}

// --- the builder -------------------------------------------------------------------------------

#[test]
fn a_power_on_reset_resets_the_vpc_and_a_button_reset_does_not() {
    let mut sg = SuperGrafx::new(SuperGrafxOptions::default()).unwrap();
    sg.set_compat(true);
    sg.page().vpc.write(0x08, 0x75, 0);
    sg.page().vpc.write(0x0A, 0x90, 0);
    sg.page().vpc.write(0x0E, 0x01, 0);
    sg.reset(ResetKind::Button);
    assert_eq!(sg.page().vpc.priority, [0x75, 0x11]);
    assert!(sg.page().vpc.st_to_vdc2);
    sg.reset(ResetKind::PowerOn);
    assert_eq!(sg.page().vpc, Vpc::new());
    assert!(sg.compat()); // the switch on the back is not a register
}

#[test]
fn the_builder_takes_a_vpc_with_other_switches_and_refuses_another_pixel_path() {
    let mut vpc = Vpc::new();
    vpc.window_off_at_or_below = 0x10;
    let mut sg = SuperGrafx::new(SuperGrafxOptions {
        vpc: Some(vpc),
        st_steers_in_compat: true,
        ..SuperGrafxOptions::default()
    })
    .unwrap();
    assert_eq!(sg.page().vpc.window_off_at_or_below, 0x10);
    assert!(sg.page().st_steers_in_compat && !sg.compat());
    let mut options = SuperGrafxOptions::default();
    options.board.pixel_path = Some(Box::new(pcengine_rust::sockets::DirectPath));
    assert!(SuperGrafx::new(options).is_err());
    // any other board has no video page
    let mut plain = Board::new(BoardOptions::default()).unwrap();
    assert!(video_page(&mut plain).is_none());
}

#[test]
fn the_five_cards_are_known_by_their_images_and_others_by_their_names() {
    use supergrafx_rust::{is_supergrafx_card, supergrafx_title};
    let image = vec![0xEA; 0x2000];
    assert_eq!(supergrafx_title(&image), None);
    assert!(!is_supergrafx_card(&image, None));
    assert!(!is_supergrafx_card(
        &image,
        Some("Bonk's Adventure (USA).pce")
    ));
    assert!(is_supergrafx_card(&image, Some("Homebrew.SGX")));
}

// --- the base board's VDC wait through the page (pcengine-python issue 0004) -------------------

/// Not a test of the reference (its page calls the VDC, whose wait calls the board): here the
/// page makes each VDC access with `pins.vdc_read` / `vdc_write` so that the base board's wait
/// reaches it (docs/rust-differences.md). With MWR access mode 3 a VDC has no CPU slot in the
/// display width, so a VRAM read set up there (MARR) is served only after the width, and the
/// read of VRR that follows waits for it. Through either VDC the value read is the word in VRAM
/// and the CPU was held.
#[test]
fn a_vdc_read_that_has_to_wait_waits_through_the_page_on_either_vdc() {
    for (vdc, base) in [(0usize, VDC1), (1, VDC2)] {
        let mut cpu = machine(false);
        standard_display(&mut cpu, VDC1);
        standard_display(&mut cpu, VDC2);
        vdc_reg(&mut cpu, base, 0x09, 0x0003); // MWR: access mode 3
        cpu.board.vdcs[vdc].vram[0x100] = 0x1234;
        // into frame 1, line 40 (a display line), 600 master clocks after its start: in the width
        let to = LINE_CLOCKS * (263 + 40) + 600 - cpu.board.mclk;
        cpu.board.advance(to);
        vdc_reg(&mut cpu, base, 0x01, 0x0100); // MARR: the read is queued
        cpu.write(base, 0x02); // select VRR
        let before = cpu.board.mclk;
        let low = cpu.read(base + 2);
        let high = cpu.read(base + 3);
        assert_eq!((low, high), (0x34, 0x12), "VDC {}: the word read", vdc + 1);
        assert!(
            cpu.board.mclk > before + 100,
            "VDC {}: the CPU was held",
            vdc + 1
        );
    }
}
