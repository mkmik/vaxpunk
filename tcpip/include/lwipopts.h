/* lwIP's configuration: the raw API, no OS (NO_SYS), IPv4 over Ethernet
 * with TCP, UDP, ARP, ICMP and raw ICMP, configured by the executive or by DHCP. */
#ifndef LWIPOPTS_H
#define LWIPOPTS_H

#define NO_SYS 1
#define SYS_LIGHTWEIGHT_PROT 0
#define LWIP_SOCKET 0
#define LWIP_NETCONN 0

#define LWIP_IPV4 1
#define LWIP_IPV6 0
#define LWIP_ARP 1
#define LWIP_ETHERNET 1
#define LWIP_ICMP 1
#define LWIP_TCP 1
#define LWIP_UDP 1
#define LWIP_DHCP 1
#define LWIP_RAW 1 /* ICMP echoes, for PING */
/* ponytail: no ARP probe of the offered address, which takes seconds; turn
 * it on where another host may hold it. */
#define LWIP_DHCP_DOES_ACD_CHECK 0
#define LWIP_DNS 0
#define LWIP_IGMP 0
#define LWIP_STATS 0
#define LWIP_NETIF_LOOPBACK 1 /* to its own address, through netif_poll */

#define MEM_ALIGNMENT 8
#define MEM_SIZE (128 * 1024)
#define MEMP_NUM_PBUF 32
#define MEMP_NUM_TCP_PCB 32
#define MEMP_NUM_TCP_PCB_LISTEN 8
#define MEMP_NUM_TCP_SEG 64
#define PBUF_POOL_SIZE 64

#define TCP_MSS 1460
#define TCP_WND (8 * TCP_MSS)
#define TCP_SND_BUF (8 * TCP_MSS)
#define TCP_SND_QUEUELEN 32

#endif
