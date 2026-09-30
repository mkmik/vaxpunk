/* Static, non-PIE root task. seL4 maps it at these virtual addresses. */
ENTRY(_start)

SECTIONS
{
	. = 0x400000;
	.text : { *(.text.start) *(.text .text.*) }
	.rodata : { *(.rodata .rodata.*) }

	. = ALIGN(4096);
	.data : { *(.data .data.*) }
	.tdata : { *(.tdata .tdata.*) }
	.tbss : { *(.tbss .tbss.*) }
	.bss : { *(.bss .bss.*) *(COMMON) }
	/* The shim appends exec.exe here, and seL4 maps it with the root task. */
	. = ALIGN(4096);
	_end = .;

	/DISCARD/ : { *(.comment) *(.note .note.*) *(.eh_frame*) }
}

ASSERT(SIZEOF(.tdata) == 0, "initialised thread-locals need a TLS template copy in start.S")
ASSERT(SIZEOF(.tbss) <= 48, "grow tls_block in start.S")
