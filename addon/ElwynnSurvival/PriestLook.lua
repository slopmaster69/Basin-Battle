-- ElwynnSurvival: the Priest's own looks in Corey's art (docs/priest-draft3.md). Every one is in the
-- world, on the body or the ground it's about (Corey: a spell's state shows in the spell, not in a
-- gauge on the reticle).
--
-- On bodies (SetUnitSprite, patch 0045): Penance's rings closing on each body in turn (`PL penance`,
-- and `penance_hit` as each lands or is lost), a perfect Shield's flare on the friend, Fade's
-- afterimage, Shadowguard's shard over the Priest's shoulder growing with what it stores, Shadow
-- Weaving's shards and Searing Light's brand over the foe (`SK weave` and `SK sear`), and the
-- Weaving's count in a ring of five under it; the shadow staff glowing while its next bolt waits
-- Empowered (`PL empower`, SetUnitWeaponGlow).
-- On the ground (SetGroundRing, patch 0041; `PG kind key x y z radius secs`): Angelic Feather,
-- Void Rift (growing and turning inward, its true edge drawn, collapsing into its heart as it
-- implodes), Mind Blast's sigil at its true edge with its heart; the Hollow Star's reach round
-- the orb (`PL hollow`, SetGroundRingOnUnit).

local ES = ElwynnSurvival
local ART = "Interface\\AddOns\\ElwynnSurvival\\art\\check\\"

local look = CreateFrame("Frame", "ElwynnSurvivalPriestLook", WorldFrame)
look:Hide()

-- Looks on bodies: key -> { unit, tex, at, secs, size, height, kind, ... }.
local onUnits = {}
-- Marks on the ground: key -> { kind, x, y, z, r, at, secs }.
local onGround = {}

-- Penance's rings: how long a ring takes to close, and its size (yards) open and closed.
-- (The ring's art fills the middle 70% of its texture: a size of 1.5 draws a ring a yard across.)
local RING_OPEN, RING_SHUT, RING_LEAD = 4.2, 1.5, 0.9
-- How a ring that landed flares, and one that was lost breaks apart.
local HIT, LOST = 0.35, 0.9
-- Mind Blast's heart (yards: the rules' `heart`), drawn inside its edge.
local BLAST_HEART = 1.0
-- The shadow staff's glow while the next bolt waits Empowered.
local EMPOWER_GLOW = "B266FF"

local function Body(key, t)
  t.at = t.at or GetTime()
  onUnits[key] = t
  look:Show()
end

function ES.PriestLookLine(f)
  local kind, n = f[2], tonumber(f[3]) or 0
  local guid, a, b, own = f[4], tonumber(f[5]) or 0, tonumber(f[6]) or 0, f[7] == "1"
  local now = GetTime()
  if not guid or guid == "0x0" then guid = nil end
  if kind == "penance" and guid and SetUnitSprite then
    -- Ring `n` closes on that body `a` seconds from now (`b` 1 a friend).
    Body("pen" .. n, { unit = guid, tex = "priest_penance_ring", kind = "ring", beat = now + a, friend = b >= 1, height = 1.1 })
  elseif kind == "penance_ally" and SetUnitSprite then
    -- A friend's Penance ring closing on us: faint (nothing to press), so we know light is coming.
    Body("penally" .. n, { unit = "player", tex = "priest_penance_ring", kind = "ring", beat = now + a, friend = true, faint = true, height = 1.1 })
  elseif kind == "rift" then
    -- Our Void Rift open (`b` its seconds left; -1 gone): its key again implodes it at once.
    ES.riftOpenUntil = (b > 0) and (now + b) or nil
  elseif kind == "empower" then
    -- The next bolt waits Empowered (Corey: a very clear sign when it's up): the staff glows
    -- violet, pulsing, until it goes; a pop of shadow at the hands as it comes up.
    if SetUnitWeaponGlow then
      if n >= 1 then SetUnitWeaponGlow("player", EMPOWER_GLOW, 60) else SetUnitWeaponGlow("player", nil) end
    end
    if n >= 1 and SetUnitSprite then
      Body("empower", { unit = "player", tex = "priest_weave_shard", secs = 0.45, size = 1.6, height = 1.3, kind = "pop" })
    end
  elseif kind == "penance_hit" then
    local u = onUnits["pen" .. n]
    if u then
      u.done, u.doneAt, u.dead = (a >= 1) and "hit" or "lost", now, b >= 1
    end
  elseif kind == "shell" then
    if b >= 1 or n == 0 then
      local u = onUnits["shell"]
      if u then u.doneAt = now end
    elseif SetUnitSprite then
      local u = onUnits["shell"] or { unit = "player", tex = "priest_guard_shard", kind = "shell", height = 2.3 }
      u.share, u.doneAt = a, nil
      Body("shell", u)
    end
  elseif kind == "hollow" then
    if n == 0 or not guid then
      if onUnits["hollow"] then onUnits["hollow"].doneAt = now end
    elseif SetGroundRingOnUnit then
      Body("hollow", { unit = guid, kind = "hollow", reach = a, share = n / math.max(1, b) })
    end
  elseif SetUnitSprite and guid then
    -- On a body, for everyone: the hero's own when it's theirs.
    local unit = (own and kind == "fade") and "player" or guid
    if kind == "shield" then
      Body("shield" .. guid, { unit = unit, tex = "priest_shield_perfect_light", secs = 0.6, size = 2.6, height = 1.0, kind = kind })
    elseif kind == "fade" then
      Body("fade" .. guid, { unit = unit, tex = "priest_fade", secs = 0.8, size = 2.4, height = 1.0, kind = kind })
    elseif kind == "death" then
      -- Death's lance struck this body: a flash of shadow on it (Corey couldn't tell it had hit),
      -- bigger for a judgement (`a`) and a kill (`n` 2).
      local big = (n >= 2) and 3.4 or (a >= 1 and 2.9) or 2.2
      Body("death" .. guid, { unit = guid, tex = "priest_weave_shard", secs = (n >= 2) and 0.7 or 0.5, size = big, height = 1.3, kind = "pop" })
    end
  end
