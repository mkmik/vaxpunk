//! DEC's own require files, STARLET.R64, LIB.R64, TPAMAC.REQ, CLIMAC.REQ
//! and ARCH_DEFS.REQ from the BLISS-64 kit, compiled as libraries and
//! required, and their macros used (t8y:/host/vbliss#dec-libraries). The
//! files are read where $VBLISS_DEC says, by default
//! ~/Library/Caches/vaxpunk/bliss-oracle/syslib; DEC's licence keeps them
//! out of the repository.
//!
//!     cargo test -p vbliss --test dec -- --ignored

use std::path::{Path, PathBuf};

fn syslib() -> PathBuf {
    std::env::var("VBLISS_DEC")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap())
                .join("Library/Caches/vaxpunk/bliss-oracle/syslib")
        })
}

/// The assembly of a module that must compile with no warning.
fn asm(source: &str) -> String {
    let opts = vbliss::Options {
        include: vec![syslib()],
        ..vbliss::Options::default()
    };
    let out = vbliss::translate(Path::new("dec.b64"), source, &opts);
    let complaints: Vec<_> = out.diags.iter().filter(|d| d.sev != 'I').collect();
    assert!(complaints.is_empty(), "{complaints:?}");
    out.asm.unwrap()
}

#[test]
#[ignore = "needs DEC's require files, which aren't in the repository"]
fn starlet_and_lib() {
    let a = asm("MODULE T1 (MAIN = M) =
BEGIN
LIBRARY 'SYS$LIBRARY:STARLET';
LIBRARY 'SYS$LIBRARY:LIB';
OWN
    FAB : $FAB(FNM = 'X.DAT', FAC = GET),
    RAB : $RAB(FAB = FAB),
    XP : $XABPRO(),
    CHAN : WORD;
ROUTINE M =
    BEGIN
    LOCAL S;
    S = $ASSIGN(DEVNAM = %ASCID 'TT', CHAN = CHAN);
    S = $QIOW(CHAN = .CHAN, FUNC = IO$_WRITEVBLK, P1 = 0, P2 = 0);
    S = $OPEN(FAB = FAB);
    $FAB_INIT(FAB = FAB, FNM = 'Y.DAT');
    .S
    END;
END
ELUDOM
");
    for call in ["SYS$ASSIGN", "SYS$QIOW", "SYS$OPEN"] {
        assert!(a.contains(&format!("bl      {call}")), "{call}:\n{a}");
    }
    // The FAB's first byte is its block id, FAB$C_BID, 3; the RAB's, 1.
    assert!(a.contains("FAB:\n        .BYTE  3"), "{a}");
    assert!(a.contains("RAB:\n        .BYTE  1"), "{a}");
}

#[test]
#[ignore = "needs DEC's require files, which aren't in the repository"]
fn tparse_tables() {
    let a = asm("MODULE T2 (MAIN = M) =
BEGIN
LIBRARY 'SYS$LIBRARY:STARLET';
REQUIRE 'SYS$LIBRARY:TPAMAC';
REQUIRE 'SYS$LIBRARY:CLIMAC';
REQUIRE 'SYS$LIBRARY:ARCH_DEFS';
OWN
    OPTS;
$INIT_STATE (WORD_STB, WORD_KTB);
$STATE (,
    ('HELLO',,, 1, OPTS),
    ('BYE',,, 2, OPTS)
    );
$STATE (,
    (TPA$_EOS, TPA$_EXIT)
    );
ROUTINE M =
    BEGIN
    EXTERNAL ROUTINE LIB$TPARSE;
    LOCAL B : $BBLOCK[TPA$K_LENGTH0];
    %IF ALPHA %THEN LIB$TPARSE(B, WORD_STB, WORD_KTB) %FI
    END;
END
ELUDOM
");
    // The keywords, each ended by 255, and the transitions, packed by
    // ALIGN(0): a word of type and flags, then the mask's address and the
    // mask.
    assert!(
        a.contains(".BYTE   72, 69, 76, 76, 79\n        .BYTE  255"),
        "{a}"
    );
    assert!(a.contains("WORD_STB::\nTPA$TYPE:\n        .WORD"), "{a}");
    assert!(
        a.contains("TPA$ADDR:\n        .LONG  OPTS\nTPA$MASK:"),
        "{a}"
    );
}
