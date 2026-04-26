#!/usr/bin/env bash
# Regression test: exercise every CLI recipe the v1 hearth skills depend on.
# Requires: jq, built `hearth` binary (HEARTH_BIN or src-tauri/target/release/hearth).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

HEARTH="${HEARTH_BIN:-$REPO_ROOT/src-tauri/target/release/hearth}"
if [[ ! -x "$HEARTH" ]]; then
  echo "hearth binary not found at: $HEARTH" >&2
  echo "Build first: (cd src-tauri && cargo build --release -p hearth-cli)" >&2
  echo "Or export HEARTH_BIN=<path-to-hearth>" >&2
  exit 1
fi

command -v jq >/dev/null 2>&1 || { echo "jq is required" >&2; exit 1; }

TMP="$(mktemp -d -t hearth-smoke.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

export HEARTH_DB="$TMP/smoke.db"
SCANDIR="$TMP/workspace"
mkdir -p "$SCANDIR/alpha" "$SCANDIR/beta"

assert_ok() {
  local label="$1" out="$2"
  if ! echo "$out" | jq -e '.ok == true' >/dev/null 2>&1; then
    echo "FAIL: $label" >&2
    echo "$out" >&2
    exit 1
  fi
  echo "PASS: $label"
}

# Portable "tomorrow"/"yesterday" — BSD date on macOS vs GNU date on Linux.
date_rel() {
  if date -v+0d +%Y-%m-%d >/dev/null 2>&1; then
    date -v"$1"d +%Y-%m-%d
  else
    case "$1" in
      +1) date -d 'tomorrow' +%Y-%m-%d ;;
      -1) date -d 'yesterday' +%Y-%m-%d ;;
      *) date -d "$1 days" +%Y-%m-%d ;;
    esac
  fi
}

TODAY="$(date +%Y-%m-%d)"
YESTERDAY="$(date_rel -1)"

echo "== migrate =="
assert_ok "db migrate" "$("$HEARTH" db migrate)"

echo "== seed fixture data =="
P1=$("$HEARTH" project create "Alpha Dashboard" --priority P0 | jq '.data.id')
P2=$("$HEARTH" project create "Side Experiment" --priority P2 | jq '.data.id')
"$HEARTH" memo create "Alpha Dashboard sprint notes" >/dev/null
"$HEARTH" memo create "Side Experiment notebook"     >/dev/null
"$HEARTH" memo create "Unrelated thought"             >/dev/null
"$HEARTH" schedule create "$TODAY"     --time 10:00 --description "team sync"       >/dev/null
"$HEARTH" schedule create "$YESTERDAY"              --description "overdue cleanup" >/dev/null
echo "   seeded projects #$P1 (P0), #$P2 (P2), 3 memos, 2 schedules"

echo "== hearth integrated router recipes =="
assert_ok "project list" "$("$HEARTH" project list)"
assert_ok "search project" "$("$HEARTH" search "Alpha" --scope project --limit 5)"
ROUTER_PROJECT=$("$HEARTH" project create "Router Skill Project" --priority P2 --evaluation "created by integrated skill smoke" | jq '.data.id')
[[ -n "$ROUTER_PROJECT" && "$ROUTER_PROJECT" != "null" ]] || { echo "router project id missing" >&2; exit 1; }
assert_ok "project update" "$("$HEARTH" project update "$ROUTER_PROJECT" --priority P1)"
ROUTER_MEMO=$("$HEARTH" memo create "Router Skill Project follow-up memo" --color yellow --project "$ROUTER_PROJECT" | jq '.data.id')
[[ -n "$ROUTER_MEMO" && "$ROUTER_MEMO" != "null" ]] || { echo "router memo id missing" >&2; exit 1; }
assert_ok "memo update" "$("$HEARTH" memo update "$ROUTER_MEMO" --color blue)"
ROUTER_SCHEDULE=$("$HEARTH" schedule create "$TODAY" --time 15:00 --description "router skill smoke" --notes "integrated skill recipe" | jq '.data.id')
[[ -n "$ROUTER_SCHEDULE" && "$ROUTER_SCHEDULE" != "null" ]] || { echo "router schedule id missing" >&2; exit 1; }
assert_ok "schedule list day" "$("$HEARTH" schedule list --from "$TODAY" --to "$TODAY")"
assert_ok "schedule update" "$("$HEARTH" schedule update "$ROUTER_SCHEDULE" --time 15:30 --remind-start true)"
echo "PASS: integrated router create/update recipes"

echo "== hearth today/overdue read recipes =="
assert_ok "today"   "$("$HEARTH" today)"
assert_ok "overdue" "$("$HEARTH" overdue)"

echo "== hearth project scan read + mutation recipes =="
SCAN=$("$HEARTH" project scan "$SCANDIR")
assert_ok "project scan" "$SCAN"
UNREG=$(echo "$SCAN" | jq '[.data[] | select(.already_registered == false)] | length')
[[ "$UNREG" -eq 2 ]] || { echo "expected 2 unregistered, got $UNREG" >&2; echo "$SCAN" >&2; exit 1; }
echo "PASS: project scan found 2 unregistered candidates"

BEFORE=$("$HEARTH" project list | jq '.data | length')
for sub in alpha beta; do
  assert_ok "project create $sub" "$("$HEARTH" project create "$sub" --priority P2 --path "$SCANDIR/$sub")"
done
AFTER=$("$HEARTH" project list | jq '.data | length')
[[ $((BEFORE + 2)) -eq "$AFTER" ]] || { echo "expected +2 projects ($BEFORE -> $AFTER)" >&2; exit 1; }
echo "PASS: project count $BEFORE -> $AFTER"

echo "== hearth memo organize read + mutation recipes =="
MEMO_LIST=$("$HEARTH" memo list)
assert_ok "memo list" "$MEMO_LIST"
MID=$(echo "$MEMO_LIST" | jq -r '.data[] | select(.content == "Alpha Dashboard sprint notes") | .id')
[[ -n "$MID" && "$MID" != "null" ]] || { echo "seeded memo not found" >&2; exit 1; }
assert_ok "memo update --project" "$("$HEARTH" memo update "$MID" --project "$P1")"
LINKED=$("$HEARTH" memo get "$MID" | jq -r '.data.project_id // "null"')
[[ "$LINKED" == "$P1" ]] || { echo "memo project_id expected $P1, got $LINKED" >&2; exit 1; }
echo "PASS: memo #$MID linked to project #$P1"

echo "== undo round-trip =="
assert_ok "undo" "$("$HEARTH" undo)"
LINKED2=$("$HEARTH" memo get "$MID" | jq -r '.data.project_id // "null"')
[[ "$LINKED2" != "$P1" ]] || { echo "undo did not revert memo project_id" >&2; exit 1; }
echo "PASS: undo reverted memo #$MID link (now project_id=$LINKED2)"

echo "== log show (close-phase recipe) =="
assert_ok "log show --limit 10" "$("$HEARTH" log show --limit 10)"

echo
echo "ALL GOOD"
