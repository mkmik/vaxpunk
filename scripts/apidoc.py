#!/usr/bin/env python3
"""Write docs/api/index.html, the API reference, from the sources.

Nothing in the page is written by hand: it is all read from the code and the
design documents, so keeping it current means keeping these current:
- the executive's routines and data (roottask/exec/*.mar, consolio.mar): the
  comment block right above each `NAME::` or `.ENTRY`, whose first line reads
  `NAME: what it does` or, for a system service, `$NAME args: what it does`;
- the system service vector, the SERVICE and STUB lines of syssrv.mar;
- the macro libraries (vtools/lib/*.mlb, roottask/sysexe/*.mlb): the comment
  block right above each `.MACRO`, and the `SYM = value ; meaning` lines of
  the $xxxDEF macros;
- the PAL calls: the tables of DESIGN-0001 (docs/design/0001-pal-interface.md).

Run it after changing any of them, and commit the page with the change:

    scripts/apidoc.py
"""
import html
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs/api/index.html"
EXEC = sorted((ROOT / "roottask/exec").glob("*.mar")) + [ROOT / "vtools/lib/consolio.mar"]
LIBS = sorted((ROOT / "vtools/lib").glob("*.mlb")) + sorted((ROOT / "roottask/sysexe").glob("*.mlb"))
PAL_DOC = ROOT / "docs/design/0001-pal-interface.md"

LABEL = re.compile(r"^([A-Za-z][\w$]*)(::?)(.*)")
ENTRY = re.compile(r"^\s+\.ENTRY\s+([\w$]+)", re.I)
DIRECTIVE = re.compile(r"\.(LONG|WORD|BYTE|QUAD|BLK[BWLQ]|ASCI[CDZI]|ADDRESS)\b[^;]*", re.I)
CALL = re.compile(r"^(BSBB|BSBW|JSB|JMP|BRB|BRW|CALLS|CALLG)\s+(.*)", re.I)
SYMBOL = re.compile(r"\$?[A-Za-z][\w$]*")


def rel(path):
    return path.relative_to(ROOT).as_posix()


def code(line):
    # ponytail: a ';' inside a string literal would cut the line; the
    # sources have none
    return line.split(";")[0]


def comment(line):
    text = line.strip()[1:]
    return text[1:] if text.startswith(" ") else text


def blocks(lines):
    """Comment lines as paragraphs ("p"), indented examples ("pre") and
    ponytail notes ("note")."""
    out, kind, buf = [], None, []

    def flush():
        if buf:
            out.append((kind, "\n".join(buf) if kind == "pre" else " ".join(buf)))
            buf.clear()

    for line in lines:
        if not line.strip():
            flush()
            kind = None
        elif line.startswith("ponytail:"):
            flush()
            kind = "note"
            buf.append(line[len("ponytail:"):].strip())
        elif line.startswith(("  ", "\t")) and kind != "note":
            if kind != "pre":
                flush()
                kind = "pre"
            buf.append(line)
        else:
            if kind not in ("p", "note"):
                flush()
                kind = "p"
            buf.append(line.strip())
    flush()
    return out


def head(doc):
    """Splits `NAME args: text` off the first paragraph: (NAME, args, rest)."""
    if doc and doc[0][0] == "p":
        m = re.match(r"(\$?[\w$]+)([^:]*):\s+(.*)", doc[0][1], re.S)
        if m:
            return m[1], m[2].strip(), [("p", m[3][:1].upper() + m[3][1:])] + doc[1:]
    return None, "", doc


def facts(doc):
    """Moves "Uses R0.", "Keeps every register." and "IPL$_SYNCH." out of
    the description: (description, registers, IPL)."""
    regs, ipl, out = [], [], []
    for kind, text in doc:
        if kind == "p":
            keep = []
            for s in re.split(r"(?<=\.)\s+", text):
                if re.match(r"(Uses|Keeps) .*\.$", s):
                    regs.append(s)
                elif re.fullmatch(r"IPL\$_\w+\.", s):
                    ipl.append(s[:-1])
                else:
                    keep.append(s)
            text = " ".join(keep)
            if not text:
                continue
        out.append((kind, text))
    return out, " ".join(regs), ", ".join(ipl)


# ---------------------------------------------------------------- sources

def parse_mar(path):
    mod = {"path": rel(path), "stem": path.stem, "title": "", "intro": [], "routines": [], "data": []}
    lines = path.read_text().splitlines()
    block, cur, in_code, in_macro = [], None, True, False
    for no, line in enumerate(lines, 1):
        s = line.strip()
        if s.startswith(";"):
            block.append(comment(line))
            continue
        if not mod["intro"] and block and not mod["routines"] and not mod["data"]:
            mod["intro"] = blocks(block)
        if not s:
            block = []
            continue
        up = s.upper()
        if up.startswith(".MACRO"):
            in_macro = True
        if in_macro:
            in_macro = not up.startswith(".ENDM")
            block = []
            continue
        if up.startswith(".TITLE"):
            mod["title"] = s.split(None, 2)[2] if len(s.split()) > 2 else ""
        elif up.startswith(".PSECT"):
            in_code = "NOEXE" not in up
        m, entry = ENTRY.match(line), True
        if not m:
            m, entry = LABEL.match(line), False
        if m and not in_code:
            if m[2] == "::":
                rest = m[3]
                d = DIRECTIVE.search(code(rest))
                if not d and no < len(lines):
                    d = DIRECTIVE.match(code(lines[no]).strip())
                note = rest.split(";", 1)[1].strip() if ";" in rest else ""
                mod["data"].append({
                    "name": m[1], "line": no, "mod": mod, "doc": blocks(block + ([note] if note else [])),
                    "storage": d[0].strip() if d else "",
                })
        elif m and (entry or m[2] == "::" or block):
            cur = {"name": m[1], "line": no, "mod": mod, "entry": entry, "global": entry or m[2] == "::",
                   "comment": block, "body": [], "labels": []}
            mod["routines"].append(cur)
        elif m and cur:
            cur["labels"].append(m[1])
        if cur and not entry:
            insn = re.sub(r"^\s*(?:[\w$]+::?\s*)+", "", code(line)).strip()
            if insn and "=" not in insn and not insn.startswith("."):
                cur["body"].append(insn)
        block = []
    return mod


