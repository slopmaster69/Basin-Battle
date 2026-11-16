-- Elwynn Survival: the Bookmaker's pages. A wager's terms and the board's records on a plain dark
-- panel that stays up till it's closed (Escape, the X, or Close), each wager or record a button.
-- (Words to read, a note, a secret, the book's record, a run's tally, and what the camp's people
-- say, are told in chat: `Words.lua`.)
--
-- The server sends `NT kind title place text clue links`: kind is wager or board here; `{n}`
-- breaks a line in the text; links are `command~label` with `^` between.
--
-- Plain colors and the game's own font.
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local FACE = "Fonts\\FRIZQT__.TTF"

local W = 460
local PAD = 28
local INNER = W - 2 * PAD
local MAX_LINKS = 10
-- A page too tall for the screen is laid out wider first (a wager's rules ran off the top and
-- the foot of a laptop's), then scaled down if it still doesn't fit.
local WIDTHS = { 460, 600, 740 }
local LINK_H = 26
local LINK_W = 280

-- The colors: the title's, the text's, the faded line, the links', the panel's edge.
local WHITE = { 1, 1, 1 }
local TEXT = { 0.86, 0.86, 0.88 }
local FADED = { 0.58, 0.6, 0.64 }
local ACCENT = { 0.44, 0.78, 1 }
local EDGE = { 0.32, 0.34, 0.4 }
-- The line at each page's foot.
local FOOT = {
  wager = "Click a wager to take it. The dungeon starts fresh.",
  board = "Every server's runs, through the board's keeper.",
}

local f = CreateFrame("Frame", "ElwynnSurvivalLetter", UIParent)
f:SetWidth(W)
f:SetHeight(300)
f:SetPoint("CENTER", UIParent, "CENTER", 0, 60)
f:SetFrameStrata("DIALOG")
f:SetToplevel(true)
f:EnableMouse(true)
f:SetMovable(true)
if f.SetClampedToScreen then f:SetClampedToScreen(true) end
f:RegisterForDrag("LeftButton")
f:SetScript("OnDragStart", function() this:StartMoving() end)
f:SetScript("OnDragStop", function() this:StopMovingOrSizing() end)
f:Hide()
ES.letter = f

-- A flat rectangle of one color.
local function Fill(parent, layer, r, g, b, a)
  local t = parent:CreateTexture(nil, layer)
  t:SetTexture(r, g, b, a or 1)
  return t
end

-- The panel: dark and a little see-through, with a thin edge.
f.paper = Fill(f, "BACKGROUND", 0.05, 0.055, 0.07, 0.92)
f.paper:SetAllPoints(f)
local function Edge(p1, p2, w, h)
  local t = Fill(f, "BORDER", EDGE[1], EDGE[2], EDGE[3], 0.9)
  t:SetPoint(p1, f, p1, 0, 0)
  t:SetPoint(p2, f, p2, 0, 0)
  if w then t:SetWidth(w) end
  if h then t:SetHeight(h) end
  return t
end
Edge("TOPLEFT", "TOPRIGHT", nil, 1)
Edge("BOTTOMLEFT", "BOTTOMRIGHT", nil, 1)
Edge("TOPLEFT", "BOTTOMLEFT", 1, nil)
Edge("TOPRIGHT", "BOTTOMRIGHT", 1, nil)

local function Text(size, color, justify)
  local t = f:CreateFontString(nil, "ARTWORK")
  t:SetFont(FACE, size, "")
  t:SetTextColor(color[1], color[2], color[3])
  t:SetShadowOffset(0, 0)
  t:SetJustifyH(justify or "LEFT")
  if t.SetJustifyV then t:SetJustifyV("TOP") end
  t:SetWidth(INNER)
  return t
end

f.title = Text(18, WHITE, "CENTER")
f.place = Text(12, FADED, "CENTER")
f.rule = Fill(f, "ARTWORK", EDGE[1], EDGE[2], EDGE[3], 0.7)
f.rule:SetHeight(1)
f.body = Text(13, TEXT, "LEFT")
if f.body.SetSpacing then f.body:SetSpacing(3) end
f.foot = Text(11, FADED, "CENTER")

-- The X, top right.
f.x = CreateFrame("Button", nil, f)
f.x:SetWidth(24)
f.x:SetHeight(24)
f.x:SetPoint("TOPRIGHT", f, "TOPRIGHT", -6, -6)
f.x.label = f.x:CreateFontString(nil, "ARTWORK")
f.x.label:SetFont(FACE, 14, "")
f.x.label:SetShadowOffset(0, 0)
f.x.label:SetPoint("CENTER", f.x, "CENTER", 0, 0)
f.x.label:SetText("X")
f.x.label:SetTextColor(FADED[1], FADED[2], FADED[3])
f.x:SetScript("OnEnter", function() this.label:SetTextColor(1, 1, 1) end)
f.x:SetScript("OnLeave", function() this.label:SetTextColor(FADED[1], FADED[2], FADED[3]) end)
f.x:SetScript("OnClick", function() f:Hide() end)

-- The links: flat buttons that light under the mouse.
f.links = {}
for i = 1, MAX_LINKS do
  local b = CreateFrame("Button", nil, f)
  b:SetWidth(LINK_W)
  b:SetHeight(LINK_H)
  b.bg = Fill(b, "BACKGROUND", 0.14, 0.15, 0.19, 0.95)
  b.bg:SetAllPoints(b)
  b.label = b:CreateFontString(nil, "ARTWORK")
  b.label:SetFont(FACE, 13, "")
  b.label:SetShadowOffset(0, 0)
  b.label:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.label:SetTextColor(ACCENT[1], ACCENT[2], ACCENT[3])
  b:SetScript("OnEnter", function()
    this.bg:SetTexture(0.22, 0.25, 0.32, 1)
    this.label:SetTextColor(1, 1, 1)
  end)
  b:SetScript("OnLeave", function()
    this.bg:SetTexture(0.14, 0.15, 0.19, 0.95)
    this.label:SetTextColor(ACCENT[1], ACCENT[2], ACCENT[3])
  end)
  b:SetScript("OnClick", function()
    if this.cmd and ES.Say then
      if ES.Play then ES.Play("click") end
      ES.Say(this.cmd)
    end
  end)
  b:Hide()
  f.links[i] = b
end

-- How tall a string comes out at the page's width: the client measures it; a rough count of its
-- lines where nothing does.
local inner = INNER
local function Tall(fs, size, text)
  local h = fs:GetHeight() or 0
  if h > 1 then return h end
  local lines = 0
  local rest = (text or "") .. "\n"
  for line in string.gfind(rest, "([^\n]*)\n") do
    lines = lines + math.max(1, math.ceil(string.len(line) * size * 0.47 / inner))
  end
  return lines * (size + 4)
end

-- The page, laid out top to bottom at `w` wide and sized to what's on it.
local function Lay(kind, title, place, text, links, w)
  w = w or W
  inner = w - 2 * PAD
  f:SetWidth(w)
  local words = { f.title, f.place, f.body, f.foot }
  for i = 1, table.getn(words) do words[i]:SetWidth(inner) end
  local y = 22
  local function put(region, h, gap)
    region:ClearAllPoints()
    region:SetPoint("TOP", f, "TOP", 0, -y)
    y = y + h + (gap or 0)
  end
  f.title:SetText(title)
  put(f.title, Tall(f.title, 18, title), 4)
  f.place:SetText(place)
  if place ~= "" then
    f.place:Show()
    put(f.place, Tall(f.place, 12, place), 6)
  else
    f.place:Hide()
  end
  f.rule:ClearAllPoints()
  f.rule:SetPoint("TOPLEFT", f, "TOPLEFT", PAD, -y)
  f.rule:SetPoint("TOPRIGHT", f, "TOPRIGHT", -PAD, -y)
  y = y + 13
  f.body:SetText(text)
  put(f.body, Tall(f.body, 13, text), 14)
  local n = 0
  for i = 1, MAX_LINKS do
    if links[i] then n = n + 1 end
  end
  -- Two abreast past three (when the page is wide enough for two), else one under another.
  local cols = (n > 3 and inner >= 2 * 180 + 12) and 2 or 1
  local bw = math.min(cols == 2 and math.floor((inner - 12) / 2) or LINK_W, inner)
  local bh = LINK_H
  local gap = 6
  local top = y
  for i = 1, MAX_LINKS do
    local l = links[i]
    local b = f.links[i]
    if l then
      b.cmd = l[1]
      b:SetWidth(bw)
      if b.label.SetWidth then b.label:SetWidth(bw - 20) end
      b.label:SetText(l[2])
      local row, col = math.floor((i - 1) / cols), math.mod(i - 1, cols)
      local x = 0
      -- (An odd last one under the middle.)
      if cols == 2 and not (i == n and math.mod(n, 2) == 1) then x = (col == 0) and -(bw + 12) / 2 or (bw + 12) / 2 end
      b:ClearAllPoints()
      b:SetPoint("TOP", f, "TOP", x, -(top + row * (bh + gap)))
      b:Show()
    else
      b:Hide()
    end
  end
  if n > 0 then y = top + math.ceil(n / cols) * (bh + gap) + 8 end
  f.foot:SetText(FOOT[kind] or FOOT.wager)
  put(f.foot, 14, 0)
  f:SetHeight(math.max(140, y + 22))
end

-- Laid out at the narrowest width that fits the screen's height; past the widest, scaled down.
local function Fit(kind, title, place, text, links)
  local room = (UIParent and UIParent:GetHeight() or 768) - 60
  local roomW = (UIParent and UIParent:GetWidth() or 1024) - 40
  for i = 1, table.getn(WIDTHS) do
    local w = WIDTHS[i]
    if i == 1 or w <= roomW then
      Lay(kind, title, place, text, links, w)
      if (f:GetHeight() or 0) <= room then break end
    end
  end
  local h, w = f:GetHeight() or 1, f:GetWidth() or W
  f:SetScale(math.max(0.5, math.min(1, room / math.max(1, h), roomW / math.max(1, w))))
end

-- A page from the server.
function ES.Letter(fields)
  local kind = fields[2] or "wager"
  local text = string.gsub(fields[5] or "", "{n}", "\n")
  local links = {}
  if fields[7] and fields[7] ~= "" and ES.Split then
    local items = ES.Split(fields[7], "^")
    for i = 1, table.getn(items) do
      local l = ES.Split(items[i], "~")
      if l[1] and l[1] ~= "" then table.insert(links, { l[1], l[2] or l[1] }) end
    end
  end
  Fit(kind, fields[3] or "", fields[4] or "", text, links)
  if not f:IsShown() then
    PlaySound("igQuestListOpen")
    f.switched = ES.mode ~= "cursor"
    f:Show()
    if ES.WindowMode then ES.WindowMode() end
  end
  ES.letterShown = { kind = kind, title = fields[3] or "", links = table.getn(links), height = f:GetHeight() }
end

f:SetScript("OnHide", function()
  -- Hidden with the whole interface (the map): it's still open.
  if this:IsShown() then return end
  PlaySound("igQuestListClose")
  -- Back to the fight, if opening it took the cursor and nothing else wants it now.
  if this.switched and ES.awake and ES.SetMode and not (ES.CursorWanted and ES.CursorWanted()) then
    ES.SetMode("action")
  end
  this.switched = nil
end)

-- Escape closes it, as it does the game's own windows.
if UISpecialFrames then
  table.insert(UISpecialFrames, "ElwynnSurvivalLetter")
end
