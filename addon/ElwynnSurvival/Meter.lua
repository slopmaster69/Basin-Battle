-- Elwynn Survival: the damage meter. A small panel at the right of the screen with two tabs:
--
--   Fight: you and the heroes near you fighting with you, in the fight you're in (or were last
--          in), the most damage first, with how long it has run.
--   You:   your own damage (or healing) by card since you last reset it, with your totals, your
--          per-second over your time in fights, and the damage you took.
--
-- A button between Damage and Healing; Reset (on You) starts your own count over. Drag the title
-- to move the panel; click it to fold it to its title. /esmeter hides or shows it.
--
-- The server sends, about once a second while they change:
--   DP secs dealt healed taken source~dealt~healed^...
--   DM secs live name~class~dealt~healed~taken~you^...
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local FONTS = "Interface\\AddOns\\ElwynnSurvival\\fonts\\"
local TITLE = FONTS .. "IMFellEnglishSC-Regular.ttf"
local BODY = FONTS .. "CrimsonPro-Regular.ttf"
local BAR = "Interface\\TargetingFrame\\UI-StatusBar"
local ROWS = 8
local ROW_H = 16
local W = 230
local TOP = 44

local CLASS = {
  warrior = { 0.78, 0.61, 0.43 }, paladin = { 0.96, 0.55, 0.73 }, hunter = { 0.67, 0.83, 0.45 },
  rogue = { 1, 0.96, 0.41 }, priest = { 0.95, 0.95, 0.95 }, shaman = { 0.0, 0.44, 0.87 },
  mage = { 0.41, 0.8, 0.94 }, warlock = { 0.58, 0.51, 0.79 }, druid = { 1, 0.49, 0.04 },
}
local SOURCE = { 0.85, 0.62, 0.3 }
local HEAL = { 0.35, 0.8, 0.35 }

local m = { tab = "fight", heal = false, folded = false, own = nil, fight = nil }
ES.meter = m

local function Settings()
  ElwynnSurvivalSettings = ElwynnSurvivalSettings or {}
  return ElwynnSurvivalSettings
end

-- 12345 as 12.3k, 1234567 as 1.23m.
local function Short(n)
  n = n or 0
  if n >= 1000000 then return string.format("%.2fm", n / 1000000) end
  if n >= 10000 then return string.format("%.1fk", n / 1000) end
  return tostring(math.floor(n + 0.5))
end

local function Clock(secs)
  secs = math.max(0, math.floor(secs or 0))
  return string.format("%d:%02d", math.floor(secs / 60), math.mod(secs, 60))
end

local f = CreateFrame("Frame", "ElwynnSurvivalMeter", UIParent)
f:SetWidth(W)
f:SetHeight(TOP)
f:SetFrameStrata("LOW")
if f.SetClampedToScreen then f:SetClampedToScreen(true) end
f:EnableMouse(true)
f:SetMovable(true)
f:RegisterForDrag("LeftButton")
f:Hide()
ES.meterFrame = f

f.bg = f:CreateTexture(nil, "BACKGROUND")
f.bg:SetTexture(0.05, 0.04, 0.03, 0.72)
f.bg:SetPoint("TOPLEFT", f, "TOPLEFT", -6, 6)
f.bg:SetPoint("BOTTOMRIGHT", f, "BOTTOMRIGHT", 6, -6)
f.edge = f:CreateTexture(nil, "BORDER")
f.edge:SetTexture(0.55, 0.42, 0.22, 0.9)
f.edge:SetHeight(1)
f.edge:SetPoint("TOPLEFT", f, "TOPLEFT", -6, 6)
f.edge:SetPoint("TOPRIGHT", f, "TOPRIGHT", 6, 6)

local function Text(parent, face, size, r, g, b)
  local t = parent:CreateFontString(nil, "OVERLAY")
  t:SetFont(face, size, "OUTLINE")
  t:SetTextColor(r, g, b)
  t:SetShadowOffset(0, 0)
  return t
end

f.title = Text(f, TITLE, 14, 0.95, 0.82, 0.5)
f.title:SetPoint("TOPLEFT", f, "TOPLEFT", 0, -1)
f.sub = Text(f, BODY, 11, 0.78, 0.72, 0.6)
f.sub:SetPoint("TOPLEFT", f, "TOPLEFT", 0, -20)
f.sub:SetWidth(W)
f.sub:SetJustifyH("LEFT")

-- Drag the panel by anywhere on it; a click without a drag folds it.
f:SetScript("OnDragStart", function() this.dragged = true this:StartMoving() end)
f:SetScript("OnDragStop", function()
  this:StopMovingOrSizing()
  local x, y = this:GetLeft(), this:GetBottom()
  this:ClearAllPoints()
  this:SetPoint("BOTTOMLEFT", UIParent, "BOTTOMLEFT", x, y)
  Settings().meterPos = { x = x, y = y }
end)
f:SetScript("OnMouseUp", function()
  if this.dragged then this.dragged = nil return end
  m.folded = not m.folded
  ES.DrawMeter()
end)

local function Button(label, w, onClick, tip)
  local b = CreateFrame("Button", nil, f)
  b:SetWidth(w)
  b:SetHeight(16)
  b.bg = b:CreateTexture(nil, "BACKGROUND")
  b.bg:SetAllPoints(b)
  b.bg:SetTexture(0.2, 0.15, 0.1, 0.8)
  b.label = Text(b, BODY, 11, 0.8, 0.75, 0.6)
  b.label:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.label:SetText(label)
  b:SetScript("OnClick", onClick)
  b:SetScript("OnEnter", function()
    GameTooltip:SetOwner(this, "ANCHOR_LEFT")
    GameTooltip:SetText(tip, 1, 0.9, 0.7, 1, 1)
    GameTooltip:Show()
  end)
  b:SetScript("OnLeave", function() GameTooltip:Hide() end)
  return b
end

f.you = Button("You", 38, function() m.tab = "own" m.folded = false ES.DrawMeter() end,
  "Your own damage or healing by card, since you last reset it.")
f.you:SetPoint("TOPRIGHT", f, "TOPRIGHT", 0, 0)
f.fightTab = Button("Fight", 42, function() m.tab = "fight" m.folded = false ES.DrawMeter() end,
  "You and the heroes fighting with you, in this fight (or the last one).")
f.fightTab:SetPoint("RIGHT", f.you, "LEFT", -3, 0)
f.mode = Button("Healing", 52, function() m.heal = not m.heal m.folded = false ES.DrawMeter() end,
  "Show healing instead of damage, or back again.")
f.mode:SetPoint("RIGHT", f.fightTab, "LEFT", -3, 0)
f.reset = Button("Reset", 42, function() if ES.Say then ES.Say(".es meter reset") end end,
  "Start your own count over (the fight's stays).")
f.reset:SetPoint("TOPRIGHT", f.you, "BOTTOMRIGHT", 0, -3)

f.rows = {}
for i = 1, ROWS do
  local r = CreateFrame("StatusBar", nil, f)
  r:SetHeight(ROW_H - 2)
  r:SetPoint("TOPLEFT", f, "TOPLEFT", 0, -TOP - (i - 1) * ROW_H)
  r:SetPoint("TOPRIGHT", f, "TOPRIGHT", 0, -TOP - (i - 1) * ROW_H)
  r:SetStatusBarTexture(BAR)
  r:SetMinMaxValues(0, 1)
  r.back = r:CreateTexture(nil, "BACKGROUND")
  r.back:SetAllPoints(r)
  r.back:SetTexture(0, 0, 0, 0.35)
  r.name = Text(r, BODY, 11, 1, 1, 1)
  r.name:SetPoint("LEFT", r, "LEFT", 4, 0)
  r.value = Text(r, BODY, 11, 1, 1, 1)
  r.value:SetPoint("RIGHT", r, "RIGHT", -4, 0)
  r:Hide()
  f.rows[i] = r
end

local function Place()
  local p = Settings().meterPos
  f:ClearAllPoints()
  if p and p.x and p.y then
    f:SetPoint("BOTTOMLEFT", UIParent, "BOTTOMLEFT", p.x, p.y)
  else
    f:SetPoint("BOTTOMRIGHT", UIParent, "BOTTOMRIGHT", -30, 240)
  end
end
Place()

-- The rows to show: { name, n, col, you }.
local function List()
  local out = {}
  if m.tab == "fight" then
    local fight = m.fight
    if not fight then return out end
    for i = 1, table.getn(fight.rows) do
      local e = fight.rows[i]
      local n = m.heal and e.healed or e.dealt
      if n > 0 or e.you then
        table.insert(out, { name = e.name, n = n, col = CLASS[e.class] or SOURCE, you = e.you })
      end
    end
  else
    local own = m.own
    if not own then return out end
    for i = 1, table.getn(own.rows) do
      local e = own.rows[i]
      local n = m.heal and e.healed or e.dealt
      if n > 0 then table.insert(out, { name = e.name, n = n, col = m.heal and HEAL or SOURCE }) end
    end
  end
  table.sort(out, function(a, b) return a.n > b.n end)
  return out