def parse_sstab(path):
    services, group, prev = [], "", ""
    for line in path.read_text().splitlines():
        m = re.match(r"\s+(SERVICE|STUB)\s+(\w+),\s*(\d+)", line)
        if m:
            if prev.startswith(";"):
                group = comment(prev)
            services.append({"name": m[2], "code": len(services), "nargs": int(m[3]),
                             "stub": m[1] == "STUB", "group": group})
        prev = line.strip()
    return services


def parse_mlb(path):
    lib = {"path": rel(path), "stem": path.stem, "name": path.name, "notes": [], "macros": []}
    block, mac = [], None
    for no, line in enumerate(path.read_text().splitlines(), 1):
        s = line.strip()
        if mac is not None:
            if re.match(r"\.ENDM\b", s, re.I):
                mac = None
            else:
                mac["body"].append(line.rstrip())
            continue
        if s.startswith(";"):
            block.append(comment(line))
            continue
        m = re.match(r"\.MACRO\s+([\w$]+)\s*(.*)", s, re.I)
        if m:
            mac = {"name": m[1], "params": m[2], "line": no, "lib": lib, "comment": block, "body": []}
            lib["macros"].append(mac)
        elif block:
            lib["notes"].append(blocks(block))
        block = []
    for mac in lib["macros"]:
        # "SETIPL, DSBINT, ENBINT, SOFTINT: ..." documents all four
        doc = blocks(mac["comment"])
        name, args, rest = head(doc)
        names = re.findall(r"[\w$]+", f"{name} {args}") if name and args.startswith(",") else [name]
        mac["shares"] = names if len(names) > 1 else []
        mac["doc"] = rest if mac["name"] in names else doc
        if not mac["comment"]:
            owner = next((o for o in lib["macros"] if mac["name"] in o.get("shares", [])), None)
            if owner:
                mac["doc"], mac["shared"] = owner["doc"], owner
        mac["defs"] = []
        if mac["name"].endswith("DEF"):
            for b in mac["body"]:
                d = re.match(r"\s*([\w$]+)\s*=\s*([^;]*?)\s*(?:;\s*(.*))?$", b)
                if d:
                    mac["defs"].append((d[1], d[2], d[3] or ""))
    return lib


def md_tables(text, heading):
    """The tables under a heading, up to the next one: [(caption, rows)],
    the first row the header."""
    lines = text.splitlines()
    if heading not in lines:
        raise SystemExit(f"apidoc: no {heading!r} in {rel(PAL_DOC)}")
    tables, cur, caption = [], None, ""
    for line in lines[lines.index(heading) + 1:]:
        if line.startswith("#"):
            break
        if line.startswith("|"):
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            if cur is None:
                cur = []
                tables.append((caption, cur))
            if not set("".join(cells)) <= set("-: "):
                cur.append(cells)
        else:
            cur = None
            if line.strip():
                caption = line.strip()
    return tables


# ---------------------------------------------------------------- analysis

mods = [parse_mar(p) for p in EXEC]
libs = [parse_mlb(p) for p in LIBS]
services = parse_sstab(ROOT / "roottask/exec/syssrv.mar")
paldoc = PAL_DOC.read_text()

routines = {r["name"]: r for m in mods for r in m["routines"]}
data = {d["name"]: d for m in mods for d in m["data"]}
owner = {label: r for r in routines.values() for label in r["labels"]}
macros = [mac for lib in libs for mac in lib["macros"]]
starlet = {mac["name"]: mac for mac in macros if mac["lib"]["stem"] == "starlet"}
ssdef = next(mac for mac in macros if mac["name"] == "$SSDEF")
paldef = {d[0] for mac in macros if mac["name"] == "$PALDEF" for d in mac["defs"]}

for svc in services:
    svc["routine"] = routines.get("EXE$" + svc["name"])
    svc["macro"] = starlet.get(f"${svc['name']}_S")

PALMAP = [
    (r"CALL_PAL\s+#PAL\$_(\w+)", lambda m: [m[1]]),
    (r"MTPR\s+.*#PR\$_(\w+)", lambda m: ["MTPR_" + m[1]]),
    (r"MFPR\s+#PR\$_(\w+)", lambda m: ["MFPR_" + m[1]]),
    (r"DSBINT\b", lambda m: ["MFPR_IPL", "MTPR_IPL"]),
    (r"(SETIPL|ENBINT)\b", lambda m: ["MTPR_IPL"]),
    (r"SOFTINT\b", lambda m: ["MTPR_SIRR"]),
    (r"(REI|HALT|CHMK)\b", lambda m: [m[1].upper()]),
]

