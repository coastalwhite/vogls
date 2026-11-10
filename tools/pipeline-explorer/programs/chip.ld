/* Linker script for the memory of the pipeline-explorer testbenches.
 *
 * Every testbench declares `reg [31:0] memory [0:255]` for code at address 0
 * and a second array of 1001 words for data from 1 KiB upwards. Each section
 * goes at the start of a fixed range, the memory map of `src/cores/mod.rs`:
 *
 *     .text    BOOT  .. 0x300
 *     .rodata  0x300 .. 0x400
 *     .data    0x400 .. 0x800
 *     .bss     0x800 .. 0x13a4
 *
 * Pass the reset vector of the chip with `-Wl,--defsym=BOOT=<addr>`; it is
 * 0x80 for Ibex and zero for the others.
 */

BOOT_ADDR = DEFINED(BOOT) ? BOOT : 0x0;

RODATA_BASE = 0x300;
DATA_BASE   = 0x400;
BSS_BASE    = 0x800;
BSS_END     = 1024 + 1001 * 4;

ENTRY(_start)

SECTIONS
{
    . = BOOT_ADDR;
    .text : { *(.text .text.*) }
    ASSERT(. <= RODATA_BASE, "text does not fit below 0x300")

    . = RODATA_BASE;
    .rodata : { *(.rodata .rodata.* .srodata .srodata.*) }
    ASSERT(. <= DATA_BASE, "rodata does not fit below 0x400")

    . = DATA_BASE;
    .data : { *(.data .data.* .sdata .sdata.*) }
    ASSERT(. <= BSS_BASE, "data does not fit below 0x800")

    . = BSS_BASE;
    .bss (NOLOAD) : { *(.bss .bss.* .sbss .sbss.* COMMON) }
    ASSERT(. <= BSS_END, "bss does not fit in the data memory")

    /DISCARD/ : { *(.comment) *(.note*) *(.eh_frame*) *(.riscv.attributes) }
}
