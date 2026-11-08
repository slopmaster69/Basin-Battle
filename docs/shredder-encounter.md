# Sneed's Shredder: the machine and the coward

Claim before implementation for the agreed Deadmines encounter design.

The machine cycles through a saw sweep, a stamp and rake, and a committed straight grind.
Completed strings build steam pressure. The grind spends that pressure and ends in a generous
stall recovery. No health threshold adds frenzy or a damage multiplier.

Breaking the machine releases a fragile, mobile Sneed. He throws a wrench, retreats and flurries
when cornered. Once per encounter he may reach the wreck and visibly repair it for 4 sec.
Heroes can interrupt or delay that work with ordinary attacks. A completed repair returns the
machine at a third of its health. Reset, wipe and death clear the wreck and transition exactly once.

Implementation scope: a separate machinery simulation module, minimal shared hooks, the exact
Shredder and Sneed content sections, focused lifecycle/combat tests and a real-client recording
scenario. Rhahk'Zor, Gilnid, client patches and other boss content remain their current owners'.

Validation pending: release server build, release workspace tests, repository checks and real-client
showcase footage. Dev-command isolated demonstrations will be labelled as such.
