// vrun boot stub.
//
// QEMU starts it at EL1 with the MMU off. It turns the MMU on with the page
// tables vrun built, then enters the image at EL0. After that it only runs
// for exceptions: monitor calls (SVC) and faults. It reports how the image
// ended with a "!vrun" report on the UART, which may follow the image's own
// output on the same line, and powers off through PSCI.
// docs/runner-abi.md describes what the image sees.
//
// UART, BOOT and EL1_STACK come from src/layout.rs through --defsym.
// vrun writes the boot block at BOOT (src/plan.rs):
//	+0	TTBR0_EL1: the root page table
//	+8	image entry point
//	+16	image stack pointer
//	+24	image x0: the runner info block
//	+32	image x30: the return page

	.equ	SVC_PC, BOOT + 0x40	// address of the SVC being served, or 0

	// Attribute 0 Device-nGnRE, attribute 1 Normal write-back.
	.equ	MAIR, 0xff04
	// 48-bit VA, 4 KB granule, write-back inner-shareable walks, no TTBR1
	// walks, 32-bit physical addresses.
	.equ	TCR, 0x80903510
	// MMU, caches, SP alignment checks, write implies execute-never; EL0 may
	// use DC ZVA, CTR_EL0 and cache maintenance; WFI and WFE at EL0 trap.
	.equ	SCTLR, 0x34d8d81d

	.equ	EC_SVC64, 0x15
	.equ	SVC_EXIT, 1
	.equ	SVC_PUT, 2
	.equ	SVC_DUMP, 3
	.equ	FRAME, 32 * 8		// x0-x30 and SP_EL0

	// Saves or restores x0-x29 in the frame at sp.
	.macro	pairs op
	\op	x0, x1, [sp, #16 * 0]
	\op	x2, x3, [sp, #16 * 1]
	\op	x4, x5, [sp, #16 * 2]
	\op	x6, x7, [sp, #16 * 3]
	\op	x8, x9, [sp, #16 * 4]
	\op	x10, x11, [sp, #16 * 5]
	\op	x12, x13, [sp, #16 * 6]
	\op	x14, x15, [sp, #16 * 7]
	\op	x16, x17, [sp, #16 * 8]
	\op	x18, x19, [sp, #16 * 9]
	\op	x20, x21, [sp, #16 * 10]
	\op	x22, x23, [sp, #16 * 11]
	\op	x24, x25, [sp, #16 * 12]
	\op	x26, x27, [sp, #16 * 13]
	\op	x28, x29, [sp, #16 * 14]
	.endm

	.text
	.global	_start
_start:
	ldr	x0, =EL1_STACK
	mov	sp, x0
	adr	x0, vectors
	msr	vbar_el1, x0
	mov	x0, #(3 << 20)		// CPACR_EL1.FPEN: FP and SIMD don't trap
	msr	cpacr_el1, x0
	msr	cntkctl_el1, xzr	// no counters or timers at EL0
	ldr	x0, =MAIR
	msr	mair_el1, x0
	ldr	x0, =TCR
	msr	tcr_el1, x0
	ldr	x9, =BOOT
	ldr	x0, [x9]
	msr	ttbr0_el1, x0
	isb
	tlbi	vmalle1
	dsb	nsh
	isb
	ldr	x0, =SCTLR
	msr	sctlr_el1, x0
	isb

	// Enter the image at EL0, interrupts masked, every other register zero.
	ldp	x0, x1, [x9, #8]	// entry point, stack pointer
	msr	elr_el1, x0
	msr	sp_el0, x1
	mov	x0, #0x3c0		// SPSR: EL0t, DAIF masked
	msr	spsr_el1, x0
	msr	fpcr, xzr
	msr	fpsr, xzr
	.irp	n, 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31
	movi	v\n\().2d, #0
	.endr
	ldp	x0, x30, [x9, #24]	// info block, return page
	.irp	n, 1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29
	mov	x\n, xzr
	.endr
	eret

// Synchronous exception from the image: a monitor call or a fault.
el0_sync:
	sub	sp, sp, #FRAME
	pairs	stp
	mrs	x0, sp_el0
	stp	x30, x0, [sp, #16 * 15]
	mrs	x0, esr_el1
	lsr	x1, x0, #26
	cmp	x1, #EC_SVC64
	b.ne	el0_fault
	mrs	x1, elr_el1		// SVC returns to the next instruction
	sub	x1, x1, #4
	ldr	x2, =SVC_PC
	str	x1, [x2]
	and	x0, x0, #0xffff
	cmp	x0, #SVC_EXIT
	b.eq	svc_exit
	cmp	x0, #SVC_PUT
	b.eq	svc_put
	cmp	x0, #SVC_DUMP
	b.eq	svc_dump
	mov	x19, x1			// no such service: report the SVC
	adr	x20, msg_fault
	b	report

svc_return:
	ldr	x2, =SVC_PC
	str	xzr, [x2]
	ldp	x30, x0, [sp, #16 * 15]
	msr	sp_el0, x0
	pairs	ldp
	add	sp, sp, #FRAME
	eret

// SVC #1: end the run. x0 is the status.
svc_exit:
	ldr	x19, [sp]
	adr	x0, msg_exit
	bl	puts
	mov	x0, x19
	bl	puthex
	bl	newline
poweroff:
	ldr	x0, =0x84000008		// PSCI SYSTEM_OFF
	hvc	#0
1:	wfi
	b	1b

// SVC #2: write x1 bytes from x0 to the console. The bytes are read with
// unprivileged loads, so the image's own permissions apply.
svc_put:
	ldp	x19, x20, [sp]
1:	cbz	x20, svc_return
	ldtrb	w0, [x19]
	bl	putc
	add	x19, x19, #1
	sub	x20, x20, #1
	b	1b

// SVC #3: print the image's registers.
svc_dump:
	mov	x19, #0			// register number
1:	cmp	x19, #31
	b.eq	2f
	mov	x21, #10
	udiv	x22, x19, x21		// tens
	msub	x21, x22, x21, x19	// ones
	mov	w0, #'x'
	bl	putc
	add	w0, w22, #'0'
	bl	putc
	add	w0, w21, #'0'
	bl	putc
	b	3f
2:	adr	x0, msg_sp
	bl	puts
3:	bl	space
	ldr	x0, [sp, x19, lsl #3]
	bl	puthex
	add	x19, x19, #1
	tst	x19, #3			// four registers per line
	b.ne	4f
	bl	newline
	cmp	x19, #32
	b.lo	1b
	adr	x0, msg_pc
	bl	puts
	ldr	x0, =SVC_PC
	ldr	x0, [x0]
	bl	puthex
	bl	newline
	b	svc_return
4:	bl	space
	bl	space
	b	1b

// Faults. x19 is the PC to report.
el0_fault:
	mrs	x19, elr_el1
	adr	x20, msg_fault
	b	report
el1_sync:
	// During a monitor call, a fault is the image's: it passed a bad pointer.
	ldr	x0, =SVC_PC
	ldr	x19, [x0]
	adr	x20, msg_fault
	cbnz	x19, report
stub_fault:
	mrs	x19, elr_el1
	adr	x20, msg_stubfault
report:
	mov	x0, x20
	bl	puts
	mrs	x0, esr_el1
	bl	puthex
	bl	space
	mov	x0, x19
	bl	puthex
	bl	space
	mrs	x0, far_el1
	bl	puthex
	bl	newline
	b	poweroff

// Console. The helpers use only x0-x7.

newline:
	mov	w0, #'\n'
	b	putc

space:
	mov	w0, #' '
	b	putc

// Writes the byte in w0.
putc:
	ldr	x1, =UART
1:	ldr	w2, [x1, #0x18]		// UARTFR
	tbnz	w2, #5, 1b		// transmit FIFO full
	strb	w0, [x1]
	ret

// Writes the NUL-terminated string at x0.
puts:
	mov	x4, x0
	mov	x5, x30
1:	ldrb	w0, [x4], #1
	cbz	w0, 2f
	bl	putc
	b	1b
2:	ret	x5

// Writes x0 as 16 hex digits.
puthex:
	mov	x4, x0
	mov	x5, x30
	mov	x6, #60
1:	lsr	x0, x4, x6
	and	x0, x0, #0xf
	add	x7, x0, #'0'
	cmp	x0, #10
	add	x0, x0, #('a' - 10)
	csel	x0, x7, x0, lo
	bl	putc
	subs	x6, x6, #4
	b.pl	1b
	ret	x5

msg_exit:	.asciz	"!vrun exit "
msg_fault:	.asciz	"!vrun fault "
msg_stubfault:	.asciz	"!vrun stubfault "
msg_sp:		.asciz	"sp "
msg_pc:		.asciz	"pc  "
	.ltorg

	.macro	ventry	target
	.balign	128
	b	\target
	.endm

	.balign	2048
vectors:
	ventry	stub_fault		// current EL with SP_EL0: unused
	ventry	stub_fault
	ventry	stub_fault
	ventry	stub_fault
	ventry	el1_sync		// current EL with SP_EL1: the stub
	ventry	stub_fault
	ventry	stub_fault
	ventry	stub_fault
	ventry	el0_sync		// lower EL, AArch64: the image
	ventry	el0_fault
	ventry	el0_fault
	ventry	el0_fault
	ventry	el0_fault		// lower EL, AArch32: never used
	ventry	el0_fault
	ventry	el0_fault
	ventry	el0_fault
