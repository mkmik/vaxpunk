/*
 * seL4 starts the root task here with x0 = seL4_BootInfo *. Set up a stack
 * and a TLS block (libsel4 keeps the IPC buffer pointer in a thread-local),
 * then call main(bootinfo).
 */
	.section .text.start, "ax"
	.global	_start
_start:
	adrp	x1, stack_top
	add	sp, x1, :lo12:stack_top
	adrp	x1, tls_block
	add	x1, x1, :lo12:tls_block
	msr	tpidr_el0, x1
	bl	main
1:	b	1b

	.bss
	.balign	16
	.space	16384
stack_top:

/*
 * AArch64 local-exec TLS: TPIDR_EL0 points at a 16-byte TCB, and the .tbss
 * variables follow it. linker.ld checks they fit in the 48 bytes after it.
 */
	.balign	16
tls_block:
	.space	64
