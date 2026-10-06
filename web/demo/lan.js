// A LAN between the demo's tabs. QEMU's -netdev socket connects with a socket, which emscripten
// turns into a WebSocket on the page; WebSocket is replaced here by a wire that cuts QEMU's stream,
// each frame after its length in 4 bytes, big-endian, into Ethernet frames and passes them to the
// browser's other tabs on a BroadcastChannel. The wire answers its own system's DHCP: the
// interface gets 10.0.2.N/24, N from the MAC, and no gateway, as there is nothing to route to.

// lan() returns the MAC and address of this tab's system and puts the wire in place.
export function lan() {
  // ponytail: random host number, two tabs out of 239 can get the same one; reload one of them.
  const host = 16 + Math.floor(Math.random() * 239);
  const mac = [0x52, 0x54, 0x00, rnd(), rnd(), host];
  const ip = [10, 0, 2, host];
  const bc = new BroadcastChannel('vaxpunk-lan');

  globalThis.WebSocket = class {
    CONNECTING = 0; OPEN = 1; CLOSING = 2; CLOSED = 3; readyState = 1;
    #buf = new Uint8Array(0);
    constructor() {
      setTimeout(() => this.onopen?.());
      bc.onmessage = e => {
        const f = new Uint8Array(e.data);
        // Frames for this system's MAC, and broadcasts and multicasts.
        if (f[0] & 1 || mac.every((b, i) => f[i] === b)) this.#deliver(f);
      };
    }
    send(data) {
      let b = new Uint8Array(this.#buf.length + data.byteLength);
      b.set(this.#buf); b.set(new Uint8Array(data), this.#buf.length);
      while (b.length >= 4) {
        const n = new DataView(b.buffer, b.byteOffset).getUint32(0);
        if (b.length < 4 + n) break;
        const f = b.slice(4, 4 + n);
        b = b.subarray(4 + n);
        const reply = dhcp(f, mac, ip);
        if (reply) queueMicrotask(() => this.#deliver(reply));
        else bc.postMessage(f.buffer);
      }
      this.#buf = b.slice();
    }
    close() { this.readyState = 3; }
    #deliver(f) {
      const m = new Uint8Array(4 + f.length);
      new DataView(m.buffer).setUint32(0, f.length);
      m.set(f, 4);
      this.onmessage?.({data: m.buffer});
    }
  };
  return {mac: mac.map(b => b.toString(16).padStart(2, '0')).join(':'), ip: ip.join('.')};
}

function rnd() { return Math.floor(Math.random() * 256); }

// dhcp returns the OFFER or ACK for a DHCP DISCOVER or REQUEST frame f from mac, giving it ip,
// or null if f is anything else.
function dhcp(f, mac, ip) {
  const ihl = (f[14] & 15) * 4, b = 14 + ihl + 8;
  if (f[12] !== 8 || f[13] !== 0 || f[23] !== 17 || f[14 + ihl + 2] !== 0 || f[14 + ihl + 3] !== 67) return null;
  let type = 0;
  for (let o = b + 240; o < f.length && f[o] !== 255; o += f[o] ? 2 + f[o + 1] : 1)
    if (f[o] === 53) type = f[o + 2];
  if (type !== 1 && type !== 3) return null;
  const server = [10, 0, 2, 2];
  const opts = [53, 1, type === 1 ? 2 : 5, 54, 4, ...server, 51, 4, 0, 1, 0x51, 0x80, 1, 4, 255, 255, 255, 0, 255];
  const r = new Uint8Array(14 + 20 + 8 + 240 + opts.length);
  r.set(mac, 0); r.set([0x52, 0x54, 0, 0, 0, 2, 8, 0], 6);
  // IP to 255.255.255.255, then UDP from 67 to 68 without a checksum.
  const ipLen = r.length - 14, d = new DataView(r.buffer);
  r.set([0x45, 0, ipLen >> 8, ipLen & 255, 0, 0, 0, 0, 64, 17, 0, 0, ...server, 255, 255, 255, 255], 14);
  let sum = 0;
  for (let i = 14; i < 34; i += 2) sum += d.getUint16(i);
  while (sum > 0xffff) sum = (sum & 0xffff) + (sum >> 16);
  d.setUint16(24, ~sum & 0xffff);
  d.setUint16(34, 67); d.setUint16(36, 68); d.setUint16(38, ipLen - 20);
  // BOOTP reply: the request's xid, flags and chaddr, yiaddr ip; then the magic cookie and options.
  r.set([2, 1, 6, 0], 42);
  r.set(f.subarray(b + 4, b + 8), 46);
  r.set(f.subarray(b + 10, b + 12), 52);
  r.set(ip, 58);
  r.set(f.subarray(b + 28, b + 44), 70);
  r.set([99, 130, 83, 99, ...opts], 278);
  return r;
}
