/*
 * The TCP/IP component (docs/prd/0002-networking.md): lwIP, a virtio-net
 * driver and the adapter between lwIP's raw API and the port, the pages
 * it shares with the executive (include/port.h). It is a seL4 thread of
 * its own, in an address space of its own, which the root task starts
 * (include/component.h), and it waits on its notification for the
 * executive's doorbell, the device's interrupt and the PAL's clock tick.
 * Each time it wakes it takes the device's frames to lwIP, runs lwIP's
 * timers, carries out the commands on the port, answers those it can, and
 * signals the executive's completion interrupt once if it answered any.
 *
 * Nothing of seL4 or lwIP is seen above the port: what the executive needs
 * to know is a message there.
 */
#include <stdarg.h>
#include <stdint.h>
#include <string.h>

#include <sel4/sel4.h>

#include "component.h"
#include "lwip/dhcp.h"
#include "lwip/init.h"
#include "lwip/etharp.h"
#include "lwip/netif.h"
#include "lwip/pbuf.h"
#include "lwip/tcp.h"
#include "lwip/timeouts.h"
#include "netif/ethernet.h"
#include "port.h"

LIBSEL4_THREAD_LOCAL seL4_IPCBuffer *__sel4_ipc_buffer;

/* The console, the PAL's PL011, shared for the component's few lines. */
static volatile uint32_t *const uart = (volatile uint32_t *)TCPIP_UART_VA;

static void putc_(char c)
{
	if (c == '\n')
		putc_('\r');
	while (uart[0x18 / 4] & 1 << 5)
		;
	uart[0] = (uint8_t)c;
}

/* printf subset: %s, %c, %u, %d, %x, %lu, %lx, %%. */
void tcpip_print(const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	for (; *fmt; fmt++) {
		if (*fmt != '%') {
			putc_(*fmt);
			continue;
		}
		int wide = *++fmt == 'l';
		fmt += wide;
		if (*fmt == 's') {
			for (const char *s = va_arg(ap, const char *); *s; s++)
				putc_(*s);
			continue;
		}
		if (*fmt == 'c') {
			putc_(va_arg(ap, int));
			continue;
		}
		if (*fmt != 'u' && *fmt != 'd' && *fmt != 'x') {
			putc_(*fmt);
			continue;
		}
		unsigned long v = wide ? va_arg(ap, unsigned long) : va_arg(ap, unsigned);
		char buf[20];
		int n = 0;
		do {
			buf[n++] = "0123456789abcdef"[v % (*fmt == 'x' ? 16 : 10)];
			v /= *fmt == 'x' ? 16 : 10;
		} while (v);
		while (n)
			putc_(buf[--n]);
	}
	va_end(ap);
}

/* Stops the component with a fault, which the PAL reports. */
void tcpip_halt(void)
{
	for (;;)
		*(volatile int *)0 = 0;
}

unsigned tcpip_rand(void)
{
	static unsigned x = 2463534242u;
	x ^= x << 13, x ^= x >> 17, x ^= x << 5;
	return x;
}

/* lwIP's clock: the PAL's ticks. */
static uint32_t ticks;

uint32_t sys_now(void)
{
	return ticks * TCPIP_TICK_MS;
}

/* The physical address of va, in the component's 2 MB page. */
static uint64_t base_pa;

static uint64_t pa(const void *va)
{
	return base_pa + ((uintptr_t)va - TCPIP_BASE);
}

/*
 * The network interface: a virtio-net device on a virtio-mmio transport
 * (virtio 1.x, modern), with a receive queue and a transmit queue of
 * VQ_SIZE frames each, its buffers in the component's page.
 * ponytail: a frame is copied in and out of a buffer of its own, one
 * descriptor each; no offloads.
 */
