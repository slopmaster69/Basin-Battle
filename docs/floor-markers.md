# Optional floor markers

Implementation claim: the agreed Deadmines encounter designs use animation, sound and
environmental effects as the default warning language. Floor markers remain available in
Controls for players who want them, but start hidden until explicitly enabled.

Scope: the add-on setting and its toggle, the add-on/server version notice, and the existing
add-on checks. Spell effects and actual environmental warnings remain visible. No server
damage shapes, combat timing, encounter code or client patches are part of this change.

Validation will check both the unset default and explicit shown/hidden choices, including
reload persistence. The repository's required build and test checks still apply.
