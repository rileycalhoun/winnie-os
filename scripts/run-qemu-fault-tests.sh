#!/bin/bash
set -euo pipefail

for feature in \
    test-panic \
    test-invalid-opcode \
    test-general-protection \
    test-page-fault \
    test-double-fault
do
    printf '==> running %s\n' "$feature"
    cargo run --features "$feature"
done
