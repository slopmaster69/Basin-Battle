-- Elwynn Survival: the letter. A page to read at leisure: a note found below, a secret come upon,
-- the book's record of a dungeon (what's found, the rumours of the rest, every note to read
-- again). Parchment that stays up till it's closed (Escape, the ink cross, or Close), and a
-- clue in red ink under a note while what it points to is still hidden.
--
-- The server sends `NT kind title place text clue links`: kind is note, secret or lore; `{n}`
-- breaks a line in the text; links are `command~label` with `^` between.
--
-- It loads after Ledger.lua and wears its art and fonts (a plain dark page without it).
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local A = "Interface\\AddOns\\ElwynnSurvival\\art\\ledger\\"
local FONTS = "Interface\\AddOns\\ElwynnSurvival\\fonts\\"
local TITLE = FONTS .. "IMFellEnglishSC-Regular.ttf"
local SERIF = FONTS .. "IMFellEnglish-Regular.ttf"
local BODY = FONTS .. "CrimsonPro-Regular.ttf"

local W = 460
local PAD = 36
local INNER = W - 2 * PAD
local MAX_LINKS = 10

-- The inks: the page's, a faded one, the clue's red, the links' blue.
local INK = { 0.165, 0.13, 0.094 }
local SEPIA = { 0.353, 0.275, 0.188 }
local WAX = { 0.604, 0.169, 0.125 }
local FROST = { 0.184, 0.373, 0.47 }
-- The seal on each kind of page.
local SEALS = { note = "seal_red", secret = "seal_check", lore = "seal_blue", wager = "seal_grey", board = "seal_grey" }
-- And the line at its foot.
local FOOT = {
  note = "Kept in the logbook, under Secrets and Notes.",
  secret = "Written into the logbook, under Secrets and Notes.",
  lore = "Click a note to read it again.",
  wager = "Click a wager to take it: down you go, fresh.",
  board = "Every server's runs, through the board's keeper.",
}

local ledger = ES.ledger

local f = CreateFrame("Frame", "ElwynnSurvivalLetter", UIParent)
f:SetWidth(W)
f:SetHeight(300)
f:SetPoint("CENTER", UIParent, "CENTER", 0, 60)
f:SetFrameStrata("DIALOG")
f:SetToplevel(true)
f:EnableMouse(true)
f:SetMovable(true)
f:RegisterForDrag("LeftButton")
f:SetScript("OnDragStart", function() this:StartMoving() end)
f:SetScript("OnDragStop", function() this:StopMovingOrSizing() end)
f:Hide()
ES.letter = f

-- The page: parchment and its torn rim with the Ledger's art, a dark sheet without it.
if ledger and ledger.Parchment then
  f.paper = ledger.Parchment(f, "BACKGROUND", 4, 0.8)
  if ledger.Slice and ledger.FRAME_RECTS then
    f.rim = ledger.Slice(f, "parchment_frame", "BORDER", 512, 512, ledger.FRAME_RECTS, 40, 6, true)
  end
else
  f.paper = f:CreateTexture(nil, "BACKGROUND")
  f.paper:SetAllPoints(f)
  f.paper:SetTexture(0.08, 0.06, 0.04, 0.94)
  INK = { 0.95, 0.88, 0.72 }
  SEPIA = { 0.75, 0.66, 0.5 }
  WAX = { 1, 0.45, 0.35 }
  FROST = { 0.55, 0.78, 1 }
end

local function Text(face, size, color, justify)
  local t = f:CreateFontString(nil, "ARTWORK")
  t:SetFont(face, size, "")
  t:SetTextColor(color[1], color[2], color[3])
  t:SetShadowOffset(0, 0)
  t:SetJustifyH(justify or "LEFT")
  if t.SetJustifyV then t:SetJustifyV("TOP") end
  t:SetWidth(INNER)
  t.ledgerInk = true
  return t
end

local function Art(layer, file, w, h)
  local t = f:CreateTexture(nil, layer)
  t:SetTexture(A .. file)
  t:SetWidth(w)
  t:SetHeight(h)
  return t
end

f.seal = Art("OVERLAY", "seal_red", 46, 46)
f.seal:SetPoint("CENTER", f, "TOP", 0, -6)
f.title = Text(TITLE, 21, INK, "CENTER")
f.place = Text(SERIF, 12, SEPIA, "CENTER")
f.rule = Art("ARTWORK", "divider_vine", 300, 18)
f.rule:SetVertexColor(1, 1, 1, 0.8)
f.body = Text(SERIF, 15, INK, "LEFT")
if f.body.SetSpacing then f.body:SetSpacing(3) end
f.clueRule = Art("ARTWORK", "divider_small", 48, 24)
f.clueHead = Text(TITLE, 13, WAX, "CENTER")
f.clueHead:SetText("A Clue")
f.clue = Text(SERIF, 14, WAX, "CENTER")
f.foot = Text(TITLE, 12, SEPIA, "CENTER")

