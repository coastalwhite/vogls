ENTRY(_start)

SECTIONS {
    . = 0x00000000;
    /* . = 0x00000080; */

    .text : { *(.text*) }
    .data : { *(.data*) }
    .rodata : { *(.rodata*) }

    /* Place .bss at a real address so symbol references resolve,
       but it won't appear in the flat binary (it's after .data). */
    . = 0x00000400;
    .bss    : { *(.bss*) *(COMMON) }

    /* .bss is zero-initialised RAM — not included in the binary image */
    /DISCARD/ : { *(.comment) *(.riscv.attributes) }
}
