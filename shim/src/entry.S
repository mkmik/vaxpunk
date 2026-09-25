/*
 * Limine enters _start at EL1 with the MMU on. Switch to our own stack on
 * SP_EL1 so that exceptions land on it too, install a vector table that
 * reports any exception, and run shim_main().
 */
	.section .text.entry, "ax"
	.global _start
_start:
	msr	spsel, #1
	adrp	x0, stack_top
	add	sp, x0, :lo12:stack_top
	adrp	x0, vectors
	add	x0, x0, :lo12:vectors
	msr	vbar_el1, x0
	isb
	bl	shim_main
1:	wfe
	b	1b

	.balign	0x800
vectors:
	.rept	16
	.balign	0x80
	mrs	x0, esr_el1
	mrs	x1, elr_el1
	mrs	x2, far_el1
	b	shim_exception
	.endr

	.bss
	.balign	16
	.space	16384
stack_top:
