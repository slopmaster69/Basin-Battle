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
-- slip shows it and a Ready button (`.es ready`). Then `split~best^...`, in seconds, for each boss:
-- when it fell this run, and in the camp's quickest run in time at this level; a slain boss's row
-- shows its split and how far ahead (green) or behind (red) that best it is.
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local TITLE = "Fonts\\FRIZQT__.TTF"
local BODY = "Fonts\\FRIZQT__.TTF"
local DEATH = 10
-- The slip's colors, on a dark panel: the white of a title, the pale text, the faded (slain) line,
-- gold for the clock while there's room, red once it's over, green for a win.
local WHITE = { 1, 1, 1 }
local TEXT = { 0.86, 0.86, 0.88 }
local FADED = { 0.55, 0.57, 0.6 }
local GOLD = { 1, 0.82, 0.3 }
local AMBER = { 1, 0.62, 0.2 }
local RED = { 1, 0.35, 0.3 }
local GREEN = { 0.45, 0.85, 0.4 }

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

-- The slip: a dark, a little see-through panel with a thin edge, at any height.
f.paper = f:CreateTexture(nil, "BACKGROUND")
f.paper:SetTexture(0.05, 0.055, 0.07, 0.85)
f.paper:SetAllPoints(f)
local function Edge(p1, p2, w, h)
  local t = f:CreateTexture(nil, "BORDER")
  t:SetTexture(0.32, 0.34, 0.4, 0.9)
  t:SetPoint(p1, f, p1, 0, 0)
  t:SetPoint(p2, f, p2, 0, 0)
  if w then t:SetWidth(w) end
  if h then t:SetHeight(h) end
end
Edge("TOPLEFT", "TOPRIGHT", nil, 1)
Edge("BOTTOMLEFT", "BOTTOMRIGHT", nil, 1)
Edge("TOPLEFT", "BOTTOMLEFT", 1, nil)
Edge("TOPRIGHT", "BOTTOMRIGHT", 1, nil)
-- The words in from the sides.
local PAD = 8

local function Text(face, size, c)
  local t = f:CreateFontString(nil, "ARTWORK")
  t:SetFont(face, size, "")
  t:SetTextColor(c[1], c[2], c[3])
  t:SetShadowOffset(0, 0)
  t:SetJustifyH("CENTER")
  t:SetWidth(250 - 2 * PAD - 4)
  return t
end

