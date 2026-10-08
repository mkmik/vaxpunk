// Command tailgate is the demo's way out of the browser: a tailcat client (github.com/tailscale/tailcat),
// compiled to WebAssembly, behind a gVisor TCP/IP stack that is 10.0.2.2 on the tab's LAN. A
// connection from the VMS system to 10.0.2.2:port goes to that port of the tailcat server, which
// proxies it to its own localhost; one to any other address goes through the server as an exit
// node ("tailcat serve exit-node"). UDP, DNS among it, goes the same two ways; ICMP goes nowhere,
// so only 10.0.2.2 answers a ping. Tailcat reaches the server over DERP relays, on WebSockets.
//
// It sets one global JavaScript function:
//
//	tailgate({addr, mac, output, status}) returns {input, close}
//
// addr is the server's tailcat address, mac the gateway's, output(Uint8Array) is called with each
// Ethernet frame the gateway sends, and input(Uint8Array) takes each frame the system sends.
// status(ok, text) is called once tailcat has reached the server, with the round trip, or has
// given up, with why. close() takes the gateway down; another tailgate call can put one up.
package main

import (
	"context"
	"io"
	"log"
	"net"
	"net/netip"
	"syscall/js"
	"time"

	"github.com/tailscale/tailcat"
	"gvisor.dev/gvisor/pkg/buffer"
	"gvisor.dev/gvisor/pkg/tcpip"
	"gvisor.dev/gvisor/pkg/tcpip/adapters/gonet"
	"gvisor.dev/gvisor/pkg/tcpip/header"
	"gvisor.dev/gvisor/pkg/tcpip/link/channel"
	"gvisor.dev/gvisor/pkg/tcpip/link/ethernet"
	"gvisor.dev/gvisor/pkg/tcpip/network/arp"
	"gvisor.dev/gvisor/pkg/tcpip/network/ipv4"
	"gvisor.dev/gvisor/pkg/tcpip/stack"
	"gvisor.dev/gvisor/pkg/tcpip/transport/tcp"
	"gvisor.dev/gvisor/pkg/tcpip/transport/udp"
	"gvisor.dev/gvisor/pkg/waiter"
	"tailscale.com/types/logger"
)

var gw = netip.MustParseAddr("10.0.2.2")

func main() {
	js.Global().Set("tailgate", js.FuncOf(start))
	select {}
}

