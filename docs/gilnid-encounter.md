# Gilnid: trouble above the furnace

Status: implementation claim; this document describes the agreed design, not completed behavior.

Codex owns this encounter, following the independent Codex and Claude design comparison.
Claude's monster yells and Rhahk'Zor engine are already on main.

The fight uses low, visible perches over the Foundry pit. Engineers throw dynamite with clear
fuses and arcs. A ranged hit brings an engineer down through a continuous fall, followed by
a stunned landing and melee combat. A reachable support offers a melee alternative, and a
bounded automatic descent prevents loadout restrictions. The first fall is demonstrated by
the Foundry's overflow. Workers are finite; they are not a health-threshold summon wave.

Once the workers are dealt with, Gilnid operates the furnace and commits to a visible molten
discharge with a dry escape route. He then finishes the fight directly, without a generic
low-health damage multiplier.

Implementation scope: new encounter code with minimal hooks into existing simulation,
content definitions, focused lifecycle and combat tests, and a real-client recording scenario.
Preserve ordinary ledge safety, the existing ogre encounter, enemy variety, and the HumanMale
VanCleef animation correction. No client animation patches are claimed here.

Acceptance includes solo and party targeting, one hit triggering one fall, melee support and
automatic fallbacks, safe landing, no duplicate workers on reset, and a full fight with optional
floor markers disabled. The showcase must identify any dev-controlled demonstration footage.
