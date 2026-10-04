/* The component lives in one 2 MB large page the root task maps at
 * TCPIP_BASE (include/component.h): its image, its data and its DMA
 * buffers, whose physical addresses follow from the page's. */
ENTRY(_start)

SECTIONS
{
	. = 0x200000;
	.text : { *(.text.start) *(.text .text.*) }
	.rodata : { *(.rodata .rodata.*) }

	. = ALIGN(4096);
	.data : { *(.data .data.*) }
	.tdata : { *(.tdata .tdata.*) }
	.tbss : { *(.tbss .tbss.*) }
	.bss : { *(.bss .bss.*) *(COMMON) }
	_end = .;

	/DISCARD/ : { *(.comment) *(.note .note.*) *(.eh_frame*) }
}

ASSERT(_end <= 0x400000, "the component outgrew its 2 MB page")
ASSERT(SIZEOF(.tdata) == 0, "initialised thread-locals need a TLS template copy in start.S")
ASSERT(SIZEOF(.tbss) <= 48, "grow tls_block in start.S")