for r in routines.values():
    name, args, doc = head(blocks(r["comment"]))
    r["args"] = args
    r["doc"], r["regs"], r["ipl"] = facts(doc)
    r["status"] = set()
    r["edges"], r["pal"], r["svcs"], r["uses"] = [], [], [], set()
    body = r["body"]
    for i, insn in enumerate(body):
        r["status"] |= set(re.findall(r"SS\$_\w+", insn))
        r["uses"] |= {t for t in SYMBOL.findall(insn) if t in data}
        for pat, f in PALMAP:
            m = re.match(pat, insn, re.I)
            if m:
                r["pal"] += [p for p in f(m) if p not in r["pal"]]
        m = re.match(r"(\$\w+)_S\b", insn)
        if m and m[1] not in r["svcs"]:
            r["svcs"].append(m[1])
        m = CALL.match(insn)
        if not m:
            continue
        target = re.sub(r"^[@#]|G\^", "", m[2].split(",")[-1].strip())
        callee = routines.get(target) or owner.get(target)
        if target.startswith("SYS$"):
            if "$" + target[4:] not in r["svcs"]:
                r["svcs"].append("$" + target[4:])
        if not callee or callee is r:
            continue
        op = m[1].upper()
        if op in ("BRB", "BRW", "JMP"):
            passes = True  # a tail call: its status is ours
        else:
            # a call whose failure is ours: BLBC R0 right after it, or a return
            after = body[i + 1:i + 3]
            passes = any(re.match(r"BLBC\s+R0\b", a, re.I) for a in after) or after[:1] in (["RET"], ["RSB"])
        r["edges"].append((callee, passes))


def closure(r, seen=None):
    seen = seen if seen is not None else set()
    if r["name"] in seen:
        return set()
    seen.add(r["name"])
    out = set(r["status"])
    for callee, passes in r["edges"]:
        if passes:
            out |= closure(callee, seen)
    return out


def calls(r, seen=None):
    """The global routines r calls, through its local ones."""
    seen = seen if seen is not None else {r["name"]}
    out = []
    for callee, _ in r["edges"]:
        if callee["name"] in seen:
            continue
        seen.add(callee["name"])
        out += [callee] if callee["global"] else calls(callee, seen)
    return out


for r in routines.values():
    r["returns"] = closure(r)
    r["calls"] = calls(r)
called_by = {}
for r in routines.values():
    for c in r["calls"]:
        called_by.setdefault(c["name"], []).append(r)
for r in [r for r in routines.values() if not r["global"]]:  # locals pass their PAL calls up
    for caller in routines.values():
        if any(c is r for c, _ in caller["edges"]):
            caller["pal"] += [p for p in r["pal"] if p not in caller["pal"]]

# PAL calls from DESIGN-0001
impl = md_tables(paldoc, "### Implemented")[0][1]
vaxmap = md_tables(paldoc, "## How MACRO-32 calls it")[0][1]
pals = []
for row in impl[1:]:
    p = dict(zip(impl[0], row))
    name = p["Call"].strip("`")
    p["name"], p["code"] = name, int(p["Code"], 16)
    p["vax"] = [v[0] for v in vaxmap[1:] if name in re.findall(r"`([^`]+)`", v[1])]
    p["users"] = [r for r in routines.values() if r["global"] and name in r["pal"]]
    pals.append(p)
PAL_REFS = [
    ("Calling the PAL", "## Calling the PAL"),
    ("VAX instructions that are PAL calls", "## How MACRO-32 calls it"),
    ("What the PAL sets up", "## What the PAL sets up"),
    ("The restart parameter block", "### The restart parameter block"),
    ("Page table entries", "## Memory"),
    ("The interrupt frame", "## Interrupts and exceptions"),
    ("Faults and HALT", "### Faults and HALT"),
    ("The Alpha OpenVMS calls", "### The Alpha calls"),
]

# ---------------------------------------------------------------- anchors

SYMS = {}


def sym(name, anchor):
    SYMS.setdefault(name, anchor)


for svc in services:
    for n in ("$", "SYS$", "EXE$"):
        sym(n + svc["name"], "svc-" + svc["name"])
for r in routines.values():
    sym(r["name"], "r-" + r["name"])
for d in data.values():
    sym(d["name"], "d-" + d["name"])
for p in pals:
    sym(p["name"], "pal-" + p["name"])
    sym("PAL$_" + p["name"], "pal-" + p["name"])
for name, _, _ in ssdef["defs"]:
    sym(name, "ss-" + name)
for mac in macros:
    sym(mac["name"], f"m-{mac['lib']['stem']}-{mac['name']}")
for mac in macros:
    for name, _, _ in mac["defs"]:
        sym(name, "s-" + name)

# ---------------------------------------------------------------- HTML

esc = html.escape


def link(text):
    def one(m):
        a = SYMS.get(m[0])
        return f'<a href="#{a}">{m[0]}</a>' if a else m[0]
    return SYMBOL.sub(one, esc(text, quote=False))


INLINE = re.compile(r"`([^`]+)`|\[([^\]]+)\]\(([^)]+)\)|\*\*([^*]+)\*\*|\*([^*]+)\*")


def md(text):
    """Markdown inline: code, links, bold and italics, and symbol links."""
    out, pos = [], 0
    for m in INLINE.finditer(text):
        out.append(link(text[pos:m.start()]))
        if m[1]:
            out.append(f"<code>{link(m[1])}</code>")
        elif m[2]:
            url = m[3] if "://" in m[3] or m[3].startswith("#") else "../design/" + m[3]
            out.append(f'<a href="{esc(url)}">{esc(m[2])}</a>')
        elif m[4]:
            out.append(f"<b>{link(m[4])}</b>")
        else:
            out.append(f"<i>{link(m[5])}</i>")
        pos = m.end()
    out.append(link(text[pos:]))
    return "".join(out)


def prose(doc):
    out = []
    for kind, text in doc:
        if kind == "pre":
            out.append(f"<pre>{link(text)}</pre>")
        elif kind == "note":
            out.append(f'<p class="note"><b>Limit.</b> {link(text)}</p>')
        else:
            out.append(f"<p>{link(text)}</p>")
    return "".join(out)


def table(rows, cell=md, ids=None):
    th = "".join(f"<th>{esc(c)}</th>" for c in rows[0])
    body = ""
    for i, row in enumerate(rows[1:]):
        rid = f' id="{ids[i]}"' if ids else ""
        body += f"<tr{rid}>" + "".join(f"<td>{cell(c)}</td>" for c in row) + "</tr>"
    return f'<div class="tw"><table><thead><tr>{th}</tr></thead><tbody>{body}</tbody></table></div>'