end

-- `SK weave|sear <n> <most> <guid>`: the Priest's stacks on a foe (other kinds are the Mage's).
function ES.PriestStacksLine(f)
  local kind, n, most, guid = f[2], tonumber(f[3]) or 0, tonumber(f[4]) or 1, f[5]
  if (kind ~= "weave" and kind ~= "sear") or not guid or guid == "" or not SetUnitSprite then return end
  local key = kind .. guid
  if n <= 0 then
    if onUnits[key] then onUnits[key].doneAt = GetTime() end
    return
  end
  local u = onUnits[key] or { unit = guid, kind = kind, tex = (kind == "weave") and "priest_weave_shard" or "priest_sear_brand", height = (kind == "weave") and 2.6 or 1.4 }
  if (u.n or 0) < n then u.stepAt = GetTime() end
  u.n, u.most, u.doneAt = n, most, nil
  Body(key, u)
end

-- `PG kind key x y z radius secs`: a Priest's mark on the ground (radius 0: it's gone).
function ES.PriestGroundLine(f)
  if not SetGroundRing then return end
  local kind, key = f[2], f[3]
  local x, y, z, r, secs = tonumber(f[4]), tonumber(f[5]), tonumber(f[6]), tonumber(f[7]) or 0, tonumber(f[8]) or 0
  if not (kind and key and x and y and z) then return end
  local now = GetTime()
  if r <= 0 then
    local g = onGround[key]
    if g then g.goneAt = now end
    return
  end
  onGround[key] = { kind = kind, x = x, y = y, z = z, r = r, at = now, secs = secs }
  look:Show()
end

local function Gone(key, g)
  if ClearGroundRing then
    ClearGroundRing("es_pg_" .. key)
    ClearGroundRing("es_pg_" .. key .. "_glow")
  end
  onGround[key] = nil
end

