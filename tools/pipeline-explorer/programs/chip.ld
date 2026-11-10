/* Linker script for the memory of the pipeline-explorer testbenches.
 *
 * Every testbench declares `reg [31:0] memory [0:255]` for code at address 0
 * and a second array of 1001 words for data from 1 KiB upwards; these are the
 * `imem_addr`/`dmem_addr` of each core's `Description` in `src/cores/`.
 *
 * `.text` goes in the instruction memory, from the reset vector. `.data`,
 * `.rodata` and `.bss` go after each other in the data memory, as the explorer
 * lays out assembly:
 *
 *     .text    BOOT  .. 0x400
 *     .data    0x400 ..
 *     .rodata  (after .data)
 *     .bss     (after .rodata) .. 0x13a4
 *
 * Pass the reset vector of the chip with `-Wl,--defsym=BOOT=<addr>`; it is
 * 0x80 for Ibex and zero for the others.
 */

BOOT_ADDR = DEFINED(BOOT) ? BOOT : 0x0;

IMEM_END  = 1024;
DMEM_BASE = 1024;
DMEM_END  = 1024 + 1001 * 4;

ENTRY(_start)

SECTIONS
{
    . = BOOT_ADDR;
    .text : { *(.text .text.*) }
    ASSERT(. <= IMEM_END, "text does not fit in the instruction memory")

    . = DMEM_BASE;
    .data : { *(.data .data.* .sdata .sdata.*) }
    . = ALIGN(4);
    .rodata : { *(.rodata .rodata.* .srodata .srodata.*) }
    . = ALIGN(4);
    .bss (NOLOAD) : { *(.bss .bss.* .sbss .sbss.* COMMON) }
    ASSERT(. <= DMEM_END, "data, rodata and bss do not fit in the data memory")

    /DISCARD/ : { *(.comment) *(.note*) *(.eh_frame*) *(.riscv.attributes) }
}
