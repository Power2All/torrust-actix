#!/bin/sh
# Linux/macOS counterpart of version.bat: sets the torrust-actix version everywhere it is pinned.
set -eu

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>"
    echo "  e.g. $0 4.2.11"
    exit 1
fi

TA_VER=$1
if ! printf '%s\n' "$TA_VER" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "Error: \"$TA_VER\" is not a valid X.Y.Z version."
    exit 1
fi

cd "$(dirname "$0")"
echo "Setting torrust-actix version to $TA_VER ..."

# s <file> <perl substitution>: rewrites <file> in place, reporting whether anything changed.
s() {
    if [ ! -f "$1" ]; then echo "  [skip] $1 (not found)"; return; fi
    if TA_VER=$TA_VER perl -0777 -i -pe "\$c = ($2); END { exit(\$c ? 0 : 1) }" "$1"; then
        echo "  [ok]   $1"
    else
        echo "  [warn] $1 (no version match)"
    fi
}

s Cargo.toml        's/^version = "\K\d+\.\d+\.\d+(?=")/$ENV{TA_VER}/mg'
s Cargo.lock        's/(name = "torrust-actix"\r?\nversion = ")\d+\.\d+\.\d+/${1}$ENV{TA_VER}/'
s docker/build.bat  's/torrust-actix:v\K\d+\.\d+\.\d+/$ENV{TA_VER}/g'
s src/openapi.json  's/"version": "v\K\d+\.\d+\.\d+/$ENV{TA_VER}/'

echo
echo "Done. Review the changes with: git diff"
