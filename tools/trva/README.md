# trva — Tiny RISC-V Assembler

A small RISC-V assembler that emits raw section data. It performs two-pass assembly with basic
branch/jump relaxation, and is used to build test programs for the simulator and the pipeline
explorer without needing a cross toolchain on `PATH`.

- `trva/` — the library
- `cli/` — the `trva` command
- `snapshot-tests/` — fixtures and their recorded output

## Using the CLI

```sh
cargo run -p trva-cli -- prog.s \
    --text 0x8000_0000 \
    --data after:text:align=16 \
    --rodata after:data:align=4 \
    --bss after:rodata:align=4
```

Each section's start is written to stdout along with its bytes:

```text
-- .text (0x80000000-0x80000010) --
13051000930520003306B50073000000
-- .data (0x80000010-0x80000015) --
0000000000
-- .rodata (0x80000018-0x8000001B) --
000000
-- .bss (0x8000001C-0x8000005C) --
```

`--isa` selects the target (`rv32im` by default), and `-o` writes to a file.

## Section positions

All four of `--text`, `--data`, `--rodata` and `--bss` are required, and each takes either a
fixed address or a position relative to another section:

```text
position  := <address> | "after" ":" <section> ( ":" <attribute> )*
address   := ( "0x" | "0o" | "0b" )? <digits>
section   := "."? ( "text" | "data" | "rodata" | "bss" )
attribute := ( "align" | "gap" ) "=" <address>
```

Digits may be separated with `_`, and names and prefixes are case-insensitive.

| Attribute | Meaning                                                            |
| --------- | ------------------------------------------------------------------ |
| `gap=<n>` | leave `n` bytes between the preceding section and this one         |
| `align=<n>` | round the resulting address up to a multiple of `n` bytes        |

`gap` is applied before `align`. Like the `.align` directive, `align` is a **byte count**, not a
power-of-two exponent, and need not be a power of two.

A section named by `after` may itself be placed relatively, so sections can be chained in any
order — `--text after:bss` is fine as long as the chain does not close on itself. A cycle, or a
section placed after itself, is rejected before the source is even read. Nothing stops two
sections from overlapping, which is sometimes deliberate: placing both `.rodata` and `.bss` after
`.data` gives them the same base.

Relative positions are resolved exactly. Sections are laid out in dependency order, so by the
time a section's base is needed, the size of whatever it follows is already final — including any
`.align` padding and any instruction that relaxation grew or shrank.

## Supported directives

| Directive          | Notes                                                       |
| ------------------ | ----------------------------------------------------------- |
| `.section <name>`  | also as a bare `.text` / `.data` / `.rodata` / `.bss`       |
| `.global <name>`   | records a symbol at the current position                    |
| `.align <n>`       | pad to a multiple of `n` **bytes**; `0` and `1` are no-ops  |
| `.space <n>`       | emit `n` zero bytes                                         |

Alignment applies to the absolute address, so a section with an unaligned base still aligns
correctly.

Note that instructions currently always land in `.text` regardless of the section in effect, and
there are no data-emitting directives, so `.data` and `.rodata` can only hold `.align`, `.space`
and labels.

## Snapshot tests

```sh
cargo test -p snapshot-tests          # run them
TRVA_UPDATE_SNAPSHOTS=1 cargo run -p snapshot-tests  # re-record them
```

Every `fixtures/<category>/<name>.s` is assembled and rendered to
`snapshots/<category>/<name>.snap`. A fixture can override where its sections go with a comment
using the same syntax as the CLI:

```asm
# trva-text: 0x8000_0000
# trva-bss: after:data:align=4
```

The disassembly block needs `riscv32-none-elf-objdump` (in this repo, `nix develop .#bench`).
Without it the block is skipped and also stripped from the recorded snapshot, so the rest is
still checked; re-recording without it is refused, since that would drop the disassembly from
every snapshot.
