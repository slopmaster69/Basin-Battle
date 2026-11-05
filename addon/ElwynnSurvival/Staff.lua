-- Elwynn Survival: a casting staff's mark on the reticle, and what its bolts have built.
-- The spells keep the skill checks; a staff builds and spends (the server's [staffs]):
--   ember   four flames round the reticle; each hit heats the foe and the flames close in, the
--           third ignites it (the flames burst outward and settle back).
--   rime    four ice shards; each hit chills the foe and the shards close in, the third freezes it.
--   arcane  a rune ring with three sparks; each hit lights a spark (a charge), up to three; the
--           heavy orb spends them all.
-- The heavy bolt's charge brightens the mark and swells it, with a flash as it's whole; no ring,
-- no flare on the body. Lines: ST (which staff), SK (stacks), SH (the heavy's charge).
-- Written for the 1.12 client's Lua 5.0: no #, no %, table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local ART = "Interface\\AddOns\\ElwynnSurvival\\art\\check\\"
local SIZE = 46
-- Each staff: its mark, the colour of its lit pips and pops, and its sounds.
local STAFF = {
  ember = { tex = "staff_ember", r = 1.0, g = 0.55, b = 0.15, kind = "heat",
    pop = "Sound\\Spells\\FireBallImpactA.wav", step = "Sound\\interface\\MagicClick.wav" },
  rime = { tex = "staff_rime", r = 0.55, g = 0.85, b = 1.0, kind = "chill",
    pop = "Sound\\Spells\\FrostNova.wav", step = "Sound\\interface\\MagicClick.wav" },
  arcane = { tex = "staff_arcane", r = 1.0, g = 0.35, b = 0.9, kind = "charge",
    pop = "Sound\\Spells\\ArcaneMissileImpact1a.wav", step = "Sound\\interface\\MagicClick.wav" },
}
-- The Arcane mark's three sparks, from its middle, as shares of its size (x right, y up).
local SPARKS = { { 0, 0.258 }, { -0.214, -0.125 }, { 0.216, -0.125 } }

local mark = CreateFrame("Frame", "ElwynnSurvivalStaff", WorldFrame)
mark:SetWidth(1)
mark:SetHeight(1)
mark:SetPoint("CENTER", WorldFrame, "CENTER", 0, 0)
mark:SetFrameStrata("HIGH")
mark:Hide()
mark.tex = mark:CreateTexture(nil, "ARTWORK")
mark.tex:SetBlendMode("ADD")
mark.tex:SetPoint("CENTER", mark, "CENTER", 0, 0)
mark.pop = mark:CreateTexture(nil, "OVERLAY")
mark.pop:SetBlendMode("ADD")
mark.pop:SetPoint("CENTER", mark, "CENTER", 0, 0)
mark.pop:Hide()
mark.sparks = {}
for i = 1, 3 do
  local t = mark:CreateTexture(nil, "OVERLAY")
  t:SetTexture(ART .. "check_glow")
  t:SetBlendMode("ADD")
  t:Hide()
  mark.sparks[i] = t
end
mark.staff = nil
mark.n, mark.most = 0, 3
mark.popAt, mark.stepAt = nil, nil
mark.heavy = nil
ES.staffMark = mark

local function Paint()
  local s = mark.staff and STAFF[mark.staff]
  if not s then
    mark:Hide()
    return
  end
  mark.tex:SetTexture(ART .. s.tex)
  mark.pop:SetTexture(ART .. s.tex)
  mark:Show()
end

-- `ST <ember|rime|arcane|>`: the staff in hand (none: the mark goes).
function ES.StaffLine(f)
  local which = f[2] or ""
  if not STAFF[which] then which = nil end
  if which ~= mark.staff then
    mark.n = 0
    mark.heavy = nil
  end
  mark.staff = which
  Paint()
end

-- `SK <kind> <n> <most> <guid>`: heat or chill on the foe last hit, or the caster's charges.
function ES.StacksLine(f)
  local s = mark.staff and STAFF[mark.staff]
  if not s or f[2] ~= s.kind then return end
  local n = tonumber(f[3]) or 0
  mark.most = math.max(1, tonumber(f[4]) or 3)
  local now = GetTime()
  if n >= mark.most and s.kind ~= "charge" then
    -- The third: it ignites or freezes; the mark bursts, then it's spent.
    mark.popAt = now
    mark.n = 0
    PlaySoundFile(s.pop)
  else
    if n > mark.n then
      mark.stepAt = now
      PlaySoundFile(s.step)
      if n >= mark.most then
        -- The last charge gathered: the whole ring flares.
        mark.popAt = now
        PlaySoundFile(s.pop)
      end
    end
    mark.n = n
  end
end

-- `SH <lo> <hi>`: the heavy bolt's charge began (goes from lo sec, whole at hi).
function ES.StaffHeavyLine(f)
  mark.heavy = { at = GetTime(), lo = tonumber(f[2]) or 0, hi = math.max(0.05, tonumber(f[3]) or 1), flashed = false }
end

-- The heavy is let go (or called off): the left button up, the right button down.
local function HeavyDone()
  mark.heavy = nil
end
ES.StaffHeavyDone = HeavyDone

mark:SetScript("OnUpdate", function()
  local s = this.staff and STAFF[this.staff]
  if not s then return end
  local now = GetTime()
  -- A spell's skill check owns the reticle while it's up.
  local busy = ES.check and ES.check:IsShown()
  local alpha = busy and 0 or 0.62
  local size = SIZE
  local glow = 0
  -- Heat and chill: the flames or shards close in with each.
  if s.kind ~= "charge" then
    size = SIZE * (1 - 0.12 * this.n)
    glow = 0.2 * this.n
  end
  -- A new stack: a quick swell.
  if this.stepAt then
    local t = (now - this.stepAt) / 0.18
    if t >= 1 then
      this.stepAt = nil
    else
      size = size * (1 + 0.18 * (1 - t))
      glow = glow + 0.4 * (1 - t)
    end
  end
  -- The heavy's charge: brighter and bigger as it fills, a flash as it's whole.
  local h = this.heavy
  if h then
    local held = now - h.at
    local up = IsMouseButtonDown and not IsMouseButtonDown("LeftButton")
    if held > h.hi + 4 or up and held > 0.1 then
      this.heavy = nil
    else
      local p = math.min(1, held / h.hi)
      size = size * (1 + 0.22 * p)
      glow = glow + 0.5 * p
      if p >= 1 then
        if not h.flashed then
          h.flashed = true
          this.popAt = now
          PlaySoundFile(s.pop)
        end
        -- Whole: it breathes.
        glow = glow + 0.2 * (0.5 + 0.5 * math.sin(now * 9))
      end
    end
  end
  this.tex:SetWidth(size)
  this.tex:SetHeight(size)
  this.tex:SetVertexColor(1, 1, 1, math.min(1, alpha + glow) * (busy and 0 or 1))
  -- The burst: the mark flung outward and fading.
  if this.popAt and not busy then
    local t = (now - this.popAt) / 0.4
    if t >= 1 then
      this.popAt = nil
      this.pop:Hide()
    else
      local ps = SIZE * (1 + 0.9 * t)
      this.pop:SetWidth(ps)
      this.pop:SetHeight(ps)
      this.pop:SetVertexColor(1, 1, 1, 1 - t)
      this.pop:Show()
    end
  else
    this.pop:Hide()
  end
  -- Arcane: a lit spark per charge, the newest breathing.
  for i = 1, 3 do
    local t = this.sparks[i]
    if s.kind == "charge" and i <= this.n and not busy then
      local k = SPARKS[i]
      local d = SIZE * 0.42
      if i == this.n then d = d * (1 + 0.12 * math.sin(now * 7)) end
      t:SetWidth(d)
      t:SetHeight(d)
      t:ClearAllPoints()
      t:SetPoint("CENTER", this, "CENTER", k[1] * size, k[2] * size)
      t:SetVertexColor(s.r, s.g, s.b, 0.95)
      t:Show()
    else
      t:Hide()
    end
  end
end)
