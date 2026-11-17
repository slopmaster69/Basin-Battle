-- Elwynn Survival: the Rogue's Flourish (Blades' trait) beside the reticle.
-- Each perfect dodge with 3 Blades abilities on the keys gives one, up to five, and a hit on the
-- Rogue takes them all (the server's [rogue]). Five violet diamonds under the reticle, one lit
-- for each: a chime and a flare as one comes, a crack and the diamonds bursting as they're lost.
-- Nothing on the ground: it reads with floor markers off. Line: SK flourish <n> <most>.
-- Written for the 1.12 client's Lua 5.0: no #, no %, table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local ART = "Interface\\AddOns\\ElwynnSurvival\\art\\"
local PIP, GAP = 12, 4

-- A perfect dodge's violet corners (Cues.lua's `corners`): four glows of their own, one in each
-- corner of the screen, each anchored to its own corner and sized by the screen. The shared edge
-- (ES.Edge) sits under the HUD and drew them faint, the top-left alone showing in the Nov 17 film:
-- these stand over the HUD (HIGH) on the world's frame, so the interface hidden or not they show, each
-- the vignette's corner laid twice for a solid colour, full at once and gone over their seconds.
local cn = CreateFrame("Frame", "ElwynnSurvivalDodgeCorners", WorldFrame)
cn:SetAllPoints(WorldFrame)
cn:SetFrameStrata("HIGH")
cn:Hide()
cn.pieces = {}
cn.secs, cn.left = 1, 0
local CUT = 0.3
local SPOTS = {
  { "TOPLEFT", 0, CUT, 0, CUT },
  { "TOPRIGHT", 1 - CUT, 1, 0, CUT },
  { "BOTTOMLEFT", 0, CUT, 1 - CUT, 1 },
  { "BOTTOMRIGHT", 1 - CUT, 1, 1 - CUT, 1 },
}
for i = 1, 4 do
  local s = SPOTS[i]
  for layer = 1, 2 do
    local t = cn:CreateTexture(nil, "ARTWORK")
    t:SetTexture(ART .. "vignette")
    t:SetTexCoord(s[2], s[3], s[4], s[5])
    t:SetPoint(s[1], cn, s[1], 0, 0)
    table.insert(cn.pieces, t)
  end
end
ES.dodgeCorners = cn

-- `corners = { r, g, b, secs }`: the four corners in that colour, a third of the screen's height
-- each.
function ES.RogueCorners(r, g, b, secs)
  local h = WorldFrame:GetHeight() or 768
  local size = math.floor(math.max(120, h * 0.34))
  for i = 1, table.getn(cn.pieces) do
    local t = cn.pieces[i]
    t:SetWidth(size)
    t:SetHeight(size)
    t:SetVertexColor(r or 1, g or 1, b or 1, 1)
  end
  cn.secs = math.max(0.05, tonumber(secs) or 1)
  cn.left = cn.secs
  cn:SetAlpha(1)
  cn:Show()
end

cn:SetScript("OnUpdate", function()
  this.left = this.left - (arg1 or 0)
  if this.left <= 0 then
    this:SetAlpha(0)
    this:Hide()
    return
  end
  this:SetAlpha(this.left / this.secs)
end)

local fl = CreateFrame("Frame", "ElwynnSurvivalFlourish", WorldFrame)
fl:SetWidth(1)
fl:SetHeight(1)
fl:SetPoint("CENTER", WorldFrame, "CENTER", 0, -44)
fl:SetFrameStrata("HIGH")
fl:Hide()
fl.pips = {}
fl.n, fl.most = 0, 5
fl.stepAt, fl.lostAt = nil, nil

local function Pip(i)
  local t = fl.pips[i]
  if t then return t end
  t = fl:CreateTexture(nil, "OVERLAY")
  t:SetTexture(ART .. "diamond")
  t:SetBlendMode("ADD")
  fl.pips[i] = t
  return t
