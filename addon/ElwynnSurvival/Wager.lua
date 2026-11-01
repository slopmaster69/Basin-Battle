-- Elwynn Survival: the Bookmaker's clock. While a wager is on below, a slip at the right of the
-- screen: the level and the dungeon, the time against the time to beat (gold while there's room,
-- red once it's over), the falls and what they've cost, and the wager's terms.
--
-- Under them, the bosses the run has to beat, each crossed off as it falls, so you know what's
-- left and when it ends.
--
-- The server sends `WT name level elapsed limit falls state terms bosses` (state: wait, run, won,
-- over; bosses `name~down^...`, missing from older servers), and a bare `WT` when there's no wager.
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
f.bossHead = Text(TITLE, 13, 0.165, 0.13, 0.094)
f.bosses = {}
local ROW = 16

-- The boss list under the rest: "Bosses 1 / 3", then each name, a slain one greyed and marked.
local function DrawBosses(list, below)
  local n, down = table.getn(list), 0
  for i = 1, n do if list[i].down then down = down + 1 end end
  if n == 0 then
    f.bossHead:Hide()
  else
    f.bossHead:ClearAllPoints()
    f.bossHead:SetPoint("TOP", below, "BOTTOM", 0, -6)
    f.bossHead:SetText("Bosses  " .. down .. " / " .. n)
    f.bossHead:Show()
  end
  for i = 1, math.max(n, table.getn(f.bosses)) do
    local row = f.bosses[i]
    if not row and i <= n then
      row = Text(BODY, 13, 0.165, 0.13, 0.094)
      f.bosses[i] = row
    end
    if row then
      if i <= n then
        row:ClearAllPoints()
        row:SetPoint("TOP", f.bossHead, "BOTTOM", 0, -2 - (i - 1) * ROW)
        if list[i].down then
          row:SetText(list[i].name .. "  - slain")
          row:SetTextColor(0.5, 0.45, 0.38)
        else
          row:SetText(list[i].name)
          row:SetTextColor(0.165, 0.13, 0.094)
        end
        row:Show()
      else
        row:Hide()
      end
    end
  end
  if n == 0 then return 0 end
  return 24 + n * ROW
end

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
  local list = {}
  if fields[9] and fields[9] ~= "" then
    local rest = fields[9]
    while rest and rest ~= "" do
      local i = string.find(rest, "^", 1, true)
      local part = i and string.sub(rest, 1, i - 1) or rest
      rest = i and string.sub(rest, i + 1) or nil
      local _, _, name, down = string.find(part, "^(.-)~(%d)$")
      if name then table.insert(list, { name = name, down = down == "1" }) end
    end
  end
  last.bosses = list
  f.title:SetText("+" .. last.level .. "  " .. last.name)
  f.terms:SetText(last.terms)
  local extra = DrawBosses(list, last.terms ~= "" and f.terms or f.falls)
  f:SetHeight((last.terms ~= "" and 96 or 80) + extra)
  Draw()
  f:Show()
  ES.wagerShown = last
end