enum { VIO_DEVFEAT = 0x010, VIO_DEVFEATSEL = 0x014, VIO_DRVFEAT = 0x020, VIO_DRVFEATSEL = 0x024,
       VIO_QSEL = 0x030, VIO_QNUMMAX = 0x034, VIO_QNUM = 0x038, VIO_QREADY = 0x044,
       VIO_QNOTIFY = 0x050, VIO_INTSTATUS = 0x060, VIO_INTACK = 0x064, VIO_STATUS = 0x070,
       VIO_QDESC = 0x080, VIO_QDRIVER = 0x090, VIO_QDEVICE = 0x0a0, VIO_CONFIG = 0x100 };
enum { VIO_ACK = 1, VIO_DRIVER = 2, VIO_DRIVER_OK = 4, VIO_FEATURES_OK = 8, VIOD_WRITE = 2,
       VIO_NET_F_MAC = 5 };
#define VQ_SIZE 16
#define FRAME 2048
#define NET_HDR 12 /* struct virtio_net_hdr, with num_buffers, all 0 */

struct vq {
	struct {
		uint64_t addr;
		uint32_t len;
		uint16_t flags, next;
	} desc[VQ_SIZE];
	struct {
		uint16_t flags, idx, ring[VQ_SIZE];
	} avail __attribute__((aligned(64)));
	struct {
		uint16_t flags, idx;
		struct {
			uint32_t id, len;
		} ring[VQ_SIZE];
	} used __attribute__((aligned(64)));
	uint16_t last_used; /* the used ring's entries taken so far */
	uint8_t buf[VQ_SIZE][FRAME] __attribute__((aligned(4096)));
};

static struct vq rxq __attribute__((aligned(4096))), txq __attribute__((aligned(4096)));
static volatile uint32_t *vio;
static struct netif netif;
static int tx_free = VQ_SIZE; /* transmit buffers the device gave back */

static void dmb(void)
{
	__asm__ volatile("dmb sy" ::: "memory");
}

static void wr64(unsigned reg, uint64_t v)
{
	vio[reg / 4] = (uint32_t)v;
	vio[reg / 4 + 1] = v >> 32;
}

static int queue_setup(unsigned n, struct vq *q)
{
	vio[VIO_QSEL / 4] = n;
	if (vio[VIO_QNUMMAX / 4] < VQ_SIZE)
		return 0;
	vio[VIO_QNUM / 4] = VQ_SIZE;
	wr64(VIO_QDESC, pa(q->desc));
	wr64(VIO_QDRIVER, pa(&q->avail));
	wr64(VIO_QDEVICE, pa(&q->used));
	vio[VIO_QREADY / 4] = 1;
	return 1;
}

/* Gives receive buffer i to the device. */
static void rx_post(unsigned i)
{
	rxq.desc[i] = (typeof(rxq.desc[0])){ pa(rxq.buf[i]), FRAME, VIOD_WRITE, 0 };
	rxq.avail.ring[rxq.avail.idx % VQ_SIZE] = i;
	dmb();
	rxq.avail.idx++;
}

static err_t linkoutput(struct netif *nif, struct pbuf *p)
{
	(void)nif;
	if (!tx_free || p->tot_len > FRAME - NET_HDR)
		return ERR_MEM;
	unsigned i = txq.avail.idx % VQ_SIZE;
	memset(txq.buf[i], 0, NET_HDR);
	pbuf_copy_partial(p, txq.buf[i] + NET_HDR, p->tot_len, 0);
	txq.desc[i] = (typeof(txq.desc[0])){ pa(txq.buf[i]), NET_HDR + p->tot_len, 0, 0 };
	txq.avail.ring[i] = i;
	dmb();
	txq.avail.idx++;
	tx_free--;
	dmb();
	vio[VIO_QNOTIFY / 4] = 1;
	return ERR_OK;
}

/* Takes what the device is done with: frames received, to lwIP, and
 * transmit buffers back. */
