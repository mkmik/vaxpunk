#!/bin/sh
# Makes the RMS fixtures in ods/fixtures/rms on OpenVMS in AXPbox: puts
# the inputs here on a scratch volume, STAGE, as DKA100:, runs MAKE.COM
# there (vms/run-vms.py), and copies what it made back: the files, their
# FDL and /CHECK reports as text, and each KEYn.SEQ dump as a .dump file,
# a line per record. Needs ods/vms set up (vms/setup.sh) and a release
# build of ods-cli.
#
#   fixtures/rms/make/make.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
vms=${VMSDIR:-$here/../../../vms}
ods=${ODS:-$here/../../../../target/release/ods}
out=$here/..

cd "$vms"
rm -f disk2.img
"$ods" init disk2.img --size 200M --label STAGE >/dev/null 2>&1
"$ods" mkdir disk2.img '[OUT]'
for f in "$here"/*.txt "$here"/*.fdl "$here"/make.com; do
	"$ods" copy-in disk2.img "$f" "[000000]$(basename "$f" | tr a-z A-Z)" --mode lines-to-records >/dev/null 2>&1
done
cmd=$(mktemp)
printf 'MOUNT/SYSTEM DKA100: STAGE\nSET DEFAULT DKA100:[OUT]\n@DKA100:[000000]MAKE.COM\nSET DEFAULT DKA100:[000000]\nDISMOUNT DKA100:\n' >"$cmd"
python3 run-vms.py "$cmd" "$here/make.log"
rm -f "$cmd"
for f in $("$ods" dir disk2.img '[OUT]*.*;' | grep -o '^[A-Z0-9_]*\.[A-Z]*;[0-9]*'); do
	n=$(echo "${f%;*}" | tr A-Z a-z)
	case $n in
	*.fdl | *.chk) "$ods" copy-out disk2.img "[OUT]$f" "$out/$n" --mode records-to-lines >/dev/null 2>&1 ;;
	*_key*.seq) "$ods" copy-out disk2.img "[OUT]$f" - --mode binary | python3 "$here/dump.py" >"$out/${n%.seq}.dump" ;;
	*.txt) ;;
	*) "$ods" copy-out disk2.img "[OUT]$f" "$out/$n" --mode binary >/dev/null 2>&1 ;;
	esac
done
