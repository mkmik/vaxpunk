+++
title = "SEARCH"
summary = "Finds the lines of files that contain given strings, VMS's grep"
aka = ["SEARCH", "SEARCH.EXE", "SEARCH/MATCH", "SEARCH/WINDOW", "grep"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-n-z/", "https://wasd.vsm.com.au/htbin/helpgate/HELP/SEARCH"]

[[needs]]
on = "//rms/sequential"
why = "Reads each file's records"

[[needs]]
on = "//rms/wildcards"
why = "Searches every file a wildcard or list names"

[[needs]]
on = "//rtl/cli"
why = "Its parameters and qualifiers"

[[needs]]
on = "//rtl/librtl"
why = "Writes the matching lines"
+++

`SEARCH files strings` writes the records of files that contain one of
the strings, each file's name first. `/MATCH=AND|OR|NOR|NAND` combines
several strings, `/EXACT` respects case, `/WINDOW=(m,n)` shows lines
around a match, `/NUMBERS` their record numbers, `/STATISTICS` what it
read, `/KEY` limits the search to a field, and `/WILDCARD_MATCHING`
allows `*` and `%` in a string. Its status says whether anything
matched (`%SEARCH-I-NOMATCHES`), so procedures use it to test files. It
has no regular expressions.

vaxpunk has no `SEARCH`. DCL's `F$LOCATE` and TPU's `SEARCH` cover
some of what it is used for.
