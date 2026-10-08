//! cargo test -p boot --test network: TCP/IP (docs/prd/0002-networking.md),
//! in two parts, one after the other.
//!
//! One vaxpunk on QEMU's user network, with nothing saved on its ramdisk,
//! so START COMMUNICATION asks QEMU's DHCP server at boot for the address,
//! mask and gateway: SHOW NAME_SERVICE, then a DNS server here for the
//! process's resolver, through TCPIP$BIND_PORT; then TCPIP's SET INTERFACE, SET ROUTE,
//! one without /DEFAULT that fails, PING to QEMU's gateway and to
//! an address nobody has, and at the TCPIP> prompt SHOW
//! INTERFACE, HELP, an interface there is not, and EXIT; then the hosts
//! database: SHOW HOST, which makes it with LOCALHOST, SET HOST with
//! aliases, SHOW HOST of them all, of an alias and of a name it hasn't,
//! SET NOHOST, confirmed, and PING by an alias; then SET and SHOW
//! NAME_SERVICE, nslookup of a name, an address, one there isn't, one that
//! gets no answer, and at its > prompt, and PING by a name DNS knows;
//! then TCPTEST, which
//! connects to a server here, through QEMU's guestfwd, accepts a
//! connection from a client here, through hostfwd, and sends a datagram
//! from here back twice, to its sender and connected to it; and SET HOST to
//! itself, SHOW SYSTEM there, and LOGOUT; then TELNET, by a name DNS knows, to a port here,
//! through guestfwd, a line each way, and with /PORT to one there cannot be;
//! then COPY/HTTP from web servers here, through guestfwd, to the data
//! disk: by URL, by node and path, one that says 404, by URL with a
//! host's name, and an https URL.
//!
//! Two vaxpunks on one QEMU socket network, A and B, where no DHCP server
//! answers at boot: each saves its address and gateway with SET
//! CONFIGURATION INTERFACE and SET ROUTE /PERMANENT, on its ramdisk, as
//! SYSTARTUP_VMS.COM would, and START COMMUNICATION applies them. B logs
//! in to A with TELNET, SHOW SYSTEM lists A's processes, and LOGOUT comes
//! back to B, which pings A.
//! Then B saves another address, keeping the saved gateway, another
//! gateway with SET ROUTE /PERMANENT, keeping that address, and DHCP,
//! keeping both, which SHOW INTERFACE doesn't show, since they are for
//! the next START COMMUNICATION, and which TYPE shows in the saved file.

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

use ods_image::{Conversion, Image, InitParams, Mode};

/// A vaxpunk in QEMU, its console and its log.
struct Vax {
    qemu: Child,
    console: ChildStdin,
    log: PathBuf,
}

impl Vax {
    /// Boots one: with the boot binary, which builds out/ first, or with
    /// run-qemu.sh on what is there. env: LOG, DATADISK, NETDEV, MAC.
    fn boot(cmd: &Path, env: &[(&str, String)]) -> Vax {
        let log = PathBuf::from(&env.iter().find(|(k, _)| *k == "LOG").unwrap().1);
        let _ = fs::remove_file(&log);
        let mut qemu = Command::new(cmd)
            .envs(env.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let console = qemu.stdin.take().unwrap();
        let vax = Vax { qemu, console, log };
        vax.wait_for("\n$ ", 0, 60);
        vax
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&fs::read(&self.log).unwrap_or_default()).replace('\r', "")
    }

    /// Waits until the log has `what` past `from`; returns where it ends.
    fn wait_for(&self, what: &str, from: usize, secs: u64) -> usize {
        let end = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < end {
            if let Some(i) = self.text().get(from..).and_then(|t| t.find(what)) {
                return from + i + what.len();
            }
            sleep(Duration::from_millis(200));
        }
        panic!(
            "{}: no {what:?} after {from}:\n{}",
            self.log.display(),
            self.text()
        );
    }

    /// Types a command at DCL's prompt and waits for the next one.
    fn command(&mut self, line: &str) -> usize {
        self.reply(line, "$ ")
    }

    /// Types a line, again until it is echoed (keys typed while DCL starts
    /// can be lost), and waits for `prompt`.
    fn reply(&mut self, line: &str, prompt: &str) -> usize {
        let from = self.text().len();
        for _ in 0..5 {
            self.console
                .write_all(format!("{line}\r").as_bytes())
                .unwrap();
            sleep(Duration::from_secs(2));
            if self.text()[from..].contains(line) {
                break;
            }
        }
        let echo = from + self.text()[from..].find(line).expect("no echo");
        self.wait_for(prompt, echo + line.len(), 60)
    }