static void net_poll(void)
{
	vio[VIO_INTACK / 4] = vio[VIO_INTSTATUS / 4];
	dmb();
	int posted = 0;
	while (rxq.last_used != *(volatile uint16_t *)&rxq.used.idx) {
		dmb();
		unsigned i = rxq.used.ring[rxq.last_used % VQ_SIZE].id;
		uint32_t len = rxq.used.ring[rxq.last_used % VQ_SIZE].len;
		rxq.last_used++;
		if (len > NET_HDR && len <= FRAME) {
			struct pbuf *p = pbuf_alloc(PBUF_RAW, len - NET_HDR, PBUF_POOL);
			if (p) {
				pbuf_take(p, rxq.buf[i] + NET_HDR, len - NET_HDR);
				if (netif.input(p, &netif) != ERR_OK)
					pbuf_free(p);
			}
		}
		rx_post(i);
		posted = 1;
	}
	if (posted) {
		dmb();
		vio[VIO_QNOTIFY / 4] = 0;
	}
	while (txq.last_used != *(volatile uint16_t *)&txq.used.idx) {
		txq.last_used++;
		tx_free++;
	}
}

static err_t netif_setup(struct netif *nif)
{
	nif->name[0] = 'v', nif->name[1] = 'n';
	nif->output = etharp_output;
	nif->linkoutput = linkoutput;
	nif->mtu = 1500;
	nif->hwaddr_len = 6;
	for (int i = 0; i < 6; i++)
		nif->hwaddr[i] = ((volatile uint8_t *)vio)[VIO_CONFIG + i];
	nif->flags = NETIF_FLAG_BROADCAST | NETIF_FLAG_ETHARP | NETIF_FLAG_ETHERNET;
	return ERR_OK;
}

/* Sets up the device and lwIP's interface on it, with no address. */
static int net_init(void)
{
	vio[VIO_STATUS / 4] = 0;
	vio[VIO_STATUS / 4] = VIO_ACK | VIO_DRIVER;
	vio[VIO_DEVFEATSEL / 4] = 0;
	uint32_t mac = vio[VIO_DEVFEAT / 4] & 1u << VIO_NET_F_MAC;
	vio[VIO_DRVFEATSEL / 4] = 0;
	vio[VIO_DRVFEAT / 4] = mac;
	vio[VIO_DRVFEATSEL / 4] = 1;
	vio[VIO_DRVFEAT / 4] = 1; /* VIRTIO_F_VERSION_1 */
	vio[VIO_STATUS / 4] = VIO_ACK | VIO_DRIVER | VIO_FEATURES_OK;
	if (!(vio[VIO_STATUS / 4] & VIO_FEATURES_OK) || !queue_setup(0, &rxq) ||
	    !queue_setup(1, &txq))
		return 0;
	for (unsigned i = 0; i < VQ_SIZE; i++)
		rx_post(i);
	vio[VIO_STATUS / 4] = VIO_ACK | VIO_DRIVER | VIO_FEATURES_OK | VIO_DRIVER_OK;
	vio[VIO_QNOTIFY / 4] = 0;

	lwip_init();
	netif_add(&netif, IP4_ADDR_ANY4, IP4_ADDR_ANY4, IP4_ADDR_ANY4, 0, netif_setup,
		  ethernet_input);
	netif_set_default(&netif);
	netif_set_link_up(&netif);
	uint8_t *m = netif.hwaddr;
	tcpip_print("tcpip: lwIP %s on virtio-net, MAC %x:%x:%x:%x:%x:%x\n", LWIP_VERSION_STRING,
		    m[0], m[1], m[2], m[3], m[4], m[5]);
	return 1;
}

/*
 * The port's connections, by number, from 1: a TCP PCB each, what it
 * received and the executive hasn't read, and for one that listens the
 * connections accepted that no ACCEPT has taken yet.
 */
#define NCONN 32
#define BACKLOG 4
#define DHCP_WAIT (10000 / TCPIP_TICK_MS) /* ticks an IFCONFIG waits for DHCP's address */
static struct conn {
	int used, connected, closed; /* closed: the peer sent FIN */
	uint32_t err;		     /* a PORT_ST_ the connection failed with */
	struct tcp_pcb *pcb;
	struct pbuf *rx;
	int listening, nbacklog;
	uint32_t backlog[BACKLOG];
} conns[NCONN];