def refs(items):
    return " ".join(f'<a class="ref" href="#{SYMS[i if isinstance(i, str) else i["name"]]}">'
                    f'{esc(i if isinstance(i, str) else i["name"])}</a>' for i in items)


def src(path, line):
    return f'<a class="src" href="../../{path}" title="open the source">{esc(path)}:{line}</a>'


def summary(doc):
    text = next((t for k, t in doc if k == "p"), "")
    s = re.split(r"(?<=\.)\s", text, maxsplit=1)[0]
    return s if len(s) < 110 else s[:107] + "…"


INDEX = []
TOC = []  # (level, id, title)
toc_mark = [0]  # where the next chapter's entry goes: before its sections


def entry(anchor, name, tag, fields, source, summ, kind):
    INDEX.append([name, kind, summ, anchor])
    rows = "".join(f"<dt>{k}</dt><dd>{v}</dd>" for k, v in fields if v)
    return (f'<article class="entry" id="{anchor}"><header><h4><a href="#{anchor}">{esc(name)}</a></h4>'
            f'<span class="tag t-{tag.lower().split()[0]}">{tag}</span>{source}</header><dl>{rows}</dl></article>')


def statuses(codes):
    order = [n for n, _, _ in ssdef["defs"]]
    return refs(sorted(codes, key=lambda c: order.index(c) if c in order else 999))


def chapter(num, anchor, title, intro, body):
    TOC.insert(toc_mark[0], (1, anchor, f"{num} {title}"))
    toc_mark[0] = len(TOC)
    return (f'<section class="chapter" id="{anchor}"><h2>{f'<span class="num">{num}</span>' if num else ''}{esc(title)}</h2>'
            f"{intro}{body}</section>")


def section(num, anchor, title, intro, body, toc_title=None):
    TOC.append((2, anchor, f"{num} {toc_title or title}"))
    return f'<section id="{anchor}"><h3><span class="num">{num}</span>{title}</h3>{intro}{body}</section>'


def routine_entry(r):
    tag = "CALL" if r["entry"] else "Handler" if "REI" in r["body"] else "JSB"
    if not r["global"]:
        tag += " local"
    linkage = {
        "CALL": f"<code>CALLS #n, G^{esc(r['name'])}</code> or <code>CALLG</code>; arguments in the AP list",
        "JSB": f"<code>JSB G^{esc(r['name'])}</code>; arguments in registers, returns with <code>RSB</code>",
        "Handler": "reached through the SCB; returns with <code>REI</code>",
    }[tag.split()[0]]
    if not r["global"]:
        linkage = f"<code>BSBW {esc(r['name'])}</code>, from {esc(r['mod']['path'])} only"
    return entry("r-" + r["name"], r["name"], tag, [
        ("Linkage", linkage),
        ("Arguments", f"<code>{link(r['args'])}</code>" if r["args"] and r["entry"] else ""),
        ("Description", prose(r["doc"]) or "<p><i>Undocumented.</i></p>"),
        ("IPL", link(r["ipl"])),
        ("Registers", link(r["regs"])),
        ("Status", statuses(r["returns"])),
        ("Calls", refs(r["calls"])),
        ("Called by", refs(called_by.get(r["name"], []))),
        ("Services", refs(r["svcs"])),
        ("PAL calls", refs(p for p in r["pal"] if p in SYMS)),
        ("Data", refs(sorted(r["uses"]))),
    ], src(r["mod"]["path"], r["line"]), summary(r["doc"]), "routine" if r["global"] else "local routine")


def service_entry(svc):
    r = svc["routine"]
    name = "$" + svc["name"]
    if svc["stub"]:
        doc = [("p", "Not implemented yet: returns SS$_ILLSER.")]
        fields = [("Format", f"<code>SYS${svc['name']}</code>, {svc['nargs']} arguments"),
                  ("Description", prose(doc)), ("Status", statuses({"SS$_ILLSER"}))]
        return entry("svc-" + svc["name"], name, "Stub", fields, "", summary(doc), "system service, stub")
    codes = set(r["returns"]) | ({"SS$_INSFARG"} if svc["nargs"] else set())
    mac = svc["macro"]
    fields = [
        ("Format", f"<code><b>SYS${svc['name']}</b> {link(r['args'])}</code>"),
        ("Macro", f"<code>{link(mac['name'])} {link(mac['params'])}</code>" if mac else ""),
        ("Dispatch", f"<code>CHMK #{svc['code']}</code> to {link('EXE$CMODKRNL')}, "
                     f"which calls {esc(r['name'])} with at least {svc['nargs']} arguments"),
        ("Description", prose(r["doc"])),
        ("IPL", link(r["ipl"])),
        ("Status", statuses(codes)),
        ("Calls", refs(r["calls"])),
        ("PAL calls", refs(p for p in r["pal"] if p in SYMS)),
        ("Data", refs(sorted(r["uses"]))),
    ]
    return entry("svc-" + svc["name"], name, "Service", fields, src(r["mod"]["path"], r["line"]),
                 summary(r["doc"]), "system service")


def macro_entry(mac):
    lib = mac["lib"]
    anchor = f"m-{lib['stem']}-{mac['name']}"
    svc = re.fullmatch(r"\$(\w+)_S", mac["name"])
    doc = mac["doc"]
    if not doc and svc:
        doc = [("p", f"Pushes its arguments and calls SYS${svc[1]}, the ${svc[1]} service.")]
    if mac.get("shared"):
        doc = doc + [("p", f"Documented with {mac['shared']['name']}.")]
    fields = [("Format", f"<code><b>{esc(mac['name'])}</b> {link(mac['params'])}</code>"),
              ("Description", prose(doc) or "<p><i>Undocumented.</i></p>")]
    if mac["defs"]:
        rows = [["Symbol", "Value", "Meaning"]] + [list(d) for d in mac["defs"]]
        ids = ["s-" + d[0] if SYMS.get(d[0]) == "s-" + d[0] else None for d in mac["defs"]]
        fields.append(("Symbols", table(rows, link, ids).replace(' id="None"', "")))
        for d in mac["defs"] if mac is not ssdef else []:  # chapter 5 indexes those
            INDEX.append([d[0], f"{mac['name']} symbol", f"= {d[1]}" + (f" — {d[2]}" if d[2] else ""), SYMS[d[0]]])
    else:
        fields.append(("Expansion", "<details><summary>show</summary><pre>"
                       + link("\n".join(mac["body"])) + "</pre></details>"))
    tag = "Definitions" if mac["defs"] else "Macro"
    return entry(anchor, mac["name"], tag, fields, src(lib["path"], mac["line"]), summary(doc),
                 f"macro, {lib['name']}")


def pal_entry(p):
    priv = "privileged, kernel mode only" if p["code"] < 0x80 else "any mode"
    fields = [
        ("Format", f"<code>CALL_PAL #{link('PAL$_' + p['name'])}</code>" if "PAL$_" + p["name"] in paldef
         else f"<code>CALL_PAL #^X{p['code']:02X}</code>"),
        ("VAX", "<br>".join(f"<code>{md(v)}</code>" for v in p["vax"])),
        ("Code", f"<code>^X{p['code']:02X}</code>, {priv}; {md(p['Origin'])}"),
        ("In", md(p["In"]) or "—"),
        ("Out", md(p["Out"]) or "—"),
        ("Description", f"<p>{md(p['What the PAL does'][:1].upper() + p['What the PAL does'][1:])}</p>" if p["What the PAL does"] else ""),
        ("Used by", refs(p["users"])),
    ]
    return entry("pal-" + p["name"], p["name"], "PAL", fields,
                 f'<a class="src" href="../design/0001-pal-interface.md">DESIGN-0001</a>',
                 re.sub(r"`", "", p["What the PAL does"]) or f"PAL call ^X{p['code']:02X}", "PAL call")


# ---------------------------------------------------------------- chapters

def ch_overview():
    rows = [["Tag", "How it is called"],
            ["Service", "`SYS$name` with `CALLS` or `CALLG`, or the `$name_S` macro; `CHMK` takes it to the executive"],
            ["CALL", "`CALLS` or `CALLG`: arguments in the argument list at AP, status in R0, `RET`"],
            ["JSB", "`JSB` or `BSBW`: arguments and results in registers, `RSB`"],
            ["Handler", "an SCB vector: the PAL delivers an interrupt or `CHMK` to it; it ends with `REI`"],
            ["PAL", "a privileged VAX instruction, or `CALL_PAL`; vmacro compiles it into `svc #0`"],
            ["Macro", "a macro from a `.LIBRARY`; Definitions macros define symbols"]]
    intro = (
        "<p>The interfaces of vaxpunk's executive and the PAL below it, as code written against them sees "
        "them. Every entry is read from the sources by <code>scripts/apidoc.py</code>: the comment above "
        "each routine and macro, the system service vector, and the tables of "
        '<a href="../design/0001-pal-interface.md">DESIGN-0001</a>. How the pieces fit is in '
        '<a href="../design/0001-pal-interface.md">DESIGN-0001</a> (the PAL) and '
        '<a href="../design/0002-executive-processes.md">DESIGN-0002</a> (the executive).</p>'
        "<p>Press <kbd>/</kbd> to search. Every name in the text links to its entry. "
        "<i>Status</i> lists the condition values found in a routine's code and in what it calls "
        "whose failure it passes on. A <i>Limit</i> is a deliberate shortcut, with the way past it.</p>")
    return chapter("", "ch-overview", "Overview", intro, table(rows))


def ch_services():
    body, groups = "", []
    for svc in services:
        if svc["group"] not in groups:
            groups.append(svc["group"])
    for i, g in enumerate(groups, 1):
        items = [s for s in services if s["group"] == g]
        done = sum(not s["stub"] for s in items)
        intro = f'<p class="meta">{done} of {len(items)} implemented.</p>'
        body += section(f"1.{i}", f"svc-group-{i}", esc(g), intro, "".join(service_entry(s) for s in items), g)
    intro = ("<p>Called from any mode with <code>CALLS</code> or <code>CALLG</code> to <code>SYS$name</code>, "
             "which is in <code>syssrv.mar</code>, or with the <code>$name_S</code> macros of "
             '<a href="#lib-starlet">starlet.mlb</a>. Every process runs in kernel mode for now. A service '
             "called with fewer arguments than it takes returns SS$_INSFARG.</p>")
    return chapter("1", "ch-services", "System services", link_intro(intro), body)


def link_intro(text):
    return re.sub(r"SS\$_\w+", lambda m: f'<a href="#{SYMS[m[0]]}">{m[0]}</a>' if m[0] in SYMS else m[0], text)


def ch_routines():
    body = ""
    for i, m in enumerate(mods, 1):
        rs = [r for r in m["routines"] if not (r["entry"] and any(s["routine"] is r for s in services))]
        rs.sort(key=lambda r: not r["global"])
        intro = f'<p class="meta">{src(m["path"], 1)}</p>' + prose(m["intro"])
        dl = ""
        if m["data"]:
            rows = [["Name", "Storage", "Holds"]] + [
                [d["name"], d["storage"], " ".join(t for _, t in d["doc"])] for d in m["data"]]
            ids = ["d-" + d["name"] for d in m["data"]]
            dl = "<h5>Data</h5>" + table(rows, link, ids)
            for d in m["data"]:
                INDEX.append([d["name"], "data", " ".join(t for _, t in d["doc"]) or d["storage"], "d-" + d["name"]])
        body += section(f"2.{i}", "mod-" + m["stem"], f"{esc(m['stem'].upper())} <small>{esc(m['title'])}</small>",
                        intro, "".join(routine_entry(r) for r in rs) + dl, m["stem"].upper())
    intro = ("<p>Routines and data of <code>EXEC.EXE</code>, by module. Programs link against "
             "<code>SYS.STB</code>, so kernel-mode code reaches every global here directly. The system "
             'services\' own routines, <code>EXE$name</code>, are in <a href="#ch-services">chapter 1</a>; '
             "local routines can only be called from their own module.</p>")
    return chapter("2", "ch-routines", "Executive routines", intro, body)


def ch_pal():
    body = section("3.1", "pal-calls", "Calls", "", "".join(pal_entry(p) for p in sorted(pals, key=lambda p: p["code"])))
    for i, (title, heading) in enumerate(PAL_REFS, 2):
        t = "".join((f'<p class="cap">{md(cap)}</p>' if cap and not cap.endswith(".") or cap.endswith(":") else "")
                    + table(rows) for cap, rows in md_tables(paldoc, heading))
        anchor = "pal-ref-" + re.sub(r"\W+", "-", title.lower()).strip("-")
        body += section(f"3.{i}", anchor, esc(title), f'<p class="meta">From <a href="../design/0001-pal-interface.md">'
                        f"DESIGN-0001</a>, {esc(heading.lstrip('# '))}.</p>", t)
    intro = ("<p>The executive calls the PAL, the root task, with <code>svc #0</code>: the function code in "
             "x7, arguments in x0-x5, the result in x0. MACRO-32 code doesn't write that: vmacro compiles "
             "privileged VAX instructions and <code>CALL_PAL</code> into it.</p>")
    return chapter("3", "ch-pal", "PAL calls", intro, body)


def ch_libs():
    body = ""
    for i, lib in enumerate(libs, 1):
        intro = f'<p class="meta">{src(lib["path"], 1)}</p>' + "".join(prose(n) for n in lib["notes"])
        body += section(f"4.{i}", "lib-" + lib["stem"], esc(lib["name"]), intro,
                        "".join(macro_entry(m) for m in lib["macros"]))
    intro = "<p>Macro libraries, for <code>.LIBRARY</code> with <code>vasm -I</code>.</p>"
    return chapter("4", "ch-libs", "Macros and definitions", intro, body)


def ch_status():
    users = {}
    for svc in services:
        codes = {"SS$_ILLSER"} if svc["stub"] else svc["routine"]["returns"] | ({"SS$_INSFARG"} if svc["nargs"] else set())
        for c in codes:
            users.setdefault(c, []).append("$" + svc["name"])
    sev = {0: "Warning", 1: "Success", 2: "Error", 3: "Informational", 4: "Fatal"}
    rows = [["Code", "Value", "Severity", "Returned by", "Meaning"]]
    for name, value, note in ssdef["defs"]:
        v = int(value)
        rows.append([name, f"{v} (^X{v:X})", sev.get(v & 7, "?"), " ".join(users.get(name, [])), note])
        INDEX.append([name, "condition value", f"{v}, {sev.get(v & 7, '?').lower()}", "ss-" + name])
    ids = ["ss-" + n for n, _, _ in ssdef["defs"]]
    intro = ('<p>From <a href="#m-starlet-$SSDEF">$SSDEF</a>, numbered as VMS numbers them. The low three bits '
             "are the severity: odd is success.</p>")
    return chapter("5", "ch-status", "Condition values", intro, table(rows, link, ids))


def ch_index():
    seen, letters = set(), {}
    for name, kind, _, anchor in sorted(INDEX, key=lambda e: (e[0].lstrip("$").upper(), e[0])):
        if (name, anchor) in seen:
            continue
        seen.add((name, anchor))
        letters.setdefault(name.lstrip("$")[0].upper(), []).append(
            f'<li><a href="#{anchor}">{esc(name)}</a> <span>{esc(kind)}</span></li>')
    body = "".join(f'<h5 id="ix-{k}">{k}</h5><ul class="ix">{"".join(v)}</ul>' for k, v in letters.items())
    jump = " ".join(f'<a href="#ix-{k}">{k}</a>' for k in letters)
    return chapter("", "ch-index", "Index", f'<p class="jump">{jump}</p>', body)


chapters = [ch_overview(), ch_services(), ch_routines(), ch_pal(), ch_libs(), ch_status()]
chapters.append(ch_index())


def toc():
    out, open2 = [], False
    for level, anchor, title in TOC:
        if level == 1:
            if open2:
                out.append("</ul></details>")
            out.append(f'<details open><summary><a href="#{anchor}">{esc(title.strip())}</a></summary><ul>')
            open2 = True
        else:
            out.append(f'<li><a href="#{anchor}">{esc(title)}</a></li>')
    return "".join(out) + "</ul></details>"


CSS = """
:root{--paper:#f6f1e3;--paper2:#ece4cf;--ink:#1c1a17;--dim:#6b6355;--rule:#1c1a17;--hair:#cfc4a8;
--orange:#e0661b;--maroon:#7b1d22;--link:#7b1d22;--code:#efe7d2;--note:#fff4d6;
--sans:"Helvetica Neue",Helvetica,Arial,sans-serif;--serif:Palatino,"Palatino Linotype","Book Antiqua",Georgia,serif;
--mono:ui-monospace,"SF Mono",Menlo,Consolas,"Courier New",monospace;--glow:none}
html.vt{--paper:#0c0904;--paper2:#140f06;--ink:#ffb000;--dim:#b07a00;--rule:#ffb000;--hair:#4a3500;
--orange:#ffb000;--maroon:#ffb000;--link:#ffd166;--code:#1b1407;--note:#1f1606;
--sans:var(--mono);--serif:var(--mono);--glow:0 0 3px rgba(255,176,0,.55)}
*{box-sizing:border-box}html{scroll-padding-top:5.5rem}
body{margin:0;background:var(--paper);color:var(--ink);font:16px/1.5 var(--serif);text-shadow:var(--glow)}
html.vt body::after{content:"";position:fixed;inset:0;pointer-events:none;
background:repeating-linear-gradient(transparent 0 2px,rgba(0,0,0,.25) 2px 3px)}
a{color:var(--link);text-decoration:none}a:hover{text-decoration:underline}
code,pre,kbd{font-family:var(--mono);font-size:.86em}
pre{background:var(--code);padding:.6rem .8rem;overflow:auto;border-left:3px solid var(--hair)}
kbd{border:1px solid var(--dim);border-bottom-width:2px;padding:0 .3em;border-radius:3px}
.top{position:fixed;top:0;left:0;right:0;height:4rem;z-index:10;display:flex;align-items:center;gap:1.2rem;
padding:0 1rem;background:var(--orange);color:#111;border-bottom:3px solid var(--rule);text-shadow:none}
html.vt .top{background:var(--paper2);color:var(--ink)}
.logo{display:flex;gap:2px}.logo span{display:inline-block;width:1.45rem;height:1.9rem;background:var(--maroon);
color:#fff;font:700 1.25rem/1.9rem var(--sans);text-align:center;text-transform:lowercase}
html.vt .logo span{background:none;color:var(--ink);border:1px solid var(--ink)}
.ttl{font:700 1.05rem/1.15 var(--sans);letter-spacing:.02em}.ttl small{display:block;font-weight:400;font-size:.75rem}
.search{position:relative;margin-left:auto;width:min(30rem,45vw)}
#q{width:100%;font:.95rem var(--sans);padding:.45rem .7rem;border:2px solid var(--rule);background:var(--paper);
color:var(--ink);border-radius:0}#q:focus{outline:3px solid var(--maroon);outline-offset:0}
#hits{position:absolute;right:0;left:0;top:2.6rem;margin:0;padding:0;list-style:none;background:var(--paper);
border:2px solid var(--rule);max-height:70vh;overflow:auto;display:none;box-shadow:6px 6px 0 rgba(0,0,0,.25)}
#hits.on{display:block}#hits li a{display:block;padding:.35rem .7rem;color:var(--ink);border-bottom:1px solid var(--hair)}
#hits li.sel a,#hits li a:hover{background:var(--paper2);text-decoration:none}
#hits b{font-family:var(--mono)}#hits .k{float:right;font:.7rem var(--sans);text-transform:uppercase;color:var(--dim)}
#hits .s{display:block;font-size:.8rem;color:var(--dim);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
#theme{font:700 .75rem var(--sans);padding:.4rem .6rem;border:2px solid var(--rule);background:var(--paper);
color:var(--ink);cursor:pointer}
#toc{position:fixed;top:4rem;bottom:0;left:0;width:17rem;overflow:auto;padding:1rem .8rem 3rem 1rem;
border-right:1px solid var(--hair);background:var(--paper2);font:.85rem/1.4 var(--sans)}
#toc summary{font-weight:700;cursor:pointer;margin:.6rem 0 .2rem;text-transform:uppercase;font-size:.78rem;letter-spacing:.05em}
#toc ul{list-style:none;margin:0;padding-left:.9rem}#toc li a{display:block;padding:.08rem .3rem;color:var(--ink)}
#toc a.on{background:var(--ink);color:var(--paper);text-shadow:none}
main{margin:4rem 0 0 17rem;padding:1.5rem 2.5rem 6rem;max-width:62rem}
h2,h3,h4,h5{font-family:var(--sans)}
h2{font-size:1.9rem;margin:3rem 0 1rem;padding-top:.6rem;border-top:6px solid var(--rule)}
h3{font-size:1.3rem;margin:2.4rem 0 .6rem;padding-top:.3rem;border-top:2px solid var(--rule)}
h3 small{font-weight:400;color:var(--dim);font-size:.85rem;margin-left:.5rem}
h5{text-transform:uppercase;letter-spacing:.08em;font-size:.75rem;margin:1.5rem 0 .4rem}
.num{display:inline-block;min-width:2.6rem;color:var(--orange)}
html:not(.vt) .num{color:var(--maroon)}
.meta,.cap{font:.8rem var(--sans);color:var(--dim)}.cap{font-weight:700;color:var(--ink);margin-bottom:.2rem}
.entry{margin:1.6rem 0 2rem;padding-top:.4rem;border-top:1px solid var(--rule)}
.entry header{display:flex;align-items:baseline;gap:.7rem;flex-wrap:wrap}
.entry h4{margin:0;font:700 1.25rem var(--mono)}.entry h4 a{color:var(--ink)}
.entry:target{background:linear-gradient(90deg,var(--note),transparent 70%);outline:0}
.tag{font:700 .65rem var(--sans);text-transform:uppercase;letter-spacing:.08em;padding:.1rem .45rem;
border:1.5px solid var(--ink)}.t-service,.t-pal{background:var(--ink);color:var(--paper);text-shadow:none}
.t-stub{border-style:dashed;color:var(--dim)}
.src{margin-left:auto;font:.75rem var(--mono);color:var(--dim)}
dl{display:grid;grid-template-columns:8.5rem 1fr;gap:.35rem 1rem;margin:.7rem 0 0}
dt{font:700 .7rem/1.9 var(--sans);text-transform:uppercase;letter-spacing:.1em;color:var(--dim)}
dd{margin:0;min-width:0}dd p:first-child{margin-top:0}dd p{margin:.2rem 0 .5rem}
.ref{font-family:var(--mono);font-size:.82em;margin-right:.55em;white-space:nowrap}
.note{background:var(--note);border-left:3px solid var(--orange);padding:.3rem .6rem;font-size:.92em}
.tw{overflow:auto}table{border-collapse:collapse;font-size:.85rem;margin:.3rem 0 1rem;width:100%}
th{font:700 .7rem var(--sans);text-transform:uppercase;letter-spacing:.06em;text-align:left;
border-bottom:2px solid var(--rule);padding:.3rem .5rem}
td{border-bottom:1px solid var(--hair);padding:.25rem .5rem;vertical-align:top}
td:first-child{font-family:var(--mono);white-space:nowrap}tr:target{background:var(--note)}
details>summary{cursor:pointer}dd details summary{font:.75rem var(--sans);color:var(--dim)}
.ix{columns:3 14rem;list-style:none;padding:0;font-size:.85rem}.ix li{break-inside:avoid}
.ix li a{font-family:var(--mono)}.ix span{color:var(--dim);font:.7rem var(--sans)}.jump a{margin-right:.5em;font-family:var(--sans)}
@media (max-width:900px){#toc{display:none}main{margin-left:0;padding:1rem}.ttl{display:none}
dl{grid-template-columns:1fr}dt{line-height:1.2;margin-top:.4rem}}
@media print{.top,#toc{display:none}main{margin:0}}
"""

