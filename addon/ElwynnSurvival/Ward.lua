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
  if this.lagShare > this.share and GetTime() >= this.lagHold then
    this.lagShare = math.max(this.share, this.lagShare - arg1 * 0.6)
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
