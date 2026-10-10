/*
 * The port (docs/design/0003-tcpip-port.md): the pages the executive and
 * the TCP/IP component share, and the messages they pass there. The
 * executive's side is $PORTDEF in crosstools/vtools/lib/lib.mlb, which must match.
 *
 * Page 0 holds the header and the two rings; page 1 + n is tag n's data
 * buffer, for tags below PORT_BUFFERS. The executive writes commands and
 * cmd_put, the component responses and rsp_put; each reads the other's
 * ring up to its put index and moves its own get index. The indices run
 * free; an entry is at index % PORT_RING. A command takes a tag, a credit,
 * which its response gives back: there are PORT_TAGS of them, as many as
 * a ring holds, so neither ring ever overflows.
 */
#ifndef PORT_H
#define PORT_H

#include <stdint.h>

#define PORT_VERSION 2
#define PORT_PAGES 17
#define PORT_RING 32
#define PORT_TAGS 32
#define PORT_BUFFERS 16 /* tags 0-15 have a buffer, 16-31 none */
#define PORT_BUFSIZE 4096

struct port_msg {
	uint8_t type, flags;
	uint16_t tag;
	uint32_t conn; /* the connection; 0 is the control connection */
	uint32_t status; /* a response's PORT_ST_ */
	uint32_t len; /* bytes in the tag's buffer */
	uint32_t addr; /* an IPv4 address, in network order */
	uint16_t port, proto;
	uint32_t arg1, arg2;
};

struct port_hdr {
	uint32_t version; /* the component's PORT_VERSION, 0 until it is ready */
	uint32_t cmd_put, cmd_get, rsp_put, rsp_get;
	uint32_t reserved[59];
	struct port_msg cmd[PORT_RING]; /* at 0x100 */
	struct port_msg rsp[PORT_RING]; /* at 0x500 */
};

enum {
	PORT_OPEN = 1, /* proto: PORT_TCP, PORT_UDP or PORT_ICMP, a raw socket; then as
			  SETMODE; response: conn */
	PORT_SETMODE, /* flags PORT_BIND: addr, port the local address, 0 for any;
			 PORT_LISTEN: arg1 the backlog */
	PORT_ACCEPT, /* response: arg1 the new conn, addr and port the peer's */
	PORT_CONNECT, /* addr, port: the peer's, a TCP connection's or a datagram's default */
	PORT_SEND, /* len bytes in the buffer, a datagram to addr, port if flags PORT_TO;
		      response: len sent */
	PORT_RECV, /* len: at most so many; response: len bytes, 0 if the peer closed, and a
		      datagram's sender in addr, port; one datagram, the rest of it lost */
	PORT_CLOSE, /* ends the connection and its requests */
	PORT_IFCONFIG, /* sets those flagged: PORT_ADDR addr, PORT_MASK arg1 the mask,
			  PORT_GW arg2 the gateway, or with PORT_DHCP those a DHCP server
			  gives, once it has; response: all three, flags PORT_LINKUP and
			  len the DNS server DHCP gave, or 0 */
	PORT_CANCEL, /* ends the connection's requests with PORT_ST_ABORTED */
	PORT_GETNAME, /* response: addr, port the local name, arg1, arg2 the peer's */
	PORT_SHUTDOWN, /* arg1: 0 no more receives, 1 no more sends, 2 neither */
};

enum { PORT_ICMP = 1, PORT_TCP = 6, PORT_UDP = 17 };
enum { PORT_BIND = 1, PORT_LISTEN = 2, PORT_TO = 1 };
enum { PORT_ADDR = 1, PORT_MASK = 2, PORT_GW = 4, PORT_DHCP = 8, PORT_LINKUP = 16 };

enum {
	PORT_ST_OK, PORT_ST_BADPARAM, PORT_ST_NOMEM, PORT_ST_INUSE, PORT_ST_REFUSED,
	PORT_ST_RESET, PORT_ST_TIMEOUT, PORT_ST_ABORTED, PORT_ST_UNREACH, PORT_ST_CLOSED,
	PORT_ST_NOLINKS, /* not connected, or a datagram with nowhere to go */
	PORT_ST_ISCONN, /* a datagram's address on a connected socket, or connected twice */
	PORT_ST_IVADDR, /* port 0, or an address that can't be */
};

#endif