-- The ink cross, top right; Close, at the foot.
f.x = CreateFrame("Button", nil, f)
f.x:SetWidth(28)
f.x:SetHeight(28)
f.x:SetPoint("TOPRIGHT", f, "TOPRIGHT", -10, -10)
f.x.mark = f.x:CreateTexture(nil, "ARTWORK")
f.x.mark:SetAllPoints(f.x)
f.x.mark:SetTexture(A .. "close_ink")
f.x:SetScript("OnEnter", function() this.mark:SetTexture(A .. "close_ink_hover") end)
f.x:SetScript("OnLeave", function() this.mark:SetTexture(A .. "close_ink") end)
f.x:SetScript("OnClick", function() f:Hide() end)

-- The links: a line of blue ink each, read on by clicking.
f.links = {}
for i = 1, MAX_LINKS do
  local b = CreateFrame("Button", nil, f)
  b:SetWidth(INNER)
  b:SetHeight(18)
  b.label = b:CreateFontString(nil, "ARTWORK")
  b.label:SetFont(SERIF, 14, "")
  b.label:SetShadowOffset(0, 0)
  b.label:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.label:SetTextColor(FROST[1], FROST[2], FROST[3])
  b.label.ledgerInk = true
  b:SetScript("OnEnter", function() this.label:SetTextColor(WAX[1], WAX[2], WAX[3]) end)
  b:SetScript("OnLeave", function() this.label:SetTextColor(FROST[1], FROST[2], FROST[3]) end)
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
local function Tall(fs, size, text)
  local h = fs:GetHeight() or 0
  if h > 1 then return h end
  local lines = 0
  local rest = (text or "") .. "\n"
  for line in string.gfind(rest, "([^\n]*)\n") do
    lines = lines + math.max(1, math.ceil(string.len(line) * size * 0.47 / INNER))
  end
  return lines * (size + 4)
end

-- The page, laid out top to bottom and sized to what's on it.
local function Lay(kind, title, place, text, clue, links)
  f.seal:SetTexture(A .. (SEALS[kind] or "seal_red"))
  local y = 34
  local function put(region, h, gap)
    region:ClearAllPoints()
    region:SetPoint("TOP", f, "TOP", 0, -y)
    y = y + h + (gap or 0)
  end
  f.title:SetText(title)
  put(f.title, Tall(f.title, 21, title), 2)
  f.place:SetText(place)
  if place ~= "" then
    f.place:Show()
    put(f.place, Tall(f.place, 12, place), 4)
  else
    f.place:Hide()
  end
  put(f.rule, 18, 10)
  f.body:SetText(text)
  put(f.body, Tall(f.body, 15, text), 14)
  if clue ~= "" then
    f.clueRule:Show()
    f.clueHead:Show()
    f.clue:Show()
    put(f.clueRule, 20, 0)
    f.clueHead:SetText(kind == "secret" and "Left Here" or "A Clue")
    put(f.clueHead, 16, 2)
    f.clue:SetText(clue)
    put(f.clue, Tall(f.clue, 14, clue), 12)
  else
    f.clueRule:Hide()
    f.clueHead:Hide()
    f.clue:Hide()
  end
  local n = 0
  for i = 1, MAX_LINKS do
    local l = links[i]
    local b = f.links[i]
    if l then
      n = n + 1
      b.cmd = l[1]
      b.label:SetText("~ " .. l[2] .. " ~")
      b:Show()
      put(b, 18, 2)
    else
      b:Hide()
    end
  end
  if n > 0 then y = y + 8 end
  f.foot:SetText(FOOT[kind] or FOOT.note)
  put(f.foot, 14, 0)
  f:SetHeight(math.max(180, y + 30))
  if f.paper.Fit then f.paper.Fit() end
end

-- A page from the server.
function ES.Letter(fields)
  local kind = fields[2] or "note"
  local text = string.gsub(fields[5] or "", "{n}", "\n")
  local clue = string.gsub(fields[6] or "", "{n}", "\n")
  local links = {}
  if fields[7] and fields[7] ~= "" and ES.Split then
    local items = ES.Split(fields[7], "^")
    for i = 1, table.getn(items) do
      local l = ES.Split(items[i], "~")
      if l[1] and l[1] ~= "" then table.insert(links, { l[1], l[2] or l[1] }) end
    end
  end
  Lay(kind, fields[3] or "", fields[4] or "", text, clue, links)
  if not f:IsShown() then
    PlaySound("igQuestListOpen")
    f.switched = ES.mode ~= "cursor"
    f:Show()
    if ES.WindowMode then ES.WindowMode() end
  end
  ES.letterShown = { kind = kind, title = fields[3] or "", clue = clue, links = table.getn(links), height = f:GetHeight() }
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
