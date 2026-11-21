<!-- SPDX-License-Identifier: MIT -->
# Seeded faults

`scripts/sgx_seeded_faults.py` first checks that the unfaulted tree passes rung 1 and the short cases, then plants one single-line fault at a time in a copy of `src/` (with `tests/` and the README, which `src/lib.rs` runs as a doctest), builds it,
and puts it to both cheap checks, so the table says which of them see it: rung 1 (`cargo test`) and
the gate on its short cases (Covell's test and the probe card of rung 2, the probe's switch cases of
rung 3, the Covell cases of rung 4). Only a fault neither catches goes on to the cards of rung 3 and
the round trips of rung 5. It uses the reference cache, so run it after a full gate run.

## 2026-11-19: on the fixed base board (pcengine-rust `37f17ba`)

**34 caught, 0 missed**, with no `--known-failing` exclusions (against the rebuilt cache of
supergrafx-python `8347704` on pcengine-python `03ac699`, base board pcengine-rust `37f17ba`, which
crops a display that starts before the row: [notes on the reference](notes-on-the-reference.md),
7): 24 by both the unit tests and the gate, 8 by the unit tests alone, 2 by the gate alone
(`second-window-msb-not-replayed`, `vdc2-waiting-write-dropped`, both on `probe`). The only change
from the run below: `builder-ignores-the-switch` is seen by the gate on `sw-compat-probe` again,
now that the case runs.

## 2026-11-19: the clean rewrite

**34 caught, 0 missed** (against the rebuilt cache of supergrafx-python `f04d648`, base board
pcengine-rust `26ea621`): 23 by both the unit tests and the gate, 9 by the unit tests alone,
2 by the gate alone. Run with `--known-failing 'sw-compat-probe|sw-base-probe|probe-compat-at'`:
the unfaulted tree fails those three (the base board's display-before-the-row bug,
[notes on the reference](notes-on-the-reference.md), 7), so they are left out of every stage;
`builder-ignores-the-switch` was seen by the gate on `sw-compat-probe` before and now only by rung 1.

What changed in the seeder and the list:

- **The seeder's test stage was broken in 0.2.0**: its tree had no `README.md`, which `src/lib.rs`
  includes as a doctest, so `cargo test` failed to build whatever the fault and every fault that
  reached it counted as caught by rung 1 (the doctest runs last, so a test that failed first was
  still named; the 0.2.0 run's two `*-waiting-*` rows, "the script could not name it", were the
  doctest alone, and one of them is caught by the gate). It now copies the README and checks the unfaulted tree first. A second
  hole: `copytree` kept each file's old time, so a file restored after a fault looked older than
  the last build and cargo kept the previous fault in (a fault in `board.rs` was built with the
  `machine.rs` fault before it); files are now copied with fresh times.
- With the stage fixed, `vdc1-waiting-read-dropped` was missed (neither a unit test nor a short case
  reads VDC 1 while it has to wait). A test was added,
  `a_vdc_read_that_has_to_wait_waits_through_the_page_on_either_vdc` (`tests/board.rs`).
- The seven faults whose text was a line of the withdrawn code (`docs/restart.md`) were withdrawn
  unread with it and written anew against the transcription, with the same names and meaning:
  `window-of-0x40-is-on`, `window-origin-ignores-the-dot-clock`, `both-off-shows-vdc1`,
  `transparent-pair-shows-vdc1`, `vdc2-sprites-over-vdc1-sprites`, `code-10-is-the-default-order`,
  `priority-labels-not-read`. Two added: `window-origin-before-the-repin` (the 7.16 MHz origin of the
  old base board) and `vdc2-alone-shows-vdc1`.

