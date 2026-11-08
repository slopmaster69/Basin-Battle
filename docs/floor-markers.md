# Optional floor markers

The agreed encounter designs use animation, sound and
environmental effects as the default warning language. Floor markers remain available in
Controls for players who want them, but start hidden until explicitly enabled.

Scope: the add-on setting and its toggle, the add-on/server version notice, and the existing
add-on checks. Spell effects and actual environmental warnings remain visible. No server
damage shapes, combat timing, encounter code or client patches are part of this change.

An unset preference or saved `noMarks = 1` hides overlays. Explicitly enabling them in
Controls saves `noMarks = false`; toggling them off saves `1` and clears visible overlays.
Add-on version 60 matches the server's update notice.

Automated validation passed: `scripts/check-addon.sh` reported zero Lua errors for both add-ons,
`cargo build --release -p htl-server` succeeded, and `cargo test --release --no-fail-fast`
passed 1,114 tests (19 ignored). Repository rules passed with no warnings. On Windows GNU,
the add-on harness needed a local import library for the native UCRT timezone functions;
no repository source or shared toolchain was changed for that workaround.

A focused harness verified absent settings, an unset preference, saved `1`, explicit `false`,
the Controls labels and toggle, suppression of TG/TA/MK overlays, and immediate clearing.
Real-client rendering and persistence through an actual save and reload remain untested.
Use a fresh temporary client home for that check; the player's settings must stay untouched.
