//! cargo test -p boot --test network: TCP/IP (docs/prd/0002-networking.md),
//! in two parts, one after the other.
//!
//! One vaxpunk on QEMU's user network, whose data disk, made here, says
//! DHCP, which START COMMUNICATION asks QEMU's DHCP server at boot for
//! the address, mask and gateway: then TCPIP's SET INTERFACE, SET ROUTE,
//! one without /DEFAULT that fails, PING to QEMU's gateway and to
//! an address nobody has, and at the TCPIP> prompt SHOW
//! INTERFACE, HELP, an interface there is not, and EXIT; then TCPTEST, which
//! connects to a server here, through QEMU's guestfwd, and accepts a
//! connection from a client here, through hostfwd, and SET HOST to
//! itself, SHOW SYSTEM there, and LOGOUT.
//!
//! Two vaxpunks on one QEMU socket network, A and B, each with a data disk
//! made here holding the configuration SET CONFIGURATION INTERFACE saves,
//! which START COMMUNICATION applies at boot: B logs in to A with SET
//! HOST, SHOW SYSTEM lists A's processes, and LOGOUT comes back to B,
//! which pings A.
//! Then B saves another address, keeping the saved gateway, another
//! gateway with SET ROUTE /PERMANENT, keeping that address, and DHCP,
//! keeping both, which SHOW INTERFACE doesn't show, since they are for
//! the next boot, and which are on its data disk once it is down.

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
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

/// A port nothing listens on, for now.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// A data disk with TCPIP$CONFIG.DAT holding command, as SET
/// CONFIGURATION INTERFACE saves it.
fn data_disk(path: &Path, command: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let _ = fs::remove_file(path);
    let params = InitParams {
        label: b"DATA".to_vec(),
        ..Default::default()
    };
    let mut vol = Image::create(path, 4096, &params).unwrap();
    let text = format!("{command}\n");
    vol.copy_in(
        &mut text.as_bytes(),
        "[000000]TCPIP$CONFIG.DAT",
        Conversion::LinesToRecords,
        Some(text.len() as u64),
        None,
    )
    .unwrap();
    vol.flush().unwrap();
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
    let fwd = free_port();
    data_disk(
        &out.join("net-data.img"),
        "INTERFACE 0.0.0.0 0.0.0.0 0.0.0.0 DHCP",
    );
    let netdev = format!(
        "user,id=net0,guestfwd=tcp:10.0.2.100:7777-tcp:127.0.0.1:{server_port},\
         hostfwd=tcp:127.0.0.1:{fwd}-:7778"
    );
    let mut vax = Vax::boot(
        &boot,
        &[
            ("LOG", path("net.log")),
            ("DATADISK", path("net-data.img")),
            ("NETDEV", netdev),
        ],
    );
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
    vax.command("RUN TCPTEST");
    vax.command("SET HOST 10.0.2.15");
    vax.command("SHOW SYSTEM");
    vax.command("LOGOUT");
    let text = vax.stop();
    print!("{text}");
    assert_eq!(host.join().unwrap(), "hello from vaxpunk\r\n");
    assert_eq!(client.join().unwrap(), "ping from the host\r\n");
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
        "TCPTEST: connected to 10.0.2.100 port 7777",
        "hello from the host",
        "TCPTEST: accepted a connection from address 0202000A",
        "ping from the host",
        "TCPTEST: ok",
        "TCPIP$TELNET    LEF     4 TELNETD.EXE",
        "_BG02:          CUR     4 SHOW.EXE",
        "%REM-S-END, control returned to the local node",
    ] {
        assert!(text.contains(line), "no {line:?}");
    }

    // Two vaxpunks.
    let port = free_port();
    data_disk(
        &out.join("a-data.img"),
        "INTERFACE 10.0.0.1 255.255.255.0 10.0.0.1",
    );
    data_disk(
        &out.join("b-data.img"),
        "INTERFACE 10.0.0.2 255.255.255.0 10.0.0.1",
    );
    let a = Vax::boot(
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
    b.command("SET HOST 10.0.0.1");
    b.command("SHOW SYSTEM");
    b.command("LOGOUT");
    b.command("TCPIP PING /NUMBER_PACKETS=2 10.0.0.1");
    b.command("TCPIP SET CONFIGURATION INTERFACE WE0 /HOST=10.0.0.3 /NETWORK_MASK=255.255.255.0");
    b.command("TCPIP SET ROUTE /DEFAULT /GATEWAY=10.0.0.9 /PERMANENT");
    b.command("TCPIP SET CONFIGURATION INTERFACE WE0 /DHCP");
    b.command("TCPIP SHOW INTERFACE");
    let (a, b) = (a.stop(), b.stop());
    print!("{a}{b}");
    assert!(a.contains("%TCPIP-I-SET, WE0: 10.0.0.1         255.255.255.0    10.0.0.1"));
    for line in [
        "%TCPIP-I-SET, WE0: 10.0.0.2         255.255.255.0    10.0.0.1",
        "TCPIP$TELNET    LEF     4 TELNETD.EXE",
        "_BG02:          CUR     4 SHOW.EXE",
        "%REM-S-END, control returned to the local node",
        "64 bytes from 10.0.0.1: icmp_seq=1 ttl=255 time=",
        "2 packets transmitted, 2 packets received, 0% packet loss",
        " WE0       10.0.0.2         255.255.255.0    10.0.0.1         up",
    ] {
        assert!(b.contains(line), "no {line:?}");
    }
    let mut img = Image::open(out.join("b-data.img"), Mode::ReadOnly).unwrap();
    let fid = img.lookup("[000000]TCPIP$CONFIG.DAT").unwrap();
    let mut saved = Vec::new();
    img.copy_out(fid, &mut saved, Conversion::RecordsToLines)
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&saved),
        "INTERFACE 10.0.0.3 255.255.255.0 10.0.0.9 DHCP\n"
    );
}