    fn stop(mut self) -> String {
        // boot and run-qemu.sh are QEMU's parents: this QEMU, not another.
        let _ = Command::new("pkill")
            .arg("-P")
            .arg(self.qemu.id().to_string())
            .status();
        let _ = self.qemu.kill();
        let _ = self.qemu.wait();
        self.text()
    }
}

/// A web server for one request, through guestfwd, whose host side
/// connects once: it answers with status and body and returns the request.
fn web_server(status: &'static str, body: Vec<u8>) -> (u16, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut c, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut buf = [0; 512];
        while !request.ends_with(b"\r\n\r\n") {
            let n = c.read(&mut buf).unwrap();
            assert!(n > 0, "the request ended early");
            request.extend_from_slice(&buf[..n]);
        }
        write!(
            c,
            "HTTP/1.0 {status}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n",
            body.len()
        )
        .unwrap();
        c.write_all(&body).unwrap();
        String::from_utf8_lossy(&request).into_owned()
    });
    (port, server)
}

/// A DNS server on UDP, which the guest reaches at 10.0.2.2: WWW.EXAMPLE.ORG
/// is at 192.0.2.7, and 192.0.2.7 is www.example.org; ECHO.EXAMPLE.ORG is
/// 10.0.2.100, the echo server's guestfwd; SILENT.EXAMPLE.ORG gets no
/// answer, and every other name NXDOMAIN. Returns its port.
fn dns_server() -> u16 {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    thread::spawn(move || {
        let mut q = [0; 512];
        while let Ok((n, from)) = socket.recv_from(&mut q) {
            // The question: its name's labels, then type and class.
            let (mut at, mut labels) = (12, Vec::new());
            while at < n && q[at] != 0 {
                let len = q[at] as usize;
                labels.push(String::from_utf8_lossy(&q[at + 1..at + 1 + len]).to_lowercase());
                at += 1 + len;
            }
            let qtype = u16::from_be_bytes([q[at + 1], q[at + 2]]);
            let question = &q[12..at + 5];
            let name = labels.join(".");
            let rdata: Option<(u16, Vec<u8>)> = match (name.as_str(), qtype) {
                ("silent.example.org", _) => continue,
                ("www.example.org", 1) => Some((1, vec![192, 0, 2, 7])),
                ("echo.example.org", 1) => Some((1, vec![10, 0, 2, 100])),
                ("7.2.0.192.in-addr.arpa", 12) => {
                    Some((12, b"\x03www\x07example\x03org\x00".to_vec()))
                }
                _ => None,
            };
            let mut r = q[..2].to_vec();
            // A recursive answer, NXDOMAIN without data; one question.
            let (rcode, answers) = if rdata.is_some() { (0, 1) } else { (3, 0) };
            r.extend((0x8180u16 | rcode).to_be_bytes());
            r.extend([0, 1, 0, answers, 0, 0, 0, 0]);
            r.extend(question);
            if let Some((t, d)) = rdata {
                r.extend([0xc0, 12, 0, t as u8, 0, 1, 0, 0, 1, 44, 0, d.len() as u8]);
                r.extend(d);
            }
            socket.send_to(&r, from).unwrap();
        }
    });
    port
}

/// A port nothing listens on, for now.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// A data disk with an empty volume, DATA, for COPY's files.
fn data_disk(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let _ = fs::remove_file(path);
    let params = InitParams {
        label: b"DATA".to_vec(),
        ..Default::default()
    };
    let mut vol = Image::create(path, 4096, &params).unwrap();
    vol.flush().unwrap();
}

/// Whether SHOW SYSTEM, in a remote login, lists itself in a process named
/// for its connection's unit, _BGnn:, whose number depends on the sockets
/// before it.
fn remote_login(text: &str) -> bool {
    text.lines()
        .any(|l| l.contains(" _BG") && l.contains(":          CUR     4 SHOW.EXE"))
}