end

function ES.DrawMeter()
  if Settings().meterHidden or (not m.own and not m.fight) then
    f:Hide()
    return
  end
  local word = m.heal and "Healing" or "Damage"
  f.title:SetText(word)
  f.mode.label:SetText(m.heal and "Damage" or "Healing")
  local on, off = { 1, 0.85, 0.45 }, { 0.55, 0.5, 0.42 }
  local a = m.tab == "fight" and on or off
  local b = m.tab == "own" and on or off
  f.fightTab.label:SetTextColor(a[1], a[2], a[3])
  f.you.label:SetTextColor(b[1], b[2], b[3])
  local sub = ""
  if m.tab == "fight" then
    local fight = m.fight
    if fight then
      local total = 0
      for i = 1, table.getn(fight.rows) do
        local e = fight.rows[i]
        total = total + (m.heal and e.healed or e.dealt)
      end
      sub = (fight.live and "This fight " or "Last fight ") .. Clock(fight.secs) .. "  ·  " .. Short(total) .. "  ·  " .. Short(total / math.max(1, fight.secs)) .. "/s"
    else
      sub = "No fight yet"
    end
    f.reset:Hide()
  else
    local own = m.own
    if own then
      local n = m.heal and own.healed or own.dealt
      sub = Short(n) .. "  ·  " .. Short(n / math.max(1, own.secs)) .. "/s over " .. Clock(own.secs) .. "  ·  took " .. Short(own.taken)
    end
    f.reset:Show()
  end
  f.sub:SetText(sub)
  local list = List()
  local top, sum = 0, 0
  for i = 1, table.getn(list) do
    if list[i].n > top then top = list[i].n end
    sum = sum + list[i].n
  end
  local shown = 0
  for i = 1, ROWS do
    local r = f.rows[i]
    local e = list[i]
    if e and not m.folded then
      shown = i
      r:SetStatusBarColor(e.col[1], e.col[2], e.col[3], e.you and 0.9 or 0.65)
      r:SetValue(top > 0 and e.n / top or 0)
      r.name:SetText(e.name)
      local pct = sum > 0 and math.floor(e.n * 100 / sum + 0.5) or 0
      r.value:SetText(Short(e.n) .. "  " .. pct .. "%")
      r:Show()
    else
      r:Hide()
    end
  end
  local h = TOP - 6 + shown * ROW_H
  if m.folded then h = 18 end
  f.sub:SetAlpha(m.folded and 0 or 1)
  f:SetHeight(h)
  f:Show()
end

local function Num(s) return tonumber(s or "") or 0 end

-- `DP secs dealt healed taken source~dealt~healed^...`
function ES.OwnMeter(fields)
  local own = { secs = Num(fields[2]), dealt = Num(fields[3]), healed = Num(fields[4]), taken = Num(fields[5]), rows = {} }
  local list = ES.Split(fields[6] or "", "^")
  for i = 1, table.getn(list) do
    local p = ES.Split(list[i], "~")
    if p[1] and p[1] ~= "" then
      table.insert(own.rows, { name = p[1], dealt = Num(p[2]), healed = Num(p[3]) })
    end
  end
  m.own = own
  ES.DrawMeter()
end

-- `DM secs live name~class~dealt~healed~taken~you^...`
function ES.FightMeter(fields)
  local fight = { secs = Num(fields[2]), live = fields[3] == "1", rows = {} }
  local list = ES.Split(fields[4] or "", "^")
  for i = 1, table.getn(list) do
    local p = ES.Split(list[i], "~")
    if p[1] and p[1] ~= "" then
      table.insert(fight.rows, { name = p[1], class = p[2] or "", dealt = Num(p[3]), healed = Num(p[4]), taken = Num(p[5]), you = p[6] == "1" })
    end
  end
  m.fight = fight
  ES.DrawMeter()
end

function ES.ToggleMeter()
  local s = Settings()
  s.meterHidden = not s.meterHidden
  if s.meterHidden then
    DEFAULT_CHAT_FRAME:AddMessage("Elwynn Survival: the damage meter is hidden (/esmeter to show it).")
  else
    DEFAULT_CHAT_FRAME:AddMessage("Elwynn Survival: the damage meter is shown.")
  end
  ES.DrawMeter()
end

SLASH_ELWYNNMETER1 = "/esmeter"
if SlashCmdList then SlashCmdList["ELWYNNMETER"] = ES.ToggleMeter end