| Fault | Module | What it changes | Rung 1 (the test) | The gate (the first case) |
| --- | --- | --- | --- | --- |
| `window-msb-three-bits` | `vpc.rs` | window 1's MSB register keeps three bits (it reads back $04-$07) | `every_vpc_register_reads_back_through_the_bus` | `probe` |
| `steer-register-reads-back` | `vpc.rs` | $0E and $0F read the steering bit, not $00 | `every_vpc_register_reads_back_through_the_bus` | `probe` |
| `steer-on-any-bit` | `vpc.rs` | any bit of $0E steers ST0-ST2, not bit 0 alone | `register_0e_steers_st_but_reads_zero_and_0f_is_unused` | not caught |
| `window-two-alone-takes-the-overlap-nibble` | `vpc.rs` | the area inside window 2 alone is mixed by the overlap's group | `the_four_areas_take_their_own_nibbles` | `probe` |
| `no-window-takes-window-ones-nibble` | `vpc.rs` | the area outside both windows is mixed by window 1's group | `a_register_written_mid_line_takes_effect_at_that_dot` | `probe` |
| `window-of-0x40-is-on` | `vpc.rs` | a window of width $40 is on (U-SGX2: at or below $40 is off) | `a_window_of_0x40_or_less_is_disabled` | `probe` |
| `window-origin-ignores-the-dot-clock` | `vpc.rs` | every dot clock takes the 5.37 MHz window origin (U-SGX1) | `the_edge_origin_follows_the_dot_clock` | `probe` |
| `window-origin-before-the-repin` | `vpc.rs` | the 7.16 MHz window origin measured on the old base board (46, before 3318027) | `the_edge_origin_follows_the_dot_clock` | `probe` |
| `second-window-msb-not-replayed` | `vpc.rs` | the mix sees one bit of window 2's MSB register | not caught | `probe` |
| `rows-swapped` | `vpc.rs` | the two VDCs' pixel buses are swapped | `a_register_written_mid_line_takes_effect_at_that_dot` | `covell` |
| `mid-line-write-a-dot-early` | `vpc.rs` | a register written inside a dot takes effect from that dot, not the next (U-SGX3) | `a_write_inside_a_dot_takes_effect_at_the_next` | `probe` |
| `both-off-shows-vdc1` | `vpc.rs` | an area with both layers off shows VDC 1, not colour 0 | `both_off_shows_colour_zero_in_the_overscan_too` | `probe` |
| `vdc2-alone-shows-vdc1` | `vpc.rs` | an area with VDC 2 alone enabled shows VDC 1 | `a_register_written_mid_line_takes_effect_at_that_dot` | `probe` |
| `transparent-pair-shows-vdc1` | `vpc.rs` | where both pixels are transparent VDC 1's value shows, not VDC 2's (U-SGX5) | `both_transparent_shows_vdc2s_pixel` | `probe` |
| `vdc2-sprites-over-vdc1-sprites` | `vpc.rs` | under code 01b VDC 2's sprites come in front of VDC 1's sprites too | `vdc2_sprites_in_front_of_vdc1_background_behind_vdc1_sprites` | not caught |
| `code-10-is-the-default-order` | `vpc.rs` | code 10b mixes as the default order (VDC 1 in front wherever it is opaque) | `vdc1_sprites_behind_vdc2_background_but_in_front_of_vdc2_sprites` | `probe` |
| `priority-labels-not-read` | `vpc.rs` | the PRIORITY_LABELS switch is ignored (C-SGX1) | `sgxtech_labels_swap_the_two_codes_and_nothing_else` | `sw-labels-probe` |
| `vdc2-waiting-write-dropped` | `vpc.rs` | a write to VDC 2 that has to wait never reaches the board's wait and is lost | not caught | `probe` |
| `vdc1-waiting-read-dropped` | `vpc.rs` | a read of VDC 1 that has to wait never reaches the board's wait and answers $FF | `a_vdc_read_that_has_to_wait_waits_through_the_page_on_either_vdc` | not caught |
| `plain-stores-steered` | `vpc.rs` | with $0E set every write of the area goes to VDC 2, not only ST0-ST2 | `st_goes_to_vdc2_only_while_0e_bit_0_is_set_and_only_for_st_instructions` | `probe` |
| `st2-not-steered` | `vpc.rs` | ST2 is not taken for a store-immediate | `st_goes_to_vdc2_only_while_0e_bit_0_is_set_and_only_for_st_instructions` | `probe` |
| `compat-still-steers` | `vpc.rs` | ST0-ST2 follow a left-over $0E bit in PC Engine position whatever the switch (U-SGX4) | `st_steering_is_off_with_the_switch_in_compat_position` | `sw-compat-at-probe` |
| `steers-in-compat-not-read` | `vpc.rs` | the ST_STEERS_IN_COMPAT switch is not wired (U-SGX4) | `st_steering_is_off_with_the_switch_in_compat_position` | `sw-steers-probe` |
| `vdc2-write-mirror-missing` | `vpc.rs` | writes to $14-$17 do not reach VDC 2 | `native_decode_maps_two_vdcs_and_the_vpc_in_a_32_byte_block` | `probe` |
| `block-not-repeated-for-writes` | `vpc.rs` | the 32-byte block takes writes only in its first copy | `the_block_repeats_every_32_bytes_through_the_1k_area` | `probe` |
| `gap-reads-zero` | `vpc.rs` | $18-$1F read $00 | `the_gap_18_to_1f_reads_ff_and_ignores_writes` | `probe` |
| `compat-reads-keep-the-native-decode` | `vpc.rs` | in PC Engine position only $00-$07 of the block read VDC 1 | `compat_reads_are_vdc1s_everywhere` | `sw-compat-at-probe` |
| `button-reset-resets-the-vpc` | `vpc.rs` | the VPC is reset by the button and not by power-on | `a_power_on_reset_resets_the_vpc_and_a_button_reset_does_not` | not caught |
| `state-forgets-the-switch-position` | `vpc.rs` | a state is saved with the back-panel switch in SuperGrafx position | `the_vpc_the_page_and_the_second_vdc_come_back` | not caught |
| `state-forgets-the-pending-writes` | `vpc.rs` | a loaded state keeps the VPC writes the system it is loaded into had not drawn yet | `a_loaded_run_continues_bus_cycle_for_bus_cycle` | not caught |
| `state-loses-the-line-shadow` | `vpc.rs` | a loaded state mixes the line in progress with the windows as last written | `a_loaded_run_continues_bus_cycle_for_bus_cycle` | not caught |
| `work-ram-8k` | `machine.rs` | the SuperGrafx row has the PC Engine's 8 KiB, mirrored | `work_ram_is_32k_in_banks_f8_to_fb_without_the_8k_mirror` | not caught |
| `builder-ignores-the-switch` | `board.rs` | a SuperGrafx built in PC Engine position is in SuperGrafx position | `compat_reads_are_vdc1s_everywhere` | not caught |
| `builder-ignores-the-vpc-given` | `board.rs` | the builder puts a VPC with default switches on the board | `the_builder_takes_a_vpc_with_other_switches_and_refuses_another_pixel_path` | `sw-labels-probe` |

