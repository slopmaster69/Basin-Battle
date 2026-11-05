-- Elwynn Survival: the ward bar, a caster's guard (a staff's). A slim bar under stamina that the
-- server fills (WA ward most broken; WA alone: the hero guards with something else and the bar
-- is hidden). It drains as blows are stopped, dims and flashes while the ward is broken (it can't
-- go up again until the bar is whole), and flares when the bar is whole again. No words: the bar
-- says it all.
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local ART = "Interface\\AddOns\\ElwynnSurvival\\art\\"
local H = 5
local GAP = 4
-- Arcane: a violet-blue, paler where a blow has just taken a piece.
local COLOR = { r = 0.52, g = 0.42, b = 0.96 }
local CHUNK = { r = 0.85, g = 0.8, b = 1.0 }
local BROKEN = { r = 0.45, g = 0.2, b = 0.3 }

local vit = ES.vitals
if not vit then return end

local ward = CreateFrame("Frame", "ElwynnSurvivalWard", vit)
ward:SetHeight(H)
ward:SetWidth(100)
ward.rim = ward:CreateTexture(nil, "BACKGROUND")
ward.rim:SetTexture(ART .. "hairline")
ward.rim:SetPoint("TOPLEFT", ward, "TOPLEFT", -1.5, 1.5)
ward.rim:SetPoint("BOTTOMRIGHT", ward, "BOTTOMRIGHT", 1.5, -1.5)
ward.rim:SetVertexColor(0.55, 0.5, 0.75, 0.85)
ward.track = ward:CreateTexture(nil, "BACKGROUND")
ward.track:SetTexture(ART .. "bar")
ward.track:SetAllPoints(ward)
ward.track:SetVertexColor(0.03, 0.03, 0.05, 0.82)
ward.lag = ward:CreateTexture(nil, "BORDER")
ward.lag:SetTexture(ART .. "bar")
ward.lag:SetPoint("TOPLEFT", ward, "TOPLEFT", 0, 0)
ward.lag:SetPoint("BOTTOMLEFT", ward, "BOTTOMLEFT", 0, 0)
ward.lag:SetVertexColor(CHUNK.r, CHUNK.g, CHUNK.b, 0.85)
ward.fill = ward:CreateTexture(nil, "ARTWORK")
ward.fill:SetTexture(ART .. "bar")
ward.fill:SetPoint("TOPLEFT", ward, "TOPLEFT", 0, 0)
ward.fill:SetPoint("BOTTOMLEFT", ward, "BOTTOMLEFT", 0, 0)
ward.fill:SetVertexColor(COLOR.r, COLOR.g, COLOR.b, 0.97)
ward.full = 100
ward.share, ward.lagShare, ward.lagHold = 1, 1, 0
ward.broken = false
ward.shown = false
ward:Hide()
ES.ward = ward

local function Paint()
  if ward.broken then
    ward.fill:SetVertexColor(BROKEN.r, BROKEN.g, BROKEN.b, 0.97)
    ward.rim:SetVertexColor(0.8, 0.3, 0.35, 0.9)
  else
    ward.fill:SetVertexColor(COLOR.r, COLOR.g, COLOR.b, 0.97)
    ward.rim:SetVertexColor(0.55, 0.5, 0.75, 0.85)
  end
end

local function SetShare(share, instant)
  if share < 0 then share = 0 end
  if share > 1 then share = 1 end
  if share < ward.share and not instant then
    if ward.lagShare < ward.share then ward.lagShare = ward.share end
    ward.lagHold = GetTime() + 0.55
  elseif share > ward.lagShare or instant then
    ward.lagShare = share
  end
  ward.share = share
  ward.fill:SetWidth(math.max(1, ward.full * share))
  ward.lag:SetWidth(math.max(1, ward.full * ward.lagShare))
end