#[test]
fn network() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let out = root.join("out");
    let boot = PathBuf::from(env!("CARGO_BIN_EXE_boot"));
    let run_qemu = root.join("scripts/run-qemu.sh");
    let path = |name: &str| out.join(name).display().to_string();

    // One vaxpunk and the host.
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_port = server.local_addr().unwrap().port();
    let host = thread::spawn(move || {
        let (mut c, _) = server.accept().unwrap();
        let mut buf = [0; 100];
        let n = c.read(&mut buf).unwrap();
        c.write_all(b"hello from the host\r\n").unwrap();
        String::from_utf8_lossy(&buf[..n]).into_owned()
    });
    // Greets, echoes a line back and closes: for TELNET address port.
    let echo = TcpListener::bind("127.0.0.1:0").unwrap();
    let echo_port = echo.local_addr().unwrap().port();
    let echoer = thread::spawn(move || {
        let (mut c, _) = echo.accept().unwrap();
        c.write_all(b"hello from port 7779\r\n").unwrap();
        let mut buf = [0; 100];
        let n = c.read(&mut buf).unwrap();
        c.write_all(b"echo: ").unwrap();
        c.write_all(&buf[..n]).unwrap();
        String::from_utf8_lossy(&buf[..n]).into_owned()
    });
    let (readme_port, readme) = web_server(
        "200 OK",
        b"line one\r\nline two\nno newline at the end".to_vec(),
    );
    let (index_port, index) = web_server("200 OK", b"<html></html>\n".to_vec());
    let (missing_port, missing) = web_server("404 Not Found", b"gone\n".to_vec());
    // Every byte value, LFs and CRs too, past one $WRITE's 32768.
    let blob: Vec<u8> = (0..40000u32).map(|i| (i * 7 % 256) as u8).collect();
    let (blob_port, blob_server) = web_server("200 OK", blob.clone());
    let fwd = free_port();
    let ufwd = free_port();
    let dns = dns_server();
    data_disk(&out.join("net-data.img"));
    let netdev = format!(
        "user,id=net0,guestfwd=tcp:10.0.2.100:7777-tcp:127.0.0.1:{server_port},\
         guestfwd=tcp:10.0.2.100:7779-tcp:127.0.0.1:{echo_port},\
         guestfwd=tcp:10.0.2.100:7780-tcp:127.0.0.1:{readme_port},\
         guestfwd=tcp:10.0.2.100:80-tcp:127.0.0.1:{index_port},\
         guestfwd=tcp:10.0.2.100:7781-tcp:127.0.0.1:{missing_port},\
         guestfwd=tcp:10.0.2.100:7782-tcp:127.0.0.1:{blob_port},\
         hostfwd=tcp:127.0.0.1:{fwd}-:7778,hostfwd=udp:127.0.0.1:{ufwd}-:7780"
    );
    let mut vax = Vax::boot(
        &boot,
        &[
            ("LOG", path("net.log")),
            ("DATADISK", path("net-data.img")),
            ("NETDEV", netdev),
        ],
    );
    // The resolver asks the DNS server here, not the startup's 8.8.8.8.
    vax.command("TCPIP SHOW NAME_SERVICE");
    vax.command("TCPIP SET NAME_SERVICE /SERVER=10.0.2.2");
    vax.command(&format!("DEFINE TCPIP$BIND_PORT {dns}"));
    vax.command("TCPIP SET INTERFACE WE0 /HOST=10.0.2.15 /NETWORK_MASK=255.255.255.0");
    vax.command("TCPIP SET ROUTE /DEFAULT /GATEWAY=10.0.2.2");
    vax.command("TCPIP SET ROUTE /GATEWAY=10.0.2.3");
    vax.command("TCPIP PING 10.0.2.2 /NUMBER_PACKETS=2");
    vax.command("TCPIP PING 10.0.2.99 /NUMBER_PACKETS=1");
    vax.reply("TCPIP", "TCPIP> ");
    vax.reply("SHOW INTERFACE", "TCPIP> ");
    vax.reply("HELP", "TCPIP> ");
    vax.reply("HELP SET", "TCPIP> ");
    vax.reply("SHOW INTERFACE XE0", "TCPIP> ");
    vax.command("EXIT");
    vax.command("TCPIP SHOW HOST LOCALHOST");
    vax.command(r#"TCPIP SET HOST GATEWAY /ADDRESS=10.0.2.2 /ALIAS=(GW,"qemu")"#);
    vax.command(r#"TCPIP SET HOST "web.example" /ADDRESS=10.0.2.100"#);
    vax.command(r#"TCPIP SET HOST "gone" /ADDRESS=10.0.2.50 /ALIAS="gone2""#);
    vax.command("TCPIP SHOW HOST");
    vax.command("TCPIP SHOW HOST QEMU");
    vax.command("TCPIP SHOW HOST NOPE");
    vax.reply("TCPIP SET NOHOST GONE", "Remove? [N]: ");
    vax.command("Y");
    vax.command("TCPIP SHOW HOST /ADDRESS=10.0.2.50");
    vax.command("TCPIP PING GW /NUMBER_PACKETS=1");
    vax.command("TCPIP SET NAME_SERVICE /NOSERVER");
    vax.command("TCPIP SET NAME_SERVICE /SERVER=GATEWAY /DOMAIN=example.org /PATH=(A.TEST,B.TEST)");
    vax.command("TCPIP SET NAME_SERVICE /NOPATH");
    vax.command("TCPIP SHOW NAME_SERVICE");
    vax.command(&format!("NSLOOKUP -PORT={dns} WWW"));
    vax.command(&format!("NSLOOKUP -PORT={dns} 192.0.2.7"));
    vax.command(&format!("NSLOOKUP -PORT={dns} NOPE.EXAMPLE.ORG"));
    vax.command(&format!(
        "NSLOOKUP -PORT={dns} -TIMEOUT=1 -RETRY=1 SILENT.EXAMPLE.ORG"
    ));
    vax.reply(&format!("NSLOOKUP -PORT={dns}"), "> ");
    vax.reply("www", "> ");
    vax.command("exit");
    vax.command("TCPIP PING WWW /NUMBER_PACKETS=1");
    let client = thread::spawn(move || {
        for _ in 0..100 {
            if let Ok(mut k) = TcpStream::connect(("127.0.0.1", fwd)) {
                k.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                k.write_all(b"ping from the host\r\n").unwrap();
                let mut buf = [0; 100];
                if let Ok(n @ 1..) = k.read(&mut buf) {
                    return String::from_utf8_lossy(&buf[..n]).into_owned();
                }
            }
            sleep(Duration::from_millis(500));
        }
        String::new()
    });
    // Until TCPTEST's socket is there, the datagram is lost: again and again.
    let datagrams = thread::spawn(move || {
        let u = UdpSocket::bind("127.0.0.1:0").unwrap();
        u.set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        let mut buf = [0; 100];
        for _ in 0..100 {
            u.send_to(b"datagram from the host\r\n", ("127.0.0.1", ufwd))
                .unwrap();
            if let Ok(n) = u.recv(&mut buf) {
                let first = String::from_utf8_lossy(&buf[..n]).into_owned();
                u.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                let n = u.recv(&mut buf).unwrap_or(0);
                return (first, String::from_utf8_lossy(&buf[..n]).into_owned());
            }
        }
        (String::new(), String::new())
    });
    vax.command("RUN TCPTEST");
    vax.command("SET HOST 10.0.2.15");
    vax.command("SHOW SYSTEM");
    vax.command("LOGOUT");
    vax.reply("TELNET ECHO 7779", "hello from port 7779");
    vax.reply("netcat", "echo: netcat");
    // The host closing doesn't reach the guest through guestfwd: CTRL/Z.
    let at = vax.text().len();
    vax.console.write_all(b"\x1a").unwrap();
    vax.wait_for("$ ", at, 60);
    vax.command("TELNET 10.0.2.100 /PORT=70000");
    vax.command(r#"COPY/HTTP/LOG URL::"http://10.0.2.100:7780/pub/Read.Me.txt" DKB0:[000000]"#);
    vax.command("TYPE DKB0:[000000]READ_ME.TXT");
    vax.command("DIRECTORY/FULL DKB0:[000000]READ_ME.TXT");
    vax.command(r#"COPY/HTTP/LOG 10.0.2.100::"/" DKB0:[000000]"#);
    vax.command(r#"COPY/HTTP URL::"http://10.0.2.100:7781/nope" DKB0:[000000]"#);
    vax.command(r#"COPY/HTTP/LOG URL::"http://web.example:7782/kit/blob.bin" DKB0:[000000]"#);
    vax.command(r#"COPY/HTTP URL::"https://10.0.2.100/x" DKB0:[000000]"#);
    let text = vax.stop();
    print!("{text}");
    assert_eq!(host.join().unwrap(), "hello from vaxpunk\r\n");
    assert_eq!(client.join().unwrap(), "ping from the host\r\n");
    let echo = "datagram from the host\r\n".to_string();
    assert_eq!(datagrams.join().unwrap(), (echo.clone(), echo));
    assert_eq!(echoer.join().unwrap(), "netcat\r\n");
    assert_eq!(
        readme.join().unwrap(),
        "GET /pub/Read.Me.txt HTTP/1.0\r\nHost: 10.0.2.100:7780\r\n\r\n"
    );
    assert_eq!(
        index.join().unwrap(),
        "GET / HTTP/1.0\r\nHost: 10.0.2.100\r\n\r\n"
    );
    assert_eq!(
        missing.join().unwrap(),
        "GET /nope HTTP/1.0\r\nHost: 10.0.2.100:7781\r\n\r\n"
    );
    assert!(
        blob_server
            .join()
            .unwrap()
            .starts_with("GET /kit/blob.bin HTTP/1.0\r\n")
    );
    for line in [
        "%TCPIP-I-SET, WE0: 10.0.2.15        255.255.255.0    10.0.2.2",
        " WE0       10.0.2.15        255.255.255.0    10.0.2.2         up",
        "  HELP command describes a command",
        "  SET INTERFACE interface\n    /DHCP\n    /HOST=value",
        "%SYSTEM-W-NOSUCHDEV",
        "illegal combination of command elements",
        "PING 10.0.2.2 (10.0.2.2): 56 data bytes",
        "64 bytes from 10.0.2.2: icmp_seq=1 ttl=",
        "----10.0.2.2 PING Statistics----\n2 packets transmitted, 2 packets received, 0% packet loss",
        "1 packets transmitted, 0 packets received, 100% packet loss",
        "\n     LOCAL database\n  \nHost address    Host name\n \n127.0.0.1       LOCALHOST, localhost\n",
        "Host address    Host name\n \n10.0.2.2        GATEWAY, GW, qemu\n\
         127.0.0.1       LOCALHOST, localhost\n\
         10.0.2.50       gone, gone2\n\
         10.0.2.100      web.example\n",
        "Host address    Host name\n \n10.0.2.2        GATEWAY, GW, qemu\n$ ",
        "%TCPIP-E-HOSTERROR, cannot process host request\n\
         -TCPIP-W-NORECORD, information not found\n\
         -RMS-E-RNF, record not found\n$ TCPIP SET NOHOST",
        "10.0.2.50       gone, gone2\nRemove? [N]: Y\n$ ",
        "ADDRESS=10.0.2.50\n%TCPIP-E-HOSTERROR, cannot process host request\n",
        "PING GW (10.0.2.2): 56 data bytes",
        "BIND Resolver Parameters\n\n Local domain: \n\n System\n\n  State:     Started, Enabled\n\n  \
         Transport: UDP\n  Domain:    \n  Retry:     2\n  Timeout:   5\n  \
         Servers:   8.8.8.8\n  Path:      \n\n Process\n\n  State:     Enabled\n\n  \
         Transport: \n  Domain:    \n  Retry:     \n  Timeout:   \n  Servers:   \n  Path:      \n$ ",
        " Local domain: EXAMPLE.ORG\n",
        "  Domain:    EXAMPLE.ORG\n  Retry:     \n  Timeout:   \n  Servers:   GATEWAY\n  Path:      \n$ ",
        &format!(
            "Server:\t\tGATEWAY\nAddress:\t10.0.2.2#{dns}\n\nNon-authoritative answer:\n\
             Name:\tWWW.EXAMPLE.ORG\nAddress: 192.0.2.7\n\n$ "
        ),
        "Non-authoritative answer:\n7.2.0.192.in-addr.arpa\tname = www.example.org.\n",
        "** server can't find NOPE.EXAMPLE.ORG: NXDOMAIN\n",
        ";; connection timed out; no servers could be reached\n",
        "> www\nServer:\t\tGATEWAY\n",
        "Name:\twww.EXAMPLE.ORG\nAddress: 192.0.2.7\n\n> exit\n$ ",
        "PING WWW (192.0.2.7): 56 data bytes",
        "TCPTEST: connected to 10.0.2.100 port 7777",
        "hello from the host",
        "TCPTEST: accepted a connection from address 0202000A",
        "ping from the host",
        "TCPTEST: UDP on port 7780",
        "TCPTEST: a datagram from address 0202000A",
        "datagram from the host",
        "TCPTEST: ok",
        "TCPIP$TELNET    LEF     4 TELNETD.EXE",
        "echo: netcat",
        "%SYSTEM-F-BADPARAM",
        "%REM-S-END, control returned to the local node",
        r#"%COPY-S-COPIED, URL::"http://10.0.2.100:7780/pub/Read.Me.txt" copied to "#,
        "READ_ME.TXT;1 (1 blocks)",
        "line one\nline two\nno newline at the end\n",
        "Record format:      Stream_LF",
        r#"%COPY-S-COPIED, 10.0.2.100::"/" copied to "#,
        "INDEX.HTML;1 (1 blocks)",
        "BLOB.BIN;1 (79 blocks)",
        "%RMS-E-FNF, file not found",
        "%RMS-F-SUPPORT, network operation not supported",
    ] {
        assert!(text.contains(line), "no {line:?}");
    }
    assert!(remote_login(&text), "no remote DCL's SHOW SYSTEM");
    let mut img = Image::open(out.join("net-data.img"), Mode::ReadOnly).unwrap();
    let fid = img.lookup("[000000]READ_ME.TXT").unwrap();
    let mut fetched = Vec::new();
    img.copy_out(fid, &mut fetched, Conversion::Binary).unwrap();
    // STREAM_LF, the bytes as they came, to the last.
    assert_eq!(
        String::from_utf8_lossy(&fetched),
        "line one\r\nline two\nno newline at the end"
    );
    let fid = img.lookup("[000000]BLOB.BIN").unwrap();
    let mut fetched = Vec::new();
    img.copy_out(fid, &mut fetched, Conversion::Binary).unwrap();
    assert!(fetched == blob, "BLOB.BIN isn't what the server sent");

    // Two vaxpunks.
    let port = free_port();
    // Blank: run-qemu.sh makes them, and SYSTARTUP_VMS.COM leaves them be.
    let _ = fs::remove_file(out.join("a-data.img"));
    let _ = fs::remove_file(out.join("b-data.img"));
    let mut a = Vax::boot(
        &run_qemu,
        &[
            ("LOG", path("a.log")),
            ("DATADISK", path("a-data.img")),
            ("NETDEV", format!("socket,id=net0,listen=127.0.0.1:{port}")),
            ("MAC", "52:54:00:00:00:0a".into()),
        ],
    );
    let mut b = Vax::boot(
        &run_qemu,
        &[
            ("LOG", path("b.log")),
            ("DATADISK", path("b-data.img")),
            ("NETDEV", format!("socket,id=net0,connect=127.0.0.1:{port}")),
            ("MAC", "52:54:00:00:00:0b".into()),
        ],
    );
    for (vax, host) in [(&mut a, "10.0.0.1"), (&mut b, "10.0.0.2")] {
        vax.command(&format!(
            "TCPIP SET CONFIGURATION INTERFACE WE0 /HOST={host} /NETWORK_MASK=255.255.255.0"
        ));
        vax.command("TCPIP SET ROUTE /DEFAULT /GATEWAY=10.0.0.1 /PERMANENT");
        vax.command("TCPIP START COMMUNICATION");
    }
    b.command("TELNET 10.0.0.1");
    b.command("SHOW SYSTEM");
    b.command("LOGOUT");
    b.command("TCPIP PING /NUMBER_PACKETS=2 10.0.0.1");
    b.command("TCPIP SET CONFIGURATION INTERFACE WE0 /HOST=10.0.0.3 /NETWORK_MASK=255.255.255.0");
    b.command("TCPIP SET ROUTE /DEFAULT /GATEWAY=10.0.0.9 /PERMANENT");
    b.command("TCPIP SET CONFIGURATION INTERFACE WE0 /DHCP");
    b.command("TCPIP SHOW INTERFACE");
    b.command("TYPE MDA0:[000000]TCPIP$CONFIG.DAT");
    let (a, b) = (a.stop(), b.stop());
    print!("{a}{b}");
    assert!(a.contains("%TCPIP-I-SET, WE0: 10.0.0.1         255.255.255.0    10.0.0.1"));
    for line in [
        "%TCPIP-I-SET, WE0: 10.0.0.2         255.255.255.0    10.0.0.1",
        "TCPIP$TELNET    LEF     4 TELNETD.EXE",
        "%REM-S-END, control returned to the local node",
        "64 bytes from 10.0.0.1: icmp_seq=1 ttl=255 time=",
        "2 packets transmitted, 2 packets received, 0% packet loss",
        " WE0       10.0.0.2         255.255.255.0    10.0.0.1         up",
        "TCPIP$CONFIG.DAT\nINTERFACE 10.0.0.3 255.255.255.0 10.0.0.9 DHCP\n",
    ] {
        assert!(b.contains(line), "no {line:?}");
    }
    assert!(remote_login(&b), "no remote DCL's SHOW SYSTEM");
}
