# Names and file specifications

On disk a name is "NAME.TYPE" (the dot always present, either part may be
empty); the version is separate. Code: `crates/ods-core/src/name.rs`.

## ODS-2

Name and type up to 39 characters each, from `A-Z 0-9 $ - _`. Input is
uppercased.

## ODS-5

Name and type together up to 236 bytes (ISO Latin-1), or 118 UCS-2
characters. Any character except C0 controls (0x00-0x1F) and
`" * \ : < > / ? |`. Case is preserved as first created (all versions of a
name share the case of the first) and ignored when comparing and matching.

In a specification, `^` escapes a character:

| Escape | Meaning |
| --- | --- |
| `^_` or `^` space | space |
| `^.` `^,` `^;` `^[` `^]` `^%` `^^` `^&` | the character itself |
| `^hh` | the byte with hex value hh |
| `^Uhhhh` | UCS-2 character (`ods` accepts those that fit Latin-1) |

Dots other than the last one in a name are literal. `ods` prints names with
the same escapes, so what it prints parses back.

## Specifications

`[DIR.SUB]NAME.TYPE;VERSION`, every part optional:

- A device (`DKA0:`) is ignored. `<...>` works like `[...]`.
- `[000000]` is the MFD; `[000000.A]` and `[.A]` mean `[A]`.
- Version: none, `;` or `;0` is the highest; `;n` exactly n (1-32767);
  `;-n` n below the highest; `;*` all. `NAME.TYPE.n` also gives a version,
  when what follows the last dot is a number.
- Wildcards: `*` any characters, `%` or `?` exactly one, matched against
  name and type separately, case-blind; `...` in a directory means that
  directory and everything below.

`ods` also accepts `/dir/sub/name.type;ver` for scripting: a trailing `/`
names a directory, and characters VMS syntax gives a meaning to are
escaped on the way (`path.rs` in ods-image).
