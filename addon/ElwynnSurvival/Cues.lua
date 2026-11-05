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

-- Each beat: the hit-stop (ms) on the hero and on the unit, the shake (inches, seconds), the
-- camera's kick (back, seconds), and an edge: the screen's corners flashed in a colour (r, g, b)
-- that fades over its seconds (ES.Edge; a hurt's red is only ever damage taken). A field left
-- out isn't played. None of these has been seen in the game yet: they're starting numbers, set
-- against the blows' own (a heavy blow stops 115 ms and shakes 3.0 for 0.26 s; a parry stops
-- 150 ms).
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
  -- A parry that only blocks (a giant's blow): driven back into the guard, the view shoved
  -- harder than any other block takes it, so it reads with no word (Corey: louder cues, not text),
  -- and the corners flashed the dust's colour, not red (red would say damage taken).
  too_heavy = { stop = 95, shake = { 6.5, 0.34 }, kick = { 0.12, 0.34 }, self = true, edge = { 0.78, 0.68, 0.50, 0.40 } },
  -- A body crashing into a wall: a stop on it and a thump of the view.
  wall = { stop = 120, shake = { 3.2, 0.24 } },
  -- A killing blow cheated: the world held around the hero a long moment, the view barely moving.
  cheat_death = { stop = 260, shake = { 1.2, 0.4 }, self = true },
  -- Valor given back, and every cooldown readied: on the HUD (below), not the camera.
  valor = { hud = "valor" },
  readied = { hud = "keys" },
  -- Thrown up by the Uppercut (Raptor Strike): the camera comes in close and tips down onto the
  -- fight to aim the Stoop that follows (Corey's review: "zoom in and tip down"): distance,
  -- right, up, the tip down (radians; the sixteenth client, older ones only come in), and for
  -- how long (the rise's time in the air), then back to the aiming camera or off.
  rise = { cam = { 2.6, 0.4, 1.2, 0.55, 1.4 } },
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
  if c.cam then
    if SetAimCamera then
      ES.riseCamAt = GetTime() + c.cam[5]
      SetAimCamera(1, c.cam[1], c.cam[2], c.cam[3], c.cam[4])
    end
    return
  end
  if c.stop and HitStop then
    HitStop("player", c.stop)
    if guid and not c.self then HitStop(guid, c.stop) end
  end
  if c.shake and ShakeCamera then ShakeCamera(c.shake[1], c.shake[2]) end
  if c.kick and CameraKick then CameraKick(c.kick[1], c.kick[2]) end
  if c.edge and ES.Edge then ES.Edge(c.edge[1], c.edge[2], c.edge[3], c.edge[4]) end
end

-- The rise's camera let go as its time runs out: back to the aiming camera if one is up (the
-- Stoop's reticle), else off. While it's up, DrawAimCamera leaves the camera to it.
local rise = CreateFrame("Frame", nil, UIParent)
rise:SetScript("OnUpdate", function()
  local at = ES.riseCamAt
  if not at or GetTime() < at then return end
  ES.riseCamAt = nil
  if not SetAimCamera then return end
  if ES.aimCam then
    SetAimCamera(1, 2.4, 0.75, 0.25)
  else
    SetAimCamera(nil)
  end
end)

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

-- The game's Error Speech setting (Sound): the server voices a refusal ("Out of range.") only
-- while it's on, so it's never forced on anyone. Told once a server is awake and again whenever
-- the setting changes; a client without the setting leaves the server's default (on).
local speech = CreateFrame("Frame", nil, UIParent)
speech.wait, speech.sent = 0, nil
speech:SetScript("OnUpdate", function()
  this.wait = this.wait - (arg1 or 0)
  if this.wait > 0 then return end
  this.wait = 1
  if not ES.awake then
    this.sent = nil
    return
  end
  local v = ES.CVar and ES.CVar("EnableErrorSpeech")
  if v == nil then return end
  local on = 0
  if v ~= 0 then on = 1 end
  if on ~= this.sent and ES.Say then
    this.sent = on
    ES.Say(".es speech " .. on)
  end
end)