f.title = Text(TITLE, 13, WHITE)
f.title:SetPoint("TOP", f, "TOP", 0, -PAD - 4)
f.clock = Text(TITLE, 22, GOLD)
f.clock:SetPoint("TOP", f.title, "BOTTOM", 0, -2)
f.falls = Text(BODY, 11, FADED)
f.falls:SetPoint("TOP", f.clock, "BOTTOM", 0, -1)
f.terms = Text(BODY, 11, AMBER)
f.terms:SetPoint("TOP", f.falls, "BOTTOM", 0, -1)
f.bossHead = Text(TITLE, 12, WHITE)
f.bosses = {}
local ROW = 16
-- The count of foes: a word and a bar (a dark trough, a gold fill, green once it's enough).
f.forces = Text(TITLE, 12, WHITE)
f.forceBar = CreateFrame("Frame", nil, f)
f.forceBar:SetWidth(180)
f.forceBar:SetHeight(8)
f.forceBar.back = f.forceBar:CreateTexture(nil, "BACKGROUND")
f.forceBar.back:SetAllPoints(f.forceBar)
f.forceBar.back:SetTexture(0.2, 0.22, 0.27, 0.8)
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
f.ready.back:SetTexture(0.14, 0.15, 0.19, 0.95)
f.ready.label = f.ready:CreateFontString(nil, "ARTWORK")
f.ready.label:SetFont(TITLE, 12, "")
f.ready.label:SetPoint("CENTER", f.ready, "CENTER", 0, 0)
f.ready.label:SetTextColor(0.44, 0.78, 1)
f.ready.label:SetText("Ready")
f.ready:SetScript("OnEnter", function() this.back:SetTexture(0.22, 0.25, 0.32, 1) end)
f.ready:SetScript("OnLeave", function() this.back:SetTexture(0.14, 0.15, 0.19, 0.95) end)
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
  f.forces:SetText("Enemies  " .. fc.felled .. " / " .. fc.needed)
  f.forces:Show()
  f.forceBar:ClearAllPoints()
  f.forceBar:SetPoint("TOP", f.forces, "BOTTOM", 0, -3)
  local share = 1
  if fc.needed > 0 then share = math.min(1, fc.felled / fc.needed) end
  f.forceBar.fill:SetWidth(math.max(1, 180 * share))
  if share >= 1 then
    f.forceBar.fill:SetVertexColor(0.4, 0.75, 0.35, 0.95)
  else
    f.forceBar.fill:SetVertexColor(1, 0.82, 0.3, 0.95)
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
      row = Text(BODY, 12, TEXT)
      f.bosses[i] = row
    end
    if row then
      if i <= n then
        row:ClearAllPoints()
        row:SetPoint("TOP", f.bossHead, "BOTTOM", 0, -2 - (i - 1) * ROW)
        if list[i].down and list[i].split then
          local text, r, g, b = list[i].name .. "  " .. ES.WagerClock(list[i].split), FADED[1], FADED[2], FADED[3]
          if list[i].best then
            local d = list[i].split - list[i].best
            if d <= 0 then
              text = text .. "  (-" .. ES.WagerClock(-d) .. ")"
              r, g, b = GREEN[1], GREEN[2], GREEN[3]
            else
              text = text .. "  (+" .. ES.WagerClock(d) .. ")"
              r, g, b = RED[1], RED[2], RED[3]
            end
          end
          row:SetText(text)
          row:SetTextColor(r, g, b)
        elseif list[i].down then
          row:SetText(list[i].name .. "  - slain")
          row:SetTextColor(FADED[1], FADED[2], FADED[3])
        else
          row:SetText(list[i].name)
          row:SetTextColor(TEXT[1], TEXT[2], TEXT[3])
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
ES.WagerClock = Clock

-- The last line, and when it came (the clock runs on between lines).
local last

local function Draw()
  if not last then return end
  local elapsed = last.elapsed
  if last.state == "run" then elapsed = elapsed + (GetTime() - last.at) end
  f.clock:SetText(Clock(elapsed) .. "  /  " .. Clock(last.limit))
  if last.state == "over" or elapsed > last.limit then
    f.clock:SetTextColor(RED[1], RED[2], RED[3])
  elseif last.state == "won" then
    f.clock:SetTextColor(GREEN[1], GREEN[2], GREEN[3])
  elseif elapsed > last.limit * 0.8 then
    f.clock:SetTextColor(AMBER[1], AMBER[2], AMBER[3])
  else
    f.clock:SetTextColor(GOLD[1], GOLD[2], GOLD[3])
  end
  -- (An older server sends no ready field: its clock starts when someone leaves the lamp.)
  local words = { wait = "The clock starts when you leave the lamp", run = "", won = "Won in time", over = "Over time" }
  local rd = last.ready
  if last.state == "wait" and rd then
    if rd.count then
      words.wait = "Everyone is ready. The clock starts in " .. math.max(0, math.ceil(rd.count - (GetTime() - last.at)))
    else
      words.wait = "Ready " .. rd.ready .. " of " .. rd.heroes .. ". Step off the lamp to start"
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
  -- Each boss's split and the best to beat, in the bosses' order.
  if fields[12] and fields[12] ~= "" then
    local k, rest = 1, fields[12]
    while rest and k <= table.getn(list) do
      local i = string.find(rest, "^", 1, true)
      local part = i and string.sub(rest, 1, i - 1) or rest
      rest = i and string.sub(rest, i + 1) or nil
      local _, _, split, best = string.find(part, "^(%d*)~(%d*)$")
      list[k].split = tonumber(split or "")
      list[k].best = tonumber(best or "")
      k = k + 1
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
    f.ready:SetPoint("TOPRIGHT", f, "TOPRIGHT", -PAD - 2, -PAD - 2)
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
  f:SetHeight((last.terms ~= "" and 96 or 80) + extra + PAD)
  Draw()
  f:Show()
  ES.wagerShown = last
end
