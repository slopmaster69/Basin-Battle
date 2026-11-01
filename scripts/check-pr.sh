#!/usr/bin/env bash
# The repository rules a pull request can break without any test noticing (AGENTS.md):
# no game files, saves, build output or secrets; client patches numbered once each; a server
# protocol change shipped with its add-on. CI runs it on every pull request; run it yourself
# before you push:
#   scripts/check-pr.sh                 # this branch against origin/main
#   scripts/check-pr.sh <base-ref>      # against another base
# A failure means the branch must change before it merges. A warning is worth a look and a line
# in the pull request saying why it's fine.
set -uo pipefail
cd "$(dirname "$0")/.."

base="${1:-origin/main}"
if ! git rev-parse -q --verify "$base^{commit}" >/dev/null; then
    echo "check-pr: no base '$base' here; run 'git fetch origin' first." >&2
    exit 2
fi
mergebase=$(git merge-base "$base" HEAD) || {
    echo "check-pr: this branch shares no history with '$base'." >&2
    exit 2
}

failures=0
warnings=0
# In GitHub Actions the messages become annotations on the pull request's files.
fail() { # file message
    failures=$((failures + 1))
    if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error file=$1::$2"; else echo "FAIL  $1: $2"; fi
}
warn() {
    warnings=$((warnings + 1))
    if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::warning file=$1::$2"; else echo "warn  $1: $2"; fi
}

# Files this branch adds or changes (renames count by their new name), and the ones it changes.
added=$(git diff --name-only --diff-filter=ACMR "$mergebase" HEAD)
modified=$(git diff --name-only --diff-filter=M "$mergebase" HEAD)

# --- files that must never be committed ----------------------------------------------------------
while IFS= read -r f; do
    [ -n "$f" ] || continue
    lower=$(printf '%s' "$f" | tr '[:upper:]' '[:lower:]')
    case "$lower" in
        *.mpq|*.dbc|*.blp|*.m2|*.adt|*.wmo|*.wdt|*.wdl|*.skin|*.anim)
            fail "$f" "a WoW game file (or extracted from one); these never go in the repository" ;;
        saves/*|*/saves/*.toml)
            fail "$f" "a save file; saves stay on the host's machine" ;;
        logs/*|*.log)
            fail "$f" "a log file" ;;
        target/*|benilla/*|*/target/*)
            fail "$f" "build output or the benilla checkout" ;;
        settings.sh|settings-windows.txt)
            fail "$f" "a local settings file (it holds paths on someone's machine)" ;;
        *.zip|*.exe|*.dll|*.dylib|*.so|*.app/*)
            fail "$f" "a zip or executable; releases build these (.github/workflows/release.yml)" ;;
        .env|*.pem|*.key|*id_rsa*|*id_ed25519*)
            fail "$f" "looks like a secret or a key file" ;;
    esac
    if [ -f "$f" ]; then
        size=$(wc -c <"$f" | tr -d ' ')
        if [ "$size" -gt 5242880 ]; then
            warn "$f" "$((size / 1048576)) MB; large files bloat every clone. Is it needed in the repository?"
        fi
    fi
done <<<"$added"

# --- secrets in the added lines ------------------------------------------------------------------
secret_re='sk-ant-[A-Za-z0-9_-]{20,}|gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----|xox[abprs]-[A-Za-z0-9-]{20,}'
git diff -U0 "$mergebase" HEAD -- . ':(exclude)scripts/check-pr.sh' | awk '
    /^\+\+\+ b\// { file = substr($0, 7); next }
    /^\+/ && !/^\+\+\+/ { print file "\t" substr($0, 2) }
' | grep -E "$secret_re" | cut -f1 | sort -u | while IFS= read -r f; do
    echo "$f"
done >/tmp/check-pr-secrets.$$ || true
while IFS= read -r f; do
    [ -n "$f" ] && fail "$f" "an added line looks like an API key, token or private key; remove it and rotate the key"
done </tmp/check-pr-secrets.$$
rm -f /tmp/check-pr-secrets.$$

# --- client patches --------------------------------------------------------------------------------
# Applied in order on the pinned benilla, so each number is used once, and an existing patch is
# changed only by its author (a new behaviour gets the next free number).
dups=$(ls patches/benilla 2>/dev/null | grep -E '\.patch$' | sed -E 's/^([0-9]+)-.*/\1/' | sort | uniq -d)
for n in $dups; do
    fail "patches/benilla/$n-*.patch" "two patches share the number $n; the later one to merge takes the next free number"