static struct port_hdr *const port = (struct port_hdr *)TCPIP_PORT_VA;

static uint8_t *buffer(unsigned tag)
{
	return (uint8_t *)TCPIP_PORT_VA + 4096 * (1 + tag);
}

/* The commands waiting for something: an ACCEPT, CONNECT, SEND or RECV,
 * or an IFCONFIG for DHCP's address, by tag, since a tag is in one command at a time. */
static struct port_msg pend[PORT_TAGS];
static int answered;

static void respond(struct port_msg *m, uint32_t status)
{
	m->status = status;
	port->rsp[port->rsp_put % PORT_RING] = *m;
	dmb();
	port->rsp_put++;
	answered = 1;
	if (m->tag < PORT_TAGS)
		pend[m->tag].type = 0;
}

static void wait_for(struct port_msg *m)
{
	if (m->tag < PORT_TAGS)
		pend[m->tag] = *m;
	else
		respond(m, PORT_ST_BADPARAM);
}

static uint32_t st(err_t err)
{
	switch (err) {
	case ERR_OK:
		return PORT_ST_OK;
	case ERR_MEM:
	case ERR_BUF:
		return PORT_ST_NOMEM;
	case ERR_USE:
	case ERR_ISCONN:
		return PORT_ST_INUSE;
	case ERR_RST:
		return PORT_ST_RESET;
	case ERR_TIMEOUT:
	case ERR_ABRT:
		return PORT_ST_TIMEOUT;
	case ERR_RTE:
		return PORT_ST_UNREACH;
	case ERR_CLSD:
		return PORT_ST_CLOSED;
	default:
		return PORT_ST_BADPARAM;
	}
}

static struct conn *conn_of(uint32_t id)
{
	return id && id < NCONN && conns[id].used ? &conns[id] : 0;
}

static err_t on_recv(void *arg, struct tcp_pcb *pcb, struct pbuf *p, err_t err)
{
	(void)pcb, (void)err;
	struct conn *c = &conns[(uintptr_t)arg];
	if (!p)
		c->closed = 1;
	else if (c->rx)
		pbuf_cat(c->rx, p);
	else
		c->rx = p;
	return ERR_OK;
}

static void on_err(void *arg, err_t err)
{
	struct conn *c = &conns[(uintptr_t)arg];
	c->pcb = 0;
	c->err = err == ERR_RST && !c->connected ? PORT_ST_REFUSED : st(err);
}

static err_t on_connected(void *arg, struct tcp_pcb *pcb, err_t err)
{
	(void)pcb, (void)err;
	conns[(uintptr_t)arg].connected = 1;
	return ERR_OK;
}

/* A free connection for pcb, with the callbacks set, or 0. */
static uint32_t conn_new(struct tcp_pcb *pcb)
{
	for (uint32_t id = 1; id < NCONN; id++)
		if (!conns[id].used) {
			conns[id] = (struct conn){ .used = 1, .pcb = pcb };
			tcp_arg(pcb, (void *)(uintptr_t)id);
			tcp_recv(pcb, on_recv);
			tcp_err(pcb, on_err);
			return id;
		}
	return 0;
}

static err_t on_accept(void *arg, struct tcp_pcb *pcb, err_t err)
{
	struct conn *l = &conns[(uintptr_t)arg];
	if (err != ERR_OK || !pcb || l->nbacklog == BACKLOG)
		return ERR_MEM;
	uint32_t id = conn_new(pcb);
	if (!id)
		return ERR_MEM;
	conns[id].connected = 1;
	l->backlog[l->nbacklog++] = id;
	return ERR_OK;
}

/* Ends the connection's waiting commands with status. */
static void cancel(uint32_t id, uint32_t status)
{
	for (unsigned t = 0; t < PORT_TAGS; t++)
		if (pend[t].type && pend[t].conn == id)
			respond(&pend[t], status);
}

