# Deadmines after the collapse

Codex claims the finished destroyed state of the entire dungeon, at Corey's request.
This is the environment after the heroes and Rhahk'Zor have already passed through it.
The boss fight and a new cinematic spectator system are outside this first change.

## Intended result

- Every tunnel and major room has a composed aftermath: large cave-ins, persistent rubble,
  broken supports and wrecked machinery or cargo appropriate to that room.
- Destruction changes the silhouette of the space. Dust is atmosphere around real solids,
  not the sole representation of a collapse.
- VanCleef's ship is gone from the final chamber. Rendering and collision must agree; hiding
  it behind a pile of props while leaving the intact ship underneath is not acceptance.
- The state is deterministic and inspectable without replaying a boss fight. A scoped preview
  control can select the final state and restore the intact map without touching player saves.
- Native installed assets only; no WoW asset files or extracted game data enter the repository.

## Work and validation

First survey the complete instance, usable rock/timber/wreckage models, and the ship's static
geometry. Author distinct destruction compositions for tunnels, workshop/mast room, foundry,
cannon approaches and ship cavern, including branches and side rooms. Large pieces establish
the damaged shape; smaller debris and restrained residual dust fill it out.

Check actual asset bounds and floors, visual density, visibility range, performance and cleanup.
Inspect the result in the native client from clean static views and camera moves. A debug
command succeeding, a count of spawned objects or a dust effect alone is not visual acceptance.

Client patch work, if required to remove built-in ship geometry, will be coordinated with
Claude's existing patch claims before implementation. Full spectator filming will be a
separate task after this finished environment is established.