done
for f in $(ls patches/benilla 2>/dev/null | grep -E '\.patch$' | grep -vE '^[0-9]{4}-[A-Za-z0-9._-]+\.patch$'); do
    fail "patches/benilla/$f" "patch names are NNNN-what-it-does.patch"
done
while IFS= read -r f; do
    case "$f" in
        patches/benilla/*.patch)
            warn "$f" "changes an existing client patch; if it isn't yours, add a new patch with the next free number instead" ;;
    esac
done <<<"$modified"

# --- server protocol and the add-ons together ------------------------------------------------------
changed=$(git diff --name-only "$mergebase" HEAD)
if printf '%s\n' "$changed" | grep -q '^crates/htl-server/src/survival/ui\.rs$' &&
    ! printf '%s\n' "$changed" | grep -q '^addon/ElwynnSurvival/'; then
    warn "crates/htl-server/src/survival/ui.rs" "the add-on line formats changed but addon/ElwynnSurvival didn't; change both in one pull request, and keep old fields working"
fi

# --- Lua 5.0 in the add-ons --------------------------------------------------------------------------
# WoW 1.12 runs Lua 5.0: no # length operator, no % operator (math.mod), no ... in a function body
# (use arg). A heuristic on added lines, comments and strings stripped, so only a warning.
git diff -U0 "$mergebase" HEAD -- 'addon/*.lua' | awk '
    /^\+\+\+ b\// { file = substr($0, 7); next }
    /^@@/ { split($3, a, ","); line = substr(a[1], 2) + 0; next }
    /^\+/ && !/^\+\+\+/ {
        code = substr($0, 2)
        sub(/--.*/, "", code)
        gsub(/"([^"\\]|\\.)*"/, "\"\"", code)
        gsub(/'"'"'([^'"'"'\\]|\\.)*'"'"'/, "\"\"", code)
        what = ""
        if (code ~ /#[A-Za-z_(]/) what = "# (use table.getn or string.len)"
        else if (code ~ /[A-Za-z0-9_)\]] *% *[A-Za-z0-9_(]/) what = "% (use math.mod)"
        else if (code ~ /[=,({] *\.\.\.|\.\.\. *[,)}]/ && code !~ /function *[A-Za-z0-9_.:]* *\(/) what = "... outside a parameter list (use arg)"
        if (what != "") print file ":" line "\t" what
        line++
    }
' | while IFS=$'\t' read -r where what; do
    echo "$where	$what"
done >/tmp/check-pr-lua.$$ || true
while IFS=$'\t' read -r where what; do
    [ -n "$where" ] || continue
    file=${where%%:*}
    ln=${where##*:}
    if [ -n "${GITHUB_ACTIONS:-}" ]; then
        warnings=$((warnings + 1))
        echo "::warning file=$file,line=$ln::Lua 5.0 has no $what"
    else
        warn "$where" "Lua 5.0 has no $what"
    fi
done </tmp/check-pr-lua.$$
rm -f /tmp/check-pr-lua.$$

# --- the tools pin the same client revision as the scripts ----------------------------------------
# The add-on harness and the clip census read benilla's own code; a different revision from
# scripts/common.sh's would test or measure against another client than the one people play.
pinned=$(sed -n 's/^BENILLA_REV="\([0-9a-f]*\)"/\1/p' scripts/common.sh)
for toml in tools/*/Cargo.toml; do
    [ -f "$toml" ] || continue
    for rev in $(sed -n 's/.*samwhosung\/benilla", rev = "\([0-9a-f]*\)".*/\1/p' "$toml"); do
        if [ -n "$pinned" ] && [ "$rev" != "$pinned" ]; then
            fail "$toml" "benilla rev $rev, but scripts/common.sh pins $pinned: bump them together"
        fi
    done
done

# --- summary ---------------------------------------------------------------------------------------
n=$(printf '%s\n' "$changed" | grep -c . || true)
echo "check-pr: $n files changed against $base; $failures failures, $warnings warnings."
[ "$failures" -eq 0 ]
