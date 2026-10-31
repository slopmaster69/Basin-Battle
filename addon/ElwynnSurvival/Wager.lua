-- Elwynn Survival: the Bookmaker's clock. While a wager is on below, a slip at the right of the
-- screen: the level and the dungeon, the time against the time to beat (gold while there's room,
-- red once it's over), the falls and what they've cost, and the wager's terms.
--
-- The server sends `WT name level elapsed limit falls state terms` (state: wait, run, won, over),
-- and a bare `WT` when there's no wager.
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local A = "Interface\\AddOns\\ElwynnSurvival\\art\\ledger\\"
local FONTS = "Interface\\AddOns\\ElwynnSurvival\\fonts\\"
local TITLE = FONTS .. "IMFellEnglishSC-Regular.ttf"
local BODY = FONTS .. "CrimsonPro-Regular.ttf"
local DEATH = 10

local f = CreateFrame("Frame", "ElwynnSurvivalWager", UIParent)
f:SetWidth(250)
f:SetHeight(96)
f:SetPoint("TOPRIGHT", UIParent, "TOPRIGHT", -24, -230)
f:SetFrameStrata("MEDIUM")
f:Hide()
ES.wagerFrame = f

f.paper = f:CreateTexture(nil, "BACKGROUND")
f.paper:SetTexture(A .. "scrap_note_b")
f.paper:SetPoint("TOPLEFT", f, "TOPLEFT", -14, 10)
f.paper:SetPoint("BOTTOMRIGHT", f, "BOTTOMRIGHT", 14, -10)

local function Text(face, size, r, g, b)
  local t = f:CreateFontString(nil, "ARTWORK")
  t:SetFont(face, size, "")
  t:SetTextColor(r, g, b)
  t:SetShadowOffset(0, 0)
  t:SetJustifyH("CENTER")
  t:SetWidth(240)
  return t
end

f.title = Text(TITLE, 14, 0.165, 0.13, 0.094)
f.title:SetPoint("TOP", f, "TOP", 0, -8)
f.clock = Text(TITLE, 24, 0.55, 0.38, 0.08)
f.clock:SetPoint("TOP", f.title, "BOTTOM", 0, -2)
f.falls = Text(BODY, 13, 0.353, 0.275, 0.188)
f.falls:SetPoint("TOP", f.clock, "BOTTOM", 0, -1)
f.terms = Text(BODY, 12, 0.604, 0.169, 0.125)
f.terms:SetPoint("TOP", f.falls, "BOTTOM", 0, -1)

local function Clock(secs)
  secs = math.max(0, math.floor(secs or 0))
  return string.format("%d:%02d", math.floor(secs / 60), math.mod(secs, 60))
end

-- The last line, and when it came (the clock runs on between lines).
local last

local function Draw()
  if not last then return end
  local elapsed = last.elapsed
  if last.state == "run" then elapsed = elapsed + (GetTime() - last.at) end
  f.clock:SetText(Clock(elapsed) .. "  /  " .. Clock(last.limit))
  if last.state == "over" or elapsed > last.limit then
    f.clock:SetTextColor(0.7, 0.12, 0.08)
  elseif last.state == "won" then
    f.clock:SetTextColor(0.24, 0.42, 0.14)
  elseif elapsed > last.limit * 0.8 then
    f.clock:SetTextColor(0.62, 0.3, 0.08)
  else
    f.clock:SetTextColor(0.55, 0.38, 0.08)
  end
  local words = { wait = "The clock starts when you leave the lamp", run = "", won = "Won in time", over = "Over time" }
  local falls = ""
  if last.falls > 0 then falls = last.falls .. " fall" .. (last.falls == 1 and "" or "s") .. " (+" .. Clock(last.falls * DEATH) .. ")" end
  local w = words[last.state] or ""
  if w ~= "" and falls ~= "" then w = w .. " · " .. falls elseif falls ~= "" then w = falls end
  f.falls:SetText(w)
end

f:SetScript("OnUpdate", function()
  this.t = (this.t or 0) + (arg1 or 0)
  if this.t < 0.25 then return end
  this.t = 0
  Draw()
end)

function ES.WagerLine(fields)
  if not fields[2] or fields[2] == "" then
    last = nil
    f:Hide()
    return
  end
  last = {
    name = fields[2],
    level = tonumber(fields[3]) or 1,
    elapsed = tonumber(fields[4]) or 0,
    limit = tonumber(fields[5]) or 0,
    falls = tonumber(fields[6]) or 0,
    state = fields[7] or "run",
    terms = fields[8] or "",
    at = GetTime(),
  }
  f.title:SetText("+" .. last.level .. "  " .. last.name)
  f.terms:SetText(last.terms)
  f:SetHeight(last.terms ~= "" and 96 or 80)
  Draw()
  f:Show()
  ES.wagerShown = last
end
