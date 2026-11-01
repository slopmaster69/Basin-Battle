# Working on this repo: people and agents together

Several people and several AI agents (Claude, Codex and others) work on this repository at the
same time, often on neighbouring files. These rules keep everyone building on the latest working
version instead of over each other. They apply to humans too. Read the [README](README.md) for
what the project is and how to run it.

## The short version

1. **Sync before you start, and again before you push.** `git fetch --tags && git pull --rebase`.
2. **Work on a branch, in small pieces.** One change per branch and pull request.
3. **Test it, then push it.** Run the checks below; push working, tested commits often, at least
   every hour of work, so others see what you're doing.
4. **Never push to `main` directly, and never force-push a shared branch.** Merge through a pull
   request once the checks pass.
5. **Say what you're taking on** before you start something bigger than an hour (an issue or a
   draft pull request), and look for someone else's claim first.
6. **Never commit** WoW game files, saves, logs, build output, settings files or secrets.

## Starting a session

Every session, before changing anything:

```sh
git fetch --tags --prune
git switch main && git pull --rebase
git log --oneline -15                   # what landed since you last looked
gh release list --limit 3               # the latest release, if you have the GitHub CLI
gh pr list                              # what others are working on right now
```

- Check the latest release (`git describe --tags --abbrev=0`). Releases are known-good points;
  if `main` is broken, compare against the last release before assuming your change broke it.
- Read the open pull requests and issues that touch the area you're about to work in. If someone
  is already on it, coordinate on their pull request rather than starting a second copy.
- Agents: re-read any file right before editing it. Never write a file back from a copy you read
  earlier in a long session; someone may have changed it since. Prefer small, exact edits over
  rewriting whole files.

## Branches, commits and pull requests

- Branch from the latest `main`: `git switch -c <who>/<what>`, e.g. `claude/ledger-ui-tooltips`,
  `dave/siege-balance`.
- Keep a branch to one purpose. A pull request a reviewer can read in ten minutes gets merged the
  same day; a sprawling one rots while `main` moves under it.
- Rebase on `main` before opening the pull request and again before it merges
  (`git pull --rebase origin main`), and rerun the checks after rebasing.
- Commit messages: a short summary line of what changes for the player or the code ("A left click
  no longer runs the hero forward"), then why, if it isn't obvious.
- Open a **draft** pull request early to show what you're on; mark it ready when the checks pass.
- In the pull request say what you tested and how (which checks, what you played), and what you
  didn't.
- Once it's ready, turn on **auto-merge** (`gh pr merge --auto --squash`, or the button on the
  pull request): it merges by itself when the required checks pass, so nobody has to watch it.

## Reviews

Every pull request gets two automatic looks besides CI:

- **PR rules** (`scripts/check-pr.sh`, seconds): fails on a game file, save, log, build output,
  settings file, zip, executable or secret in the branch, or two client patches with the same
  number; notes on the files flag an edited patch, an add-on line-format change without the
  add-on, and Lua the 1.12 client can't run. Run it yourself before you push.
- **Claude review**, once the pull request is out of draft and after each push to it: comments on
  bugs and on breaking other people's work, nothing else. It never approves or merges.

Read the review comments and either fix what they point at or answer why it's fine, in a reply on
the comment. A reviewer can be wrong; a comment nobody answered is the one that gets missed.

## Checks before every push

Run what covers the files you changed. A pull request isn't ready until these pass:

| You changed | Run |
| --- | --- |
| Anything in `crates/` or `content/` | `cargo build --release -p htl-server` and `cargo test --release` |
| An add-on (`addon/`) | `scripts/check-addon.sh` (no Lua errors) |
| A client patch (`patches/benilla/`) | Apply the whole series on a clean benilla checkout (`scripts/play.sh` does it) and build the client |
| A script (`scripts/`, `*.command`, `*.bat`) | Run it on the system it's for |
| Anything | `scripts/check-pr.sh` (the repository rules, against `origin/main`) |

Tests that need the WoW 1.12.1 data are marked `#[ignore]`; run them locally when you touch what
they cover. CI runs the rest on every pull request.

The end-to-end tests (`crates/htl-server/tests/`) play against the server in real time, so on a
busy machine a timing test can fail on its own (a parry window missed, a flash not seen in time).
Rerun that test alone before deciding your change broke it. In CI they run in their own job,
"End-to-end, real time", which retries a failure twice and doesn't block a merge by itself; if
it lists a test as flaky or failing, open an issue for it rather than ignoring it.

## Areas that need extra care

- **Client patches** (`patches/benilla/NNNN-*.patch`) apply in order on the pinned benilla
  revision (`scripts/common.sh`). Add new behaviour as a new patch with the next free number
  rather than editing someone else's patch. If two branches both add patch `0013`, the second to
  merge renumbers. Any change here means every player's client rebuilds, so test the full series.
- **Server protocol and the add-ons together.** The add-on line formats are listed in
  `crates/htl-server/src/survival/ui.rs` (and Basin Battle's equivalent). Change the server and
  the add-on in the same pull request, and keep old fields working where you can: players may run
  an older add-on against a newer server for a while.
  When an add-on change matters to players, bump the add-on's `VERSION` and the server's
  `ADDON_NEWEST` (`survival/ui.rs`) together: a hero on an older add-on is then told how to update.
- **Saves.** Changes to what's saved must still load older save files. Never commit `saves/`.
- **Content tuning** (`content/*.toml`) is the easiest place to collide. Keep tuning changes in
  their own small pull requests, with the reason in a comment beside the number.
- **The add-on's main chunk is near Lua 5.0's limit of 200 locals.** Group new state in tables,
  or put a new feature in its own file listed in the `.toc` (as `Ledger.lua` does).
- **Windows builds.** Releases carry the Windows zip; don't commit zips or executables.

## Never commit

WoW game files or anything extracted from them (MPQ, DBC, BLP, M2, ADT, WMO), `saves/`, `logs/`,
`target/`, the `benilla/` checkout, `settings.sh`, `settings-windows.txt`, zips, executables,
API keys or passwords. `.gitignore` covers most of these; check `git status` before every commit.

## Releases

- `main` should always build and pass its checks. A release is a tag on a `main` commit that has
  also been played (`v0.4.0`), with notes on what changed for players.
- Pushing a tag builds the Windows zip and attaches it to the GitHub release (see
  `.github/workflows/release.yml`).
- If a merge breaks `main`, revert it first (`git revert <commit>`) and fix it on a branch;
  don't pile fixes on a broken `main`.

## When things collide

- **Merge conflict:** rebase, resolve keeping both sides' intent, rerun the checks. If you can't
  tell what the other change meant, ask on its pull request rather than guessing.
- **Two people on the same thing:** the one with the earlier claim or open pull request keeps it;
  the other reviews it or picks something else.
- **An agent that finds its files changed under it** stops, re-reads, and re-applies its edit to
  the new version. It never overwrites the newer file with its own older copy.