local function StepGround(now)
  if not SetGroundRing then
    onGround = {}
    return false
  end
  local busy = false
  for key, g in pairs(onGround) do
    local age = now - g.at
    local fade = 1
    if g.goneAt and g.kind ~= "rift" then fade = 1 - (now - g.goneAt) / 0.35 end
    if g.goneAt and g.kind == "rift" and now - g.goneAt > 0.35 then fade = 0 end
    if age > g.secs + 1.5 then fade = 0 end
    if fade <= 0 then
      Gone(key, g)
    else
      busy = true
      local k = "es_pg_" .. key
      local into = math.min(1, age / 0.25)
      if g.kind == "feather" then
        -- Lying there, breathing, turning slowly; the last two seconds it fades.
        -- A thin feather flat on the ground hides in the grass seen from behind (the first film):
        -- drawn bigger, with a breathing ring of gold round it that reads from any angle.
        local left = math.min(1, (g.secs - age) / 2)
        local a = into * fade * math.max(0.25, left)
        SetGroundRing(k, g.x, g.y, g.z + 0.08, 2.4 * (1 + 0.06 * math.sin(now * 3)), ART .. "priest_feather", 1, 1, 1, a, now * 0.25)
        SetGroundRing(k .. "_glow", g.x, g.y, g.z + 0.06, (g.r + 0.3) / 0.68 * (1 + 0.08 * math.sin(now * 4)), ART .. "priest_penance_ring", 1, 0.9, 0.6, a * 0.8, -now * 0.4)
      elseif g.kind == "rift" then
        -- Growing for 2 sec to 1.75 times as wide (the rules' 4 to 7 yd), turning inward faster as
        -- it grows, its true edge a thin violet line (Corey: the circle drawn where it reaches).
        -- Gone, it collapses into its heart in a moment: the implosion fills it, edge to middle.
        local grow = 1 + 0.75 * math.min(1, ((g.goneAt or now) - g.at) / 2)
        local r = g.r * grow
        if g.goneAt then
          local q = math.min(1, (now - g.goneAt) / 0.35)
          r = r * (1 - 0.85 * q)
          fade = 1 - q * q
        end
        SetGroundRing(k, g.x, g.y, g.z + 0.05, r / 0.72, ART .. "priest_void_rift", 1, 1, 1, into * fade, -now * (0.6 + 0.6 * grow) * (g.goneAt and 4 or 1))
        SetGroundRing(k .. "_glow", g.x, g.y, g.z + 0.07, r / 0.68, ART .. "priest_penance_ring", 0.7, 0.4, 1, into * fade * 0.85, now * 0.3)
      elseif g.kind == "blast" then
        -- At its true edge, still: it never narrows (Corey: a ring closing read as a press to
        -- time). Its heart a violet ring in the middle, beating; both brighten until it erupts.
        local q = math.min(1, age / math.max(0.1, g.secs))
        SetGroundRing(k, g.x, g.y, g.z + 0.05, g.r / 0.72, ART .. "priest_blast_sigil", 1, 1, 1, into * fade * (0.55 + 0.45 * q), now * 0.5)
        SetGroundRing(k .. "_glow", g.x, g.y, g.z + 0.07, BLAST_HEART / 0.68 * (1 + 0.08 * math.sin(now * 12)), ART .. "priest_penance_ring", 0.75, 0.45, 1, into * fade * (0.65 + 0.35 * q), -now * 1.5)
        if age > g.secs + 0.1 then g.goneAt = g.goneAt or now end
      end
    end
  end
  return busy
end

local function StepBodies(now)
  -- A client without the sprites (or the harness's stand-ins gone): nothing to draw on bodies.
  if not SetUnitSprite then
    onUnits = {}
    return false
  end
  local busy = false
  for key, u in pairs(onUnits) do
    local sk = "es_pl_" .. key
    local alpha, size, done = 1, u.size or 1, false
    if u.kind == "ring" then
      -- Closing on the body as its beat comes; a flare as it lands, grey as it's lost.
      local left = u.beat - now
      if u.done then
        local q = (now - u.doneAt) / ((u.done == "hit") and HIT or LOST)
        if q >= 1 then done = true end
        if u.done == "hit" then
          size, alpha = RING_SHUT * (1 + (u.dead and 1.2 or 0.7) * q), 1 - q
        end
      elseif left > RING_LEAD + 0.6 then
        alpha = 0
      elseif left < -0.6 then
        done = true
      else
        local q = math.max(0, math.min(1, 1 - left / RING_LEAD))
        size = RING_OPEN + (RING_SHUT - RING_OPEN) * q
        alpha = math.min(1, (RING_LEAD + 0.6 - left) / 0.3) * (0.55 + 0.45 * q)
      end
      if not done then
        local r, g, b = 1, 0.86, 0.45
        if u.friend then r, g, b = 0.9, 1, 0.75 end
        local tex, spin = u.tex, now * 0.8
        if u.done == "lost" then
          -- Let pass: that body's own ring breaks, grey, its pieces flying apart (Corey: the miss
          -- shows on the ring that was missed).
          local q = (now - u.doneAt) / LOST
          -- A dull red, bigger than the ring was, so it reads from behind at a pack's distance
          -- (grey was lost in the grass on the first film).
          r, g, b = 1, 0.42, 0.32
          tex, spin = "priest_penance_ring_broken", now * 0.8 + 1.2 * q
          size, alpha = RING_SHUT * (1.6 + 1.2 * q), 1 - q * q
        end
        if u.faint then alpha, size = alpha * 0.45, size * 0.8 end
        SetUnitSprite(sk, u.unit, ART .. tex, size, u.height, r, g, b, alpha, spin, 1)
      end
    elseif u.kind == "hollow" then
      if u.doneAt and now - u.doneAt > 0.3 then
        done = true
      else
        local fade = u.doneAt and (1 - (now - u.doneAt) / 0.3) or 1
        if SetGroundRingOnUnit then SetGroundRingOnUnit(sk, u.unit, (u.reach or 3) / 0.68, ART .. "priest_hollow_reach", 0.85, 0.7, 1, (0.55 + 0.35 * (u.share or 0.5)) * fade, -now * 0.4) end
      end
    elseif u.kind == "weave" or u.kind == "sear" or u.kind == "shell" then
      local fade = u.doneAt and (1 - (now - u.doneAt) / 0.3) or 1
      if fade <= 0 then
        done = true
      else
        local swell = 0
        if u.stepAt and now - u.stepAt < 0.2 then swell = 1 - (now - u.stepAt) / 0.2 end
        if u.kind == "weave" then
          -- A shard a stack: bigger and darker-bright with each, turning.
          size = (0.35 + 0.12 * (u.n or 1)) * (1 + 0.3 * swell)
          alpha = (0.5 + 0.1 * (u.n or 1)) * fade
          SetUnitSprite(sk, u.unit, ART .. u.tex, size, u.height, 1, 1, 1, alpha, now * 0.9, 1)
          -- And the count, plain from any side: five segments under the foe, one lit a stack.
          if SetGroundRingOnUnit then
            SetGroundRingOnUnit(sk .. "_n", u.unit, 1.75 * (1 + 0.15 * swell), ART .. "priest_weave_count" .. math.max(1, math.min(5, u.n or 1)), 1, 1, 1, 0.9 * fade, 0)
          end
        elseif u.kind == "sear" then
          size = 0.7 * (1 + 0.08 * math.sin(now * 7)) * (1 + 0.4 * swell)
          SetUnitSprite(sk, u.unit, ART .. u.tex, size, u.height, 1, 1, 1, 0.9 * fade, 0, 1)
        else
          size = (0.35 + 0.75 * (u.share or 0)) * (1 + 0.05 * math.sin(now * 9))
          SetUnitSprite(sk, u.unit, ART .. u.tex, size, u.height, 1, 1, 1, (0.6 + 0.4 * (u.share or 0)) * fade, now * 0.5, 1)
        end
      end
    else
      local q = (now - u.at) / u.secs
      if q >= 1 then
        done = true
      elseif u.kind == "pop" then
        SetUnitSprite(sk, u.unit, ART .. u.tex, u.size * (0.5 + 0.8 * q), u.height, 0.85, 0.6, 1, 1 - q, now * 3, 1)
      elseif u.kind == "shield" then
        -- Cooled toward white: the art's gold read as a Paladin's on the grass (Corey).
        SetUnitSprite(sk, u.unit, ART .. u.tex, u.size * (0.8 + 0.6 * q), u.height, 0.8, 0.9, 1, 1 - q * q, 0, 1)
      else
        SetUnitSprite(sk, u.unit, ART .. u.tex, u.size * (1 + 0.2 * q), u.height, 1, 1, 1, 0.9 * (1 - q), 0, 1)
      end
    end
    if done then
      if u.kind == "hollow" then
        if ClearGroundRing then ClearGroundRing(sk) end
      elseif ClearUnitSprite then
        if u.kind == "weave" and ClearGroundRing then ClearGroundRing(sk .. "_n") end
        ClearUnitSprite(sk)
      end
      onUnits[key] = nil
    else
      busy = true
    end
  end
  return busy
end

look:SetScript("OnUpdate", function()
  local now = GetTime()
  local a = StepBodies(now)
  local b = StepGround(now)
  if not (a or b) then this:Hide() end
end)
