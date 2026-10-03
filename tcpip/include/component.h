/*
 * What the root task sets up for the TCP/IP component, and the component
 * expects: where things are in its address space, its capabilities and
 * the badges on its notification.
 */
#ifndef COMPONENT_H
#define COMPONENT_H

#define TCPIP_BASE 0x200000UL	   /* its 2 MB large page: image, data, DMA */
#define TCPIP_PORT_VA 0x400000UL   /* the port's PORT_PAGES pages */
#define TCPIP_MMIO_VA 0x420000UL   /* the page with its virtio-net transport */
#define TCPIP_UART_VA 0x421000UL   /* the console, for its messages */
#define TCPIP_IPCBUF_VA 0x422000UL /* its IPC buffer */

/* Its CSpace: a CNode of 8 slots. */
enum {
	TCPIP_CAP_NTFN = 1, /* its notification, which it waits on */
	TCPIP_CAP_EXEC = 2, /* the executive's completion interrupt, signalled */
	TCPIP_CAP_IRQ = 3,  /* the virtio-net interrupt's handler, acked */
};
#define TCPIP_CNODE_BITS 3

/* The bits of its notification's badge: why it woke. */
enum { TCPIP_DOORBELL = 1, TCPIP_TICK = 2, TCPIP_IRQ = 4 };
#define TCPIP_TICK_MS 10

#endif