-- As long as the stamina bar, under it; the vitals grow to make room, and in the corner the guard
-- light moves down below it.
function ES.PlaceWard()
  ward:ClearAllPoints()
  ward:SetPoint("TOPLEFT", vit.stamina, "BOTTOMLEFT", 0, -GAP)
  local w = vit.stamina.full or 100
  if math.abs(w - ward.full) >= 0.5 then
    ward.full = w
    ward:SetWidth(w)
    ward.fill:SetWidth(math.max(1, w * ward.share))
    ward.lag:SetWidth(math.max(1, w * ward.lagShare))
  end
  if ward.shown then
    vit:SetHeight(vit:GetHeight() + H + GAP)
    if ES.VitalsWhere and ES.VitalsWhere() == "corner" and vit.guard then
      vit.guard:ClearAllPoints()
      vit.guard:SetPoint("TOPLEFT", ward, "BOTTOMLEFT", 0, -7)
    end
  end
end

-- The vitals are laid out by the main file; the ward goes on after, every time.
local place = ES.PlaceVitals
if place then
  function ES.PlaceVitals()
    place()
    ES.PlaceWard()
  end
end
local length = ES.DrawVitalsLength
if length then
  function ES.DrawVitalsLength()
    length()
    local w = vit.stamina.full or 100
    if math.abs(w - ward.full) >= 0.5 then
      ward.full = w
      ward:SetWidth(w)
      ward.fill:SetWidth(math.max(1, w * ward.share))
      ward.lag:SetWidth(math.max(1, w * ward.lagShare))
    end
  end
end

function ES.WardLine(f)
  local now = tonumber(f[2])
  if not now then
    if ward.shown then
      ward.shown = false
      ward:Hide()
      if ES.PlaceVitals then ES.PlaceVitals() end
    end
    return
  end
  local most = math.max(1, tonumber(f[3]) or 100)
  local broken = (tonumber(f[4]) or 0) > 0
  if not ward.shown then
    ward.shown = true
    ward:Show()
    SetShare(now / most, true)
    if ES.PlaceVitals then ES.PlaceVitals() end
  else
    SetShare(now / most)
  end
  if broken and not ward.broken then
    -- It shatters: a hard flash, then the dimmed bar.
    ward.flashUntil = GetTime() + 0.35
    ward.flashHard = true
  elseif broken then
    -- Pressed while broken: a short flash says it can't.
    ward.flashUntil = GetTime() + 0.2
    ward.flashHard = false
  elseif ward.broken then
    -- Whole again: a flare.
    ward.flashUntil = GetTime() + 0.5
    ward.flashHard = false
  end
  ward.broken = broken
  Paint()
end

ward:SetScript("OnUpdate", function()
  local now = GetTime()
  local dt = math.min(0.1, now - (this.last or now))
  this.last = now
  if this.lagShare > this.share and now >= this.lagHold then
    this.lagShare = math.max(this.share, this.lagShare - dt * 0.6)
    this.lag:SetWidth(math.max(1, this.full * this.lagShare))
  end
  if this.flashUntil then
    local left = this.flashUntil - GetTime()
    if left <= 0 then
      this.flashUntil = nil
      Paint()
    elseif this.flashHard then
      this.fill:SetVertexColor(1, 0.9, 1, 1)
      this.track:SetVertexColor(0.6, 0.3, 0.5, 0.9)
    else
      local k = 0.6 + 0.4 * left
      this.fill:SetVertexColor(COLOR.r * k + 0.4, COLOR.g * k + 0.4, COLOR.b * k + 0.3, 1)
    end
  end
end)

ES.PlaceWard()

-- --- the ward on the reticle ----------------------------------------------------------------------
-- While it's held up (WA's fifth field), the ward stands round the left of the reticle as an
-- arcane arc: whole, cracked once below two thirds of the bar, twice below a third. A blow it stops
-- shakes it and flashes it; a perfect ward flares it; a ward that breaks shatters into shards that
-- fly off and fade. Down, it fades away (the bar under stamina still shows it refilling).