static void conn_free(uint32_t id)
{
	struct conn *c = &conns[id];
	cancel(id, PORT_ST_ABORTED);
	for (int i = 0; i < c->nbacklog; i++)
		conn_free(c->backlog[i]);
	if (c->pcb) {
		tcp_arg(c->pcb, 0);
		if (!c->listening) {
			tcp_recv(c->pcb, 0);
			tcp_err(c->pcb, 0);
		}
		if (tcp_close(c->pcb) != ERR_OK)
			tcp_abort(c->pcb);
	}
	if (c->rx)
		pbuf_free(c->rx);
	c->used = 0;
}

/* Answers an IFCONFIG with the interface's address, mask and gateway. */
static void ifconfig(struct port_msg *m, uint32_t status)
{
	m->addr = ip4_addr_get_u32(netif_ip4_addr(&netif));
	m->arg1 = ip4_addr_get_u32(netif_ip4_netmask(&netif));
	m->arg2 = ip4_addr_get_u32(netif_ip4_gw(&netif));
	m->flags = netif_is_link_up(&netif) ? PORT_LINKUP : 0;
	respond(m, status);
}

/* Answers a waiting command if it can be: m is pend[tag]. */
static void try_finish(struct port_msg *m)
{
	if (m->type == PORT_IFCONFIG) {
		/* len: the tick DHCP has until; it goes on after */
		if (dhcp_supplied_address(&netif))
			ifconfig(m, PORT_ST_OK);
		else if ((int32_t)(ticks - m->len) >= 0)
			ifconfig(m, PORT_ST_TIMEOUT);
		return;
	}
	struct conn *c = conn_of(m->conn);
	if (!c) {
		respond(m, PORT_ST_BADPARAM);
		return;
	}
	switch (m->type) {
	case PORT_ACCEPT:
		if (c->nbacklog) {
			m->arg1 = c->backlog[0];
			struct tcp_pcb *p = conns[m->arg1].pcb;
			m->addr = p ? ip4_addr_get_u32(&p->remote_ip) : 0;
			m->port = p ? p->remote_port : 0;
			memmove(c->backlog, c->backlog + 1, --c->nbacklog * sizeof c->backlog[0]);
			respond(m, PORT_ST_OK);
		}
		return;
	case PORT_CONNECT:
		if (c->connected)
			respond(m, PORT_ST_OK);
		else if (c->err)
			respond(m, c->err);
		return;
	case PORT_SEND: {
		/* arg2: the bytes written so far */
		if (!c->pcb) {
			respond(m, c->err ? c->err : PORT_ST_CLOSED);
			return;
		}
		uint32_t n = m->len - m->arg2, room = tcp_sndbuf(c->pcb);
		if (n > room)
			n = room;
		if (n && tcp_write(c->pcb, buffer(m->tag) + m->arg2, n, TCP_WRITE_FLAG_COPY) == ERR_OK)
			m->arg2 += n;
		tcp_output(c->pcb);
		if (m->arg2 == m->len)
			respond(m, PORT_ST_OK);
		return;
	}
	case PORT_RECV:
		if (c->rx) {
			uint32_t n = pbuf_copy_partial(c->rx, buffer(m->tag), m->len, 0);
			c->rx = pbuf_free_header(c->rx, n);
			if (c->pcb)
				tcp_recved(c->pcb, n);
			m->len = n;
			respond(m, PORT_ST_OK);
		} else if (c->closed || !c->pcb) {
			m->len = 0;
			respond(m, c->closed ? PORT_ST_CLOSED : c->err);
		}
		return;
	}
}

