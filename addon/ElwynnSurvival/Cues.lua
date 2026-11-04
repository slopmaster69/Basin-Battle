-- Elwynn Survival: an outcome's beat on the hero's own screen, with no word over it.
--
-- Corey's rule: no text explains a spell or a combat outcome; its look and its sound show it.
-- The look (a spell visual kit) and the sound come from the server to everyone near; what the
-- hero whose outcome it is feels besides is a beat here (CU\t<beat>\t<guid>): a hit-stop on them
-- and on the unit it happened to, a shake and a kick of the camera, each outcome its own size so
-- a critical, a shatter and an execute don't feel alike. Two beats are on the HUD, not the
-- camera: Valor given back blinks the Valor bar, and every cooldown readied flares every key.
--
-- A parry needs no line here: the swing's own IM line already rings and stops it (ImpactLine).
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

-- Each beat: the hit-stop (ms) on the hero and on the unit, the shake (inches, seconds) and the
-- camera's kick (back, seconds). A field left out isn't played. None of these has been seen in
-- the game yet: they're starting numbers, set against the blows' own (a heavy blow stops 115 ms
-- and shakes 3.0 for 0.26 s; a parry stops 150 ms).
ES.CUE = {
  -- A critical: past a heavy blow's weight, a sharp kick.
  crit = { stop = 130, shake = { 3.6, 0.26 }, kick = { 0.07, 0.28 } },
  -- The ice shattering: the longest stop and the biggest shake, as a burst should.
  shatter = { stop = 170, shake = { 5.0, 0.34 }, kick = { 0.09, 0.32 } },
  -- An execute: a hard, held stop and a deep kick, little shake (a finishing blow, not a burst).
  execute = { stop = 200, shake = { 2.4, 0.22 }, kick = { 0.11, 0.36 } },
  -- A backstab: short and sharp, all stop and no kick (a knife, not a hammer).
  backstab = { stop = 110, shake = { 1.6, 0.14 } },
  -- A champion's stance breaking: a long stop on it, a heavy shake.
  ["break"] = { stop = 160, shake = { 4.2, 0.32 }, kick = { 0.06, 0.3 } },
  -- A parry that only blocks (a giant's blow): driven into the guard, the view shoved hard.
  too_heavy = { stop = 70, shake = { 5.5, 0.3 }, kick = { 0.08, 0.3 }, self = true },
  -- A body crashing into a wall: a stop on it and a thump of the view.
  wall = { stop = 120, shake = { 3.2, 0.24 } },
  -- A killing blow cheated: the world held around the hero a long moment, the view barely moving.
  cheat_death = { stop = 260, shake = { 1.2, 0.4 }, self = true },
  -- Valor given back, and every cooldown readied: on the HUD (below), not the camera.
  valor = { hud = "valor" },
  readied = { hud = "keys" },
}

function ES.CueLine(f)
  local c = ES.CUE[f[2] or ""]
  if not c then return end
  local guid = f[3]
  if guid == "0x0" or guid == "" then guid = nil end
  if c.hud == "valor" then
    if ES.valor then ES.cueValorAt = GetTime() end
    return
  elseif c.hud == "keys" then
    local bar = ES.abar
    if bar and bar.slots then
      for i = 1, table.getn(bar.slots) do
        if bar.slots[i] then bar.slots[i].fullAt = GetTime() end
      end
    end
    return
  end
  if c.stop and HitStop then
    HitStop("player", c.stop)
    if guid and not c.self then HitStop(guid, c.stop) end
  end
  if c.shake and ShakeCamera then ShakeCamera(c.shake[1], c.shake[2]) end
  if c.kick and CameraKick then CameraKick(c.kick[1], c.kick[2]) end
end

-- Valor given back: the bar blinks twice as it fills back.
local glow = CreateFrame("Frame", nil, UIParent)
glow:SetScript("OnUpdate", function()
  local v, at = ES.valor, ES.cueValorAt
  if not v or not at then return end
  local t = (GetTime() - at) / 0.6
  if t >= 1 then
    ES.cueValorAt = nil
    v:SetAlpha(1)
    return
  end
  -- Two quick blinks as it fills back.
  v:SetAlpha(1 - 0.6 * math.abs(math.sin(t * 3.14159 * 2)))
end)
