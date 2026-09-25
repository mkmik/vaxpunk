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

/*
 * tramp(const struct handoff *h) runs at its physical address, identity
 * mapped. Page alignment keeps it inside the one 2 MiB block that maps it
 * (mmu.c). It loads everything from h while Limine's
 * TTBR1 still maps it, turns the MMU off, installs seL4's MAIR, TCR and
 * tables, turns the MMU back on and enters the kernel. The same sequence as
 * the elfloader's arm_enable_mmu.
 */
	.section .text.tramp, "ax"
	.balign	4096
	.global	tramp
tramp:
	ldp	x19, x20, [x0, #48]	/* entry, ttbr0 */
	ldp	x21, x22, [x0, #64]	/* ttbr1, mair */
	ldr	x23, [x0, #80]		/* tcr */
	ldp	x4, x5, [x0, #32]
	ldp	x2, x3, [x0, #16]
	ldp	x0, x1, [x0]

	mrs	x9, sctlr_el1
	bic	x9, x9, #(1 << 0)	/* M */
	bic	x9, x9, #(1 << 2)	/* C */
	bic	x9, x9, #(1 << 12)	/* I */
	msr	sctlr_el1, x9
	isb
	ic	iallu
	dsb	nsh
	isb

	msr	mair_el1, x22
	msr	tcr_el1, x23
	msr	ttbr0_el1, x20
	msr	ttbr1_el1, x21
	isb
	tlbi	vmalle1is
	dsb	ish
	isb

	orr	x9, x9, #(1 << 0)
	orr	x9, x9, #(1 << 2)
	orr	x9, x9, #(1 << 12)
	msr	sctlr_el1, x9
	isb
	br	x19

	.bss
	.balign	16
	.space	16384
stack_top:
