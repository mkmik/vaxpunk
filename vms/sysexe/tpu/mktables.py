#!/usr/bin/env python3
"""Writes tpukw.r64, TPU's tables of keywords, built-ins and messages, from
what DEC's TPU says of itself on the oracle (oracle/):

    keywords.txt  SHOW (KEYWORDS), without EVE's EVE$_ messages
    builtins.txt  SHOW (PROCEDURES) with no procedures of one's own
    messages.txt  name|MESSAGE_TEXT (name, 15) for each TPU$_ keyword

usage: mktables.py   (in vms/sysexe/tpu; writes tpukw.r64)

The message keywords come first, so that a keyword is a message when its
code is at most KW_NMSG. Each list is in name order.
"""
import os, re

HERE = os.path.dirname(os.path.abspath(__file__))


def lines(name):
    with open(os.path.join(HERE, "oracle", name), encoding="latin-1") as f:
        return [l.rstrip("\n") for l in f if l.strip()]


def bliss_str(s):
    return "'" + s.replace("'", "''") + "'"


def main():
    keywords = lines("keywords.txt")
    builtins = lines("builtins.txt")
    messages = {}
    if os.path.exists(os.path.join(HERE, "oracle", "messages.txt")):
        for l in lines("messages.txt"):
            name, text = l.split("|", 1)
            m = re.match(r"%TPU-([SIWEF])-(\w+), (.*)$", text)
            messages[name] = (m.group(1), m.group(3)) if m else ("E", text)
    msgs = sorted(k for k in keywords if k.startswith("TPU$_"))
    rest = sorted(k for k in keywords if not k.startswith("TPU$_"))
    out = ["! TPUKW.R64: TPU's keywords, built-ins and messages, as DEC's TPU",
           "! names them (SHOW (KEYWORDS), SHOW (PROCEDURES), MESSAGE_TEXT).",
           "! Written by mktables.py from oracle/: don't edit.", ""]

    out.append("MACRO")
    out.append("    MESSAGES =")
    items = []
    for k in msgs:
        sev, text = messages.get(k, ("E", "?"))
        items.append(f"        {k}, '{k}', '{sev}', {bliss_str(text)}")
    out.append(",\n".join(items) + " %,")
    out.append("    KEYWORDS =")
    out.append(",\n".join(f"        K_{k}, '{k}'" for k in rest) + " %,")
    out.append("    BUILTINS =")
    out.append(",\n".join(f"        B_{b}, '{b}'" for b in builtins) + " %,")
    out.append("    MSGLIT[ID, S, SEV, TEXT] = ID = %COUNT + 1 %,")
    out.append("    MSGSTR[ID, S, SEV, TEXT] = %CHAR(%CHARCOUNT(S)), S, SEV, %CHAR(%CHARCOUNT(TEXT)), TEXT %,")
    out.append("    KWLIT[ID, S] = ID = KW_NMSG + %COUNT + 1 %,")
    out.append("    BILIT[ID, S] = ID = %COUNT + 1 %,")
    out.append("    NAMESTR[ID, S] = %CHAR(%CHARCOUNT(S)), S %;")
    out.append("")
    out.append("LITERAL")
    out.append(f"    KW_NMSG = {len(msgs)},")
    out.append(f"    KW_COUNT = {len(msgs) + len(rest)},")
    out.append(f"    BI_COUNT = {len(builtins)},")
    out.append("    MSGLIT(MESSAGES),")
    out.append("    KWLIT(KEYWORDS),")
    out.append("    BILIT(BUILTINS);")
    with open(os.path.join(HERE, "tpukw.r64"), "w") as f:
        f.write("\n".join(out) + "\n")


if __name__ == "__main__":
    main()
