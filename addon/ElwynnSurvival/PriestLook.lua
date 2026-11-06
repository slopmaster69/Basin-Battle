-- ElwynnSurvival: the Priest's own looks in Corey's art (`PL kind n guid a b own`).
--
-- On the caster's screen, round the reticle: Grace's motes circling it (a halo at III, flying in as
-- a Heal spends them), Mind Flay's tension gauge arched over it (the cord's place along its range,
-- the taut zone glowing, the snap), Penance's three bolts under it lighting as they land, Holy
-- Light's gems beside it. On the bodies, for everyone (SetUnitSprite, patch 0045): a perfect
-- Shield's flare on the friend, Kindle's flame over a burning foe, Fade's afterimage.

local ES = ElwynnSurvival
local ART = "Interface\\AddOns\\ElwynnSurvival\\art\\check\\"

-- A padded texture is sampled within its middle (turned about its centre); an unpadded one whole.
local H = 0.35355
local look = CreateFrame("Frame", "ElwynnSurvivalPriestLook", WorldFrame)
look:SetWidth(1)
look:SetHeight(1)
look:SetPoint("CENTER", WorldFrame, "CENTER", 0, 0)
look:Hide()

local function Tex(file, layer)
  local t = look:CreateTexture(nil, layer or "ARTWORK")
  t:SetTexture(ART .. file)
  t:SetBlendMode("ADD")
  t:Hide()
  return t
end

-- `t` at `size` px, turned `ang`, at (x, y) from the reticle; `whole`: an unpadded texture, unturned.
local function Put(t, size, ang, alpha, x, y, whole, r, g, b)
  if whole then
    t:SetTexCoord(0, 1, 0, 1)
  else
    local c, s = math.cos(ang) * H, math.sin(ang) * H
    t:SetTexCoord(0.5 - c + s, 0.5 - s - c, 0.5 - c - s, 0.5 - s + c, 0.5 + c + s, 0.5 + s - c, 0.5 + c - s, 0.5 + s + c)
  end
  t:SetWidth(size)
  t:SetHeight(size)
  t:ClearAllPoints()
  t:SetPoint("CENTER", look, "CENTER", x or 0, y or 0)
  t:SetVertexColor(r or 1, g or 1, b or 1, math.max(0, math.min(1, alpha)))
  t:Show()
end

local GRACE_R = 74
local motes = {}
for i = 1, 3 do motes[i] = Tex("priest_grace_mote", "OVERLAY") end
local halo = Tex("priest_grace_halo", "ARTWORK")
local gauge = Tex("priest_flay_gauge", "ARTWORK")
local taut = Tex("priest_flay_taut", "OVERLAY")
local cursor = Tex("check_glow", "OVERLAY")
local snap = Tex("priest_flay_snap", "OVERLAY")
local bolts = {}
for i = 1, 3 do bolts[i] = Tex("priest_penance", "OVERLAY") end
local gems = {}
for i = 1, 3 do gems[i] = Tex("priest_holy_light", "OVERLAY") end

-- What each look last heard.
local L = { grace = 0, spent = 0, spentAt = nil, flayAt = nil, flay = 0, flayTaut = 0, flayFrom = 0.87,
  snapAt = nil, penance = 0, penanceAt = nil, friend = 0, foe = 0, light = 0, lightUntil = 0 }
-- Looks on bodies: key -> { unit, tex, at, secs, size, height, kind, n }.
local onUnits = {}

-- Mind Flay's gauge: an arc over the reticle, its channel `FLAY_R` px out, spanning 120 degrees.
local FLAY_SIZE, FLAY_R, FLAY_SPAN = 345, 118, 2.0944

local function ArcXY(share)
  local th = -FLAY_SPAN / 2 + FLAY_SPAN * share
  return FLAY_R * math.sin(th), FLAY_R * math.cos(th)
end

function ES.PriestLookLine(f)
  local kind, n = f[2], tonumber(f[3]) or 0
  local guid, a, b, own = f[4], tonumber(f[5]) or 0, tonumber(f[6]) or 0, f[7] == "1"
  local now = GetTime()
  if kind == "grace" then
    -- `a` 1: a Heal spent it (the motes fly in).
    if a >= 1 and L.grace > 0 then L.spentAt, L.spent = now, L.grace end
    L.grace = n
  elseif kind == "flay" then
    L.flayAt, L.flay, L.flayTaut = now, a, n
    if b > 0 then L.flayFrom = b end
  elseif kind == "snap" then
    L.snapAt, L.flayAt = now, nil
  elseif kind == "penance" then
    L.penance, L.penanceAt, L.friend, L.foe = n, now, a, b
  elseif kind == "light" then
    L.light, L.lightUntil = n, now + ((a > 0) and a or 10)
  elseif SetUnitSprite and guid and guid ~= "0x0" then
    -- On a body, for everyone: the hero's own when it's theirs.
    local unit = (own and kind == "fade") and "player" or guid
    if kind == "shield" then
      onUnits["shield" .. guid] = { unit = unit, tex = "priest_shield_perfect", at = now, secs = 0.6, size = 2.6, height = 1.0, kind = kind }
    elseif kind == "kindle" then
      onUnits["kindle" .. guid] = { unit = unit, tex = "priest_kindled", at = now, secs = 6, size = 0.55, height = 2.7, kind = kind, n = n }
    elseif kind == "fade" then
      onUnits["fade" .. guid] = { unit = unit, tex = "priest_fade", at = now, secs = 0.8, size = 2.4, height = 1.0, kind = kind }
    end
  end
  look:Show()
