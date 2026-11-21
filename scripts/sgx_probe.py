#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Build scratch/sgx-probe.pce: a synthetic 8 KiB HuCard that drives the VPC harder than the
five library cards' first seconds do, so the lockstep gate has a case on which every register,
every priority code, every window area, the mid-line rule, the ST steering both ways and each
named switch changes what the machine does.

Everything here is this repository's own: a few hundred bytes of HuC6280 code assembled with
the base Rust board's little assembler (``scripts/make_probe.py`` of ``$PCENGINE_RUST``, imported
by path) and data from a fixed generator. It is not a ROM of anything and is not committed
(scratch/ is ignored); the gate rebuilds it, byte for byte the same, when a case names
``$PROBE``.

What it does:

- **Both VDCs** are set up through plain stores at ``$FF:0000`` and ``$FF:0010`` (and their
  mirrors at ``$04`` and ``$14``): the same background and sprite patterns, their own sprite
  tables and scroll, so pixels of every kind (background, sprite, transparent) meet in the mix.
  Both raise the vertical-blank interrupt (VDC 1 the raster interrupt too) and the handler
  reads both status registers; after 1,536 turns of the main loop VDC 2's interrupts are
  switched off (so that the case that moves the back-panel switch still runs its main loop).
- **The VPC**: every turn of the free-running main loop writes both priority registers from a
  table of all 256 values, both window widths (window 1 sweeps all 10 bits), and reads
  ``$08-$0F``, the gap (``$18``, ``$1F``) and two mirrors of the block back. The loop's length
  varies, so the writes land on every dot of a line sooner or later (the mid-line rule).
- **ST steering**: with ``$0E`` bit 0 set, ``ST0``/``ST1``/``ST2`` set VDC 2's BXR; with it clear,
  VDC 1's BYR; plain stores to both VDCs in between, with the bit set. The bit is left set for
  most of the loop, which is what ``--compat-at`` and ``ST_STEERS_IN_COMPAT`` need.
- **VDC 2's VRAM** is read back through ``$12``/``$13`` at a moving address.
- **The VCE**: a full colour table, and the dot clock changed every 256 turns (5, 7, 10 MHz:
  the window origin differs by clock).
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = Path(os.environ.get("PCENGINE_RUST", ROOT.parent / "pcengine-rust"))
sys.dont_write_bytecode = True  # nothing is written into the base repository
sys.path.insert(0, str(BASE / "scripts"))
from make_probe import Asm, generator, words  # noqa: E402  (the base probe's assembler: tooling)

VDC1, VDC2 = 0x0000, 0x0010


def reg(a: Asm, base: int, register: int, value: int) -> None:
    """A VDC register through plain stores at `base` (STA and STZ, never ST0-ST2)."""
    a.store(base + 0, register)
    a.store(base + 2, value & 0xFF)
    a.store(base + 3, value >> 8)


