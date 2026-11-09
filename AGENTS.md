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

## Long sessions (agents)

- An agent session that runs long (a coordinator or "POV" session: Desktop, Chandrian, cloud) plans,
  designs and decides. It hands the heavy work to a fresh subagent with a self-contained brief:
  filming and the visual check, lab runs, test sweeps and simulated-player reports, builds, rebases
  and merges, log digging, and reading across many files.
- Why: every action in a long session re-sends its whole history, so the same work costs many times
  more there than in a fresh subagent.
- The brief says the goal, the files, what's already decided, what done looks like, and the standing
  rules (this file's). The subagent reports back briefly (a few hundred words), keeps screenshots
  and image reads to what it needs, and the long session relays that report rather than re-reading
  the subagent's files.
- Routine subagent work can run on a cheaper model; design work keeps the strongest.
- Design context lives in `docs/` and this file, so a fresh subagent works from the same vision.

## Branches, commits and pull requests

- Branch from the latest `main`: `git switch -c <who>/<what>`, e.g. `claude/ledger-ui-tooltips`,
  `dave/siege-balance`.
- Keep a branch to one purpose. A pull request a reviewer can read in ten minutes gets merged the
  same day; a sprawling one rots while `main` moves under it.
- Start from the latest `main`. A pull request's checks are only that it builds (Linux, Windows,
  macOS) and merges into `main` without a conflict; bring it up to `main`
  (`git pull --rebase origin main`) when "Merges cleanly with main" fails, not after every merge.
  The tests run on `main` after each merge: run them yourself before pushing
  (`cargo test --workspace`, `scripts/check-addon.sh`), or with "Run workflow" on the CI workflow.
- Commit messages: a short summary line of what changes for the player or the code ("A left click
  no longer runs the hero forward"), then why, if it isn't obvious.
- Open a **draft** pull request early to show what you're on. A draft means the work isn't done
  yet, or it waits on a decision; it never waits for someone to play it. Until the game is
  released, `main` is where it gets played: once the checks pass, mark it ready and turn on
  auto-merge, and whoever plays next tries it there.
- What nobody has seen in the real client yet (anything an agent without a display or a WoW
  install can't watch) goes under "not tested" in the pull request, and the pull request gets
  the `needs playtest` label. Whoever plays it on `main` takes the label off, or opens an issue
  for what's wrong.
- One exception, until the Mac client check (#223) runs on pull requests: a pull request with a
  client patch is ready only once the client has built with the whole series. A patch that
  doesn't apply or build stops everyone's game from starting.
- A big change everyone will see or feel (a new look, a slower frame) comes with a way to turn
  it off, or in a pull request of its own, so one revert takes `main` back.
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

Run what covers the files you changed. A pull request isn't ready until these pass, except what
can't be run where you are (the visual check, a script on another system): list that as not
tested and add the `needs playtest` label, rather than holding the pull request for it. Building
the client for a client patch isn't one of these: that still holds the pull request.

| You changed | Run |
| --- | --- |
| Anything in `crates/` or `content/` | `cargo build --release -p htl-server` and `cargo test --release` |
| An add-on (`addon/`) | `scripts/check-addon.sh` (no Lua errors) |
| A client patch (`patches/benilla/`) | Apply the whole series on a clean benilla checkout (`scripts/play.sh` does it) and build the client |
| A script (`scripts/`, `*.command`, `*.bat`) | Run it on the system it's for |
| An ability: what it does, its effects, animation, icon or key (`content/`, `crates/htl-survival/src/sim/`, the add-on's bar, `patches/benilla/`) | The [visual check](docs/visual-check.md): its scenario in the real client, the screenshots or video attached to the pull request (add or adjust the scenario in `tools/visual-check/`). With no client where you are, it's filmed on `main` after the merge, under `needs playtest` |
| Anything | `scripts/check-pr.sh` (the repository rules, against `origin/main`) |

Tests that need the WoW 1.12.1 data are marked `#[ignore]`; run them locally when you touch what
they cover. CI runs the rest on every pull request.

The end-to-end tests (`crates/htl-server/tests/`) play against the server in real time, so on a
busy machine a timing test can fail on its own (a parry window missed, a flash not seen in time).
Rerun that test alone before deciding your change broke it. In CI they run in their own job,
"End-to-end, real time", which retries a failure twice and doesn't block a merge by itself; if
it lists a test as flaky or failing, open an issue for it rather than ignoring it.

## Words the player reads

Everything a player reads in Elwynn Survival (window text, buttons, hints, tooltips, chat
messages, announcements, the journal, every `text` in `content/`) follows
[docs/writing-style.md](docs/writing-style.md): WoW's interface grammar for what does something,
short flavor for what has a story, numbers as numerals, and one word list.

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