end

local function StepGrace(now)
  local busy = false
  local spin = now * 1.4
  for i = 1, 3 do
    local m = motes[i]
    local th = spin + (i - 1) * 2.0944
    if i <= L.grace then
      busy = true
      Put(m, 34 + 4 * math.sin(now * 3 + i), 0, 0.9, GRACE_R * math.cos(th), GRACE_R * math.sin(th))
    elseif L.spentAt and i <= L.spent and now - L.spentAt < 0.35 then
      busy = true
      local q = (now - L.spentAt) / 0.35
      Put(m, 34 * (1 + q), 0, 1 - q, GRACE_R * (1 - q) * math.cos(th), GRACE_R * (1 - q) * math.sin(th))
    else
      m:Hide()
    end
  end
  if L.grace >= 3 then
    busy = true
    Put(halo, 2 * GRACE_R * 1.25 * (1 + 0.03 * math.sin(now * 2.2)), now * 0.2, 0.75 + 0.2 * math.sin(now * 3.1))
  elseif L.spentAt and L.spent >= 3 and now - L.spentAt < 0.45 then
    busy = true
    local q = (now - L.spentAt) / 0.45
    Put(halo, 2 * GRACE_R * 1.25 * (1 + 0.6 * q), now * 0.2, 1 - q)
  else
    halo:Hide()
  end
  return busy
end

local function StepFlay(now)
  local busy = false
  -- The gauge while the cord is told (gone half a second after its last word).
  if L.flayAt and now - L.flayAt < 0.6 then
    busy = true
    Put(gauge, FLAY_SIZE, 0, 0.95, 0, 0, true)
    local tx, ty = ArcXY((L.flayFrom + 1) / 2)
    local tension = (L.flayTaut == 1) and (0.8 + 0.2 * math.sin(now * 31)) or 0.35
    Put(taut, 120, 0, tension, tx, ty, true)
    local x, y = ArcXY(L.flay)
    local shake = (L.flayTaut == 1) and 2 * math.sin(now * 47) or 0
    Put(cursor, (L.flayTaut == 1) and 34 or 26, 0, 1, x + shake, y, false, 0.85, 0.7, 1)
  else
    gauge:Hide()
    taut:Hide()
    cursor:Hide()
  end
  if L.snapAt and now - L.snapAt < 0.5 then
    busy = true
    local q = (now - L.snapAt) / 0.5
    Put(snap, 260 * (1 + 0.4 * q), 0, 1 - q * q, 0, FLAY_R)
  else
    snap:Hide()
  end
  return busy
end

local function StepPenance(now)
  -- Three bolts under the reticle, lit as each lands (gone 1.5 sec after the last).
  if L.penanceAt and now - L.penanceAt < 1.5 then
    local done = L.penance >= 3 and L.friend >= 1 and L.foe >= 1
    local since = now - L.penanceAt
    for i = 1, 3 do
      local lit = i <= L.penance
      local pulse = (done and since < 0.4) and (1 + 0.5 * (1 - since / 0.4)) or 1
      Put(bolts[i], (lit and 46 or 34) * pulse, 0, lit and 1 or 0.3, (i - 2) * 26, -64)
    end
    return true
  end
  for i = 1, 3 do bolts[i]:Hide() end
  return false
end

local function StepLight(now)
  -- A gem a stack of Holy Light, beside the reticle, dimming as it runs out.
  if L.light > 0 and now < L.lightUntil then
    local left = math.min(1, (L.lightUntil - now) / 3)
    for i = 1, 3 do
      if i <= L.light then
        Put(gems[i], 30 + 3 * math.sin(now * 4 + i), 0, 0.5 + 0.5 * left, 84 + (i - 1) * 22, 0)
      else
        gems[i]:Hide()
      end
    end
    return true
  end
  for i = 1, 3 do gems[i]:Hide() end
  return false
end

local function StepBodies(now)
  local busy = false
  for key, u in pairs(onUnits) do
    local q = (now - u.at) / u.secs
    if q >= 1 or not SetUnitSprite then
      if ClearUnitSprite then ClearUnitSprite("es_pl_" .. key) end
      onUnits[key] = nil
    else
      busy = true
      local size, alpha = u.size, 1
      if u.kind == "shield" then
        size, alpha = u.size * (0.8 + 0.6 * q), 1 - q * q
      elseif u.kind == "fade" then
        size, alpha = u.size * (1 + 0.2 * q), 0.9 * (1 - q)
      else
        size = u.size * (1 + 0.12 * (u.n or 0)) * (1 + 0.05 * math.sin(now * 6))
        alpha = math.min(1, 0.5 + 0.1 * (u.n or 0)) * math.min(1, (1 - q) * 4)
      end
      SetUnitSprite("es_pl_" .. key, u.unit, ART .. u.tex, size, u.height, 1, 1, 1, alpha, 0, 1)
    end
  end
  return busy
end

look:SetScript("OnUpdate", function()
  local now = GetTime()
  local a = StepGrace(now)
  local b = StepFlay(now)
  local c = StepPenance(now)
  local d = StepLight(now)
  local e = StepBodies(now)
  if not (a or b or c or d or e) then this:Hide() end
end)
