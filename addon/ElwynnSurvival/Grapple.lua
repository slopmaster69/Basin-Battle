-- Elwynn Survival: the Hunter's grappling hook and Disengage, as the hero sees them.
--
-- While Grapple Arrow (any hook that pulls its caster) is aimed, the server says where its line
-- ends and whether it catches there (GP). The reticle turns green when the shot will catch a
-- wall, a rock, the ground or a foe and red when it will run out in the open, the strip along the
-- ground runs to that spot, and a ring lies under it: green and solid on a catch, small and red
-- at the end of its range on a miss. As the view moves the add-on tells the server the reticle's
-- line (`.es in al`), a few times a second at most, so the answer keeps up.
--
-- A leap with a look of its own (FP: Ensnaring Retreat's leap back) plays the same on every hero:
-- a tumble head over heels the way it flies and a clip borrowed from another character model,
-- whether it was cast standing or mid-jump.
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

-- The ring's key, and how long one lasts unless it's laid again (it's laid again while aimed).
local RING = "esgrapple"
local RING_SECS = 0.6
-- The reticle's line goes to the server at most this often while it moves, and at least this
-- often while it holds still (the server forgets a line older than 0.6 sec).
local SEND_EVERY, SEND_STILL = 0.1, 0.4
-- How far the view must turn (radians) to count as moved.
local TURNED = 0.004

-- The ring at the hook's end: green on a catch, red on a miss.
local function LayRing(c)
  if not SetGroundMarker or not c then return end
  local hostile = nil
  local radius = 0.9
  if not c.caught then
    hostile = 1
    radius = 0.5
  end
  SetGroundMarker(RING, c.x, c.y, c.ground, radius, RING_SECS, hostile)
  c.ringAt = GetTime()
end

-- GP: where the aimed hook would end (`GP` alone: the aim let go).
function ES.CatchLine(f)
  local caught = f[2]
  if not caught or caught == "" then
    ES.catch = nil
    if ClearGroundMarker then ClearGroundMarker(RING) end
    if ES.DrawAimMarker then ES.DrawAimMarker() end
    return
  end
  local c = ES.catch or {}
  c.caught = caught == "1"
  c.x, c.y, c.z = tonumber(f[3]) or 0, tonumber(f[4]) or 0, tonumber(f[5]) or 0
  c.yards = tonumber(f[6]) or 0
  c.ground = tonumber(f[7]) or c.z
  -- Which hook it's for: another aimed after it isn't tinted by it.
  c.key = f[8] or ""
  ES.catch = c
  LayRing(c)
  if ES.DrawAimMarker then ES.DrawAimMarker() end
end

-- The hook's word for what's aimed now, if it's for that.
function ES.CatchNow()
  local c = ES.catch
  if not c or not ES.Aiming or ES.Aiming() ~= c.key then return nil end
  return c
end

-- The reticle's colour while a hook is aimed (nil: none of its own).
function ES.CatchColor()
  local c = ES.CatchNow()
  if not c then return nil end
  if c.caught then return 0.35, 1, 0.45 end
  return 1, 0.3, 0.25
end

-- Every frame while aiming: the reticle's line to the server as it moves, the ring kept.
function ES.StepCatch()
  local c = ES.CatchNow()
  if not c then return end
  local now = GetTime()
  if now - (c.ringAt or 0) >= RING_SECS * 0.5 then LayRing(c) end
  if not GetActionAim then return end
  local f, e = GetActionAim()
  if not f or not e then return end
  local since = now - (c.sent or 0)
  local moved = not c.f or math.abs(f - c.f) > TURNED or math.abs(e - c.e) > TURNED
  if (moved and since >= SEND_EVERY) or since >= SEND_STILL then
    c.sent, c.f, c.e = now, f, e
    ES.Say(".es in al" .. string.format(" %.4f %.4f", f, e))
  end
end

-- FP: a hero's leap with its own look: the tumble the way it flies, and the clip (another
-- model's take on it) over the whole body for the flight.
function ES.FlipLine(f)
  local guid, secs = f[2], tonumber(f[3]) or 0.8
  local clip, take = tonumber(f[4]) or 0, tonumber(f[5]) or 0
  local from = nil
  if f[6] and f[6] ~= "" then from = f[6] end
  local flip, mine = f[7] == "1", f[8] == "1"
  if not guid then return end
  if mine then guid = "player" end
  local layer = ES.ActionLayer and ES.ActionLayer()
  if flip and RollUnit then
    -- About the hips, tucked with the Jump clip's shape, as a dodge's tumble.
    RollUnit(guid, secs, "roll", 0.75, 38)
  end
  if clip > 0 and PlayUnitAnimation and layer then
    if from and PreloadAnimationSource then PreloadAnimationSource(from) end
    PlayUnitAnimation(guid, clip, 0, nil, nil, take, secs, "full", from)
  end
end
