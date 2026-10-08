/* The TCP/IP component's image, tcpip.elf, which the PAL starts
 * (start_tcpip in main.c). build.rs defines TCPIP_ELF, its path. */
	.section .rodata
	.balign	8
	.global	tcpip_elf, tcpip_elf_end
tcpip_elf:
	.incbin	TCPIP_ELF
tcpip_elf_end:
