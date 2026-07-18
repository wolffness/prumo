#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

TEST_BIN="${TMPDIR:-/tmp}/tuxedo-agent-tests-$$"
trap 'rm -f "$TEST_BIN"' EXIT

swiftc -warnings-as-errors \
    packaging/agent/Paths.swift \
    packaging/agent/Summary.swift \
    packaging/agent/TagAutocomplete.swift \
    packaging/agent/tests/main.swift \
    -o "$TEST_BIN"

"$TEST_BIN"
