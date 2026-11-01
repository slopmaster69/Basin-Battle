-- Elwynn Survival: the Bookmaker's clock. While a wager is on below, a slip at the right of the
-- screen: the level and the dungeon, the time against the time to beat (gold while there's room,
-- red once it's over), the falls and what they've cost, and the wager's terms.
--
-- Under them, the bosses the run has to beat, each crossed off as it falls, so you know what's
-- left and when it ends; then the count of foes felled against what the run needs (a bar, filled
-- gold, green once there are enough).
--
-- The server sends `WT name level elapsed limit falls state terms bosses forces` (state: wait, run,
-- won, over; bosses `name~down^...` and forces `felled~needed~total`, missing from older servers),
-- and a bare `WT` when there's no wager. Before the clock, a last field `ready~heroes~count`: who's
-- said they're ready, how many are down there, the count's seconds left once everyone has; the
-- slip shows it and a Ready button (`.es ready`).
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local A = "Interface\\AddOns\\ElwynnSurvival\\art\\ledger\\"
local FONTS = "Interface\\AddOns\\ElwynnSurvival\\fonts\\"
local TITLE = FONTS .. "IMFellEnglishSC-Regular.ttf"
local BODY = FONTS .. "CrimsonPro-Regular.ttf"
local DEATH = 10

-- A dungeon's tier in words, for the book's list (BK's last field; nil when it's not there).
local TIERS = { "Tier I, a first dungeon", "Tier II, a harder one", "Tier III, among the hardest" }
function ES.TierWords(t)
  return TIERS[tonumber(t or "") or 0]
end

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
-- The count of foes: a word and a bar (an ink trough, a gold fill, green once it's enough).
f.forces = Text(TITLE, 13, 0.165, 0.13, 0.094)
f.forceBar = CreateFrame("Frame", nil, f)
f.forceBar:SetWidth(180)
f.forceBar:SetHeight(8)
f.forceBar.back = f.forceBar:CreateTexture(nil, "BACKGROUND")
f.forceBar.back:SetAllPoints(f.forceBar)
f.forceBar.back:SetTexture(0.25, 0.2, 0.14, 0.35)
f.forceBar.fill = f.forceBar:CreateTexture(nil, "ARTWORK")
f.forceBar.fill:SetPoint("TOPLEFT", f.forceBar, "TOPLEFT", 0, 0)
f.forceBar.fill:SetPoint("BOTTOMLEFT", f.forceBar, "BOTTOMLEFT", 0, 0)
f.forceBar.fill:SetTexture(1, 1, 1, 1)
f.forces:Hide()
f.forceBar:Hide()

-- Before the clock: a Ready button (`.es ready`) under the title.
f.ready = CreateFrame("Button", "ElwynnSurvivalWagerReady", f)
f.ready:SetWidth(110)
f.ready:SetHeight(22)
f.ready.back = f.ready:CreateTexture(nil, "BACKGROUND")
f.ready.back:SetAllPoints(f.ready)
f.ready.back:SetTexture(A .. "button_normal")
f.ready.label = f.ready:CreateFontString(nil, "ARTWORK")
f.ready.label:SetFont(TITLE, 13, "")
f.ready.label:SetPoint("CENTER", f.ready, "CENTER", 0, 0)
f.ready.label:SetTextColor(0.953, 0.902, 0.784)
f.ready.label:SetText("Ready")
f.ready:SetScript("OnEnter", function() this.back:SetTexture(A .. "button_hover") end)
f.ready:SetScript("OnLeave", function() this.back:SetTexture(A .. "button_normal") end)
f.ready:SetScript("OnClick", function()
  if ES.Play then ES.Play("click") end
  if ES.Say then ES.Say(".es ready") end
end)
f.ready:Hide()

-- The count under the bosses: "Foes 42 / 60", the bar full at the count needed. Its height.
local function DrawForces(fc, below)
  if not fc then
    f.forces:Hide()
    f.forceBar:Hide()
    return 0
  end
  f.forces:ClearAllPoints()
  f.forces:SetPoint("TOP", below, "BOTTOM", 0, -6)
  f.forces:SetText("Foes  " .. fc.felled .. " / " .. fc.needed)
  f.forces:Show()
  f.forceBar:ClearAllPoints()
  f.forceBar:SetPoint("TOP", f.forces, "BOTTOM", 0, -3)
  local share = 1
  if fc.needed > 0 then share = math.min(1, fc.felled / fc.needed) end
  f.forceBar.fill:SetWidth(math.max(1, 180 * share))
  if share >= 1 then
    f.forceBar.fill:SetVertexColor(0.3, 0.5, 0.18, 0.95)
  else
    f.forceBar.fill:SetVertexColor(0.72, 0.537, 0.184, 0.95)
  end
  f.forceBar:Show()
  return 34
end

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
  local words = { wait = "Ready up: the clock starts when everyone is", run = "", won = "Won in time", over = "Over time" }
  local rd = last.ready
  if last.state == "wait" and rd then
    if rd.count then
      words.wait = "Everyone's ready: the clock starts in " .. math.max(0, math.ceil(rd.count - (GetTime() - last.at)))
    else
      words.wait = "Ready " .. rd.ready .. " of " .. rd.heroes .. ", or step off from the lamp to start"
    end
  end
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
  local fc
  if fields[10] and fields[10] ~= "" then
    local _, _, felled, needed, total = string.find(fields[10], "^(%d+)~(%d+)~(%d+)$")
    if felled then fc = { felled = tonumber(felled), needed = tonumber(needed), total = tonumber(total) } end
  end
  last.forces = fc
  local rd
  if fields[11] and fields[11] ~= "" then
    local _, _, r, n, c = string.find(fields[11], "^(%d+)~(%d+)~(%d*)$")
    if r then rd = { ready = tonumber(r), heroes = tonumber(n), count = tonumber(c) } end
  end
  last.ready = rd
  if last.state == "wait" and rd and not rd.count then
    f.ready:ClearAllPoints()
    f.ready:SetPoint("TOPRIGHT", f, "TOPRIGHT", -6, -6)
    f.ready:Show()
  else
    f.ready:Hide()
  end
  f.title:SetText("+" .. last.level .. "  " .. last.name)
  f.terms:SetText(last.terms)
  local above = last.terms ~= "" and f.terms or f.falls
  local extra = DrawBosses(list, above)
  local lastBoss = list[table.getn(list)] and f.bosses[table.getn(list)] or above
  extra = extra + DrawForces(fc, lastBoss)
  f:SetHeight((last.terms ~= "" and 96 or 80) + extra)
  Draw()
  f:Show()
  ES.wagerShown = last
end
