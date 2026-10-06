/*
 * The port (docs/design/0003-tcpip-port.md): the pages the executive and
 * the TCP/IP component share, and the messages they pass there. The
 * executive's side is $PORTDEF in vtools/lib/lib.mlb, which must match.
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

#define PORT_VERSION 1
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
	PORT_OPEN = 1, /* proto: PORT_TCP; response: conn */
	PORT_BIND, /* addr, port: the local address, 0 for any */
	PORT_LISTEN, /* arg1: the backlog */
	PORT_ACCEPT, /* response: arg1 the new conn, addr and port the peer's */
	PORT_CONNECT, /* addr, port: the peer's */
	PORT_SEND, /* len bytes in the buffer; response: len sent */
	PORT_RECV, /* len: at most so many; response: len bytes, 0 if the peer closed */
	PORT_CLOSE, /* ends the connection and its requests */
	PORT_IFCONFIG, /* flags PORT_SET: addr, arg1 mask, arg2 gateway, or with PORT_DHCP
			  those a DHCP server gives, once it has; response: those and
			  flags PORT_LINKUP */
	PORT_CANCEL, /* ends the connection's requests with PORT_ST_ABORTED */
	PORT_PING, /* addr: an ICMP echo request there; response: arg1 the round trip in ms,
		      arg2 the reply's TTL, or PORT_ST_TIMEOUT after a second without one */
};

enum { PORT_TCP = 6 };
enum { PORT_SET = 1, PORT_LINKUP = 2, PORT_DHCP = 4 };

enum {
	PORT_ST_OK, PORT_ST_BADPARAM, PORT_ST_NOMEM, PORT_ST_INUSE, PORT_ST_REFUSED,
	PORT_ST_RESET, PORT_ST_TIMEOUT, PORT_ST_ABORTED, PORT_ST_UNREACH, PORT_ST_CLOSED,
};

#endif