/* Carries out a command from the executive's ring. */
static void command(struct port_msg *m)
{
	struct conn *c = conn_of(m->conn);
	ip4_addr_t ip;
	ip4_addr_set_u32(&ip, m->addr);
	switch (m->type) {
	case PORT_OPEN: {
		struct tcp_pcb *pcb = m->proto == PORT_TCP ? tcp_new() : 0;
		uint32_t id = pcb ? conn_new(pcb) : 0;
		if (pcb && !id)
			tcp_abort(pcb);
		m->conn = id;
		respond(m, m->proto != PORT_TCP ? PORT_ST_BADPARAM : id ? PORT_ST_OK : PORT_ST_NOMEM);
		return;
	}
	case PORT_IFCONFIG: {
		if (m->flags & PORT_SET) {
			dhcp_release_and_stop(&netif);
			netif_set_up(&netif);
			if (m->flags & PORT_DHCP) {
				if (dhcp_start(&netif) != ERR_OK) {
					respond(m, PORT_ST_NOMEM);
					return;
				}
				m->len = ticks + DHCP_WAIT;
				wait_for(m);
				return;
			}
			ip4_addr_t mask, gw;
			ip4_addr_set_u32(&mask, m->arg1);
			ip4_addr_set_u32(&gw, m->arg2);
			netif_set_addr(&netif, &ip, &mask, &gw);
		}
		ifconfig(m, PORT_ST_OK);
		return;
	}
	}
	if (!c) {
		respond(m, PORT_ST_BADPARAM);
		return;
	}
	switch (m->type) {
	case PORT_BIND:
		respond(m, c->pcb && !c->listening ? st(tcp_bind(c->pcb, &ip, m->port))
						   : PORT_ST_BADPARAM);
		return;
	case PORT_LISTEN: {
		struct tcp_pcb *l = c->pcb && !c->listening
					    ? tcp_listen_with_backlog(c->pcb, m->arg1 ? m->arg1 : 1)
					    : 0;
		if (!l) {
			respond(m, PORT_ST_NOMEM);
			return;
		}
		c->pcb = l, c->listening = 1;
		tcp_accept(l, on_accept);
		respond(m, PORT_ST_OK);
		return;
	}
	case PORT_CONNECT:
		if (!c->pcb || c->listening || c->connected) {
			respond(m, PORT_ST_BADPARAM);
			return;
		}
		err_t err = tcp_connect(c->pcb, &ip, m->port, on_connected);
		if (err != ERR_OK) {
			respond(m, st(err));
			return;
		}
		wait_for(m);
		return;
	case PORT_ACCEPT:
	case PORT_SEND:
	case PORT_RECV:
		if (c->listening != (m->type == PORT_ACCEPT) ||
		    (m->type != PORT_ACCEPT && (m->tag >= PORT_BUFFERS || m->len > PORT_BUFSIZE))) {
			respond(m, PORT_ST_BADPARAM);
			return;
		}
		m->arg2 = 0;
		wait_for(m);
		return;
	case PORT_CANCEL:
		cancel(m->conn, PORT_ST_ABORTED);
		respond(m, PORT_ST_OK);
		return;
	case PORT_CLOSE:
		conn_free(m->conn);
		respond(m, PORT_ST_OK);
		return;
	}
	respond(m, PORT_ST_BADPARAM);
}

int main(uint64_t paddr, uint64_t offset)
{
	seL4_SetIPCBuffer((seL4_IPCBuffer *)TCPIP_IPCBUF_VA);
	base_pa = paddr;
	vio = (volatile uint32_t *)(TCPIP_MMIO_VA + offset);
	if (!net_init()) {
		tcpip_print("tcpip: the virtio-net device won't start\n");
		tcpip_halt();
	}
	port->version = PORT_VERSION;
	for (;;) {
		seL4_Word badge;
		seL4_Wait(TCPIP_CAP_NTFN, &badge);
		if (badge & TCPIP_TICK)
			ticks++;
		net_poll();
		if (badge & TCPIP_IRQ)
			seL4_IRQHandler_Ack(TCPIP_CAP_IRQ);
		sys_check_timeouts();
		netif_poll(&netif);
		while (port->cmd_get != *(volatile uint32_t *)&port->cmd_put) {
			dmb();
			struct port_msg m = port->cmd[port->cmd_get % PORT_RING];
			port->cmd_get++;
			command(&m);
		}
		for (unsigned t = 0; t < PORT_TAGS; t++)
			if (pend[t].type)
				try_finish(&pend[t]);
		netif_poll(&netif);
		if (answered) {
			answered = 0;
			seL4_Signal(TCPIP_CAP_EXEC);
		}
	}
}
