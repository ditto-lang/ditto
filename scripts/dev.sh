#!/usr/bin/env bash

# Helper script for development
#
#   $ ./scripts/dev make check
#   $ ./scrtipts/dev fmt test

set -euo pipefail

usage() {
	cat >/dev/stderr <<-EOF
		Usage: ${0} CRATE [SUBCOMMAND] [ARGS...]
	EOF
}

main() {
	if [[ "$#" -lt 1 ]]; then
		usage
		exit 1
	fi
	local -r crate="$1"
	local -r subcommand="${2:-check}"
	local -r args="${*:3}"
	set -x
	cargo watch -cq -x "$subcommand -p ditto-$crate -- $args"
}

main "$@"