func start(this js.Value, args []js.Value) any {
	opts := args[0]
	output, status := opts.Get("output"), opts.Get("status")
	mac, err := net.ParseMAC(opts.Get("mac").String())
	if err != nil {
		panic(err)
	}
	cl := &tailcat.Client{Server: tailcat.Addr(opts.Get("addr").String()), Logf: logger.Discard}

	s := stack.New(stack.Options{
		NetworkProtocols:   []stack.NetworkProtocolFactory{ipv4.NewProtocol, arp.NewProtocol},
		TransportProtocols: []stack.TransportProtocolFactory{tcp.NewProtocol, udp.NewProtocol},
	})
	link := channel.New(256, 1500, tcpip.LinkAddress(mac))
	if err := s.CreateNIC(1, ethernet.New(link)); err != nil {
		panic(err.String())
	}
	s.AddProtocolAddress(1, tcpip.ProtocolAddress{
		Protocol:          ipv4.ProtocolNumber,
		AddressWithPrefix: tcpip.AddressWithPrefix{Address: tcpip.AddrFrom4(gw.As4()), PrefixLen: 24},
	}, stack.AddressProperties{})
	// Promiscuous and spoofing: take connections to any address, and answer from it.
	s.SetPromiscuousMode(1, true)
	s.SetSpoofing(1, true)
	s.SetRouteTable([]tcpip.Route{{Destination: header.IPv4EmptySubnet, NIC: 1}})
	fwd := tcp.NewForwarder(s, 0, 64, func(r *tcp.ForwarderRequest) {
		id := r.ID()
		dst := netip.AddrPortFrom(netip.AddrFrom4(id.LocalAddress.As4()), id.LocalPort)
		go forward(cl, r, dst)
	})
	s.SetTransportProtocolHandler(tcp.ProtocolNumber, fwd.HandlePacket)
	ufwd := udp.NewForwarder(s, func(r *udp.ForwarderRequest) bool {
		id := r.ID()
		dst := netip.AddrPortFrom(netip.AddrFrom4(id.LocalAddress.As4()), id.LocalPort)
		var wq waiter.Queue
		ep, err := r.CreateEndpoint(&wq)
		if err != nil {
			return false
		}
		go forwardUDP(cl, gonet.NewUDPConn(&wq, ep), dst)
		return true
	})
	s.SetTransportProtocolHandler(udp.ProtocolNumber, ufwd.HandlePacket)

	ctx, cancel := context.WithCancel(context.Background())
	go func() {
		pctx, pcancel := context.WithTimeout(ctx, time.Minute)
		defer pcancel()
		for {
			// The first pings can be lost while either side's DERP connection is coming up.
			c, ccancel := context.WithTimeout(pctx, 5*time.Second)
			r, err := cl.Ping(c)
			ccancel()
			if err == nil {
				status.Invoke(true, r.Latency.Round(time.Millisecond).String())
				return
			}
			if pctx.Err() != nil {
				if ctx.Err() == nil {
					status.Invoke(false, err.Error())
				}
				return
			}
		}
	}()
	go func() {
		for {
			pkt := link.ReadContext(ctx)
			if pkt == nil {
				return
			}
			b := pkt.ToView().AsSlice()
			u8 := js.Global().Get("Uint8Array").New(len(b))
			js.CopyBytesToJS(u8, b)
			pkt.DecRef()
			output.Invoke(u8)
		}
	}()
	return js.ValueOf(map[string]any{
		"input": js.FuncOf(func(this js.Value, args []js.Value) any {
			b := make([]byte, args[0].Get("length").Int())
			js.CopyBytesToGo(b, args[0])
			pkt := stack.NewPacketBuffer(stack.PacketBufferOptions{Payload: buffer.MakeWithData(b)})
			link.InjectInbound(0, pkt)
			pkt.DecRef()
			return nil
		}),
		"close": js.FuncOf(func(this js.Value, args []js.Value) any {
			cancel()
			s.Close()
			cl.Close()
			return nil
		}),
	})
}

// forward completes the system's connection r to dst once tailcat has one to the same place, and
// copies between them; or refuses r if tailcat can't connect.
func forward(cl *tailcat.Client, r *tcp.ForwarderRequest, dst netip.AddrPort) {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	var c net.Conn
	var err error
	if dst.Addr() == gw {
		c, err = cl.DialTCPPort(ctx, dst.Port())
	} else {
		c, err = cl.DialTCP(ctx, dst)
	}
	if err != nil {
		log.Printf("tailgate: %v: %v", dst, err)
		r.Complete(true)
		return
	}
	var wq waiter.Queue
	ep, terr := r.CreateEndpoint(&wq)
	if terr != nil {
		r.Complete(true)
		c.Close()
		return
	}
	r.Complete(false)
	g := gonet.NewTCPConn(&wq, ep)
	go func() { io.Copy(c, g); closeWrite(c) }()
	io.Copy(g, c)
	g.CloseWrite()
}

// forwardUDP copies datagrams between the system's flow g and dst, through tailcat, until the flow
// has been idle for a minute.
func forwardUDP(cl *tailcat.Client, g net.Conn, dst netip.AddrPort) {
	defer g.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	var c net.Conn
	var err error
	if dst.Addr() == gw {
		c, err = cl.DialUDPPort(ctx, dst.Port())
	} else {
		c, err = cl.DialUDP(ctx, dst)
	}
	if err != nil {
		log.Printf("tailgate: udp %v: %v", dst, err)
		return
	}
	defer c.Close()
	go copyUDP(g, c)
	copyUDP(c, g)
}

// copyUDP copies datagrams from src to dst until src has been idle for a minute or either fails;
// then it closes both, which stops the copy the other way too.
func copyUDP(dst, src net.Conn) {
	defer dst.Close()
	defer src.Close()
	b := make([]byte, 65535)
	for {
		src.SetReadDeadline(time.Now().Add(time.Minute))
		n, err := src.Read(b)
		if err != nil {
			return
		}
		if _, err := dst.Write(b[:n]); err != nil {
			return
		}
	}
}

func closeWrite(c net.Conn) {
	if cw, ok := c.(interface{ CloseWrite() error }); ok {
		cw.CloseWrite()
	} else {
		c.Close()
	}
}
