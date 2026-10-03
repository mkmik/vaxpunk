/*
 * The root task starts the component here with x0 = the physical address
 * of its 2 MB page and x1 = its virtio-net transport's offset in the
 * MMIO page. Set up a stack and a TLS block (libsel4 keeps the IPC buffer
 * pointer in a thread-local), then call main(paddr, offset).
 */
	.section .text.start, "ax"
	.global	_start
_start:
	adrp	x2, stack_top
	add	sp, x2, :lo12:stack_top
	adrp	x2, tls_block
	add	x2, x2, :lo12:tls_block
	msr	tpidr_el0, x2
	bl	main
1:	b	1b

	.bss
	.balign	16
	.space	32768
stack_top:

	.balign	16
tls_block:
	.space	64