local ARC_ART = "Interface\AddOns\ElwynnSurvival\art\check\\"
local ARC = 150
local arc = CreateFrame("Frame", "ElwynnSurvivalWardArc", WorldFrame)
arc:SetWidth(1)
arc:SetHeight(1)
arc:SetPoint("CENTER", WorldFrame, "CENTER", 0, 0)
arc:SetFrameStrata("HIGH")
-- Hidden at load and shown as its lines come, as the staff's mark and the checks are: a frame
-- shown before the world is up never got its updates in the lab's client.
arc:Hide()
arc.tex = arc:CreateTexture(nil, "ARTWORK")
arc.tex:SetBlendMode("ADD")
arc.tex:SetWidth(ARC)
arc.tex:SetHeight(ARC)
arc.tex:SetPoint("CENTER", arc, "CENTER", 0, 0)
arc.shards = arc:CreateTexture(nil, "OVERLAY")
arc.shards:SetTexture(ARC_ART .. "ward_shatter")
arc.shards:SetBlendMode("ADD")
arc.shards:Hide()
arc.up, arc.alpha, arc.share = false, 0, 1
arc.hitAt, arc.flareAt, arc.breakAt = nil, nil, nil
arc.stage = nil

local function ArcStage(share)
  if share > 0.66 then return "ward_whole" end
  if share > 0.33 then return "ward_crack1" end
  return "ward_crack2"
end

-- From the WA line: whether it's up, its share, and whether it just broke.
local function ArcLine(up, share, broke)
  local now = GetTime()
  -- Shown as a line comes (see above).
  arc:Show()
  if broke then
    arc.breakAt = now
    arc.up = false
    arc.alpha = 0
  elseif up and arc.up then
    if share < arc.share - 0.001 then
      arc.hitAt = now
    elseif share > arc.share + 0.001 then
      arc.flareAt = now
    end
  end
  if not broke then arc.up = up end
  arc.share = share
  local stage = ArcStage(share)
  if stage ~= arc.stage then
    arc.stage = stage
    arc.tex:SetTexture(ARC_ART .. stage)
  end
end

local wardLine = ES.WardLine
function ES.WardLine(f)
  local wasBroken = ward.broken
  wardLine(f)
  local now = tonumber(f[2])
  if not now then
    arc.up = false
    return
  end
  local most = math.max(1, tonumber(f[3]) or 100)
  local broken = (tonumber(f[4]) or 0) > 0
  ArcLine((tonumber(f[5]) or 0) > 0 and not broken, now / most, broken and not wasBroken)
end

arc:SetScript("OnUpdate", function()
  local now = GetTime()
  -- The frame's time from the clock (not arg1, which a client may leave unset).
  local dt = math.min(0.1, now - (this.last or now))
  this.last = now
  -- In and out: quick up, a little slower down.
  local want = this.up and 0.9 or 0
  local rate = this.up and 8 or 5
  this.alpha = this.alpha + (want - this.alpha) * math.min(1, dt * rate)
  local a, x, size = this.alpha, 0, ARC
  -- A stopped blow: a shake and a flash.
  if this.hitAt then
    local t = (now - this.hitAt) / 0.25
    if t >= 1 then
      this.hitAt = nil
    else
      x = math.sin(t * 40) * 5 * (1 - t)
      a = math.min(1, a + 0.5 * (1 - t))
    end
  end
  -- A perfect ward: it flares and swells.
  if this.flareAt then
    local t = (now - this.flareAt) / 0.35
    if t >= 1 then
      this.flareAt = nil
    else
      size = size * (1 + 0.12 * (1 - t))
      a = 1
    end
  end
  if a > 0.01 then
    this.tex:SetWidth(size)
    this.tex:SetHeight(size)
    this.tex:ClearAllPoints()
    this.tex:SetPoint("CENTER", this, "CENTER", x, 0)
    this.tex:SetVertexColor(1, 1, 1, a)
    this.tex:Show()
  else
    this.tex:Hide()
  end
  -- Broken: the shards fly out and fade.
  if this.breakAt then
    local t = (now - this.breakAt) / 0.55
    if t >= 1 then
      this.breakAt = nil
      this.shards:Hide()
    else
      local s = ARC * (1 + 0.35 * t)
      this.shards:SetWidth(s)
      this.shards:SetHeight(s)
      this.shards:ClearAllPoints()
      this.shards:SetPoint("CENTER", this, "CENTER", 18 * t, 0)
      this.shards:SetVertexColor(1, 1, 1, 1 - t * t)
      this.shards:Show()
    end
  end
end)