JS = """
const q=document.getElementById('q'),hits=document.getElementById('hits');let res=[],sel=0;
function score(e,t){const n=e[0].toLowerCase(),b=n.replace(/^[a-z]*\\$_?/,'');t=t.replace(/^sys\\$/,'$');
 if(n===t||b===t)return 0;if(n.startsWith(t)||b.startsWith(t))return 1;if(n.includes(t))return 2;
 const h=(n+' '+e[1]+' '+e[2]).toLowerCase();return t.split(/\\s+/).every(w=>h.includes(w))?3:-1}
function esc(s){return s.replace(/[&<>]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;'}[c]))}
function show(){const t=q.value.trim().toLowerCase();if(!t){hits.classList.remove('on');return}
 res=IDX.map(e=>[score(e,t),e]).filter(x=>x[0]>=0).sort((a,b)=>a[0]-b[0]||a[1][0].length-b[1][0].length).slice(0,40).map(x=>x[1]);
 sel=0;hits.innerHTML=res.length?res.map((e,i)=>`<li${i?'':' class=sel'}><a href="#${e[3]}"><b>${esc(e[0])}</b><span class=k>${esc(e[1])}</span><span class=s>${esc(e[2])}</span></a></li>`).join(''):'<li><a>No match</a></li>';
 hits.classList.add('on')}
function mark(){[...hits.children].forEach((li,i)=>li.classList.toggle('sel',i===sel));hits.children[sel]?.scrollIntoView({block:'nearest'})}
function go(e){location.hash=e[3];hits.classList.remove('on');q.blur()}
q.addEventListener('input',show);q.addEventListener('focus',show);
q.addEventListener('keydown',ev=>{if(ev.key==='ArrowDown'){sel=Math.min(sel+1,res.length-1);mark();ev.preventDefault()}
 else if(ev.key==='ArrowUp'){sel=Math.max(sel-1,0);mark();ev.preventDefault()}
 else if(ev.key==='Enter'&&res[sel])go(res[sel]);else if(ev.key==='Escape'){q.value='';hits.classList.remove('on');q.blur()}});
hits.addEventListener('click',()=>{hits.classList.remove('on');q.blur()});
document.addEventListener('click',ev=>{if(!ev.target.closest('.search'))hits.classList.remove('on')});
document.addEventListener('keydown',ev=>{if((ev.key==='/'||(ev.key==='k'&&(ev.metaKey||ev.ctrlKey)))&&document.activeElement!==q){q.focus();q.select();ev.preventDefault()}});
const th=document.getElementById('theme');function setvt(on){document.documentElement.classList.toggle('vt',on);th.textContent=on?'PAPER':'VT220';localStorage.setItem('vt',on?'1':'')}
setvt(!!localStorage.getItem('vt'));th.addEventListener('click',()=>setvt(!document.documentElement.classList.contains('vt')));
const links=new Map([...document.querySelectorAll('#toc a')].map(a=>[a.getAttribute('href').slice(1),a]));
const spy=new IntersectionObserver(es=>{for(const e of es)if(e.isIntersecting){const a=links.get(e.target.id);if(!a)continue;
 document.querySelectorAll('#toc a.on').forEach(x=>x.classList.remove('on'));a.classList.add('on');a.scrollIntoView({block:'nearest'})}},
 {rootMargin:'0px 0px -85% 0px'});
document.querySelectorAll('main section[id]').forEach(el=>spy.observe(el));
"""

page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<!-- Generated by scripts/apidoc.py from the sources. Don't edit: run it. -->
<title>vaxpunk Internals — API Reference</title><style>{CSS}</style></head>
<body><header class="top"><a class="logo" href="#ch-overview" aria-label="vaxpunk">{"".join(f"<span>{c}</span>" for c in "vaxpunk")}</a>
<div class="ttl">Internals and Data Structures<small>API Reference · executive, PAL and libraries</small></div>
<div class="search"><input id="q" type="search" placeholder="Search routines, services, symbols…   /" autocomplete="off" spellcheck="false" aria-label="Search"><ol id="hits"></ol></div>
<button id="theme" type="button" title="Paper or terminal">VT220</button></header>
<nav id="toc">{toc()}</nav>
<main>{"".join(chapters)}</main>
<script>const IDX={json.dumps(INDEX, ensure_ascii=False).replace("</", "<\\/")};{JS}</script>
</body></html>
"""

OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text(page)
print(f"apidoc: {rel(OUT)}: {len(INDEX)} entries")
