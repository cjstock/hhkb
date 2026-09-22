# Linux hardware acceptance — 2026-10-01 UTC

Completed on the connected `04fe:0021` HHKB Professional HYBRID Type-S,
programming interface 2, mode byte `0`, sleep time 30 minutes. The device was
returned from Windows VM passthrough to the Linux host before testing.
Device identity is retained only in the local exports/backups under `caps/`;
it is scrubbed from test fixtures and extracted write reports.

## Results

| Step | Readback and preservation | User's physical check |
| --- | --- | --- |
| Export baseline; import it unchanged | All four maps, sleep, identity, and mode equal baseline | Not needed |
| Base-only index 40: `17` → `1c` hex | Exactly one base byte changes; Fn and companions unchanged | Physical T produces `y`; Fn+T produces `t` |
| Restore baseline | All four maps, sleep, and mode verified | Not needed |
| Fn-only index 40: `17` → `1c` hex | Exactly one Fn byte changes; base and companions unchanged | Physical T produces `t`; Fn+T produces `y` |
| Restore original export | All four maps, sleep, identity, and mode equal saved full baseline | Both edits removed by verified restoration |

Each import created an adjacent backup before writing. The raw baseline is
`caps/linux-baseline-20261001.toml`; the full preservation snapshot is
`caps/linux-full-baseline-20261001.toml`. The final independent check used:

```bash
cargo run --example check_snapshot -- check caps/linux-full-baseline-20261001.toml
```

It reported: `All four maps, sleep time, identity, and mode match baseline.`
The final captured read also matches all four maps in the original official
startup fixture, with sleep 30 and mode 0.

The physical key labels here are acceptance observations for this device;
export/import still accepts only raw bytes and has no named-key mapping API.
The user's initially ambiguous base-test report was corrected to T → y and
Fn+T → t before proceeding. The Fn-only physical result was then confirmed.

## Project write capture compared with the official tool

Raw capture: `/tmp/hhkb-linux-roundtrip-20261001.pcapng`, kept outside the repo
because it includes unrelated bus traffic and keyboard input. SHA256:
`27799664dd1c70be63a8579edf20d6fc93d57ec0b88378416e8f469efcf5b9bc`.
Dumpcap reported 8,574 packets and zero drops. Extraction validated pcapng
block lengths/trailers, link type 220, and complete HHKB usbmon payloads.

There are 704 data-bearing configuration reports across 18 read sessions and
five write sessions. All 704 corresponding completions have USB status zero.
Eight additional empty IN completions have status -2 (pending-read
cancellations); they carry no configuration payload.

The five sanitized, configuration-only write transcripts are
`caps/linux-write-{1,2,3,4,5}-20261001.hex`. Each contains 40 complete 64-byte
reports, excludes keyboard input and USB metadata, and uses the same
fictional identity as the test fixtures.

| Write | Comparison with official baseline write |
| --- | --- |
| 1 — unchanged import | All 40 reports identical |
| 2 — base-only edit | Only selector `00/00` index 40 changes `17` → `1c` |
| 3 — intermediate restoration | All 40 reports identical |
| 4 — Fn-only edit | Only selector `00/01` index 40 changes `17` → `1c` |
| 5 — final restoration | All 40 reports identical |

The pause from the sleep-setting reply to the first map report measured
1.000162–1.000394 seconds. Packet comparisons cover report bytes and ordering;
timing is deliberately described separately.

## Automated validation

`cargo test` passed 13 hardware-independent tests. `cargo fmt --check` and
`cargo clippy --all-targets` passed without diagnostics. Tests cover captured
packet encoding/reassembly, transfer failures, malformed replies, timeouts,
session cleanup including pending replies, raw TOML validation, independent
layer edits, companion preservation, readback mismatches, no retries after a
write failure, export overwrite refusal, and adjacent backups.

## Repeating the controlled check

Build with `cargo build` and `cargo build --examples`. With USB access and
mode 0, save a raw export and complete preservation baseline:

```bash
target/debug/hhkb export original.toml
target/debug/examples/check_snapshot save original-full.toml
target/debug/hhkb import original.toml
target/debug/examples/check_snapshot check original-full.toml
```

Copy the export to two separate files and change one ordinary base byte in
the first and one ordinary Fn byte in the second. Test them separately,
confirming physical behavior and restoring the original between tests.
Finally import `original.toml` and compare against `original-full.toml`.
Capture writes with usbmon only when needed, following the README's capture
setup. If any import reports a failure, retain the original and backups;
the operation does not claim rollback or retry configuration writes.
