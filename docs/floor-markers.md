# Optional floor markers

Implementation prepared; required validation is pending. The agreed encounter designs use animation, sound and
environmental effects as the default warning language. Floor markers remain available in
Controls for players who want them, but start hidden until explicitly enabled.

Scope: the add-on setting and its toggle, the add-on/server version notice, and the existing
add-on checks. Spell effects and actual environmental warnings remain visible. No server
damage shapes, combat timing, encounter code or client patches are part of this change.

An unset preference or saved `noMarks = 1` hides overlays. Explicitly enabling them in
Controls saves `noMarks = false`; toggling them off saves `1` and clears visible overlays.
Add-on version 60 matches the server's update notice.

Pending validation: the existing add-on harness, server build, release tests, repository rules,
and a real-client toggle/reload check. Code inspection is not a substitute for these checks.
