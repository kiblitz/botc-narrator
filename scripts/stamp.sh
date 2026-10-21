#!/usr/bin/env bash
# Vouch for a passing scripts/check.sh: writes the stamp scripts/deploy.sh reads
# to know the checks passed, and on what.
#
# Usage: scripts/stamp.sh clear          (first thing check.sh does)
#        scripts/stamp.sh <ran|skipped>  (last, once everything passed; the
#                                         argument is whether the root deploy
#                                         tests ran on the box)
#
# Under target/ because it describes this machine, not the repository, and so
# is never committed. Cleared first, so a run that dies partway cannot leave
# yesterday's pass standing.
set -euo pipefail
cd "$(dirname "$0")/.."

readonly STAMP=target/local-check/passed

case "${1:-}" in
clear)
	rm -f "$STAMP"
	exit 0
	;;
ran | skipped) box_tests=$1 ;;
*)
	echo "usage: scripts/stamp.sh clear | ran | skipped" >&2
	exit 2
	;;
esac

# The tree rather than the commit, so that merging a checked branch unchanged
# keeps the stamp good; `clean`, because a dirty worktree is not what its HEAD
# says it is.
mkdir -p "$(dirname "$STAMP")"
clean=yes
test -z "$(git status --porcelain)" || clean=no
cat >"$STAMP" <<STAMPED
tree $(git rev-parse 'HEAD^{tree}')
commit $(git rev-parse HEAD)
clean $clean
box_tests $box_tests
when $(date -u +%Y-%m-%dT%H:%M:%SZ)
STAMPED

printf '\nall checks passed\n'
test "$box_tests" = ran ||
	printf '\n!!!! %s\n' "...except the root deploy tests, which were skipped -- see above" >&2
