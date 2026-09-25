//! ods: inspect and change Files-11 disk images. A thin layer over
//! ods-image: it parses arguments and prints results, nothing else.

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use ods_image::{Conversion, Error, Fid, Found, Header, Image, InitParams, Level, Mode, Severity, attrs, layout, time};
use serde_json::{Value, json};

#[derive(Parser)]
#[command(name = "ods", about = "Inspect and change Files-11 (ODS-2/ODS-5) disk images", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum CopyMode {
    /// Bytes as they are, up to the end of file.
    Binary,
    /// Copy out: one line per record.
    #[value(name = "records-to-lines", alias = "text")]
    RecordsToLines,
    /// Copy in: one VAR record per line.
    #[value(name = "lines-to-records")]
    LinesToRecords,
}

impl From<CopyMode> for Conversion {
    fn from(m: CopyMode) -> Conversion {
        match m {
            CopyMode::Binary => Conversion::Binary,
            CopyMode::RecordsToLines => Conversion::RecordsToLines,
            CopyMode::LinesToRecords => Conversion::LinesToRecords,
        }
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Create an image file and initialize a volume on it.
    Init {
        image: String,
        /// Size: blocks, or bytes with a K, M, G or T suffix.
        #[arg(long)]
        size: String,
        #[arg(long)]
        label: String,
        #[arg(long, conflicts_with = "ods5")]
        ods2: bool,
        #[arg(long)]
        ods5: bool,
        /// Blocks per cluster (default: by size, as VMS chooses).
        #[arg(long)]
        cluster: Option<u16>,
        #[arg(long)]
        max_files: Option<u32>,
        /// Volume owner UIC, like [1,4].
        #[arg(long)]
        owner: Option<String>,
    },
    /// Home block summary: label, structure level, cluster size, free space.
    Info {
        image: String,
        #[arg(long)]
        json: bool,
    },
    /// List a directory. Wildcards: * % ? and [...] for all levels below.
    Dir {
        image: String,
        spec: Option<String>,
        /// File ID, size, date, owner, protection and record format.
        #[arg(long)]
        full: bool,
        /// All versions, not just the highest.
        #[arg(long)]
        versions: bool,
        #[arg(long)]
        json: bool,
    },
    /// Print a text file's records as lines.
    Type { image: String, spec: String },
    /// Copy a host file (or - for stdin) in, as a new version.
    CopyIn {
        image: String,
        host: String,
        spec: String,
        /// Default: binary.
        #[arg(long, value_enum)]
        mode: Option<CopyMode>,
        /// Record format for the new file (default: UDF for binary, VAR for lines).
        #[arg(long)]
        rfm: Option<String>,
        #[arg(long)]
        rat: Option<String>,
        #[arg(long)]
        mrs: Option<u16>,
    },
    /// Copy a file out to the host (or - for stdout).
    CopyOut {
        image: String,
        spec: String,
        host: String,
        /// Default: records-to-lines for text record formats, else binary.
        #[arg(long, value_enum)]
        mode: Option<CopyMode>,
    },
    /// Delete files. A version is required, as on VMS: ;n ;0 ;-n or ;*.
    Delete { image: String, spec: String },
    /// Delete all but the highest versions.
    Purge {
        image: String,
        spec: String,
        #[arg(long, default_value_t = 1)]
        keep: usize,
    },
    /// Rename or move a file within the volume.
    Rename { image: String, from: String, to: String },
    /// Create a directory, like [A.B] or /a/b.
    Mkdir { image: String, spec: String },
    /// Change a file's protection, owner or record attributes.
    SetAttr {
        image: String,
        spec: String,
        /// Like S:RWED,O:RWED,G:RE,W:
        #[arg(long)]
        protection: Option<String>,
        /// Like [1,4].
        #[arg(long)]
        owner: Option<String>,
        /// UDF FIX VAR VFC STM STMLF STMCR.
        #[arg(long)]
        rfm: Option<String>,
        /// Like CR or FTN,BLK or NONE.
        #[arg(long)]
        rat: Option<String>,
        /// Maximum (or fixed) record size.
        #[arg(long)]
        mrs: Option<u16>,
        /// Longest record.
        #[arg(long)]
        lrl: Option<u16>,
        /// VFC control area size.
        #[arg(long)]
        vfc: Option<u8>,
        /// A directory's default version limit (0: none).
        #[arg(long)]
        version_limit: Option<u16>,
    },
    /// Decode a file header (by name or --fid), or dump a logical block.
    Dump {
        image: String,
        spec: Option<String>,
        /// File ID as NUM,SEQ (SEQ 0 matches any).
        #[arg(long)]
        fid: Option<String>,
        #[arg(long)]
        lbn: Option<u64>,
        #[arg(long)]
        json: bool,
    },
    /// Check the volume's structure; optionally rebuild the bitmaps.
    Verify {
        image: String,
        #[arg(long)]
        repair_bitmap: bool,
        #[arg(long)]
        json: bool,
    },
    /// Copy a directory tree out, with a manifest of VMS attributes.
    Export { image: String, dirspec: String, hostdir: String },
    /// Copy a host tree in, restoring attributes from its manifest.
    Import { image: String, hostdir: String, dirspec: String },
}

fn main() -> ExitCode {
    // Die quietly when a pipe reader goes away, like other Unix tools.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    match run(Cli::parse().cmd) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn rw(image: &str) -> Result<Image, Error> {
    Image::open(image, Mode::ReadWrite)
}

fn ro(image: &str) -> Result<Image, Error> {
    Image::open(image, Mode::ReadOnly)
}

fn pretty(v: &Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

/// Runs a command. `Ok(false)`: it ran, and found problems (verify).
fn run(cmd: Cmd) -> Result<bool, Error> {
    match cmd {
        Cmd::Init { image, size, label, ods2: _, ods5, cluster, max_files, owner } => {
            let blocks = parse_size(&size)?;
            let mut p = InitParams { label: label.to_ascii_uppercase().into_bytes(), ..InitParams::default() };
            p.level = if ods5 { Level::Ods5 } else { Level::Ods2 };
            p.cluster = cluster.unwrap_or(0);
            p.max_files = max_files.unwrap_or(0);
            if let Some(o) = owner {
                p.owner = attrs::parse_uic(&o).ok_or_else(|| Error::usage(format!("bad UIC {o:?}")))?;
            }
            let mut img = Image::create(&image, blocks, &p)?;
            img.flush()?;
            let i = img.info()?;
            println!(
                "{image}: ODS-{} volume {}, {} blocks, cluster {}",
                i.level.number(),
                i.label,
                i.volume_blocks,
                i.cluster
            );
        }
        Cmd::Info { image, json } => info(&image, json)?,
        Cmd::Dir { image, spec, full, versions, json } => dir(&image, spec.as_deref(), full, versions, json)?,
        Cmd::Type { image, spec } => {
            let mut img = ro(&image)?;
            let fid = img.lookup(&spec)?;
            let mut out = BufWriter::new(io::stdout().lock());
            img.copy_out(fid, &mut out, Conversion::RecordsToLines)?;
            out.flush()?;
        }
        Cmd::CopyIn { image, host, spec, mode, rfm, rat, mrs } => {
            let mut img = rw(&image)?;
            let conv = mode.map_or(Conversion::Binary, Conversion::from);
            let mut record = None;
            if rfm.is_some() || rat.is_some() || mrs.is_some() {
                let mut r = ods_image::RecordAttrs::default();
                if conv == Conversion::LinesToRecords {
                    (r.rtype, r.rattrib) = (ods_image::rfm::VAR, ods_image::rat::CR);
                }
                apply_record(&mut r, rfm.as_deref(), rat.as_deref(), mrs, None, None)?;
                record = Some(r);
            }
            let (_, name) = if host == "-" {
                img.copy_in(&mut io::stdin().lock(), &spec, conv, None, record)?
            } else {
                let f = File::open(&host).map_err(|e| Error::from(e).at(&host))?;
                let size = f.metadata().ok().map(|m| m.len());
                img.copy_in(&mut BufReader::new(f), &spec, conv, size, record)?
            };
            img.flush()?;
            eprintln!("{host} -> {name} ({})", mode_name(conv));
        }
        Cmd::CopyOut { image, spec, host, mode } => {
            let mut img = ro(&image)?;
            let fid = img.lookup(&spec)?;
            let conv = match mode {
                Some(m) => m.into(),
                None => Conversion::default_out(&img.stat(fid)?.attrs.record),
            };
            let n = if host == "-" {
                let mut out = BufWriter::new(io::stdout().lock());
                let n = img.copy_out(fid, &mut out, conv)?;
                out.flush()?;
                n
            } else {
                let f = File::create(&host).map_err(|e| Error::from(e).at(&host))?;
                let mut out = BufWriter::new(f);
                let n = img.copy_out(fid, &mut out, conv)?;
                out.flush().map_err(|e| Error::from(e).at(&host))?;
                n
            };
            eprintln!("{spec} -> {host} ({}, {n} bytes)", mode_name(conv));
        }
        Cmd::Delete { image, spec } => {
            let mut img = rw(&image)?;
            for f in img.delete(&spec)? {
                println!("deleted {f}");
            }
            img.flush()?;
        }
        Cmd::Purge { image, spec, keep } => {
            let mut img = rw(&image)?;
            for f in img.purge(&spec, keep)? {
                println!("deleted {f}");
            }
            img.flush()?;
        }
        Cmd::Rename { image, from, to } => {
            let mut img = rw(&image)?;
            let name = img.rename(&from, &to)?;
            img.flush()?;
            println!("{from} -> {name}");
        }
        Cmd::Mkdir { image, spec } => {
            let mut img = rw(&image)?;
            let fid = img.mkdir(&spec)?;
            img.flush()?;
            println!("created {spec} {fid}");
        }
        Cmd::SetAttr { image, spec, protection, owner, rfm, rat, mrs, lrl, vfc, version_limit } => {
            let mut img = rw(&image)?;
            let fid = img.lookup(&spec)?;
            let mut a = img.attributes(fid)?;
            if let Some(p) = protection {
                a.protection =
                    attrs::parse_protection(&p).ok_or_else(|| Error::usage(format!("bad protection {p:?}")))?;
            }
            if let Some(o) = owner {
                a.owner = attrs::parse_uic(&o).ok_or_else(|| Error::usage(format!("bad UIC {o:?}")))?;
            }
            apply_record(&mut a.record, rfm.as_deref(), rat.as_deref(), mrs, lrl, vfc)?;
            if let Some(v) = version_limit {
                a.record.versions = v;
            }
            img.set_attributes(fid, &a).map_err(|e| e.at(&spec))?;
            img.flush()?;
        }
        Cmd::Dump { image, spec, fid, lbn, json } => dump(&image, spec.as_deref(), fid.as_deref(), lbn, json)?,
        Cmd::Verify { image, repair_bitmap, json } => return verify(&image, repair_bitmap, json),
        Cmd::Export { image, dirspec, hostdir } => {
            let mut img = ro(&image)?;
            let m = img.export(&dirspec, Path::new(&hostdir))?;
            let files = m.entries.iter().filter(|e| !e.directory).count();
            println!("exported {files} files from {} to {hostdir}", m.root);
        }
        Cmd::Import { image, hostdir, dirspec } => {
            let mut img = rw(&image)?;
            let files = img.import(Path::new(&hostdir), &dirspec)?;
            img.flush()?;
            println!("imported {} files into {dirspec}", files.len());
        }
    }
    Ok(true)
}

fn mode_name(c: Conversion) -> &'static str {
    match c {
        Conversion::Binary => "binary",
        Conversion::RecordsToLines => "records-to-lines",
        Conversion::LinesToRecords => "lines-to-records",
    }
}

fn apply_record(
    r: &mut ods_image::RecordAttrs,
    rfm: Option<&str>,
    rat: Option<&str>,
    mrs: Option<u16>,
    lrl: Option<u16>,
    vfc: Option<u8>,
) -> Result<(), Error> {
    if let Some(f) = rfm {
        let v = attrs::parse_rfm(f).ok_or_else(|| Error::usage(format!("bad record format {f:?}")))?;
        r.rtype = r.rtype & 0xf0 | v;
    }
    if let Some(a) = rat {
        r.rattrib = attrs::parse_rat(a).ok_or_else(|| Error::usage(format!("bad record attributes {a:?}")))?;
    }
    if let Some(m) = mrs {
        r.rsize = m;
    }
    if let Some(l) = lrl {
        r.maxrec = l;
    }
    if let Some(v) = vfc {
        r.vfcsize = v;
    }
    Ok(())
}

/// Blocks, or bytes with a K/M/G/T suffix.
fn parse_size(s: &str) -> Result<u64, Error> {
    let (num, mult) = match s.char_indices().last() {
        Some((i, c)) if "KkMmGgTt".contains(c) => {
            (&s[..i], 1u64 << (10 * (" KMGT".find(c.to_ascii_uppercase()).unwrap_or(0))))
        }
        _ => (s, 0),
    };
    let n: u64 = num.parse().map_err(|_| Error::usage(format!("bad size {s:?}")))?;
    Ok(if mult == 0 { n } else { n * mult / 512 })
}

fn info(image: &str, json: bool) -> Result<(), Error> {
    let mut img = ro(image)?;
    let i = img.info()?;
    let container = match &i.container {
        ods_image::Container::Raw => "raw".to_string(),
        ods_image::Container::Simh { drive } => format!("simh {drive}"),
    };
    let level = format!("ODS-{}", i.struclev >> 8);
    if json {
        pretty(&json!({
            "label": i.label, "owner": i.owner_name, "format": i.format, "structure_level": level,
            "struclev": i.struclev, "cluster": i.cluster, "volume_blocks": i.volume_blocks,
            "free_blocks": i.free_blocks, "max_files": i.max_files, "files": i.files,
            "reserved_files": i.reserved_files, "owner_uic": attrs::uic(i.owner_uic),
            "protection": attrs::protection(i.protection), "file_protection": attrs::protection(i.file_protection),
            "created": time::format(i.created), "container": container,
        }));
        return Ok(());
    }
    println!("Volume label:     {}", i.label);
    println!("Structure level:  {level} (version {})", i.struclev & 0xff);
    println!("Format:           {}", i.format);
    println!("Owner:            {} {}", i.owner_name, attrs::uic(i.owner_uic));
    println!("Cluster size:     {} blocks", i.cluster);
    println!("Volume size:      {} blocks", i.volume_blocks);
    let pct = i.free_blocks as f64 * 100.0 / i.volume_blocks.max(1) as f64;
    println!("Free space:       {} blocks ({pct:.1}%)", i.free_blocks);
    println!("Files:            {} of {} ({} reserved)", i.files, i.max_files, i.reserved_files);
    println!("Protection:       {}", attrs::protection(i.protection));
    println!("File protection:  {}", attrs::protection(i.file_protection));
    println!("Created:          {}", time::format(i.created));
    println!("Container:        {container}");
    Ok(())
}

fn dir(image: &str, spec: Option<&str>, full: bool, versions: bool, json: bool) -> Result<(), Error> {
    let mut img = ro(image)?;
    let spec = spec.unwrap_or("[000000]");
    let vms = ods_image::path::to_vms(spec);
    let mut search = vms.clone();
    if vms.ends_with(']') || vms.ends_with('>') {
        search.push_str("*.*");
    }
    if versions && !search.contains(';') {
        search.push_str(";*");
    }
    let found = img.search(&search)?;
    if found.is_empty() {
        return Err(Error::from(ods_image::OdsError::NotFound).at(spec));
    }
    let mut rows: Vec<(&Found, Option<Result<ods_image::FileInfo, Error>>)> = Vec::new();
    for f in &found {
        let info = (full || json).then(|| img.stat(f.entry.fid));
        rows.push((f, info));
    }
    if json {
        let v: Vec<Value> = rows
            .iter()
            .map(|(f, info)| {
                let mut o = json!({
                    "dir": img.dir_spec(&f.dir),
                    "name": img.display(&f.entry.name, f.entry.name_type),
                    "version": f.entry.version,
                    "fid": f.entry.fid.to_string(),
                });
                match info {
                    Some(Ok(i)) => o["file"] = file_json(i),
                    Some(Err(e)) => o["error"] = json!(e.to_string()),
                    None => {}
                }
                o
            })
            .collect();
        pretty(&Value::Array(v));
        return Ok(());
    }
    let mut last_dir: Option<&Vec<Vec<u8>>> = None;
    let (mut used, mut alloc) = (0u64, 0u64);
    for (f, info) in &rows {
        if last_dir != Some(&f.dir) {
            if last_dir.is_some() {
                println!();
            }
            println!("Directory {}\n", img.dir_spec(&f.dir));
            last_dir = Some(&f.dir);
        }
        let name = format!("{};{}", img.display(&f.entry.name, f.entry.name_type), f.entry.version);
        match info {
            None => println!("{name}"),
            Some(Err(e)) => println!("{name:<24} {e}"),
            Some(Ok(i)) => {
                let blocks = i.attrs.record.eof_bytes().div_ceil(512);
                used += blocks;
                alloc += i.allocated;
                println!(
                    "{name:<24} {:<12} {:>7}/{:<7} {} {:<9} {:<24} {}/{}",
                    i.fid.to_string(),
                    blocks,
                    i.allocated,
                    time::format(i.attrs.created),
                    attrs::uic(i.attrs.owner),
                    attrs::protection(i.attrs.protection),
                    attrs::rfm_name(i.attrs.record.rtype),
                    attrs::rat_names(i.attrs.record.rattrib),
                );
            }
        }
    }
    if full {
        println!("\nTotal of {} files, {used}/{alloc} blocks.", rows.len());
    } else {
        println!("\nTotal of {} files.", rows.len());
    }
    Ok(())
}

fn file_json(i: &ods_image::FileInfo) -> Value {
    let a = &i.attrs;
    let r = &a.record;
    json!({
        "fid": i.fid.to_string(),
        "header_name": String::from_utf8_lossy(&i.name),
        "revision": a.revision,
        "created": time::format(a.created),
        "revised": time::format(a.revised),
        "expires": time::format(a.expires),
        "backup": time::format(a.backup),
        "owner": attrs::uic(a.owner),
        "protection": attrs::protection(a.protection),
        "characteristics": attrs::fch_names(a.filechar),
        "backlink": i.backlink.to_string(),
        "allocated": i.allocated,
        "headers": i.headers,
        "extents": i.extents,
        "eof_bytes": r.eof_bytes(),
        "rfm": attrs::rfm_name(r.rtype),
        "org": attrs::org_name(r.rtype),
        "rat": attrs::rat_names(r.rattrib),
        "mrs": r.rsize,
        "lrl": r.maxrec,
        "hiblk": r.hiblk,
        "efblk": r.efblk,
        "ffbyte": r.ffbyte,
        "vfcsize": r.vfcsize,
        "version_limit": r.versions,
    })
}

fn parse_fid(s: &str) -> Result<Fid, Error> {
    let s = s.trim_matches(|c| c == '(' || c == ')');
    let p: Vec<&str> = s.split(',').collect();
    let num = |i: usize| p.get(i).map_or(Ok(0), |v| v.trim().parse::<u32>());
    match (num(0), num(1), num(2)) {
        (Ok(n), Ok(s), Ok(r)) if n > 0 && s <= 0xffff && r <= 0xff && p.len() <= 3 => {
            Ok(Fid { num: n, seq: s as u16, rvn: r as u8 })
        }
        _ => Err(Error::usage(format!("bad file ID {s:?}, expected NUM,SEQ"))),
    }
}

fn dump(image: &str, spec: Option<&str>, fid: Option<&str>, lbn: Option<u64>, json: bool) -> Result<(), Error> {
    if let Some(lbn) = lbn {
        let b = ods_image::raw_block(image, lbn)?;
        if json {
            pretty(&json!({"lbn": lbn, "hex": hex(&b)}));
        } else {
            println!("Logical block {lbn}\n");
            print!("{}", hexdump(&b));
        }
        return Ok(());
    }
    let mut img = ro(image)?;
    let fid = match (spec, fid) {
        (_, Some(f)) => parse_fid(f)?,
        (Some(s), None) => img.lookup(s)?,
        (None, None) => return Err(Error::usage("dump needs a file, --fid or --lbn")),
    };
    let hs = img.headers(fid)?;
    let mut out: Vec<Value> = hs.iter().map(|(lbn, h)| header_json(*lbn, h)).collect();
    if hs[0].1.filechar() & ods_image::fch::DIRECTORY != 0 {
        let recs: Vec<Value> = img
            .list(fid)?
            .iter()
            .map(|e| json!({"name": img.display(&e.name, e.name_type), "version": e.version, "fid": e.fid.to_string(), "verlimit": e.verlimit}))
            .collect();
        out[0]["directory"] = Value::Array(recs);
    }
    if json {
        pretty(&Value::Array(out));
        return Ok(());
    }
    for ((lbn, h), v) in hs.iter().zip(&out) {
        println!("File header {} at LBN {lbn}", h.fid());
        print_tree(v, 1);
        println!();
        print!("{}", hexdump(&h.0));
        println!();
    }
    Ok(())
}

fn header_json(lbn: u64, h: &Header) -> Value {
    let ra = h.record_attrs();
    let mut v = json!({
        "lbn": lbn,
        "offsets": {"ident": h.idoffset(), "map": h.mpoffset(), "acl": h.acoffset(), "reserved": h.rsoffset()},
        "segment": h.seg_num(),
        "struclev": format!("{}.{}", h.struclev() >> 8, h.struclev() & 0xff),
        "fid": h.fid().to_string(),
        "ext_fid": h.ext_fid().to_string(),
        "filechar": format!("{:#010x} {}", h.filechar(), attrs::fch_names(h.filechar())),
        "recprot": format!("{:#06x}", h.recprot()),
        "map_inuse": h.map_inuse(),
        "acc_mode": h.acc_mode(),
        "owner": attrs::uic(h.fileowner()),
        "protection": format!("{:#06x} {}", h.fileprot(), attrs::protection(h.fileprot())),
        "backlink": h.backlink().to_string(),
        "journal": h.journal(),
        "linkcount": h.linkcount(),
        "highwater": h.highwater_mark(),
        "checksum": format!("{:#06x} {}", h.checksum(), if h.invalid().is_none() { "ok" } else { "BAD" }),
        "record_attributes": {
            "rtype": format!("{:#04x} {} {}", ra.rtype, attrs::rfm_name(ra.rtype), attrs::org_name(ra.rtype)),
            "rattrib": format!("{:#04x} {}", ra.rattrib, attrs::rat_names(ra.rattrib)),
            "rsize": ra.rsize, "hiblk": ra.hiblk, "efblk": ra.efblk, "ffbyte": ra.ffbyte,
            "bktsize": ra.bktsize, "vfcsize": ra.vfcsize, "maxrec": ra.maxrec, "defext": ra.defext,
            "gbc": ra.gbc, "reserved": hex(&ra.reserved), "versions": ra.versions,
        },
    });
    if let Some(id) = h.ident() {
        v["ident"] = json!({
            "name": String::from_utf8_lossy(&id.name),
            "name_type": format!("{:?}", id.name_type),
            "revision": id.revision,
            "created": time::format(id.credate),
            "revised": time::format(id.revdate),
            "expires": time::format(id.expdate),
            "backup": time::format(id.bakdate),
            "accessed": time::format(id.accdate),
            "attributes_changed": time::format(id.attdate),
        });
    }
    let map: Vec<Value> = match layout::decode_map(h.map_area()) {
        Ok(ps) => ps
            .iter()
            .map(|p| match *p {
                layout::Pointer::Placement(w) => json!({"placement": format!("{w:#06x}")}),
                layout::Pointer::Extent { format, count, lbn } => json!({"format": format, "count": count, "lbn": lbn}),
            })
            .collect(),
        Err(e) => vec![json!({"error": e})],
    };
    v["map"] = Value::Array(map);
    if !h.acl_area().is_empty() {
        v["acl"] = json!(hex(h.acl_area()));
    }
    v
}

fn verify(image: &str, repair: bool, json: bool) -> Result<bool, Error> {
    let mut img = if repair { rw(image)? } else { ro(image)? };
    let r = if repair { img.repair_bitmap()? } else { img.verify()? };
    if repair {
        img.flush()?;
    }
    let sev = |s: Severity| match s {
        Severity::Error => "error",
        Severity::Leak => "leak",
        Severity::Warning => "warning",
    };
    if json {
        let f: Vec<Value> = r
            .findings
            .iter()
            .map(|f| json!({"severity": sev(f.severity), "what": f.what, "fid": f.fid.map(|x| x.to_string()), "lbn": f.lbn}))
            .collect();
        pretty(&json!({
            "files": r.files, "directories": r.directories, "used_blocks": r.used_blocks,
            "errors": r.count(Severity::Error), "leaks": r.count(Severity::Leak), "warnings": r.count(Severity::Warning),
            "repaired_clusters": r.repaired, "findings": f,
        }));
    } else {
        for f in &r.findings {
            let fid = f.fid.map(|x| format!(" file {x}")).unwrap_or_default();
            let lbn = f.lbn.map(|x| format!(" LBN {x}")).unwrap_or_default();
            println!("{}:{fid}{lbn}: {}", sev(f.severity), f.what);
        }
        println!(
            "{} files, {} directories, {} blocks used: {} errors, {} leaks, {} warnings",
            r.files,
            r.directories,
            r.used_blocks,
            r.count(Severity::Error),
            r.count(Severity::Leak),
            r.count(Severity::Warning)
        );
        if repair {
            println!("bitmaps rewritten, {} clusters changed", r.repaired);
        }
    }
    Ok(r.is_sound())
}

fn print_tree(v: &Value, depth: usize) {
    let pad = "  ".repeat(depth);
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                if x.is_object() || x.is_array() {
                    println!("{pad}{k}:");
                    print_tree(x, depth + 1);
                } else {
                    println!("{pad}{k:<20} {}", scalar(x));
                }
            }
        }
        Value::Array(a) => {
            for x in a {
                match x {
                    Value::Object(m) => {
                        let s: Vec<String> = m.iter().map(|(k, x)| format!("{k}={}", scalar(x))).collect();
                        println!("{pad}{}", s.join(" "));
                    }
                    _ => println!("{pad}{}", scalar(x)),
                }
            }
        }
        _ => println!("{pad}{}", scalar(v)),
    }
}

fn scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "-".into(),
        _ => v.to_string(),
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hexdump(b: &[u8]) -> String {
    let mut s = String::new();
    for (i, row) in b.chunks(16).enumerate() {
        let h: Vec<String> = row.iter().map(|x| format!("{x:02x}")).collect();
        let a: String = row.iter().map(|&c| if (0x20..0x7f).contains(&c) { c as char } else { '.' }).collect();
        s += &format!("  {:04x}  {:<47}  {a}\n", i * 16, h.join(" "));
    }
    s
}