def sprite_table(shift: int) -> bytes:
    entries = []
    for i in range(64):
        if i < 20:
            y = 64 + 24 + (i // 5) * 44 + shift
            x = 32 + 16 + (i % 5) * 46 + shift * 2
            # palette, SPBG (bit 7: in front of the VDC's own background), the second pattern
            flags = (i & 0xF) | (0x80 if i % 3 else 0)
            entries += [y, x, (0x80 + (i & 1)) << 1, flags]  # the patterns at VRAM $2000, $2040
        else:
            entries += [0, 0, 0, 0]
    return words(entries)


def build() -> bytes:
    a = Asm()
    rnd = generator(6202)

    # -- data ---------------------------------------------------------------------------------
    palette = a.here
    a.code.extend(words([(i * 73 + 21) & 0x1FF for i in range(256)]))
    bat = a.here
    a.code.extend(words([((i * 5 // 7) % 16) << 12 | 0x100 | (i * 7 + i // 32) % 16 for i in range(1024)]))
    tiles = a.here
    for tile in range(16):
        # tile 0 is all transparent; the others have holes (a mask shared by the four planes)
        for _half in range(2):
            for _row in range(8):
                mask = 0 if tile == 0 else next(rnd) | next(rnd)
                a.b(next(rnd) & mask, next(rnd) & mask)
    pattern = a.here
    for _sprite in range(2):
        masks = [next(rnd) | next(rnd) << 8 for _ in range(16)]
        for _plane in range(4):
            a.code.extend(words([(next(rnd) | next(rnd) << 8) & masks[row] for row in range(16)]))
    sat1 = a.here
    a.code.extend(sprite_table(0))
    sat2 = a.here
    a.code.extend(sprite_table(11))
    table1 = a.here
    a.code.extend(next(rnd) for _ in range(256))
    table2 = a.here
    a.code.extend(next(rnd) for _ in range(256))
    clocks = a.here
    a.b(0x00, 0x01, 0x02, 0x01)

    # -- handlers -----------------------------------------------------------------------------
    irq1 = a.here
    a.b(0x48)  # PHA
    a.lda_abs(VDC1)  # both status registers: each acknowledges its own VDC
    a.lda_abs(VDC2)
    a.b(0x68, 0x40)  # PLA, RTI
    other = a.here
    a.b(0x40)  # RTI

    # -- reset --------------------------------------------------------------------------------
    reset = a.here
    a.b(0x78, 0xD4, 0xD8)  # SEI, CSH, CLD
    a.tam(0x01, 0xFF)  # MPR0: the hardware page at $0000
    a.tam(0x02, 0xF8)  # MPR1: work RAM at $2000 (zero page and stack)
    a.b(0xA2, 0xFF, 0x9A)  # LDX #$FF, TXS
    a.b(0x64, 0x00, 0x64, 0x01)  # STZ $00, STZ $01: the loop's 16-bit counter
    a.store(0x1402, 0x05)  # IRQ2 and the timer masked
    a.store(0x0400, 0x00)  # VCE: 5 MHz
    a.stz(0x0402)
    a.stz(0x0403)
    a.tia(palette, 0x0404, 512)
    a.tia(table1, 0x0404, 512)  # the sprite half: other colours (table1 and table2)
    # VDC 2's display is 32 dots narrower: at its edges its border code meets VDC 1's transparent
    # pixels, the only place the two VDCs' transparent values differ (U-SGX5)
    for base, sat, bxr, byr, cr, hdr in (
        (VDC1, sat1, 0, 0, 0x00CC, 0x041F),
        (VDC2, sat2, 0x0035, 0x0013, 0x00C8, 0x061B),
    ):
        mirror = base + 4  # the second copy of the VDC in the block
        for address, data, length in ((0x0000, bat, 2048), (0x1000, tiles, 512), (0x2000, pattern, 256), (0x3000, sat, 512)):
            reg(a, mirror, 0x00, address)  # MAWR
            a.store(base + 0, 0x02)
            a.tia(data, base + 2, length)
        reg(a, base, 0x0A, 0x0202)  # HSR
        reg(a, base, 0x0B, hdr)  # HDR
        reg(a, base, 0x0C, 0x1702)  # VPR
        reg(a, base, 0x0D, 0x00DF)  # VDW
        reg(a, base, 0x0E, 0x000C)  # VCR
        reg(a, base, 0x09, 0x0000)  # MWR: a 32x32 BAT
        reg(a, base, 0x07, bxr)
        reg(a, base, 0x08, byr)
        reg(a, base, 0x06, 0x00A0)  # RCR
        reg(a, base, 0x0F, 0x0010)  # DCR: the sprite table is transferred every frame
        reg(a, base, 0x13, 0x3000)  # DVSSR
        reg(a, base, 0x05, cr)  # CR: background, sprites, interrupts
    a.store(0x0008, 0x33)
    a.store(0x0009, 0x33)
    a.b(0x58)  # CLI

    # -- the main loop ------------------------------------------------------------------------
    loop = a.here
    a.b(0xE6, 0x00, 0xD0, 0x02, 0xE6, 0x01)  # INC $00, BNE +2, INC $01
    a.b(0xA6, 0x00)  # LDX $00
    a.w(0xBD, table1)  # LDA table1,X
    a.sta(0x0009)
    a.w(0xBD, table2)
    a.sta(0x0008)
    a.b(0xA5, 0x00)  # LDA $00
    a.sta(0x000A)
    a.b(0xA5, 0x01)
    a.sta(0x000B)  # window 1: the counter's low 10 bits
    a.w(0xBD, table1)
    a.sta(0x002C)  # window 2, through the block's second copy
    a.b(0xA5, 0x00, 0x4A)  # LDA $00, LSR A
    a.sta(0x002D)
    for address in (*range(0x0008, 0x0010), 0x0018, 0x001F, 0x0029, 0x03EA):
        a.lda_abs(address)
    # ST0-ST2 to VDC 2: BXR
    a.store(0x000E, 0x01)
    a.b(0x03, 0x07)  # ST0 #7
    a.b(0xA5, 0x00, 0x29, 0x10, 0xF0, 0x04)  # LDA $00, AND #$10, BEQ +4
    a.b(0x13, 0x30, 0x80, 0x02)  # ST1 #$30, BRA +2
    a.b(0x13, 0x08)  # ST1 #$08
    a.b(0x23, 0x00)  # ST2 #0
    # ST0-ST2 to VDC 1: BYR
    a.stz(0x000E)
    a.b(0x03, 0x08)  # ST0 #8
    a.b(0xA5, 0x00, 0x29, 0x20, 0xF0, 0x04)  # LDA $00, AND #$20, BEQ +4
    a.b(0x13, 0x45, 0x80, 0x02)  # ST1 #$45, BRA +2
    a.b(0x13, 0x05)  # ST1 #$05
    a.b(0x23, 0x00)  # ST2 #0
    a.store(0x000E, 0x01)  # left set: plain stores below are not steered
    # plain stores: VDC 2's BYR, then VDC 2's VRAM read back at a moving address
    a.store(VDC2 + 0, 0x08)
    a.b(0xA5, 0x00)
    a.sta(VDC2 + 2)
    a.stz(VDC2 + 3)
    a.store(VDC2 + 4, 0x01)  # MARR, through the mirror
    a.b(0xA5, 0x00)
    a.sta(VDC2 + 6)
    a.b(0xA5, 0x01, 0x29, 0x3F)  # LDA $01, AND #$3F
    a.sta(VDC2 + 7)
    a.store(VDC2 + 0, 0x02)
    a.lda_abs(VDC2 + 2)
    a.lda_abs(VDC2 + 3)
    # every 256 turns: the dot clock; once, VDC 2's interrupts off
    a.b(0xA5, 0x00, 0xD0, 0)  # LDA $00, BNE skip
    branch = len(a.code) - 1
    a.b(0xA5, 0x01, 0x29, 0x03, 0xAA)  # LDA $01, AND #3, TAX
    a.w(0xBD, clocks)
    a.sta(0x0400)
    a.b(0xA5, 0x01, 0xC9, 0x06, 0xD0, 0)  # LDA $01, CMP #6, BNE skip
    branch2 = len(a.code) - 1
    reg(a, VDC2, 0x05, 0x00C0)
    a.code[branch] = len(a.code) - branch - 1
    a.code[branch2] = len(a.code) - branch2 - 1
    # a delay of 1 to 32 turns
    a.b(0xA5, 0x00, 0x29, 0x1F, 0xAA, 0xE8, 0xCA, 0xD0, 0xFD)  # LDA $00, AND #$1F, TAX, INX, DEX, BNE -3
    a.w(0x4C, loop)  # JMP loop

    image = bytearray([0xFF]) * 0x2000
    if len(a.code) > 0x1FF6:
        raise SystemExit(f"the probe is {len(a.code)} bytes")
    image[: len(a.code)] = a.code
    for offset, vector in ((0x1FF6, other), (0x1FF8, irq1), (0x1FFA, other), (0x1FFC, other), (0x1FFE, reset)):
        image[offset : offset + 2] = bytes((vector & 0xFF, vector >> 8))
    return bytes(image)


def main() -> int:
    out = ROOT / "scratch" / "sgx-probe.pce"
    out.parent.mkdir(exist_ok=True)
    image = build()
    if not out.is_file() or out.read_bytes() != image:
        out.write_bytes(image)
    return 0


if __name__ == "__main__":
    sys.exit(main())
