#!/bin/sh
set -eu

if [ ! -f Cargo.lock ]; then
  echo 'release manifest refused: Cargo.lock is missing' >&2
  exit 1
fi

find . -type f \
  ! -path './.git/*' \
  ! -path './target/*' \
  ! -name 'RELEASE-MANIFEST.sha256' \
  -print \
  | LC_ALL=C sort \
  | while IFS= read -r file; do
      sha256sum "$file"
    done