end

local function Lay()
  local w = fl.most * PIP + (fl.most - 1) * GAP
  for i = 1, 8 do
    local t = Pip(i)
    if i <= fl.most then
      t:ClearAllPoints()
      t:SetPoint("CENTER", fl, "CENTER", -w / 2 + PIP / 2 + (i - 1) * (PIP + GAP), 0)
      t:Show()
    else
      t:Hide()
    end
  end
end

-- `AI <guid> <secs> <alpha> <[AA]RRGGBB> [<x> <y> <z> <stand-in>]` (AA: how far a round-34 client
-- lights the copy toward the colour, patch 0112): a see-through copy of a Rogue's
-- own body left behind (patch 0103's PlayUnitAfterimage): for a moment where a perfect dodge
-- slipped a blow, or standing as Afterimage's shade at its spot while the shade's own body is
-- hidden. `AI <guid> 0` takes the copies away. Without the patch, nothing: the server's flicker and
-- the shade's own body are the look.
function ES.RogueImageLine(f)
  if not PlayUnitAfterimage or not f[2] then return end
  local guid = f[2]
  local secs = tonumber(f[3]) or 0
  if secs <= 0 then
    PlayUnitAfterimage(guid, 0)
    return
  end
  local x, y, z = tonumber(f[6]), tonumber(f[7]), tonumber(f[8])
  local standIn = f[9]
  if standIn == "-" then standIn = nil end
  PlayUnitAfterimage(guid, secs, tonumber(f[4]) or 0.6, f[5], x, y, z, standIn)
end

-- `SK flourish <n> <most>`: the Rogue's Flourish now (other kinds are the staffs' and the Priest's).
function ES.RogueStacksLine(f)
  if f[2] ~= "flourish" then return end
  local n = tonumber(f[3]) or 0
  local most = math.max(1, math.min(8, tonumber(f[4]) or 5))
  local now = GetTime()
  if most ~= fl.most then
    fl.most = most
    Lay()
  end
  if n > fl.n then
    fl.stepAt = now
    PlaySoundFile("Sound\\interface\\MagicClick.wav")
  elseif n == 0 and fl.n > 0 then
    -- Lost (a hit) or run out: they burst and fade.
    fl.lostAt = now
    PlaySoundFile("Sound\\Spells\\Fizzle\\FizzleShadowA.wav")
  end
  fl.lit = (n > 0) and n or fl.n
  fl.n = n
  Lay()
  fl:Show()
end

fl:SetScript("OnUpdate", function()
  local now = GetTime()
  local lost = this.lostAt and (now - this.lostAt) or nil
  if this.n == 0 and (not lost or lost > 0.5) then
    this.lostAt = nil
    this:Hide()
    return
  end
  -- The newest swells and flashes white as it comes.
  local step = this.stepAt and (now - this.stepAt) or 9
  for i = 1, this.most do
    local t = Pip(i)
    local size, a = PIP, 0.28
    local r, g, b = 0.55, 0.35, 0.9
    if lost then
      -- Every lit one bursts outward and fades over half a second.
      if i <= (this.lit or 0) then
        size = PIP * (1 + lost * 2.4)
        a = math.max(0, 0.9 * (1 - lost / 0.5))
        r, g, b = 0.85, 0.6, 1.0
      else
        a = math.max(0, 0.28 * (1 - lost / 0.5))
      end
    elseif i <= this.n then
      a = 0.95
      r, g, b = 0.75, 0.5, 1.0
      if i == this.n and step < 0.25 then
        size = PIP * (1.8 - step * 3.2)
        r, g, b = 1, 0.9, 1
      end
      -- Full: they breathe.
      if this.n >= this.most then
        a = 0.75 + 0.25 * math.sin(now * 8)
      end
    end
    t:SetWidth(size)
    t:SetHeight(size)
    t:SetVertexColor(r, g, b, a)
  end
end)
