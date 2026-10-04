//! vcdu's tables: their layout (docs/command-tables.md), and the errors in
//! CLD it reports.

/// The bytes of the table vcdu makes of `cld`, from its MACRO-32.
fn table(cld: &str) -> Vec<u8> {
    let source = vcdu::compile("T", &[("t.cld".into(), cld.into())]).unwrap();
    source
        .lines()
        .filter_map(|l| l.trim().strip_prefix(".BYTE"))
        .flat_map(|l| l.split(',').map(|b| b.trim().parse::<u8>().unwrap()))
        .collect()
}

fn word(t: &[u8], at: usize) -> usize {
    u16::from_le_bytes([t[at], t[at + 1]]) as usize
}

/// The .ASCIC string at `at`.
fn ascic(t: &[u8], at: usize) -> &str {
    std::str::from_utf8(&t[at + 1..at + 1 + t[at] as usize]).unwrap()
}

fn errors(cld: &str) -> Vec<String> {
    vcdu::compile("T", &[("t.cld".into(), cld.into())]).unwrap_err()
}

#[test]
fn layout() {
    let t = table(
        "module MYTAB ! a comment
         define verb COPY
             image COPY
             synonym DUPLICATE
             parameter P1, label=FROM, prompt=\"From\",
                 value(required, list, type=$infile)
             qualifier LOG
             qualifier BY, value(type=BY_TYPE)
         define type BY_TYPE
             keyword NAME, default
             keyword SIZE, value(required, type=$number)",
    );
    assert_eq!(word(&t, 0), 1, "the version");
    assert_eq!(word(&t, 2), 2, "COPY and DUPLICATE");
    assert_eq!(ascic(&t, word(&t, 4)), "COPY");
    assert_eq!(ascic(&t, word(&t, 8)), "DUPLICATE");
    let verb = word(&t, 6);
    assert_eq!(word(&t, 10), verb, "a synonym is the same block");

    // The verb: name, image, routine, parameters, qualifiers, DISALLOW.
    assert_eq!(ascic(&t, verb), "COPY");
    assert_eq!(ascic(&t, verb + 5), "COPY");
    assert_eq!(ascic(&t, verb + 10), "");
    let mut at = verb + 11;
    assert_eq!(t[at], 1);
    let p1 = word(&t, at + 1);
    at += 3;
    assert_eq!(t[at], 2);
    let (log, by) = (word(&t, at + 1), word(&t, at + 3));
    assert_eq!(word(&t, at + 5), 0, "no DISALLOW");

    // An entity: flags, type, keywords, syntax, name, label, prompt, default.
    assert_eq!(
        t[p1],
        4 | 8 | 16 | 32,
        "VALUE, REQUIRED, LIST and so CONCAT"
    );
    assert_eq!(t[p1 + 1], 1, "$INFILE is a file");
    assert_eq!(ascic(&t, p1 + 6), "P1");
    assert_eq!(ascic(&t, p1 + 9), "FROM");
    assert_eq!(ascic(&t, p1 + 14), "From");
    assert_eq!(t[log], 2, "a qualifier is negatable");
    assert_eq!(ascic(&t, log + 6 + 4 + 4), "LOG", "the prompt is the label");
    assert_eq!(t[by + 1], 5, "a keyword type");
    let keywords = word(&t, by + 2);
    assert_eq!(t[keywords], 2);
    let name = word(&t, keywords + 1);
    assert_eq!(ascic(&t, name + 6), "NAME");
    assert_eq!(t[name], 1, "DEFAULT, and a keyword is not negatable");
    let size = word(&t, keywords + 3);
    assert_eq!((t[size], t[size + 1]), (4 | 8, 2), "VALUE REQUIRED $NUMBER");
}

#[test]
fn syntax_inherits() {
    let t = table(
        "define verb SHOW
             cliroutine SHOW
             parameter P1, value(type=WHAT)
         define type WHAT
             keyword PROCESS, syntax=SHOW_IMAGE
         define syntax SHOW_IMAGE
             image SHOW
         define syntax OTHER
             noparameters
             disallow P1 and not P2
         ",
    );
    let verb = word(&t, 6);
    assert_eq!(ascic(&t, verb + 5), "", "no image");
    assert_eq!(ascic(&t, verb + 6), "SHOW", "a CLIROUTINE");
    let p1 = word(&t, verb + 12);
    let process = word(&t, word(&t, p1 + 2) + 1);
    let syntax = word(&t, process + 4);
    assert_eq!(ascic(&t, syntax), "SHOW_IMAGE");
    assert_eq!(ascic(&t, syntax + 11), "SHOW", "its image");
    let at = syntax + 11 + 5 + 1;
    assert_eq!((t[at], t[at + 1]), (255, 255), "it inherits both");
    assert_eq!(word(&t, at + 2), 0, "and DISALLOW");
    // OTHER has no parameters, inherits qualifiers, and has a DISALLOW:
    // AND (NAME P1) (NOT (NAME P2)).
    let other = t.windows(6).position(|w| w == b"\x05OTHER").unwrap();
    let at = other + 6 + 2;
    assert_eq!((t[at], t[at + 1]), (0, 255));
    let expr = word(&t, at + 2);
    assert_eq!(&t[expr..expr + 9], b"\x04\x01\x02P1\x03\x01\x02P");
}

#[test]
fn errors_name_the_line() {
    let e = errors("define verb A\n  image A\n  parameter P2");
    assert_eq!(e, ["%CDU-E-INVDEF, t.cld:3: A: parameter P2 must be P1"]);
    let e = errors("define verb A\n  qualifier X, value(type=NOSUCH)");
    assert_eq!(e, ["%CDU-E-INVDEF, t.cld:2: NOSUCH: no such type"]);
    let e = errors("define verb A\n  qualifier X, syntax=NOSUCH");
    assert_eq!(e, ["%CDU-E-INVDEF, t.cld:2: NOSUCH: no such syntax"]);
    let e = errors("define verb A\ndefine verb A");
    assert_eq!(e, ["%CDU-E-INVDEF, t.cld:2: verb A is defined twice"]);
    let e = errors("define verb A\n parameter P1\n parameter P2, value(required)");
    assert_eq!(
        e,
        ["%CDU-E-INVDEF, t.cld:3: A: required parameter P2 after an optional one"]
    );
    let e = errors("define verb A\n  qualifier X, placement=local");
    assert!(e[0].contains("not supported yet"), "{e:?}");
    let e = errors("define verb A\n  routine A_ROUTINE");
    assert!(e[0].contains("ROUTINE is not supported yet"), "{e:?}");
    let e = errors("define verb A\n  image \"A_VERY_LONG_IMAGE_NAME_THAT_GOES_ON_AND_ON\"");
    assert_eq!(
        e,
        ["%CDU-E-INVDEF, t.cld:1: A: an image name is at most 39 characters"]
    );
    let e = errors("define verb A_NAME_THAT_IS_LONGER_THAN_31_CHARS");
    assert!(e[0].contains("longer than 31 characters"), "{e:?}");
    let e = errors("define verb A\n  frobnicate");
    assert_eq!(e, ["%CDU-E-SYNTAX, t.cld:2: FROBNICATE: not a statement"]);
    let e = errors("define verb A\n  prompt=\"x");
    assert_eq!(e, ["%CDU-E-SYNTAX, t.cld:2: a string has no closing quote"]);
}
