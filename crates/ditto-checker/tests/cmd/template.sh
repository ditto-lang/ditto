#!/usr/bin/env bash

usage() {
	cat >/dev/stderr <<-EOF
		Usage: ${0} test_name
	EOF
}

if [[ $# -lt 1 ]]; then
	usage
	exit 1
fi

set -x

mkdir "$1"
tee "$1/test.toml" <<EOF
bin.name = "ditto-checker-testbin-check-module"
args = ["$1.ditto"]
fs.sandbox = true
EOF
tee "$1/test.stdin" <<EOF
module Test exports (..);
EOF
tee "$1/test.stdout" <<EOF
{}
EOF
touch "$1/test.stderr"
mkdir "$1/test.in"
tee "$1/test.in/Dep.ditto" <<EOF
module Dep exports (..);
EOF
mkdir "$1/test.out"
tee "$1/test.out/Dep.ast" <<EOF
{}
EOF