## 0.2.0 (2026-11-13)

The record below is that run's. Where its rung-1 column names a test, that test failed; the two
rows that say "a test (the script could not name it)" were the missing README's doctest, not a
catch, so that run's honest count was 31 caught, 1 missed (`vdc1-waiting-read-dropped`).

| Fault | Module | What it changes | Rung 1 (the test) | The gate (the first case) |
| --- | --- | --- | --- | --- |
| `window-msb-three-bits` | `vpc.rs` | window 1's MSB register keeps three bits (it reads back $04-$07) | `every_vpc_register_reads_back_through_the_bus` | `probe` |
| `steer-register-reads-back` | `vpc.rs` | $0E and $0F read the steering bit, not $00 | `every_vpc_register_reads_back_through_the_bus` | `probe` |
| `steer-on-any-bit` | `vpc.rs` | any bit of $0E steers ST0-ST2, not bit 0 alone | `register_0e_steers_st_but_reads_zero_and_0f_is_unused` | not caught |
| `window-two-alone-takes-the-overlap-nibble` | `vpc.rs` | the area inside window 2 alone is mixed by the overlap's group | `the_four_areas_take_their_own_nibbles` | `probe` |
| `no-window-takes-window-ones-nibble` | `vpc.rs` | the area outside both windows is mixed by window 1's group | `a_register_written_mid_line_takes_effect_at_that_dot` | `probe` |
| `window-of-0x40-is-on` | `vpc.rs` | a window of exactly $40 is not disabled (U-SGX2) | `a_window_of_0x40_or_less_is_disabled` | `probe` |
| `window-origin-ignores-the-dot-clock` | `vpc.rs` | the window edge at 7.16 MHz uses the 5.37 MHz origin (U-SGX1) | `the_edge_origin_follows_the_dot_clock` | `probe` |
| `second-window-msb-not-replayed` | `vpc.rs` | the mix sees one bit of window 2's MSB register | not caught | `probe` |
| `rows-swapped` | `vpc.rs` | the two VDCs' pixel buses are swapped | `a_register_written_mid_line_takes_effect_at_that_dot` | `covell` |
| `mid-line-write-a-dot-early` | `vpc.rs` | a register written inside a dot takes effect from that dot, not the next (U-SGX3) | `a_write_inside_a_dot_takes_effect_at_the_next` | `probe` |
| `both-off-shows-vdc1` | `vpc.rs` | an area with both layers disabled shows VDC 1, not colour 0 | `both_off_shows_colour_zero_in_the_overscan_too` | `probe` |
| `transparent-pair-shows-vdc1` | `vpc.rs` | where both pixels are transparent the default order shows VDC 1's, not VDC 2's (U-SGX5) | `two_transparent_pixels_show_vdc2s` | `probe` |
| `vdc2-sprites-over-vdc1-sprites` | `vpc.rs` | priority code 01b puts VDC 2's sprites in front of VDC 1's sprites too | `vdc2_sprites_in_front_of_vdc1_background_behind_vdc1_sprites` | not caught |
| `code-10-is-the-default-order` | `vpc.rs` | priority code 10b mixes in the default order | `vdc1_sprites_behind_vdc2_background_but_in_front_of_vdc2_sprites` | `probe` |
| `priority-labels-not-read` | `vpc.rs` | the PRIORITY_LABELS switch is not wired (C-SGX1) | `sgxtech_labels_swap_the_two_codes_and_nothing_else` | `sw-labels-probe` |
| `vdc2-waiting-write-dropped` | `vpc.rs` | a write to VDC 2 that has to wait misses the base board's wait and is lost (0.2.0) | a test (the script could not name it) | `probe` |
| `vdc1-waiting-read-dropped` | `vpc.rs` | a read of VDC 1 that has to wait misses the base board's wait and answers $FF (0.2.0) | a test (the script could not name it) | not caught |
| `plain-stores-steered` | `vpc.rs` | with $0E set every write of the area goes to VDC 2, not only ST0-ST2 | `st_goes_to_vdc2_only_while_0e_bit_0_is_set_and_only_for_st_instructions` | `probe` |
| `st2-not-steered` | `vpc.rs` | ST2 is not taken for a store-immediate | `st_goes_to_vdc2_only_while_0e_bit_0_is_set_and_only_for_st_instructions` | `probe` |
| `compat-still-steers` | `vpc.rs` | ST0-ST2 follow a left-over $0E bit in PC Engine position whatever the switch (U-SGX4) | `st_steering_is_off_with_the_switch_in_compat_position` | `sw-compat-at-probe` |
| `steers-in-compat-not-read` | `vpc.rs` | the ST_STEERS_IN_COMPAT switch is not wired (U-SGX4) | `st_steering_is_off_with_the_switch_in_compat_position` | `sw-steers-probe` |
| `vdc2-write-mirror-missing` | `vpc.rs` | writes to $14-$17 do not reach VDC 2 | `native_decode_maps_two_vdcs_and_the_vpc_in_a_32_byte_block` | `probe` |
| `block-not-repeated-for-writes` | `vpc.rs` | the 32-byte block takes writes only in its first copy | `the_block_repeats_every_32_bytes_through_the_1k_area` | `probe` |
| `gap-reads-zero` | `vpc.rs` | $18-$1F read $00 | `the_gap_18_to_1f_reads_ff_and_ignores_writes` | `probe` |
| `compat-reads-keep-the-native-decode` | `vpc.rs` | in PC Engine position only $00-$07 of the block read VDC 1 | `compat_reads_are_vdc1s_everywhere` | `sw-compat-at-probe` |
| `button-reset-resets-the-vpc` | `vpc.rs` | the VPC is reset by the button and not by power-on | `a_power_on_reset_resets_the_vpc_and_a_button_reset_does_not` | not caught |
| `state-forgets-the-switch-position` | `vpc.rs` | a state is saved with the back-panel switch in SuperGrafx position | `the_vpc_the_page_and_the_second_vdc_come_back` | not caught |
| `state-forgets-the-pending-writes` | `vpc.rs` | a loaded state keeps the VPC writes the system it is loaded into had not drawn yet | `a_loaded_run_continues_bus_cycle_for_bus_cycle` | not caught |
| `state-loses-the-line-shadow` | `vpc.rs` | a loaded state mixes the line in progress with the windows as last written | `a_loaded_run_continues_bus_cycle_for_bus_cycle` | not caught |
| `work-ram-8k` | `machine.rs` | the SuperGrafx row has the PC Engine's 8 KiB, mirrored | `work_ram_is_32k_in_banks_f8_to_fb_without_the_8k_mirror` | not caught |
| `builder-ignores-the-switch` | `board.rs` | a SuperGrafx built in PC Engine position is in SuperGrafx position | `compat_reads_are_vdc1s_everywhere` | `sw-compat-probe` |
| `builder-ignores-the-vpc-given` | `board.rs` | the builder puts a VPC with default switches on the board | `the_builder_takes_a_vpc_with_other_switches_and_refuses_another_pixel_path` | `sw-labels-probe` |

**32 caught, 0 missed** (2026-11-13, 0.2.0, against the rebuilt cache of supergrafx-python `v0.1.1`;
the two `*-waiting-*` faults are new, on the base board's VDC wait): 23 by both the unit tests and
the gate, 8 by the unit tests alone, 1 by the gate alone. Every fault of the 0.1.0 run is caught by
the same checks as then. The gate does not see those seven on its short cases (a `$0E` value other than 0 or 1,
VDC 2 sprites over VDC 1 sprites under code 01b, a reset, the three save-state faults, the size of the
RAM); whether the cards of rung 3 or the round trips of rung 5 would was not run, since a fault goes
on to them only when neither cheap check catches it, and none did.

The probe card was corrected twice after the first runs, both holes in the case and not in the
crate: its sprites pointed at an empty pattern (so `PRIORITY_LABELS` changed nothing on it), and
nothing made the two VDCs' transparent values differ (so `transparent-pair-shows-vdc1` was missed
until the probe gave VDC 2 a narrower display and the unit test `two_transparent_pixels_show_vdc2s`
was added).
