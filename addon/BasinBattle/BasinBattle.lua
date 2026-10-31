-- Basin Battle: the card picker, the spell bar, the wave and objective trackers, banners and
-- toasts.
--
-- The server talks to this add-on in add-on lines with the prefix "BB" (see
-- crates/htl-server/src/game/ui.rs for the format) and it answers with chat commands:
-- ".bb hello", ".bb pick", ".bb banish", ".bb lock", ".bb later", ".bb open", ".bb cast",
-- ".bb auto", ".bb talent", ".bb untalent", ".bb shop", ".bb buy".
--
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., math.mod, table.getn, `this`.

local VERSION = 4
local ART = "Interface\\AddOns\\BasinBattle\\art\\"
local FONT_TITLE = "Fonts\\MORPHEUS.TTF"
local FONT_BODY = "Fonts\\FRIZQT__.TTF"
local FONT_NUM = "Fonts\\ARIALN.TTF"

BasinBattle = {}
local BB = BasinBattle
-- The key bindings window's names for Bindings.xml.
BINDING_HEADER_BASINBATTLE = "Basin Battle"
BINDING_NAME_BASINBATTLE_SHOP = "Open the shop"

-- Rarity colours: the item-quality colours, a touch brighter so they glow on dark cards.
local RARITY = {
  [0] = { name = "Common", r = 0.85, g = 0.85, b = 0.85 },
  [1] = { name = "Uncommon", r = 0.2, g = 1.0, b = 0.15 },
  [2] = { name = "Rare", r = 0.2, g = 0.55, b = 1.0 },
  [3] = { name = "Epic", r = 0.72, g = 0.32, b = 1.0 },
  [4] = { name = "Legendary", r = 1.0, g = 0.55, b = 0.08 },
}

local GOLD = { r = 1.0, g = 0.82, b = 0.45 }

-- Sound files from the 1.12 data; a missing one is silent.
local SOUND = {
  open = "Sound\\Interface\\iQuestActivate.wav",
  level = "Sound\\Interface\\LevelUp.wav",
  epic = "Sound\\Interface\\iQuestComplete.wav",
  hover = "Sound\\Interface\\uChatScrollButton.wav",
  pick = "Sound\\Interface\\iQuestComplete.wav",
  reroll = "Sound\\Interface\\iAbilitiesTurnPageA.wav",
  banish = "Sound\\Interface\\iAbilitiesTurnPageB.wav",
  wave = "Sound\\Interface\\RaidWarning.wav",
  boss = "Sound\\Interface\\PVPFlagTakenMono.wav",
  cleared = "Sound\\Interface\\iQuestComplete.wav",
  escalation = "Sound\\Interface\\RaidWarning.wav",
  defeat = "Sound\\Interface\\igQuestFailed.wav",
  claimed = "Sound\\Interface\\PVPFlagCapturedMono.wav",
  bases = "Sound\\Interface\\PVPFlagTakenMono.wav",
  run = "Sound\\Interface\\ReadyCheck.wav",
  toast = "Sound\\Interface\\iQuestActivate.wav",
  danger = "Sound\\Interface\\RaidWarning.wav",
  victory = "Sound\\Interface\\LevelUp.wav",
  tick = "Sound\\Interface\\MapPing.wav",
}

local function Play(key)
  local file = SOUND[key]
  if file then
    PlaySoundFile(file)
  end
end

-- --- small helpers -------------------------------------------------------------------------------

local function Split(s, sep)
  local t = {}
  local start = 1
  while true do
    local i = string.find(s, sep, start, true)
    if not i then
      table.insert(t, string.sub(s, start))
      break
    end
    table.insert(t, string.sub(s, start, i - 1))
    start = i + 1
  end
  return t
end

local function Spaced(s)
  local out = ""
  local n = string.len(s)
  for i = 1, n do
    out = out .. string.sub(s, i, i)
    if i < n then
      out = out .. " "
    end
  end
  return out
end

local function Comma(n)
  local s = tostring(math.floor(n + 0.5))
  local k = 1
  while k > 0 do
    s, k = string.gsub(s, "^(-?%d+)(%d%d%d)", "%1,%2")
  end
  return s
end

local function Clock(secs)
  secs = math.floor(secs + 0.5)
  local m = math.floor(secs / 60)
  local s = secs - m * 60
  if s < 10 then
    return m .. ":0" .. s
  end
  return m .. ":" .. s
end

-- Commands to the server go out as chat. The game won't let the dead speak aloud, so a fallen
-- hero whispers them to themselves instead (a pick made while dead, a purchase).
local function Say(text)
  if UnitIsDeadOrGhost and UnitIsDeadOrGhost("player") and UnitName then
    SendChatMessage(text, "WHISPER", nil, UnitName("player"))
  else
    SendChatMessage(text, "SAY")
  end
end

local function Tex(parent, layer, file, w, h)
  local t = parent:CreateTexture(nil, layer)
  t:SetTexture(ART .. file)
  if w then
    t:SetWidth(w)
    t:SetHeight(h)
  end
  return t
end

local function Text(parent, font, size, flags, layer)
  local f = parent:CreateFontString(nil, layer or "OVERLAY")
  f:SetFont(font, size, flags)
  f:SetShadowColor(0, 0, 0, 1)
  f:SetShadowOffset(1, -1)
  return f
end

-- Spin a texture by its coordinates; the quad shows the square inscribed in the circle of
-- radius 0.5, so the art must fade out inside that.
local function SetRotation(t, a)
  local h = 0.35355
  local c = math.cos(a)
  local s = math.sin(a)
  local function rx(x, y) return 0.5 + x * c - y * s end
  local function ry(x, y) return 0.5 + x * s + y * c end
  t:SetTexCoord(rx(-h, -h), ry(-h, -h), rx(-h, h), ry(-h, h), rx(h, -h), ry(h, -h), rx(h, h), ry(h, h))
end

-- --- tweens --------------------------------------------------------------------------------------

local tweens = {}

local function OutBack(t)
  local c1 = 1.70158
  local c3 = c1 + 1
  local u = t - 1
  return 1 + c3 * u * u * u + c1 * u * u
end

local function OutCubic(t)
  local u = 1 - t
  return 1 - u * u * u
end

-- Call fn(eased progress) every frame for `dur` seconds after `delay`; `done` at the end. A tween
-- with a `tag` replaces any running tween with the same tag.
local function Tween(dur, delay, ease, fn, done, tag)
  if tag then
    local i = 1
    while i <= table.getn(tweens) do
      if tweens[i].tag == tag then
        table.remove(tweens, i)
      else
        i = i + 1
      end
    end
  end
  table.insert(tweens, { start = GetTime() + (delay or 0), dur = dur, ease = ease or OutCubic, fn = fn, done = done, tag = tag })
end

local function CancelTweens(tag)
  local i = 1
  while i <= table.getn(tweens) do
    if tweens[i].tag == tag then
      table.remove(tweens, i)
    else
      i = i + 1
    end
  end
end

local ticker = CreateFrame("Frame")
ticker:SetScript("OnUpdate", function()
  local now = GetTime()
  local i = 1
  while i <= table.getn(tweens) do
    local tw = tweens[i]
    local t = (now - tw.start) / tw.dur
    if t >= 1 then
      table.remove(tweens, i)
      tw.fn(1)
      if tw.done then
        tw.done()
      end
    else
      if t >= 0 then
        tw.fn(tw.ease(t))
      end
      i = i + 1
    end
  end
end)

-- --- buttons -------------------------------------------------------------------------------------

-- A rounded button: dark fill, coloured rim, a label; `onClick` when pressed.
local function MakeButton(parent, w, h, label, color, onClick)
  local b = CreateFrame("Button", nil, parent)
  b:SetWidth(w)
  b:SetHeight(h)
  b.fill = Tex(b, "BACKGROUND", "button")
  b.fill:SetAllPoints(b)
  b.fill:SetVertexColor(0.09, 0.08, 0.07, 0.92)
  b.shine = Tex(b, "BORDER", "buttonshine")
  b.shine:SetAllPoints(b)
  b.shine:SetVertexColor(color.r, color.g, color.b, 0.12)
  b.rim = Tex(b, "ARTWORK", "buttonborder")
  b.rim:SetAllPoints(b)
  b.rim:SetVertexColor(color.r, color.g, color.b, 0.75)
  b.label = Text(b, FONT_BODY, 13)
  b.label:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.label:SetTextColor(color.r, color.g, color.b)
  b.label:SetText(label)
  b.color = color
  b:EnableMouse(true)
  b:RegisterForClicks("LeftButtonUp")
  b:SetScript("OnEnter", function()
    this.rim:SetVertexColor(this.color.r, this.color.g, this.color.b, 1)
    this.shine:SetVertexColor(this.color.r, this.color.g, this.color.b, 0.3)
    if this.tip then
      GameTooltip:SetOwner(this, "ANCHOR_TOP")
      GameTooltip:SetText(this.tip, 1, 1, 1, 1, 1)
      GameTooltip:Show()
    end
  end)
  b:SetScript("OnLeave", function()
    this.rim:SetVertexColor(this.color.r, this.color.g, this.color.b, 0.75)
    this.shine:SetVertexColor(this.color.r, this.color.g, this.color.b, 0.12)
    GameTooltip:Hide()
  end)
  b:SetScript("OnClick", function()
    Play("hover")
    onClick(this)
  end)
  return b
end

-- --- the card picker -----------------------------------------------------------------------------

local CARD_W = 236
local K = CARD_W / 224 -- art pixels to UI units: the card is 224 art pixels wide
local CARD_H = math.floor(352 * K + 0.5)
local CARD_GAP = 22

-- A card title on one line: the largest size from `sizes` that fits `width`.
local function FitTitle(fs, text, sizes, width)
  fs:SetText(text)
  for i = 1, table.getn(sizes) do
    fs:SetFont(FONT_TITLE, sizes[i])
    if fs:GetStringWidth() <= width then return end
  end
end
local MAX_CARDS = 4

local picker = CreateFrame("Frame", "BasinBattlePicker", UIParent)
BB.picker = picker
picker:SetAllPoints(UIParent)
picker:SetFrameStrata("DIALOG")
picker:Hide()
-- No Esc: the game waits, paused, until a card is chosen.

picker.dim = picker:CreateTexture(nil, "BACKGROUND")
picker.dim:SetAllPoints(picker)
picker.dim:SetTexture(0, 0, 0)
picker.dim:SetAlpha(0.62)

picker.rays = Tex(picker, "BORDER", "rays", 980, 980)
picker.rays:SetPoint("CENTER", picker, "CENTER", 0, 30)
picker.rays:SetBlendMode("ADD")
picker.rays:SetVertexColor(1.0, 0.78, 0.4, 0.3)

picker.halo = Tex(picker, "BORDER", "glow", 720, 300)
picker.halo:SetPoint("CENTER", picker, "CENTER", 0, 262)
picker.halo:SetBlendMode("ADD")
picker.halo:SetVertexColor(1.0, 0.7, 0.3, 0.35)

-- The header pill ("LEVEL 2").
picker.pill = CreateFrame("Frame", nil, picker)
picker.pill:SetWidth(150)
picker.pill:SetHeight(26)
picker.pill:SetPoint("CENTER", picker, "CENTER", 0, 312)
picker.pill.bg = Tex(picker.pill, "BACKGROUND", "pill")
picker.pill.bg:SetAllPoints(picker.pill)
picker.pill.bg:SetVertexColor(0.95, 0.8, 0.42, 1)
picker.pill.rim = Tex(picker.pill, "BORDER", "pillborder")
picker.pill.rim:SetAllPoints(picker.pill)
picker.pill.rim:SetVertexColor(1, 0.95, 0.75, 0.9)
picker.pill.text = Text(picker.pill, FONT_BODY, 12)
picker.pill.text:SetShadowOffset(0, 0)
picker.pill.text:SetPoint("CENTER", picker.pill, "CENTER", 0, 0)
picker.pill.text:SetTextColor(0.16, 0.1, 0.03)

picker.titleFrame = CreateFrame("Frame", nil, picker)
picker.titleFrame:SetWidth(700)
picker.titleFrame:SetHeight(60)
picker.titleFrame:SetPoint("CENTER", picker, "CENTER", 0, 266)
picker.title = Text(picker.titleFrame, FONT_TITLE, 52, "OUTLINE")
picker.title:SetPoint("CENTER", picker.titleFrame, "CENTER", 0, 0)
picker.title:SetTextColor(1, 0.86, 0.52)
picker.title:SetShadowOffset(2, -2)

picker.subtitle = Text(picker, FONT_BODY, 14)
picker.subtitle:SetPoint("CENTER", picker, "CENTER", 0, 224)
picker.subtitle:SetTextColor(0.78, 0.74, 0.66)

picker.more = Text(picker, FONT_BODY, 12)
picker.more:SetPoint("CENTER", picker, "CENTER", 0, 203)
picker.more:SetTextColor(1, 0.82, 0.45)

-- One card.
local function MakeCard(i)
  local holder = CreateFrame("Frame", nil, picker)
  holder:SetWidth(CARD_W)
  holder:SetHeight(CARD_H)
  holder.index = i

  local function Layer(parent, layer, file)
    local t = Tex(parent, layer, file, 256 * K, 384 * K)
    t:SetPoint("TOPLEFT", holder, "TOPLEFT", -16 * K, 16 * K)
    t:SetTexCoord(0, 1, 0, 0.75)
    return t
  end

  holder.glow = Layer(holder, "BACKGROUND", "cardglow")
  holder.glow:SetBlendMode("ADD")

  local c = CreateFrame("Button", nil, holder)
  c:SetWidth(CARD_W)
  c:SetHeight(CARD_H)
  c:SetPoint("CENTER", holder, "CENTER", 0, 0)
  c:SetFrameLevel(holder:GetFrameLevel() + 1)
  c.holder = holder
  holder.card = c

  local function CardLayer(layer, file)
    local t = Tex(c, layer, file, 256 * K, 384 * K)
    t:SetPoint("TOPLEFT", c, "TOPLEFT", -16 * K, 16 * K)
    t:SetTexCoord(0, 1, 0, 0.75)
    return t
  end
  c.body = CardLayer("BACKGROUND", "card")
  c.wash = CardLayer("BORDER", "cardwash")
  c.border = CardLayer("ARTWORK", "cardborder")

  -- The key badge.
  c.badge = Tex(c, "ARTWORK", "circle", 24, 24)
  c.badge:SetPoint("TOPLEFT", c, "TOPLEFT", 12, -12)
  c.badge:SetVertexColor(0.05, 0.05, 0.05, 0.95)
  c.badgeRing = Tex(c, "OVERLAY", "ring", 24, 24)
  c.badgeRing:SetPoint("CENTER", c.badge, "CENTER", 0, 0)
  c.badgeRing:SetVertexColor(0.55, 0.55, 0.55, 1)
  c.key = Text(c, FONT_BODY, 11)
  c.key:SetPoint("CENTER", c.badge, "CENTER", 0, 0)
  c.key:SetTextColor(0.85, 0.85, 0.85)
  c.key:SetText(i)

  -- What kind of card it is, at a glance: a coloured band across the top and a ribbon naming
  -- it (NEW SPELL, UPGRADE, TALENT, LEGENDARY).
  c.band = Tex(c, "BORDER", "glow", CARD_W + 40, 120)
  c.band:SetPoint("CENTER", c, "TOP", 0, -10)
  c.band:SetBlendMode("ADD")
  c.ribbon = Tex(c, "OVERLAY", "pill", 112, 20)
  c.ribbon:SetPoint("TOP", c, "TOP", 0, -15)
  c.ribbonText = Text(c, FONT_BODY, 10.5)
  c.ribbonText:SetShadowOffset(0, 0)
  c.ribbonText:SetPoint("CENTER", c.ribbon, "CENTER", 0, 0)
  c.ribbonText:SetTextColor(0.06, 0.06, 0.06)

  -- RANK tag.
  c.tag = Tex(c, "ARTWORK", "pill", 64, 18)
  c.tag:SetPoint("TOPRIGHT", c, "TOPRIGHT", -12, -42)
  c.tagText = Text(c, FONT_BODY, 10)
  c.tagText:SetShadowOffset(0, 0)
  c.tagText:SetPoint("CENTER", c.tag, "CENTER", 0, 0)
  c.tagText:SetTextColor(0.06, 0.06, 0.06)

  -- The icon, its glow and frame.
  c.iconGlow = Tex(c, "BORDER", "glow", 170, 170)
  c.iconGlow:SetPoint("CENTER", c, "TOP", 0, -76)
  c.iconGlow:SetBlendMode("ADD")
  c.icon = c:CreateTexture(nil, "ARTWORK")
  c.icon:SetWidth(62)
  c.icon:SetHeight(62)
  c.icon:SetPoint("CENTER", c, "TOP", 0, -76)
  c.icon:SetTexCoord(0.07, 0.93, 0.07, 0.93)
  c.iconFrame = Tex(c, "OVERLAY", "iconframe", 76, 76)
  c.iconFrame:SetPoint("CENTER", c.icon, "CENTER", 0, 0)
  -- A talent's icon sits in a ring, a spell's in a square frame.
  c.iconRing = Tex(c, "OVERLAY", "ring", 96, 96)
  c.iconRing:SetPoint("CENTER", c.icon, "CENTER", 0, 0)
  c.iconRing:Hide()
  -- An upgrade's icon (its spell's) wears a green plus.
  c.plusBack = Tex(c, "OVERLAY", "circle", 26, 26)
  c.plusBack:SetPoint("CENTER", c.icon, "BOTTOMRIGHT", 2, 2)
  c.plusBack:SetVertexColor(0.05, 0.3, 0.05, 1)
  c.plus = Text(c, FONT_TITLE, 22, "OUTLINE")
  c.plus:SetPoint("CENTER", c.plusBack, "CENTER", 0, 1)
  c.plus:SetTextColor(0.45, 1, 0.4)
  c.plus:SetText("+")
  c.plusBack:Hide()
  c.plus:Hide()

  c.rarity = Text(c, FONT_BODY, 10)
  c.rarity:SetPoint("TOP", c, "TOP", 0, -119)
  c.name = Text(c, FONT_TITLE, 23)
  c.name:SetPoint("TOP", c, "TOP", 0, -133)
  c.name:SetWidth(CARD_W - 20)
  c.sub = Text(c, FONT_BODY, 12)
  c.sub:SetPoint("TOP", c, "TOP", 0, -160)
  c.sub:SetTextColor(0.62, 0.6, 0.56)

  c.rule = Tex(c, "ARTWORK", "line", 180, 6)
  c.rule:SetPoint("TOP", c, "TOP", 0, -178)

  c.desc = Text(c, FONT_BODY, 12.5)
  c.desc:SetShadowOffset(1, -1)
  c.desc:SetPoint("TOP", c, "TOP", 0, -190)
  c.desc:SetWidth(CARD_W - 32)
  c.desc:SetJustifyH("CENTER")
  c.desc:SetSpacing(3)
  c.desc:SetTextColor(0.8, 0.78, 0.74)

  c.upgrade = Text(c, FONT_BODY, 14)
  c.upgrade:SetPoint("TOP", c.desc, "BOTTOM", 0, -10)
  c.upgrade:SetWidth(CARD_W - 30)
  c.upgrade:SetTextColor(0.4, 1, 0.35)

  c.footRule = Tex(c, "ARTWORK", "line", 180, 6)
  c.footRule:SetPoint("BOTTOM", c, "BOTTOM", 0, 42)
  c.slot = Text(c, FONT_BODY, 10.5)
  c.slot:SetPoint("BOTTOM", c, "BOTTOM", 0, 27)
  c.slot:SetTextColor(0.75, 0.72, 0.65)
  c.footRule:SetVertexColor(1, 1, 1, 0.12)
  c.foot = Text(c, FONT_BODY, 10.5)
  c.foot:SetPoint("BOTTOM", c, "BOTTOM", 0, 11)
  c.foot:SetTextColor(0.6, 0.58, 0.54)

  -- The white flash when it's picked, on top of everything.
  c.flashFrame = CreateFrame("Frame", nil, c)
  c.flashFrame:SetAllPoints(c)
  c.flashFrame:SetFrameLevel(c:GetFrameLevel() + 3)
  c.flash = Tex(c.flashFrame, "OVERLAY", "cardflash", 256 * K, 384 * K)
  c.flash:SetPoint("TOPLEFT", c, "TOPLEFT", -16 * K, 16 * K)
  c.flash:SetTexCoord(0, 1, 0, 0.75)
  c.flash:SetBlendMode("ADD")
  c.flash:SetAlpha(0)

  c:EnableMouse(true)
  c:RegisterForClicks("LeftButtonUp")
  c:SetScript("OnEnter", function()
    if BB.picking then return end
    this.hover = true
    BB.CardHover(this, true)
  end)
  c:SetScript("OnLeave", function()
    this.hover = false
    BB.CardHover(this, false)
  end)
  c:SetScript("OnClick", function()
    BB.Pick(this.holder.index)
  end)

  -- Banish and lock, under the card.
  holder.banish = MakeButton(picker, 112, 26, "Banish", { r = 1, g = 0.45, b = 0.35 }, function()
    BB.Banish(holder.index)
  end)
  holder.banish:SetPoint("TOP", holder, "BOTTOM", 0, -10)
  holder.banish.tip = "Banish this card: it won't be offered again this run, and a new card takes its place."
  holder.lock = MakeButton(picker, 60, 26, "Lock", { r = 0.6, g = 0.85, b = 1 }, function()
    BB.Lock(holder.index)
  end)
  holder.lock:SetPoint("LEFT", holder.banish, "RIGHT", 6, 0)
  holder.lock.tip = "Lock this card into your next offer."
  return holder
end

picker.cards = {}
for i = 1, MAX_CARDS do
  picker.cards[i] = MakeCard(i)
end

-- A card's Banish and Lock show while the pointer is on the card or on them (and a moment
-- after, to cross the gap): the picker reads as cards, with the extras there when wanted.
local hoverTicker = CreateFrame("Frame", nil, picker)
hoverTicker:SetScript("OnUpdate", function()
  local now = GetTime()
  for i = 1, MAX_CARDS do
    local h = picker.cards[i]
    if h:IsShown() and (h.wantBanish or h.wantLock) then
      if MouseIsOver(h) or (h.banish:IsShown() and MouseIsOver(h.banish)) or (h.lock:IsShown() and MouseIsOver(h.lock)) then
        h.hoverUntil = now + 0.5
      end
      local on = now < (h.hoverUntil or 0)
      if on and h.wantBanish then h.banish:Show() else h.banish:Hide() end
      if on and h.wantLock then h.lock:Show() else h.lock:Hide() end
    end
  end
end)

-- The bottom bar: your spells, then skip and reroll.
picker.bar = CreateFrame("Frame", nil, picker)
picker.bar:SetWidth(900)
picker.bar:SetHeight(44)
picker.bar:SetPoint("CENTER", picker, "CENTER", 0, -262)
picker.spellsLabel = Text(picker.bar, FONT_BODY, 11)
picker.spellsLabel:SetText(Spaced("SPELLS"))
picker.spellsLabel:SetTextColor(0.7, 0.68, 0.62)
picker.slots = {}

local function MakeSlot(parent, size)
  local s = CreateFrame("Frame", nil, parent)
  s:SetWidth(size)
  s:SetHeight(size)
  s.icon = s:CreateTexture(nil, "ARTWORK")
  s.icon:SetPoint("TOPLEFT", s, "TOPLEFT", 2, -2)
  s.icon:SetPoint("BOTTOMRIGHT", s, "BOTTOMRIGHT", -2, 2)
  s.icon:SetTexCoord(0.07, 0.93, 0.07, 0.93)
  s.frame = Tex(s, "OVERLAY", "iconframe")
  s.frame:SetPoint("TOPLEFT", s, "TOPLEFT", -3, 3)
  s.frame:SetPoint("BOTTOMRIGHT", s, "BOTTOMRIGHT", 3, -3)
  s.rank = Text(s, FONT_NUM, 12, "OUTLINE")
  s.rank:SetPoint("BOTTOMRIGHT", s, "BOTTOMRIGHT", 1, 0)
  s:EnableMouse(true)
  s:SetScript("OnEnter", function()
    if not this.card then return end
    GameTooltip:SetOwner(this, "ANCHOR_TOP")
    BB.CardTooltip(GameTooltip, this.card)
    GameTooltip:Show()
  end)
  s:SetScript("OnLeave", function() GameTooltip:Hide() end)
  return s
end

picker.reroll = MakeButton(picker, 150, 34, "Reroll", GOLD, function() BB.Reroll() end)
picker.reroll.tip = "New cards for this pick. (Key: R)"
picker.skip = MakeButton(picker, 150, 34, "Skip", { r = 0.75, g = 0.75, b = 0.75 }, function() BB.Skip() end)
picker.later = Text(picker, FONT_BODY, 11)
picker.later:SetPoint("CENTER", picker, "CENTER", 0, -300)
picker.later:SetTextColor(0.55, 0.53, 0.5)
picker.later:SetText("Press 1-4 to pick  ·  The game is paused until you choose")

-- --- picker logic --------------------------------------------------------------------------------

BB.offer = nil -- the offer on screen
BB.incoming = nil -- an offer arriving line by line
BB.build = {}
BB.pending = 0
BB.picking = false

function BB.CardHover(c, on)
  local h = c.holder
  local col = c.color or RARITY[0]
  if on then
    Play("hover")
    c.border:SetVertexColor(col.r, col.g, col.b, 1)
    h.glow:SetVertexColor(col.r, col.g, col.b, 0.95)
    Tween(0.12, 0, OutCubic, function(t) c:SetScale(1 + 0.045 * t) end, nil, "hover" .. h.index)
  else
    c.border:SetVertexColor(col.r, col.g, col.b, 0.8)
    h.glow:SetVertexColor(col.r, col.g, col.b, h.glowBase or 0.35)
    local from = c:GetScale()
    Tween(0.12, 0, OutCubic, function(t) c:SetScale(from + (1 - from) * t) end, nil, "hover" .. h.index)
  end
end

local function LayoutBar()
  local bar = picker.bar
  local n = table.getn(BB.build)
  local size = 30
  local gap = 5
  local shown = math.min(n, 18)
  local labelW = 60
  local width = labelW + shown * (size + gap)
  local left = -width / 2 - 170
  picker.spellsLabel:ClearAllPoints()
  picker.spellsLabel:SetPoint("LEFT", bar, "CENTER", left, 0)
  for i = 1, 18 do
    local s = picker.slots[i]
    if not s and i <= shown then
      s = MakeSlot(bar, size)
      picker.slots[i] = s
    end
    if s then
      if i <= shown then
        local h = BB.build[i]
        s:ClearAllPoints()
        s:SetPoint("LEFT", bar, "CENTER", left + labelW + (i - 1) * (size + gap), 0)
        s.icon:SetTexture("Interface\\Icons\\" .. h.icon)
        local col = RARITY[h.rarity] or RARITY[0]
        s.frame:SetVertexColor(col.r, col.g, col.b, 1)
        if h.rank > 1 then s.rank:SetText(h.rank) else s.rank:SetText("") end
        s.card = h
        s:Show()
      else
        s:Hide()
      end
    end
  end
  if n == 0 then
    picker.spellsLabel:Hide()
  else
    picker.spellsLabel:Show()
  end
  -- The buttons on the right.
  local right = width / 2 - 170 + 24
  picker.skip:ClearAllPoints()
  picker.reroll:ClearAllPoints()
  local o = BB.offer
  local x = math.max(right, 40)
  if o and o.skip >= 0 then
    picker.skip.label:SetText(o.skipLabel)
    picker.skip:SetPoint("LEFT", bar, "CENTER", x, 0)
    picker.skip:Show()
    x = x + 160
  else
    picker.skip:Hide()
  end
  if o and o.reroll >= 0 then
    picker.reroll.label:SetText(o.rerollLabel .. "  |cff8c8c8c[R]|r")
    picker.reroll:SetPoint("LEFT", bar, "CENTER", x, 0)
    picker.reroll:Show()
  else
    picker.reroll:Hide()
  end
end

-- What kind of card: its ribbon, colour and the line under its name.
local CATEGORIES = {
  spell = { key = "spell", label = "NEW SPELL", sub = "A new spell for your bar", r = 1, g = 0.8, b = 0.3 },
  upgrade = { key = "upgrade", label = "UPGRADE", sub = "", r = 0.35, g = 0.85, b = 1 },
  talent = { key = "talent", label = "TALENT", sub = "", r = 0.78, g = 0.58, b = 1 },
  legendary = { key = "legendary", label = "LEGENDARY", sub = "A power of its own", r = 1, g = 0.5, b = 0.15 },
}
function BB.CardCategory(card)
  if card.slot == "Legendary" then return CATEGORIES.legendary end
  if card.kind == "Upgrade" then return CATEGORIES.upgrade end
  if card.kind == "Talent" then return CATEGORIES.talent end
  return CATEGORIES.spell
end

-- Fill card `i` from the offer's card table.
local function FillCard(holder, card)
  local c = holder.card
  local col = RARITY[card.rarity] or RARITY[0]
  c.color = col
  holder.glowBase = 0.25 + 0.12 * card.rarity
  holder.glow:SetVertexColor(col.r, col.g, col.b, holder.glowBase)
  c.wash:SetVertexColor(col.r, col.g, col.b, 0.32 + 0.06 * card.rarity)
  c.border:SetVertexColor(col.r, col.g, col.b, 0.8)
  c.iconGlow:SetVertexColor(col.r, col.g, col.b, 0.5)
  c.iconFrame:SetVertexColor(col.r * 0.9 + 0.1, col.g * 0.9 + 0.1, col.b * 0.9 + 0.1, 1)
  c.rule:SetVertexColor(col.r, col.g, col.b, 0.45)
  c.icon:SetTexture("Interface\\Icons\\" .. card.icon)
  c.rarity:SetText(Spaced(string.upper(col.name)))
  c.rarity:SetTextColor(col.r, col.g, col.b)
  FitTitle(c.name, card.name, { 23, 21, 19, 17, 15 }, CARD_W - 24)
  c.name:SetTextColor(col.r * 0.85 + 0.15, col.g * 0.85 + 0.15, col.b * 0.85 + 0.15)
  -- The kind of card, said three ways: the ribbon and band's colour, the icon's frame, and
  -- the line under the name.
  local cat = BB.CardCategory(card)
  c.band:SetVertexColor(cat.r, cat.g, cat.b, 0.38)
  c.ribbon:SetVertexColor(cat.r, cat.g, cat.b, 1)
  c.ribbonText:SetText(cat.label)
  c.ribbon:SetWidth(string.len(cat.label) * 7 + 26)
  c.iconRing:Hide()
  c.iconFrame:Show()
  c.plusBack:Hide()
  c.plus:Hide()
  c.tag:Hide()
  c.tagText:SetText("")
  if cat.key == "upgrade" then
    -- "Upgrade 2 of 4 for Frostbolt" says which spell it changes.
    c.sub:SetText(card.slot or "")
    c.sub:SetTextColor(cat.r, cat.g, cat.b)
    c.plusBack:Show()
    c.plus:Show()
  elseif cat.key == "talent" then
    local tree = "talent"
    if card.tags ~= "" then tree = card.tags .. " tree" end
    -- A talent that teaches a spell takes one of the six keys, like a spell card.
    if card.slot and string.find(card.slot, "spell slot") then
      c.sub:SetText("Teaches a spell  ·  " .. tree)
    else
      c.sub:SetText("Passive  ·  " .. tree)
    end
    c.sub:SetTextColor(cat.r, cat.g, cat.b)
    c.iconFrame:Hide()
    c.iconRing:SetVertexColor(cat.r, cat.g, cat.b, 1)
    c.iconRing:Show()
  elseif card.rank > 1 then
    c.sub:SetText("Rank " .. (card.rank - 1) .. "  >  " .. card.rank .. "  of " .. card.maxRank)
    c.sub:SetTextColor(0.62, 0.6, 0.56)
    c.tag:Show()
    c.tag:SetVertexColor(1, 0.85, 0.35, 1)
    c.tagText:SetText("RANK " .. card.rank)
    c.tag:SetWidth(62)
  else
    c.sub:SetText(cat.sub)
    c.sub:SetTextColor(0.62, 0.6, 0.56)
  end
  local plain = string.gsub(string.gsub(card.text, "|c%x%x%x%x%x%x%x%x", ""), "|r", "")
  if string.len(plain) > 165 then c.desc:SetFont(FONT_BODY, 11.5) else c.desc:SetFont(FONT_BODY, 12.5) end
  c.desc:SetText(card.text)
  if card.upgrade ~= "" then
    c.upgrade:SetText("+ " .. card.upgrade)
    c.upgrade:Show()
  else
    c.upgrade:Hide()
  end
  local foot = card.kind
  if cat.key == "talent" then foot = "Always on" end
  if cat.key == "talent" and card.slot and string.find(card.slot, "spell slot") then foot = "Spell" end
  if card.tags ~= "" and cat.key ~= "talent" then foot = foot .. "  ·  " .. card.tags end
  if card.school ~= "" and card.school ~= card.tags then foot = foot .. "  ·  " .. card.school end
  if card.cooldown ~= "" and card.cooldown ~= "0" then foot = foot .. "  ·  every " .. card.cooldown .. " sec" end
  c.foot:SetText(foot)
  local slotLine = card.slot or ""
  if cat.key == "upgrade" then slotLine = "" end
  if cat.key == "talent" then
    -- "Talent · Arcane tree · takes a spell slot 5/6": only the slot part.
    local _, _, n = string.find(slotLine, "(takes a spell slot.*)$")
    if n then slotLine = "T" .. string.sub(n, 2) else slotLine = "" end
  end
  c.slot:SetText(slotLine)
  c.flash:SetAlpha(0)
  c:SetScale(1)
end

-- Put a whole offer on screen, animating it in.
function BB.ShowOffer(o)
  local was = picker:IsShown() and BB.offer ~= nil
  -- A card pick comes before shopping.
  if BB.HideShop then BB.HideShop() end
  BB.offer = o
  -- Keys pressed in the first moments (still casting) don't pick.
  BB.offerShownAt = GetTime()
  BB.picking = false
  BB.closeAt = nil
  CancelTweens("close")
  picker:SetAlpha(1)

  picker.pill.text:SetText(Spaced(o.header))
  picker.pill:SetWidth(math.max(110, string.len(o.header) * 13 + 30))
  picker.title:SetText(o.title)
  picker.subtitle:SetText(o.subtitle)
  if o.more > 0 then
    local s = "s"
    if o.more == 1 then s = "" end
    picker.more:SetText(o.more .. " more pick" .. s .. " waiting")
  else
    picker.more:SetText("")
  end

  -- Lay the cards out across the screen.
  local n = o.n
  local total = n * CARD_W + (n - 1) * CARD_GAP
  local scale = math.min(1, (UIParent:GetWidth() - 60) / total)
  for i = 1, MAX_CARDS do
    local h = picker.cards[i]
    if i <= n and o.cards[i] then
      FillCard(h, o.cards[i])
      h:SetScale(scale)
      h:ClearAllPoints()
      local x = ((i - 1) - (n - 1) / 2) * (CARD_W + CARD_GAP)
      h:SetPoint("CENTER", picker, "CENTER", x, 12 / scale)
      h:Show()
      -- Banish and Lock wait out of sight until the card is hovered (the ticker below).
      h.wantBanish = o.banishes > 0
      h.wantLock = o.lock and true or false
      h.hoverUntil = 0
      if h.wantBanish then h.banish.label:SetText("Banish (" .. o.banishes .. ")") end
      h.banish:Hide()
      h.lock:Hide()
      if o.banishes <= 0 and o.lock then
        h.lock:ClearAllPoints()
        h.lock:SetPoint("TOP", h, "BOTTOM", 0, -10)
      else
        h.lock:ClearAllPoints()
        h.lock:SetPoint("LEFT", h.banish, "RIGHT", 6, 0)
      end
      -- In from below, one after another.
      local c = h.card
      h:SetAlpha(0)
      local delay = 0.1 + (i - 1) * 0.07
      if not was then delay = delay + 0.12 end
      local idx = i
      Tween(0.34, delay, OutBack, function(t)
        local hh = picker.cards[idx]
        hh:SetAlpha(math.min(1, t * 1.6))
        hh.card:SetScale(0.86 + 0.14 * t)
        hh.card:ClearAllPoints()
        hh.card:SetPoint("CENTER", hh, "CENTER", 0, -36 * (1 - t))
      end, nil, "card" .. idx)
    else
      h:Hide()
      h.wantBanish = false
      h.wantLock = false
      h.banish:Hide()
      h.lock:Hide()
    end
  end
  LayoutBar()

  if not was then
    picker:Show()
    picker.dim:SetAlpha(0)
    local target = 0.66
    if BB.hudPhase == "wave" then target = 0.45 end
    Tween(0.3, 0, OutCubic, function(t) picker.dim:SetAlpha(target * t) end, nil, "dim")
    Tween(0.42, 0.05, OutBack, function(t)
      local sc = 1.6 - 0.6 * t
      picker.titleFrame:SetScale(sc)
      picker.titleFrame:ClearAllPoints()
      picker.titleFrame:SetPoint("CENTER", picker, "CENTER", 0, 266 / sc)
      picker.title:SetAlpha(math.min(1, t * 1.5))
      picker.pill:SetAlpha(math.min(1, t * 1.5))
      picker.halo:SetAlpha(t)
    end, nil, "title")
    local best = 0
    for i = 1, n do
      if o.cards[i] and o.cards[i].rarity > best then best = o.cards[i].rarity end
    end
    if o.kind == "level" then
      Play("level")
    elseif best >= 3 or o.kind == "legendary" then
      Play("epic")
    else
      Play("open")
    end
  end
  BB.UpdatePending()
end

-- Card `i` (1-based) chosen.
function BB.Pick(i)
  local o = BB.offer
  if not o or BB.picking or i > o.n then return end
  BB.picking = true
  Play("pick")
  for k = 1, o.n do
    local h = picker.cards[k]
    local kk = k
    if k == i then
      local c = h.card
      Tween(0.45, 0, OutCubic, function(t)
        c.flash:SetAlpha(0.9 * (1 - t))
        c:SetScale(1.06 + 0.06 * t)
      end, nil, "card" .. kk)
    else
      Tween(0.22, 0, OutCubic, function(t) picker.cards[kk]:SetAlpha(1 - t) end, nil, "card" .. kk)
    end
  end
  Say(".bb pick " .. (i - 1))
  -- Close unless the next pick arrives first.
  BB.closeAt = GetTime() + 0.6
end

function BB.Reroll()
  local o = BB.offer
  if not o or BB.picking or o.reroll < 0 then return end
  Play("reroll")
  BB.picking = true
  for k = 1, o.n do
    local kk = k
    Tween(0.15, 0, OutCubic, function(t) picker.cards[kk]:SetAlpha(1 - t) end, nil, "card" .. kk)
  end
  Say(".bb pick " .. o.reroll)
  BB.closeAt = GetTime() + 2
end

function BB.Skip()
  local o = BB.offer
  if not o or BB.picking or o.skip < 0 then return end
  BB.picking = true
  Say(".bb pick " .. o.skip)
  BB.closeAt = GetTime() + 0.4
end

function BB.Banish(i)
  local o = BB.offer
  if not o or BB.picking or o.banishes <= 0 then return end
  Play("banish")
  BB.picking = true
  local kk = i
  Tween(0.2, 0, OutCubic, function(t) picker.cards[kk]:SetAlpha(1 - t) end, nil, "card" .. kk)
  Say(".bb banish " .. (i - 1))
  BB.closeAt = GetTime() + 2
end

function BB.Lock(i)
  local o = BB.offer
  if not o or BB.picking or not o.lock then return end
  Say(".bb lock " .. (i - 1))
end

-- Close the picker; `quiet` means the server already knows (a pick, or no offer left).
function BB.ClosePicker(quiet)
  if not picker:IsShown() then return end
  BB.quietClose = quiet
  BB.closeAt = nil
  Tween(0.25, 0, OutCubic, function(t) picker:SetAlpha(1 - t) end, function()
    picker:Hide()
    picker:SetAlpha(1)
  end, "close")
end

picker:SetScript("OnHide", function()
  CancelTweens("close")
  picker:SetAlpha(1)
  if not BB.quietClose and BB.offer and not BB.picking then
    -- Esc: the pick waits for later.
    Say(".bb later")
  end
  BB.quietClose = false
  BB.offer = nil
  BB.picking = false
  BB.UpdatePending()
end)

picker:SetScript("OnUpdate", function()
  local now = GetTime()
  SetRotation(picker.rays, now * 0.12)
  picker.rays:SetAlpha(0.75 + 0.25 * math.sin(now * 1.3))
  -- Rare and better cards breathe.
  local o = BB.offer
  if o and not BB.picking then
    for i = 1, o.n do
      local h = picker.cards[i]
      local card = o.cards[i]
      if card and card.rarity >= 2 and not h.card.hover then
        local col = RARITY[card.rarity]
        local a = (h.glowBase or 0.35) + 0.25 * (0.5 + 0.5 * math.sin(now * 2.6 + i))
        h.glow:SetVertexColor(col.r, col.g, col.b, a)
      end
    end
  end
  if BB.closeAt and now >= BB.closeAt then
    BB.ClosePicker(true)
  end
end)

-- Keys 1-4 pick while the picker is up; otherwise 1, 2, 3, 4, Q and E cast the spells on the bar
-- (bound to the first six action buttons; the game's own bar holds nothing else). R rerolls.

-- Basin Battle's keys: A and D strafe, and Q and E cast spells 5 and 6 (all six spells, cards'
-- and talents', share keys 1-4, Q and E). Only keys still doing what the game set them to are
-- changed, so your own changes stay: rebind Action Button 1-6 in Key Bindings to move them.
function BB.ApplyKeys()
  if not SetBinding or not GetBindingAction then return end
  local changed = false
  local function Move(key, from, to)
    local now = GetBindingAction(key) or ""
    local ok = false
    for i = 1, table.getn(from) do
      if now == from[i] then ok = true end
    end
    if ok and now ~= (to or "") then
      SetBinding(key, to)
      changed = true
    end
  end
  Move("A", { "TURNLEFT" }, "STRAFELEFT")
  Move("D", { "TURNRIGHT" }, "STRAFERIGHT")
  Move("Q", { "STRAFELEFT", "" }, "ACTIONBUTTON5")
  Move("E", { "STRAFERIGHT", "" }, "ACTIONBUTTON6")
  Move("5", { "ACTIONBUTTON5" }, "ACTIONBUTTON7")
  Move("6", { "ACTIONBUTTON6" }, "ACTIONBUTTON8")
  Move("7", { "ACTIONBUTTON7" }, "ACTIONBUTTON9")
  Move("8", { "ACTIONBUTTON8" }, "ACTIONBUTTON10")
  Move("9", { "ACTIONBUTTON9" }, "ACTIONBUTTON11")
  Move("0", { "ACTIONBUTTON10" }, "ACTIONBUTTON12")
  Move("-", { "ACTIONBUTTON11" }, nil)
  Move("=", { "ACTIONBUTTON12" }, nil)
  -- Gear goes on by itself, so B (for buy) opens the shop; F12 still opens the backpack.
  Move("B", { "OPENALLBAGS" }, "BASINBATTLE_SHOP")
  if changed and SaveBindings then
    local set = 1
    if GetCurrentBindingSet then set = GetCurrentBindingSet() or 1 end
    if set ~= 1 and set ~= 2 then set = 1 end
    SaveBindings(set)
    BB.Announce("Basin Battle keys: A and D strafe, spells on 1 2 3 4 Q E, B opens the shop. Rebind Action Button 1-6 in Key Bindings to change the spell keys.")
  end
end
local origActionButtonDown = ActionButtonDown
ActionButtonDown = function(id)
  if picker:IsShown() and BB.offer and id and id >= 1 and id <= BB.offer.n then
    if GetTime() - (BB.offerShownAt or 0) >= 0.7 then
      BB.Pick(id)
    end
    return
  end
  if id and id >= 1 and id <= 12 and BB.spells and BB.spells.manual and (id <= 6 or BB.spells.slots[id]) then
    BB.Cast(id)
    return
  end
  if origActionButtonDown then origActionButtonDown(id) end
end
local origActionButtonUp = ActionButtonUp
ActionButtonUp = function(id)
  if picker:IsShown() and BB.offer and id and id >= 1 and id <= BB.offer.n then return end
  if id and id >= 1 and id <= 12 and BB.spells and BB.spells.manual then
    BB.Release(id)
    return
  end
  if origActionButtonUp then origActionButtonUp(id) end
end
local origReply = ChatFrame_ReplyTell
ChatFrame_ReplyTell = function(chatFrame)
  if picker:IsShown() and BB.offer then
    BB.Reroll()
    return
  end
  if origReply then origReply(chatFrame) end
end

-- --- the waiting-picks button --------------------------------------------------------------------

local pending = MakeButton(UIParent, 230, 38, "Choose a card", GOLD, function()
  Say(".bb open")
end)
BB.pendingButton = pending
pending:SetPoint("BOTTOM", UIParent, "BOTTOM", 0, 150)
pending:SetFrameStrata("HIGH")
pending.glow = Tex(pending, "BACKGROUND", "glow", 330, 110)
pending.glow:SetPoint("CENTER", pending, "CENTER", 0, 0)
pending.glow:SetBlendMode("ADD")
pending.glow:SetVertexColor(1, 0.75, 0.3, 0.5)
pending.label:SetFont(FONT_BODY, 14)
pending.tip = "Your card picks are waiting."
pending:Hide()
pending:SetScript("OnUpdate", function()
  local a = 0.35 + 0.3 * (0.5 + 0.5 * math.sin(GetTime() * 4))
  this.glow:SetAlpha(a)
end)

function BB.UpdatePending()
  if BB.pending > 0 and not picker:IsShown() then
    local s = "s"
    if BB.pending == 1 then s = "" end
    pending.label:SetText("Choose a card  |cffffffff(" .. BB.pending .. ")|r")
    pending.tip = BB.pending .. " card pick" .. s .. " waiting. Click to choose."
    pending:Show()
  else
    pending:Hide()
  end
end

-- --- the wave tracker ----------------------------------------------------------------------------

local hud = CreateFrame("Frame", "BasinBattleHUD", UIParent)
BB.hud = hud
hud:SetWidth(340)
hud:SetHeight(70)
hud:SetPoint("TOP", UIParent, "TOP", 0, -6)
hud:SetFrameStrata("MEDIUM")
hud:Hide()
hud.bg = Tex(hud, "BACKGROUND", "banner", 470, 96)
hud.bg:SetPoint("CENTER", hud, "CENTER", 0, -2)
hud.wave = Text(hud, FONT_TITLE, 25, "OUTLINE")
hud.wave:SetPoint("TOP", hud, "TOP", 0, -4)
hud.wave:SetTextColor(1, 0.84, 0.5)
hud.cycle = CreateFrame("Frame", nil, hud)
hud.cycle:SetWidth(132)
hud.cycle:SetHeight(18)
hud.cycle:SetPoint("LEFT", hud.wave, "RIGHT", 10, 0)
hud.cycle.bg = Tex(hud.cycle, "BACKGROUND", "pill")
hud.cycle.bg:SetAllPoints(hud.cycle)
hud.cycle.bg:SetVertexColor(0.8, 0.15, 0.25, 0.95)
hud.cycle.text = Text(hud.cycle, FONT_BODY, 9.5)
hud.cycle.text:SetPoint("CENTER", hud.cycle, "CENTER", 0, 0)
hud.cycle.text:SetTextColor(1, 0.92, 0.92)

hud.barBack = Tex(hud, "BORDER", "pill", 262, 18)
hud.barBack:SetPoint("TOP", hud, "TOP", 0, -36)
hud.barBack:SetVertexColor(0, 0, 0, 0.75)
hud.bar = CreateFrame("StatusBar", nil, hud)
hud.bar:SetWidth(252)
hud.bar:SetHeight(10)
hud.bar:SetPoint("CENTER", hud.barBack, "CENTER", 0, 0)
hud.bar:SetStatusBarTexture(ART .. "bar")
hud.bar:SetMinMaxValues(0, 1)
hud.bar:SetValue(1)
hud.barText = Text(hud.bar, FONT_NUM, 11, "OUTLINE")
hud.barText:SetPoint("CENTER", hud.bar, "CENTER", 0, 0)
hud.info = Text(hud, FONT_BODY, 11.5)
hud.info:SetPoint("TOP", hud.barBack, "BOTTOM", 0, -4)
hud.info:SetTextColor(0.85, 0.82, 0.74)

hud.start = MakeButton(hud, 92, 22, "Start now", { r = 0.55, g = 1, b = 0.45 }, function()
  Say(".start")
end)
hud.start.label:SetFont(FONT_BODY, 11)
hud.start:SetPoint("LEFT", hud.barBack, "RIGHT", 10, 0)
hud.start.tip = "Bring the next wave now."

hud.hearth = CreateFrame("Button", nil, hud)
hud.hearth:SetWidth(26)
hud.hearth:SetHeight(26)
hud.hearth:SetPoint("RIGHT", hud.barBack, "LEFT", -10, 0)
hud.hearth.icon = hud.hearth:CreateTexture(nil, "ARTWORK")
hud.hearth.icon:SetAllPoints(hud.hearth)
hud.hearth.icon:SetTexture("Interface\\Icons\\INV_Misc_Rune_01")
hud.hearth.icon:SetTexCoord(0.07, 0.93, 0.07, 0.93)
hud.hearth.frame = Tex(hud.hearth, "OVERLAY", "iconframe")
hud.hearth.frame:SetPoint("TOPLEFT", hud.hearth, "TOPLEFT", -3, 3)
hud.hearth.frame:SetPoint("BOTTOMRIGHT", hud.hearth, "BOTTOMRIGHT", 3, -3)
hud.hearth.frame:SetVertexColor(0.8, 0.7, 0.45, 1)
hud.hearth:EnableMouse(true)
hud.hearth:RegisterForClicks("LeftButtonUp")
hud.hearth:SetScript("OnClick", function() Say(".hearth") end)
hud.hearth:SetScript("OnEnter", function()
  GameTooltip:SetOwner(this, "ANCHOR_BOTTOM")
  GameTooltip:SetText("Hearthstone", 1, 1, 1)
  GameTooltip:AddLine("Channel for 3 sec to return to the Ancient. Every 20 sec.", 1, 0.82, 0, 1)
  GameTooltip:AddLine("Also in your bags.", 0.6, 0.6, 0.6, 1)
  GameTooltip:Show()
end)
hud.hearth:SetScript("OnLeave", function() GameTooltip:Hide() end)

-- Your gold, by the tracker's left edge.
hud.gold = CreateFrame("Frame", nil, hud)
hud.gold:SetWidth(90)
hud.gold:SetHeight(20)
hud.gold:SetPoint("RIGHT", hud.hearth, "LEFT", -12, 0)
hud.gold.icon = hud.gold:CreateTexture(nil, "ARTWORK")
hud.gold.icon:SetTexture("Interface\\MoneyFrame\\UI-MoneyIcons")
hud.gold.icon:SetTexCoord(0, 0.25, 0, 1)
hud.gold.icon:SetWidth(14)
hud.gold.icon:SetHeight(14)
hud.gold.icon:SetPoint("RIGHT", hud.gold, "RIGHT", 0, 0)
hud.gold.text = Text(hud.gold, FONT_NUM, 15, "OUTLINE")
hud.gold.text:SetPoint("RIGHT", hud.gold.icon, "LEFT", -3, 0)
hud.gold.text:SetTextColor(1, 0.84, 0.3)
hud.gold:EnableMouse(true)
hud.gold:SetScript("OnEnter", function()
  GameTooltip:SetOwner(this, "ANCHOR_BOTTOM")
  GameTooltip:SetText("Gold", 1, 0.84, 0.3)
  GameTooltip:AddLine("Spend it at the Ancient (right-click it): defences (Protectors, Springs, Defenders), its health, Thornbark, a weapon enchant, or rerolls.", 1, 1, 1, 1)
  GameTooltip:Show()
end)
hud.gold:SetScript("OnLeave", function() GameTooltip:Hide() end)

function BB.UpdateGold()
  local copper = GetMoney() or 0
  local gold = math.floor(copper / 10000)
  if BB.shownGold and gold > BB.shownGold then
    -- A little pop when gold comes in.
    Tween(0.35, 0, OutBack, function(t) hud.gold.text:SetTextHeight(19 - 4 * t) end, nil, "goldpop")
  end
  BB.shownGold = gold
  hud.gold.text:SetText(Comma(gold))
end

local function HubColor(f)
  if f > 0.5 then return 0.25, 0.85, 0.25 end
  if f > 0.25 then return 1, 0.8, 0.15 end
  return 1, 0.2, 0.15
end

-- The last five seconds before a wave, counted down big in the middle of the screen.
local countdown = CreateFrame("Frame", "BasinBattleCountdown", UIParent)
countdown:SetWidth(200)
countdown:SetHeight(120)
countdown:SetPoint("CENTER", UIParent, "CENTER", 0, 170)
countdown:SetFrameStrata("HIGH")
countdown:Hide()
countdown.text = Text(countdown, FONT_TITLE, 72, "OUTLINE")
countdown.text:SetPoint("CENTER", countdown, "CENTER", 0, 0)
countdown.text:SetTextColor(1, 0.84, 0.4)
countdown.sub = Text(countdown, FONT_BODY, 14, "OUTLINE")
countdown.sub:SetPoint("TOP", countdown.text, "BOTTOM", 0, -2)
countdown.sub:SetTextColor(0.9, 0.85, 0.75)

function BB.Countdown(phase, secs, wave)
  if (phase ~= "intermission" and phase ~= "countdown") or secs < 1 or secs > 5 then
    if countdown.shown then
      countdown.shown = nil
      Tween(0.3, 0, OutCubic, function(t) countdown:SetAlpha(1 - t) end, function() countdown:Hide() end, "countdown")
    end
    return
  end
  if countdown.shown == secs then return end
  countdown.shown = secs
  countdown.text:SetText(secs)
  countdown.sub:SetText("Wave " .. (wave + 1))
  countdown:SetAlpha(1)
  countdown:Show()
  Play("tick")
  -- Each number lands big and settles.
  Tween(0.35, 0, OutCubic, function(t) countdown.text:SetTextHeight(110 - 38 * t) end, nil, "countdownpop")
end

function BB.Hud(f)
  local wave = tonumber(f[2]) or 0
  local cycle = tonumber(f[3]) or 0
  local phase = f[4]
  local secs = tonumber(f[5]) or 0
  local hub = tonumber(f[6]) or 0
  local hubMax = tonumber(f[7]) or 1
  local enemies = tonumber(f[8]) or 0
  local claimed = tonumber(f[9]) or 0
  local bases = tonumber(f[10]) or 0
  local best = tonumber(f[11]) or 0
  BB.hudPhase = phase
  BB.Countdown(phase, secs, wave)
  hud:Show()
  BB.UpdateGold()
  if phase == "countdown" or wave == 0 then
    hud.wave:SetText("Basin Battle")
  else
    hud.wave:SetText("Wave " .. wave)
  end
  if cycle > 0 then
    hud.cycle.text:SetText(Spaced("ESCALATION") .. " " .. cycle)
    hud.cycle:Show()
  else
    hud.cycle:Hide()
  end
  local frac = 0
  if hubMax > 0 then frac = hub / hubMax end
  hud.bar:SetValue(frac)
  hud.bar:SetStatusBarColor(HubColor(frac))
  hud.barText:SetText("Ancient  " .. Comma(hub) .. " / " .. Comma(hubMax))
  if BB.lastHub and hub < BB.lastHub then
    hud.barBack:SetVertexColor(0.6, 0, 0, 0.85)
    Tween(0.4, 0, OutCubic, function(t) hud.barBack:SetVertexColor(0.6 * (1 - t), 0, 0, 0.75) end, nil, "hubflash")
  end
  BB.lastHub = hub
  local line
  if phase == "countdown" then
    line = "First wave in " .. Clock(secs)
  elseif phase == "intermission" then
    line = "Wave " .. (wave + 1) .. " in " .. Clock(secs)
  elseif phase == "wave" then
    if enemies == 1 then line = "1 enemy left" else line = enemies .. " enemies left" end
  elseif phase == "defeat" then
    line = "|cffff4040Defeat|r  ·  new run in " .. Clock(secs)
  else
    line = "Waiting for heroes"
  end
  if bases > 0 and wave >= 3 then
    line = line .. "  ·  Bases " .. claimed .. "/" .. bases
  end
  if best > 0 then
    line = line .. "  ·  Best " .. best
  end
  hud.info:SetText(line)
  if phase == "countdown" or phase == "intermission" then
    hud.start:Show()
  else
    hud.start:Hide()
  end
end

-- --- banners -------------------------------------------------------------------------------------

local SPLASH = {
  run = { r = 1, g = 0.84, b = 0.45 },
  wave = { r = 1, g = 0.84, b = 0.45 },
  boss = { r = 1, g = 0.38, b = 0.2 },
  cleared = { r = 0.55, g = 1, b = 0.45 },
  bases = { r = 0.45, g = 0.9, b = 1 },
  claimed = { r = 0.55, g = 1, b = 0.45 },
  escalation = { r = 1, g = 0.3, b = 0.85 },
  defeat = { r = 1, g = 0.22, b = 0.2 },
  victory = { r = 1, g = 0.9, b = 0.35 },
}

local splash = CreateFrame("Frame", "BasinBattleSplash", UIParent)
splash:SetWidth(640)
splash:SetHeight(110)
splash:SetPoint("CENTER", UIParent, "CENTER", 0, 175)
splash:SetFrameStrata("HIGH")
splash:Hide()
splash.bg = Tex(splash, "BACKGROUND", "banner", 760, 130)
splash.bg:SetPoint("CENTER", splash, "CENTER", 0, 0)
splash.top = Tex(splash, "BORDER", "banneredge", 760, 130)
splash.top:SetPoint("CENTER", splash, "CENTER", 0, 0)
splash.top:SetBlendMode("ADD")
splash.glow = Tex(splash, "BORDER", "glow", 600, 160)
splash.glow:SetPoint("CENTER", splash, "CENTER", 0, 8)
splash.glow:SetBlendMode("ADD")
splash.inner = CreateFrame("Frame", nil, splash)
splash.inner:SetWidth(640)
splash.inner:SetHeight(110)
splash.inner:SetPoint("CENTER", splash, "CENTER", 0, 0)
splash.title = Text(splash.inner, FONT_TITLE, 42, "OUTLINE")
splash.title:SetPoint("CENTER", splash.inner, "CENTER", 0, 12)
splash.title:SetShadowOffset(2, -2)
splash.sub = Text(splash.inner, FONT_BODY, 15)
splash.sub:SetPoint("CENTER", splash.inner, "CENTER", 0, -24)
splash.sub:SetTextColor(0.92, 0.9, 0.84)

function BB.Splash(style, title, sub)
  -- The picker says it louder; only a defeat or an Escalation interrupts it.
  if picker:IsShown() and style ~= "defeat" and style ~= "escalation" and style ~= "victory" then return end
  local col = SPLASH[style] or SPLASH.wave
  splash.title:SetText(title)
  splash.title:SetTextColor(col.r, col.g, col.b)
  splash.top:SetVertexColor(col.r, col.g, col.b, 0.9)
  splash.glow:SetVertexColor(col.r, col.g, col.b, 0.25)
  splash.sub:SetText(sub or "")
  splash:SetAlpha(0)
  splash:Show()
  Play(style)
  local hold = 2.6
  if style == "defeat" or style == "escalation" then hold = 4 end
  if style == "victory" then hold = 7 end
  Tween(0.35, 0, OutBack, function(t)
    splash:SetAlpha(math.min(1, t * 1.8))
    splash.inner:SetScale(1.35 - 0.35 * t)
  end, nil, "splash")
  Tween(0.7, hold, OutCubic, function(t) splash:SetAlpha(1 - t) end, function() splash:Hide() end, "splashout")
end

-- Announcements: a line or two under the tracker, fading on their own.
local notes = {}
-- The shop hides them while it's open: they'd run through its title and tabs.
local noteHolder = CreateFrame("Frame", nil, UIParent)
noteHolder:SetAllPoints(UIParent)
BB.noteHolder = noteHolder
for i = 1, 3 do
  local n = Text(noteHolder, FONT_BODY, 14, "OUTLINE")
  n:SetWidth(760)
  n:SetTextColor(1, 0.9, 0.62)
  n:Hide()
  notes[i] = n
end
-- How far the boss bar pushes the lines below the tracker down while it shows.
BB.bossDrop = 0
local function PlaceNotes()
  for i = 1, 3 do
    notes[i]:ClearAllPoints()
    notes[i]:SetPoint("TOP", UIParent, "TOP", 0, -84 - BB.bossDrop - (i - 1) * 20)
  end
end
PlaceNotes()
BB.PlaceNotes = PlaceNotes

function BB.Announce(text)
  -- Push the older lines down.
  for i = 3, 2, -1 do
    notes[i]:SetText(notes[i - 1]:GetText())
    notes[i].until_ = notes[i - 1].until_
    if notes[i - 1]:IsShown() then notes[i]:Show() else notes[i]:Hide() end
    notes[i]:SetAlpha(notes[i - 1]:GetAlpha())
  end
  notes[1]:SetText(text)
  notes[1].until_ = GetTime() + 7
  notes[1]:SetAlpha(1)
  notes[1]:Show()
end

local noteTicker = CreateFrame("Frame")
noteTicker:SetScript("OnUpdate", function()
  local now = GetTime()
  for i = 1, 3 do
    local n = notes[i]
    if n:IsShown() and n.until_ then
      local left = n.until_ - now
      if left <= 0 then
        n:Hide()
      elseif left < 1 then
        n:SetAlpha(left)
      end
    end
  end
end)

-- --- toasts: a card taken, a level gained ------------------------------------------------------

local toast = CreateFrame("Frame", nil, UIParent)
toast:SetWidth(300)
toast:SetHeight(60)
toast:SetPoint("CENTER", UIParent, "CENTER", 0, -120)
toast:SetFrameStrata("HIGH")
toast:Hide()
toast.bg = Tex(toast, "BACKGROUND", "banner", 420, 84)
toast.bg:SetPoint("CENTER", toast, "CENTER", 0, 0)
toast.glow = Tex(toast, "BORDER", "glow", 120, 120)
toast.glow:SetBlendMode("ADD")
toast.icon = toast:CreateTexture(nil, "ARTWORK")
toast.icon:SetWidth(40)
toast.icon:SetHeight(40)
toast.icon:SetTexCoord(0.07, 0.93, 0.07, 0.93)
toast.frame = Tex(toast, "OVERLAY", "iconframe")
toast.frame:SetPoint("TOPLEFT", toast.icon, "TOPLEFT", -5, 5)
toast.frame:SetPoint("BOTTOMRIGHT", toast.icon, "BOTTOMRIGHT", 5, -5)
toast.glow:SetPoint("CENTER", toast.icon, "CENTER", 0, 0)
toast.title = Text(toast, FONT_TITLE, 22, "OUTLINE")
toast.sub = Text(toast, FONT_BODY, 12)
toast.sub:SetTextColor(0.85, 0.82, 0.75)

function BB.Toast(icon, title, sub, col)
  toast.icon:SetTexture(icon)
  toast.frame:SetVertexColor(col.r, col.g, col.b, 1)
  toast.glow:SetVertexColor(col.r, col.g, col.b, 0.7)
  toast.title:SetText(title)
  toast.title:SetTextColor(col.r, col.g, col.b)
  toast.sub:SetText(sub)
  local w = math.max(toast.title:GetStringWidth(), toast.sub:GetStringWidth())
  toast.icon:ClearAllPoints()
  toast.icon:SetPoint("CENTER", toast, "CENTER", -(w + 52) / 2 + 20, 0)
  toast.title:ClearAllPoints()
  toast.title:SetPoint("BOTTOMLEFT", toast.icon, "RIGHT", 12, -1)
  toast.sub:ClearAllPoints()
  toast.sub:SetPoint("TOPLEFT", toast.icon, "RIGHT", 13, -3)
  toast:SetAlpha(0)
  toast:Show()
  Play("toast")
  Tween(0.3, 0, OutBack, function(t)
    toast:SetAlpha(math.min(1, t * 1.5))
    toast:ClearAllPoints()
    toast:SetPoint("CENTER", UIParent, "CENTER", 0, -150 + 30 * t)
  end, nil, "toast")
  Tween(0.6, 2.2, OutCubic, function(t) toast:SetAlpha(1 - t) end, function() toast:Hide() end, "toastout")
end

-- --- falling and rising --------------------------------------------------------------------------

local fallen = CreateFrame("Frame", "BasinBattleFallen", UIParent)
fallen:SetAllPoints(UIParent)
fallen:SetFrameStrata("BACKGROUND")
fallen:Hide()
fallen.vignette = Tex(fallen, "BACKGROUND", "vignette")
fallen.vignette:SetAllPoints(fallen)
fallen.vignette:SetVertexColor(0.6, 0, 0, 0.85)
fallen.title = Text(fallen, FONT_TITLE, 36, "OUTLINE")
fallen.title:SetPoint("CENTER", fallen, "CENTER", 0, -40)
fallen.title:SetTextColor(1, 0.3, 0.25)
fallen.title:SetText("You have fallen")
fallen.sub = Text(fallen, FONT_BODY, 15)
fallen.sub:SetPoint("TOP", fallen.title, "BOTTOM", 0, -8)
fallen.sub:SetTextColor(0.95, 0.85, 0.8)
fallen.sub:SetText("You rise again at the Ancient in a few seconds. Hold on, Ancient!")
fallen:SetScript("OnUpdate", function()
  if StaticPopup_Visible and StaticPopup_Visible("DEATH") then
    StaticPopup_Hide("DEATH")
  end
  if not UnitIsDead("player") then
    fallen:Hide()
  end
end)


-- --- tooltips: your cards, wherever they show ----------------------------------------------------

-- Fill `tip` with a held card as its picker card read: rank, what it does now, the next rank.
function BB.CardTooltip(tip, h)
  local col = RARITY[h.rarity] or RARITY[0]
  tip:SetText(h.name, col.r, col.g, col.b)
  local line = col.name
  if h.kind ~= "" then line = line .. " " .. string.lower(h.kind) end
  tip:AddDoubleLine(line, "Rank " .. h.rank .. " / " .. h.maxRank, col.r, col.g, col.b, 1, 0.82, 0)
  local info = ""
  if h.school ~= "" then info = h.school end
  if h.cooldown ~= "" and h.cooldown ~= "0" then
    if info ~= "" then info = info .. "  ·  " end
    info = info .. "every " .. h.cooldown .. " sec"
  end
  if info ~= "" then tip:AddLine(info, 0.6, 0.6, 0.6) end
  tip:AddLine(h.text, 0.85, 0.83, 0.78, 1)
  if h.next ~= "" then
    tip:AddLine(h.next, 0.35, 1, 0.35, 1)
  end
end

local function IconName(path)
  if not path then return nil end
  local _, _, name = string.find(path, "([^\\/]+)$")
  return string.lower(name or path)
end

-- The card behind a buff, by its icon.
function BB.CardForIcon(path)
  local want = IconName(path)
  if not want then return nil end
  for i = 1, table.getn(BB.build) do
    local h = BB.build[i]
    if string.lower(h.aura) == want or string.lower(h.icon) == want then
      return h
    end
  end
  return nil
end

local origSetPlayerBuff = GameTooltip.SetPlayerBuff
GameTooltip.SetPlayerBuff = function(tip, index)
  -- Shoulder to Shoulder wears Blessing of Kings' crown.
  if IconName(GetPlayerBuffTexture(index)) == "spell_magic_magearmor" then
    tip:ClearLines()
    tip:AddLine("Shoulder to Shoulder", 1, 0.82, 0.35)
    tip:AddLine("Another hero stands within 12 yards: you deal 10% more damage and take 10% less.", 1, 1, 1, 1)
    tip:Show()
    return
  end
  local card = BB.CardForIcon(GetPlayerBuffTexture(index))
  if card then
    BB.CardTooltip(tip, card)
    tip:Show()
    return
  end
  return origSetPlayerBuff(tip, index)
end

-- --- the run's scoreboard: who did what, when a run ends ----------------------------------------

local CLASS_COL = {
  warrior = { r = 0.78, g = 0.61, b = 0.43 },
  priest = { r = 1, g = 1, b = 1 },
  mage = { r = 0.41, g = 0.8, b = 0.94 },
  hunter = { r = 0.67, g = 0.83, b = 0.45 },
  -- The party meter's defence lines.
  base = { r = 0.35, g = 0.75, b = 0.3 },
  soldiers = { r = 0.55, g = 0.62, b = 0.75 },
}
local SCORE_COLS = { { "Hero", -250, "LEFT" }, { "Level", -10, "CENTER" }, { "Kills", 55, "CENTER" }, { "Damage", 125, "CENTER" }, { "Dodged", 190, "CENTER" }, { "Fell", 245, "CENTER" } }
local score = CreateFrame("Frame", "BasinBattleScore", UIParent)
score:SetWidth(600)
score:SetHeight(200)
score:SetPoint("CENTER", UIParent, "CENTER", 0, -20)
score:SetFrameStrata("HIGH")
score:Hide()
score.bg = score:CreateTexture(nil, "BACKGROUND")
score.bg:SetAllPoints(score)
score.bg:SetTexture(0.04, 0.03, 0.02)
score.bg:SetAlpha(0.88)
score.title = Text(score, FONT_TITLE, 30, "OUTLINE")
score.title:SetPoint("TOP", score, "TOP", 0, -14)
score.sub = Text(score, FONT_BODY, 12)
score.sub:SetPoint("TOP", score.title, "BOTTOM", 0, -4)
score.sub:SetTextColor(0.85, 0.82, 0.74)
score.head = {}
for i = 1, table.getn(SCORE_COLS) do
  local c = SCORE_COLS[i]
  local t = Text(score, FONT_BODY, 11)
  t:SetPoint(c[3], score, "TOP", c[2], -78)
  t:SetTextColor(0.7, 0.66, 0.55)
  t:SetText(c[1])
  score.head[i] = t
end
score.rows = {}
for r = 1, 4 do
  local row = {}
  for i = 1, table.getn(SCORE_COLS) do
    local c = SCORE_COLS[i]
    local t = Text(score, i == 1 and FONT_BODY or FONT_NUM, i == 1 and 13 or 14, "OUTLINE")
    t:SetPoint(c[3], score, "TOP", c[2], -82 - r * 24)
    row[i] = t
  end
  score.rows[r] = row
end
score.close = MakeButton(score, 90, 22, "Close", GOLD, function() score:Hide() end)
score.close:SetPoint("BOTTOM", score, "BOTTOM", 0, 10)
score:SetScript("OnUpdate", function()
  -- Out of the way of a new run's first pick, and gone on its own after a while.
  if picker:IsShown() or GetTime() > (this.until_ or 0) then this:Hide() end
end)

-- With thousands as "k": 12.3k.
local function Short(n)
  if n >= 1000000 then return string.format("%.1fm", n / 1000000) end
  if n >= 10000 then return string.format("%.0fk", n / 1000) end
  if n >= 1000 then return string.format("%.1fk", n / 1000) end
  return tostring(n)
end

function BB.Scoreboard(f)
  local won = f[2] == "1"
  local depth = tonumber(f[3]) or 0
  local secs = tonumber(f[4]) or 0
  if won then
    score.title:SetText("Victory")
    score.title:SetTextColor(1, 0.84, 0.4)
  else
    score.title:SetText("The Ancient has fallen")
    score.title:SetTextColor(1, 0.35, 0.25)
  end
  local renown = tonumber(f[6]) or 0
  local sub = "Wave " .. depth .. " held, in " .. math.floor(secs / 60) .. " min " .. math.mod(secs, 60) .. " sec"
  if renown > 0 then sub = sub .. "   |cffffd060+" .. renown .. " Renown|r" end
  score.sub:SetText(sub)
  local rows = Split(f[5] or "", ";")
  local n = 0
  for r = 1, 4 do
    local row = score.rows[r]
    local p = rows[r] and rows[r] ~= "" and Split(rows[r], "^")
    if p and table.getn(p) >= 6 then
      n = r
      local col = CLASS_COL[p[2]] or GOLD
      row[1]:SetText(p[1])
      row[1]:SetTextColor(col.r, col.g, col.b)
      row[2]:SetText(p[3])
      row[3]:SetText(p[4])
      row[4]:SetText(Short(tonumber(p[5]) or 0))
      row[5]:SetText(p[7] or "0")
      row[6]:SetText(p[6])
      for i = 1, 6 do row[i]:Show() end
    else
      for i = 1, 6 do row[i]:Hide() end
    end
  end
  score:SetHeight(130 + n * 24)
  score.until_ = GetTime() + 25
  score:Show()
end

-- --- "Dodged!": stepping out of a strike aimed at you, rewarded ---------------------------------

local dodged = CreateFrame("Frame", nil, UIParent)
dodged:SetWidth(400)
dodged:SetHeight(50)
dodged:SetPoint("CENTER", UIParent, "CENTER", 0, -110)
dodged:SetFrameStrata("HIGH")
dodged:Hide()
dodged.big = Text(dodged, FONT_TITLE, 26, "OUTLINE")
dodged.big:SetPoint("TOP", dodged, "TOP", 0, 0)
dodged.big:SetTextColor(0.45, 1, 0.45)
dodged.big:SetText("Dodged!")
dodged.small = Text(dodged, FONT_BODY, 11, "OUTLINE")
dodged.small:SetPoint("TOP", dodged.big, "BOTTOM", 0, -2)
dodged.small:SetTextColor(0.8, 1, 0.8)
dodged:SetScript("OnUpdate", function()
  local t = GetTime() - (this.at or 0)
  if t > 1.3 then
    this:Hide()
    return
  end
  -- Up a little and fading, after a moment's hold.
  this:SetAlpha(t < 0.5 and 1 or math.max(0, 1 - (t - 0.5) / 0.8))
  this:ClearAllPoints()
  this:SetPoint("CENTER", UIParent, "CENTER", 0, -110 + 24 * t)
end)

function BB.Dodged(name)
  dodged.small:SetText(name or "")
  dodged.at = GetTime()
  dodged:SetAlpha(1)
  dodged:Show()
end

-- --- the boss bar: under the tracker while a boss is in the field --------------------------------

local bossbar = CreateFrame("Frame", "BasinBattleBoss", UIParent)
bossbar:SetWidth(380)
bossbar:SetHeight(34)
bossbar:SetPoint("TOP", UIParent, "TOP", 0, -80)
bossbar:SetFrameStrata("MEDIUM")
bossbar:Hide()
bossbar.name = Text(bossbar, FONT_TITLE, 15, "OUTLINE")
bossbar.name:SetPoint("TOP", bossbar, "TOP", 0, 0)
bossbar.name:SetTextColor(1, 0.45, 0.3)
bossbar.back = Tex(bossbar, "BACKGROUND", "pill", 384, 20)
bossbar.back:SetPoint("BOTTOM", bossbar, "BOTTOM", 0, 0)
bossbar.back:SetVertexColor(0, 0, 0, 0.8)
bossbar.bar = CreateFrame("StatusBar", nil, bossbar)
bossbar.bar:SetWidth(372)
bossbar.bar:SetHeight(12)
bossbar.bar:SetPoint("CENTER", bossbar.back, "CENTER", 0, 0)
bossbar.bar:SetStatusBarTexture(ART .. "bar")
bossbar.bar:SetStatusBarColor(0.85, 0.2, 0.15)
bossbar.bar:SetMinMaxValues(0, 1000)
bossbar.pct = Text(bossbar.bar, FONT_NUM, 11, "OUTLINE")
bossbar.pct:SetPoint("CENTER", bossbar.bar, "CENTER", 0, 0)
-- Where the fight turns: it calls its dead at half, rages at a quarter.
bossbar.ticks = {}
for i, at in ipairs({ 0.5, 0.25 }) do
  local t = bossbar.bar:CreateTexture(nil, "OVERLAY")
  t:SetTexture(1, 0.85, 0.4, 0.9)
  t:SetWidth(2)
  t:SetHeight(16)
  t:SetPoint("CENTER", bossbar.bar, "LEFT", 372 * at, 0)
  bossbar.ticks[i] = t
end

function BB.BossBar(f)
  local name = f[2] or ""
  if name == "" then
    if bossbar:IsShown() then
      bossbar:Hide()
      BB.bossDrop = 0
      BB.PlaceNotes()
      BB.castbar:ClearAllPoints()
      BB.castbar:SetPoint("TOP", UIParent, "TOP", 0, -150)
    end
    return
  end
  local v = tonumber(f[3]) or 0
  bossbar.name:SetText(name)
  bossbar.bar:SetValue(v)
  bossbar.pct:SetText(math.floor(v / 10 + 0.5) .. "%")
  if v <= 250 then
    bossbar.name:SetTextColor(1, 0.2, 0.15)
  else
    bossbar.name:SetTextColor(1, 0.45, 0.3)
  end
  if not bossbar:IsShown() then
    bossbar:Show()
    BB.bossDrop = 40
    BB.PlaceNotes()
    BB.castbar:ClearAllPoints()
    BB.castbar:SetPoint("TOP", UIParent, "TOP", 0, -150 - BB.bossDrop)
  end
end

-- --- enemy casts: a bar to watch, a warning to move ------------------------------------------

local castbar = CreateFrame("Frame", "BasinBattleCast", UIParent)
castbar:SetWidth(300)
castbar:SetHeight(34)
castbar:SetPoint("TOP", UIParent, "TOP", 0, -150)
BB.castbar = castbar
castbar:SetFrameStrata("HIGH")
castbar:Hide()
castbar.back = Tex(castbar, "BACKGROUND", "pill", 304, 18)
castbar.back:SetPoint("BOTTOM", castbar, "BOTTOM", 0, 0)
castbar.back:SetVertexColor(0, 0, 0, 0.8)
castbar.bar = CreateFrame("StatusBar", nil, castbar)
castbar.bar:SetWidth(294)
castbar.bar:SetHeight(10)
castbar.bar:SetPoint("CENTER", castbar.back, "CENTER", 0, 0)
castbar.bar:SetStatusBarTexture(ART .. "bar")
castbar.bar:SetMinMaxValues(0, 1)
castbar.text = Text(castbar, FONT_BODY, 13, "OUTLINE")
castbar.text:SetPoint("BOTTOM", castbar.back, "TOP", 0, 2)
castbar:SetScript("OnUpdate", function()
  local left = this.ends - GetTime()
  if left <= 0 then
    this:Hide()
    return
  end
  this.bar:SetValue(1 - left / this.total)
end)

-- Your own cast bar, for shots the game draws none for (Aimed Shot): in the stock bar's place.
local mycast = CreateFrame("Frame", "BasinBattleMyCast", UIParent)
mycast:SetWidth(206)
mycast:SetHeight(26)
mycast:SetPoint("BOTTOM", UIParent, "BOTTOM", 0, 190)
mycast:SetFrameStrata("HIGH")
mycast:Hide()
mycast.back = mycast:CreateTexture(nil, "BACKGROUND")
mycast.back:SetTexture("Interface\\Tooltips\\UI-Tooltip-Background")
mycast.back:SetAllPoints(mycast)
mycast.back:SetVertexColor(0, 0, 0, 0.6)
mycast.bar = CreateFrame("StatusBar", nil, mycast)
mycast.bar:SetPoint("TOPLEFT", mycast, "TOPLEFT", 3, -3)
mycast.bar:SetPoint("BOTTOMRIGHT", mycast, "BOTTOMRIGHT", -3, 3)
mycast.bar:SetStatusBarTexture(ART .. "bar")
mycast.bar:SetStatusBarColor(1, 0.7, 0)
mycast.bar:SetMinMaxValues(0, 1)
mycast.text = Text(mycast, FONT_BODY, 12, "OUTLINE")
mycast.text:SetPoint("CENTER", mycast, "CENTER", 0, 0)
mycast:SetScript("OnUpdate", function()
  local now = BB.pausedAt or GetTime()
  local done = (now - this.started) / math.max(0.1, this.total)
  if done >= 1 then
    this:Hide()
    return
  end
  this.bar:SetValue(done)
end)

function BB.MyCast(name, ms)
  if not name or name == "" then
    mycast:Hide()
    return
  end
  mycast.started = BB.pausedAt or GetTime()
  mycast.total = (tonumber(ms) or 1000) / 1000
  mycast.text:SetText(name)
  mycast.bar:SetValue(0)
  mycast:Show()
end

-- Enemy casts: the marks on the ground and the caster's own spell say where to step, so there
-- is no warning text. Bosses and elites get a cast bar (red when it's coming for you).
function BB.EnemyCast(who, name, ms, danger, warn, boss)
  if not boss then
    return
  end
  local secs = (tonumber(ms) or 2000) / 1000
  castbar.total = secs
  castbar.ends = GetTime() + secs
  castbar.text:SetText(who .. ":  |cffffffff" .. name .. "|r")
  if danger then
    castbar.bar:SetStatusBarColor(1, 0.25, 0.15)
  else
    castbar.bar:SetStatusBarColor(1, 0.7, 0.2)
  end
  castbar:Show()
end


-- --- the spell bar: six keys ---------------------------------------------------------------------

-- Your spells (cards' and the ones talents teach, six at most) sit over the first six buttons
-- of the main action bar (keys 1, 2, 3, 4, Q, E, or whatever you bind Action Button 1-6 to).
-- They hit your target, or the nearest enemy when you have none.

local SOLID = "Interface\\Tooltips\\UI-Tooltip-Background"
local SLOT_SIZE = 36
local KEY_LABEL = { "1", "2", "3", "4", "Q", "E", "5", "6", "7", "8", "9", "0" }

-- The key slot `i` is on now: whatever Action Button i is bound to (rebind it in Key Bindings),
-- shortened to fit the button.
local function KeyLabel(i)
  local k = nil
  if GetBindingKey then k = GetBindingKey("ACTIONBUTTON" .. i) end
  if not k or k == "" then return KEY_LABEL[i] end
  k = string.gsub(k, "SHIFT%-", "s")
  k = string.gsub(k, "CTRL%-", "c")
  k = string.gsub(k, "ALT%-", "a")
  k = string.gsub(k, "BUTTON", "M")
  k = string.gsub(k, "MOUSEWHEELUP", "WU")
  k = string.gsub(k, "MOUSEWHEELDOWN", "WD")
  k = string.gsub(k, "NUMPAD", "N")
  return k
end
BB.KeyLabel = KeyLabel
BB.spells = { manual = false, gcdEnd = 0, gcd = 1, slots = {}, rage = -1 }

local function MakeSpellButton(i)
  local b = CreateFrame("Button", "BasinBattleSpell" .. i, UIParent)
  b:SetWidth(SLOT_SIZE)
  b:SetHeight(SLOT_SIZE)
  b:SetFrameStrata("HIGH")
  b.id = i
  b.back = b:CreateTexture(nil, "BACKGROUND")
  b.back:SetTexture(SOLID)
  b.back:SetAllPoints(b)
  b.back:SetVertexColor(0, 0, 0, 0.55)
  b.icon = b:CreateTexture(nil, "BORDER")
  b.icon:SetAllPoints(b)
  b.icon:SetTexCoord(0.07, 0.93, 0.07, 0.93)
  b.shade = b:CreateTexture(nil, "ARTWORK")
  b.shade:SetTexture(SOLID)
  b.shade:SetVertexColor(0, 0, 0, 0.72)
  b.shade:SetPoint("BOTTOMLEFT", b, "BOTTOMLEFT", 0, 0)
  b.shade:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", 0, 0)
  b.shade:SetHeight(SLOT_SIZE)
  b.shade:Hide()
  b.frame = Tex(b, "OVERLAY", "iconframe")
  b.frame:SetPoint("TOPLEFT", b, "TOPLEFT", -4, 4)
  b.frame:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", 4, -4)
  b.flash = Tex(b, "OVERLAY", "glow", 90, 90)
  b.flash:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.flash:SetBlendMode("ADD")
  b.flash:SetAlpha(0)
  b.on = Tex(b, "OVERLAY", "ring", 52, 52)
  b.on:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.on:SetBlendMode("ADD")
  b.on:SetVertexColor(0.7, 0.4, 1, 0.9)
  b.on:Hide()
  b.key = Text(b, FONT_NUM, 12, "OUTLINE")
  b.key:SetPoint("TOPLEFT", b, "TOPLEFT", 2, -2)
  b.key:SetTextColor(0.9, 0.9, 0.9)
  b.key:SetText(KEY_LABEL[i])
  b.count = Text(b, FONT_NUM, 16, "OUTLINE")
  b.count:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.count:SetTextColor(1, 0.95, 0.7)
  b.rank = Text(b, FONT_NUM, 10, "OUTLINE")
  b.rank:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -2, 2)
  b.rank:SetTextColor(1, 0.82, 0.3)
  b.cost = Text(b, FONT_NUM, 10, "OUTLINE")
  b.cost:SetPoint("BOTTOMLEFT", b, "BOTTOMLEFT", 2, 2)
  b.cost:SetTextColor(1, 0.35, 0.3)
  b:EnableMouse(true)
  b:RegisterForClicks("LeftButtonUp", "RightButtonUp")
  b:SetScript("OnClick", function()
    -- Shift-click two spells to swap their keys (the same as dragging one onto the other).
    if IsShiftKeyDown and IsShiftKeyDown() then
      BB.Release(this.id)
      local from = BB.swapFrom
      if from and from ~= this.id and BB.spells.slots[from] and this.id <= 6 then
        BB.swapFrom = nil
        Play("hover")
        Say(".bb move " .. from .. " " .. this.id)
      elseif BB.spells.slots[this.id] then
        BB.swapFrom = this.id
        UIErrorsFrame:AddMessage("Shift-click another spell to swap their keys.", 0.4, 0.8, 1, 1, 2)
      end
      return
    end
    BB.swapFrom = nil
    BB.Cast(this.id)
    BB.Release(this.id)
  end)
  -- Drag a spell onto another to swap their keys.
  b:RegisterForDrag("LeftButton")
  b:SetScript("OnDragStart", function()
    local slot = BB.spells.slots[this.id]
    if not slot then return end
    BB.dragFrom = this.id
    BB.dragGhost.icon:SetTexture("Interface\\Icons\\" .. slot.icon)
    BB.dragGhost:Show()
    GameTooltip:Hide()
  end)
  b:SetScript("OnDragStop", function()
    BB.dragGhost:Hide()
    -- The drop (OnReceiveDrag on the target) comes right after: let it see where it came from.
    BB.dragDoneAt = GetTime()
  end)
  b:SetScript("OnReceiveDrag", function()
    local from = BB.dragFrom
    BB.dragFrom = nil
    if not from or from == this.id then return end
    -- Onto another spell (a swap) or an empty key (a move).
    local a = BB.spells.slots[from]
    if not a or this.id > 6 then return end
    Play("hover")
    Say(".bb move " .. from .. " " .. this.id)
  end)
  b:SetScript("OnEnter", function()
    local slot = BB.spells.slots[this.id]
    GameTooltip:SetOwner(this, "ANCHOR_TOP")
    if not slot then
      GameTooltip:SetText("Spell slot " .. KeyLabel(this.id), 1, 1, 1)
      GameTooltip:AddLine("Empty: a spell card (or a talent that teaches a spell) fills it.", 0.7, 0.7, 0.7, 1)
      GameTooltip:AddLine("Drag a spell here (or Shift-click it, then this) to move it to this key.", 0.6, 0.6, 0.6, 1)
    else
      local held = nil
      for k = 1, table.getn(BB.build or {}) do
        if BB.build[k].name == slot.name then held = BB.build[k] end
      end
      if held then
        BB.CardTooltip(GameTooltip, held)
      else
        GameTooltip:SetText(slot.name, 1, 1, 1)
      end
      if slot.cost > 0 then
        GameTooltip:AddLine("Costs " .. slot.cost .. " rage.", 1, 0.35, 0.3)
      end
      GameTooltip:AddLine("Drag onto another spell (or Shift-click both) to swap their keys.", 0.6, 0.6, 0.6, 1)
      if BB.spells.manual then
        GameTooltip:AddLine("Press " .. KeyLabel(this.id) .. " or click: at your target, or the nearest enemy.", 0.4, 0.8, 1, 1)
        GameTooltip:AddLine("Change the key: Key Bindings, Action Button " .. this.id .. ".", 0.6, 0.6, 0.6, 1)
      else
        GameTooltip:AddLine("Fires on its own. Type .bb auto to cast it yourself.", 0.4, 0.8, 1, 1)
      end
    end
    GameTooltip:Show()
  end)
  b:SetScript("OnLeave", function() GameTooltip:Hide() end)
  b:Hide()
  return b
end

local spellButtons = {}
for i = 1, 12 do
  spellButtons[i] = MakeSpellButton(i)
end

-- The icon under the cursor while a spell is dragged to a new key.
BB.dragGhost = CreateFrame("Frame", nil, UIParent)
BB.dragGhost:SetWidth(32)
BB.dragGhost:SetHeight(32)
BB.dragGhost:SetFrameStrata("TOOLTIP")
BB.dragGhost.icon = BB.dragGhost:CreateTexture(nil, "ARTWORK")
BB.dragGhost.icon:SetAllPoints(BB.dragGhost)
BB.dragGhost.icon:SetTexCoord(0.07, 0.93, 0.07, 0.93)
BB.dragGhost:SetAlpha(0.8)
BB.dragGhost:Hide()
BB.dragGhost:SetScript("OnUpdate", function()
  local x, y = GetCursorPosition()
  local s = UIParent:GetEffectiveScale()
  this:ClearAllPoints()
  this:SetPoint("CENTER", UIParent, "BOTTOMLEFT", x / s, y / s)
end)

-- Over the main bar's own buttons when they're there; otherwise a row of our own.
local function PlaceSpellButtons()
  for i = 1, 12 do
    local b = spellButtons[i]
    local stock = getglobal("ActionButton" .. i)
    b:ClearAllPoints()
    if stock and stock.GetWidth and stock:GetWidth() > 0 then
      b:SetWidth(stock:GetWidth())
      b:SetHeight(stock:GetHeight())
      b:SetPoint("CENTER", stock, "CENTER", 0, 0)
    else
      b:SetPoint("BOTTOM", UIParent, "BOTTOM", (i - 6.5) * (SLOT_SIZE + 6), 60)
    end
  end
end
PlaceSpellButtons()

-- Casting by hand, or letting them fire: a little switch beside the bar.
local autoToggle = MakeButton(UIParent, 54, 20, "Auto", { r = 0.5, g = 0.8, b = 1 }, function()
  Say(".bb auto")
end)
autoToggle.label:SetFont(FONT_BODY, 10)
autoToggle:SetFrameStrata("HIGH")
autoToggle:Hide()
local function PlaceToggle()
  autoToggle:ClearAllPoints()
  autoToggle:SetPoint("BOTTOMLEFT", spellButtons[1], "TOPLEFT", -2, 8)
end
PlaceToggle()

function BB.UpdateSpellBar()
  local sb = BB.spells
  local cards = 0
  for i = 1, 12 do
    if sb.slots[i] then cards = cards + 1 end
  end
  for i = 1, 12 do
    local b = spellButtons[i]
    local slot = sb.slots[i]
    b.key:SetText(KeyLabel(i))
    if slot then
      b.icon:SetTexture("Interface\\Icons\\" .. slot.icon)
      b.icon:SetVertexColor(1, 1, 1)
      local col = RARITY[slot.rarity] or RARITY[0]
      if slot.talent then col = { r = 1, g = 0.82, b = 0.35 } end
      b.frame:SetVertexColor(col.r, col.g, col.b, 1)
      if slot.rank > 1 then b.rank:SetText(slot.rank) else b.rank:SetText("") end
      if slot.cost > 0 then b.cost:SetText(slot.cost) else b.cost:SetText("") end
      if slot.on then b.on:Show() else b.on:Hide() end
      b:Show()
    elseif i <= 6 and cards > 0 and sb.manual then
      -- Empty card slots show while you have spells, so the six keys read as a bar.
      b.icon:SetTexture(nil)
      b.frame:SetVertexColor(0.4, 0.4, 0.4, 0.6)
      b.rank:SetText("")
      b.cost:SetText("")
      b.on:Hide()
      b.shade:Hide()
      b.count:SetText("")
      b:Show()
    else
      b:Hide()
    end
  end
  -- The switch stays out of sight: you cast by hand (`.bb auto` still lets them fire alone).
  if false then
    if sb.manual then
      autoToggle.label:SetText("Manual")
      autoToggle.tip = "You cast your spells with 1, 2, 3, 4, Q and E, for 15% more spell damage. Click to let them fire on their own instead."
    else
      autoToggle.label:SetText("Auto")
      autoToggle.tip = "Your spells fire on their own. Click to cast them yourself with 1, 2, 3, 4, Q and E (15% more spell damage)."
    end
    autoToggle:Show()
  else
    autoToggle:Hide()
  end
end

-- The rage you have now (warriors).
local function Rage()
  if BB.spells.rage < 0 then return 1000 end
  return UnitMana("player") or 0
end

-- Press feedback, and a quick check so a key held down doesn't flood the server.
function BB.Cast(i)
  local sb = BB.spells
  local slot = sb.slots[i]
  local b = spellButtons[i]
  if b then
    b.icon:SetVertexColor(0.6, 0.6, 0.6)
  end
  if not slot then
    BB.CastError("No spell on key " .. KeyLabel(i) .. " yet.")
    return
  end
  local now = GetTime()
  -- Almost ready (or the global cooldown): it goes off the moment it can.
  local wait = math.max(slot.readyAt - now, sb.gcdEnd - now)
  if wait > 0.05 and wait <= 0.6 then
    BB.queued = i
    BB.queuedAt = now
    return
  end
  if slot.readyAt - now > 0.6 then
    BB.CastError(slot.name .. " is not ready yet.")
    return
  end
  if slot.cost > 0 and Rage() < slot.cost then
    BB.CastError("Not enough rage.")
    return
  end
  BB.queued = nil
  if BB.lastCast and now - BB.lastCast < 0.12 then return end
  BB.lastCast = now
  Say(".bb cast " .. i)
end

function BB.Release(i)
  local b = spellButtons[i]
  if b then b.icon:SetVertexColor(1, 1, 1) end
end

function BB.SpellLine(f)
  local sb = BB.spells
  local now = BB.pausedAt or GetTime()
  -- (Slots are by key, 1-12, with holes where a key is empty.)
  local before = {}
  for i = 1, 12 do
    if sb.slots[i] then before[sb.slots[i].key] = sb.slots[i] end
  end
  sb.manual = f[2] == "1"
  sb.gcdEnd = now + (tonumber(f[3]) or 0)
  sb.rage = tonumber(f[4]) or -1
  sb.slots = {}
  if f[5] and f[5] ~= "" then
    local items = Split(f[5], ";")
    for i = 1, table.getn(items) do
      local p = Split(items[i], "^")
      -- An empty key between spells comes as a blank entry.
      if p[1] == nil or p[1] == "" then p = nil end
      local slot = p and {
        key = p[1], icon = p[2], name = p[3], rank = tonumber(p[4]) or 1, rarity = tonumber(p[5]) or 0,
        cd = tonumber(p[6]) or 0, readyAt = now + (tonumber(p[7]) or 0), cost = tonumber(p[8]) or 0,
        talent = p[9] == "1", on = p[10] == "1",
      }
      local old = slot and before[slot.key]
      local b = spellButtons[i]
      if not slot then
        -- nothing on this key
      elseif b and not old then
        -- A new spell: a flash on its key.
        Tween(0.9, 0, OutCubic, function(t) b.flash:SetAlpha(1 - t) end, nil, "spellnew" .. i)
      elseif b and old.readyAt <= now + 0.05 and slot.readyAt > now + 0.05 then
        -- Just cast: a pop.
        Tween(0.25, 0, OutCubic, function(t) b.flash:SetAlpha(0.8 * (1 - t)) end, nil, "spellcast" .. i)
      end
      if slot then
        slot.wasReady = slot.readyAt <= now
        if i <= 12 then sb.slots[i] = slot end
      end
    end
  end
  BB.UpdateSpellBar()
end

-- Cooldown shades, counts, rage, and a flash when a spell comes back.
local barTicker = CreateFrame("Frame")
barTicker:SetScript("OnUpdate", function()
  local sb = BB.spells
  -- While the world waits on a card pick, cooldowns stand still (on the server too).
  local now = BB.pausedAt or GetTime()
  -- A queued press goes off when its spell and the global cooldown are ready.
  if BB.queued then
    local q = sb.slots[BB.queued]
    if not q or now - (BB.queuedAt or 0) > 1.2 then
      BB.queued = nil
    elseif q.readyAt <= now + 0.03 and sb.gcdEnd <= now + 0.03 then
      local i = BB.queued
      BB.queued = nil
      BB.lastCast = nil
      BB.Cast(i)
    end
  end
  local rage = Rage()
  for i = 1, 12 do
    local slot = sb.slots[i]
    local b = spellButtons[i]
    if slot then
    local left = slot.readyAt - now
    local gcdLeft = sb.gcdEnd - now
    if slot.cost > 0 and rage < slot.cost then
      b.icon:SetVertexColor(0.45, 0.45, 0.85)
    elseif not b.pressed then
      b.icon:SetVertexColor(1, 1, 1)
    end
    if left > 0 and slot.cd > 0 then
      local frac = math.min(1, left / slot.cd)
      b.shade:SetHeight(math.max(1, b:GetHeight() * frac))
      b.shade:Show()
      if left >= 1 then
        b.count:SetText(math.floor(left + 0.5))
      else
        b.count:SetText(math.floor(left * 10 + 0.5) / 10)
      end
      slot.wasReady = false
    elseif gcdLeft > 0 and sb.manual then
      b.shade:SetHeight(math.max(1, b:GetHeight() * math.min(1, gcdLeft / math.max(0.3, sb.gcd))))
      b.shade:Show()
      b.count:SetText("")
    else
      b.shade:Hide()
      b.count:SetText("")
      if not slot.wasReady then
        slot.wasReady = true
        if sb.manual then
          Tween(0.5, 0, OutCubic, function(t) b.flash:SetAlpha(0.7 * (1 - t)) end, nil, "spellready" .. i)
        end
      end
    end
    end
  end
end)

-- --- talents: three trees, a point a level -----------------------------------------------------

BB.talents = { trees = {}, defs = {}, ranks = {}, points = 0, spent = { 0, 0, 0 }, ready = false }

local TAL_W = 236
local TAL_H = 346
local TAL_ICON = 34
local talents = CreateFrame("Frame", "BasinBattleTalents", UIParent)
talents:SetWidth(TAL_W * 3 + 40)
talents:SetHeight(TAL_H + 110)
talents:SetPoint("CENTER", UIParent, "CENTER", 0, 30)
talents:SetFrameStrata("DIALOG")
talents:EnableMouse(true)
talents:Hide()
talents.bg = talents:CreateTexture(nil, "BACKGROUND")
talents.bg:SetTexture(SOLID)
talents.bg:SetAllPoints(talents)
talents.bg:SetVertexColor(0.03, 0.03, 0.04, 0.94)
talents.edge = Tex(talents, "BORDER", "cardborder")
talents.edge:SetPoint("TOPLEFT", talents, "TOPLEFT", -10, 10)
talents.edge:SetPoint("BOTTOMRIGHT", talents, "BOTTOMRIGHT", 10, -10)
talents.edge:SetVertexColor(1, 0.82, 0.45, 0.8)
talents.title = Text(talents, FONT_TITLE, 26, "OUTLINE")
talents.title:SetPoint("TOP", talents, "TOP", 0, -12)
talents.title:SetTextColor(1, 0.84, 0.5)
talents.title:SetText("Talents")
talents.points = Text(talents, FONT_BODY, 14, "OUTLINE")
talents.points:SetPoint("BOTTOMLEFT", talents, "BOTTOMLEFT", 22, 18)
talents.hint = Text(talents, FONT_BODY, 11)
talents.hint:SetPoint("BOTTOM", talents, "BOTTOM", 0, 20)
talents.hint:SetTextColor(0.7, 0.68, 0.6)
talents.hint:SetText("Talents come as cards when you level up, whole. Each row opens after 3 more points in its tree.")
talents.close = MakeButton(talents, 80, 24, "Close", GOLD, function() talents:Hide() end)
talents.close:SetPoint("BOTTOMRIGHT", talents, "BOTTOMRIGHT", -18, 14)
talents.reset = MakeButton(talents, 80, 24, "Reset", { r = 1, g = 0.45, b = 0.35 }, function() Say(".bb untalent") end)
talents.reset:SetPoint("RIGHT", talents.close, "LEFT", -8, 0)
talents.reset.tip = "Talents stay for the run."
talents.reset:Hide()
talents.panels = {}
talents.buttons = {}

local function MakePanel(t)
  local p = CreateFrame("Frame", nil, talents)
  p:SetWidth(TAL_W)
  p:SetHeight(TAL_H)
  p:SetPoint("TOPLEFT", talents, "TOPLEFT", 14 + (t - 1) * (TAL_W + 6), -50)
  -- The tree's painting, in its four pieces (320 x 384 in all).
  p.tl = p:CreateTexture(nil, "BACKGROUND")
  p.tr = p:CreateTexture(nil, "BACKGROUND")
  p.bl = p:CreateTexture(nil, "BACKGROUND")
  p.br = p:CreateTexture(nil, "BACKGROUND")
  local sx, sy = TAL_W / 320, TAL_H / 384
  p.tl:SetWidth(256 * sx)
  p.tl:SetHeight(256 * sy)
  p.tl:SetPoint("TOPLEFT", p, "TOPLEFT", 0, 0)
  p.tr:SetWidth(64 * sx)
  p.tr:SetHeight(256 * sy)
  p.tr:SetPoint("TOPLEFT", p.tl, "TOPRIGHT", 0, 0)
  p.bl:SetWidth(256 * sx)
  p.bl:SetHeight(128 * sy)
  p.bl:SetPoint("TOPLEFT", p.tl, "BOTTOMLEFT", 0, 0)
  p.br:SetWidth(64 * sx)
  p.br:SetHeight(128 * sy)
  p.br:SetPoint("TOPLEFT", p.tl, "BOTTOMRIGHT", 0, 0)
  p.shade = p:CreateTexture(nil, "BORDER")
  p.shade:SetTexture(SOLID)
  p.shade:SetAllPoints(p)
  p.shade:SetVertexColor(0, 0, 0, 0.35)
  p.icon = p:CreateTexture(nil, "ARTWORK")
  p.icon:SetWidth(28)
  p.icon:SetHeight(28)
  p.icon:SetPoint("TOPLEFT", p, "TOPLEFT", 8, -8)
  p.icon:SetTexCoord(0.07, 0.93, 0.07, 0.93)
  p.name = Text(p, FONT_TITLE, 19, "OUTLINE")
  p.name:SetPoint("LEFT", p.icon, "RIGHT", 8, 4)
  p.name:SetTextColor(1, 0.84, 0.5)
  p.spent = Text(p, FONT_NUM, 13, "OUTLINE")
  p.spent:SetPoint("TOPRIGHT", p, "TOPRIGHT", -10, -12)
  p.text = Text(p, FONT_BODY, 10)
  p.text:SetPoint("TOPLEFT", p.icon, "BOTTOMLEFT", 0, -4)
  p.text:SetWidth(TAL_W - 16)
  p.text:SetJustifyH("LEFT")
  p.text:SetTextColor(0.85, 0.83, 0.75)
  return p
end

for t = 1, 3 do
  talents.panels[t] = MakePanel(t)
end

local function TalentButton(i)
  local b = CreateFrame("Button", nil, talents)
  b:SetWidth(TAL_ICON)
  b:SetHeight(TAL_ICON)
  b:SetFrameLevel(talents:GetFrameLevel() + 5)
  b.icon = b:CreateTexture(nil, "ARTWORK")
  b.icon:SetAllPoints(b)
  b.icon:SetTexCoord(0.07, 0.93, 0.07, 0.93)
  b.frame = Tex(b, "OVERLAY", "iconframe")
  b.frame:SetPoint("TOPLEFT", b, "TOPLEFT", -4, 4)
  b.frame:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", 4, -4)
  b.glow = Tex(b, "BACKGROUND", "glow", 72, 72)
  b.glow:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.glow:SetBlendMode("ADD")
  b.rankBg = b:CreateTexture(nil, "OVERLAY")
  b.rankBg:SetTexture(SOLID)
  b.rankBg:SetVertexColor(0, 0, 0, 0.85)
  b.rankBg:SetWidth(24)
  b.rankBg:SetHeight(13)
  b.rankBg:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", 6, -5)
  b.rank = Text(b, FONT_NUM, 11, "OUTLINE")
  b.rank:SetPoint("CENTER", b.rankBg, "CENTER", 0, 0)
  b.index = i
  b:EnableMouse(true)
  b:RegisterForClicks("LeftButtonUp")
  b:SetScript("OnClick", function()
    Play("hover")
    -- Talents come from level-up cards: clicking just shows the tooltip.
  end)
  b:SetScript("OnEnter", function() BB.TalentTooltip(this) end)
  b:SetScript("OnLeave", function() GameTooltip:Hide() end)
  return b
end

-- Tier `tier` of a tree opens with this many points spent in it.
local function TierPoints(tier) return 3 * (tier - 1) end

function BB.TalentTooltip(b)
  local d = BB.talents.defs[b.index]
  if not d then return end
  local r = BB.talents.ranks[b.index] or 0
  GameTooltip:SetOwner(b, "ANCHOR_RIGHT")
  GameTooltip:SetText(d.name, 1, 1, 1)
  GameTooltip:AddLine("Rank " .. r .. "/" .. d.ranks, 1, 0.82, 0)
  if r > 0 then
    GameTooltip:AddLine(d.texts[r], 0.95, 0.9, 0.75, 1)
  end
  if r < d.ranks then
    -- Talents come whole, as level-up cards: show what the card would give.
    GameTooltip:AddLine(d.texts[d.ranks], 0.95, 0.9, 0.75, 1)
    local need = TierPoints(d.tier)
    local have = BB.talents.spent[d.tree + 1] or 0
    local tree = BB.talents.trees[d.tree + 1]
    if have < need then
      GameTooltip:AddLine("Opens after " .. need .. " points in " .. (tree and tree.name or "this tree") .. " (you have " .. have .. ").", 1, 0.45, 0.3, 1)
    else
      GameTooltip:AddLine("Can come as a card when you level up.", 0.3, 1, 0.3, 1)
    end
  end
  if d.grants then GameTooltip:AddLine("Teaches a spell: it takes one of your six spell keys.", 0.4, 0.8, 1, 1) end
  GameTooltip:Show()
end

function BB.LayoutTalents()
  local tl = BB.talents
  for t = 1, 3 do
    local p = talents.panels[t]
    local tree = tl.trees[t]
    if tree then
      local bg = "Interface\\TalentFrame\\" .. tree.background
      p.tl:SetTexture(bg .. "-TopLeft")
      p.tr:SetTexture(bg .. "-TopRight")
      p.bl:SetTexture(bg .. "-BottomLeft")
      p.br:SetTexture(bg .. "-BottomRight")
      p.icon:SetTexture("Interface\\Icons\\" .. tree.icon)
      p.name:SetText(tree.name)
      p.text:SetText(tree.text)
    end
    p.spent:SetText((tl.spent[t] or 0) .. " points")
  end
  for i = 1, table.getn(tl.defs) do
    local d = tl.defs[i]
    local b = talents.buttons[i]
    if not b then
      b = TalentButton(i)
      talents.buttons[i] = b
    end
    local p = talents.panels[d.tree + 1]
    b:ClearAllPoints()
    local x = 18 + (d.col - 1) * 54
    local y = -74 - (d.tier - 1) * 54
    b:SetPoint("TOPLEFT", p, "TOPLEFT", x, y)
    b.icon:SetTexture("Interface\\Icons\\" .. d.icon)
    local r = tl.ranks[i] or 0
    local open = (tl.spent[d.tree + 1] or 0) >= TierPoints(d.tier)
    b.rank:SetText(r .. "/" .. d.ranks)
    if r >= d.ranks then
      b.frame:SetVertexColor(1, 0.82, 0.3, 1)
      b.rank:SetTextColor(1, 0.82, 0.3)
      b.icon:SetVertexColor(1, 1, 1)
      b.glow:SetVertexColor(1, 0.8, 0.3, 0.35)
    elseif r > 0 or (open and tl.points > 0) then
      b.frame:SetVertexColor(0.3, 1, 0.3, 1)
      b.rank:SetTextColor(0.3, 1, 0.3)
      b.icon:SetVertexColor(1, 1, 1)
      b.glow:SetVertexColor(0.3, 1, 0.3, r > 0 and 0.25 or 0.15)
    elseif open then
      b.frame:SetVertexColor(0.7, 0.7, 0.7, 1)
      b.rank:SetTextColor(0.8, 0.8, 0.8)
      b.icon:SetVertexColor(1, 1, 1)
      b.glow:SetVertexColor(0, 0, 0, 0)
    else
      b.frame:SetVertexColor(0.35, 0.35, 0.35, 1)
      b.rank:SetTextColor(0.5, 0.5, 0.5)
      b.icon:SetVertexColor(0.3, 0.3, 0.3)
      b.glow:SetVertexColor(0, 0, 0, 0)
    end
    b:Show()
  end
  local total = (tl.spent[1] or 0) + (tl.spent[2] or 0) + (tl.spent[3] or 0)
  talents.points:SetText(total .. " talent points taken")
end

function BB.ToggleTalents()
  if not BB.talents.ready then return false end
  if talents:IsShown() then
    talents:Hide()
  else
    BB.LayoutTalents()
    talents:Show()
    Play("open")
  end
  return true
end

-- A button by the spell bar opens them too, and glows while points wait.
local talentButton = MakeButton(UIParent, 92, 20, "Talents", GOLD, function() BB.ToggleTalents() end)
talentButton.label:SetFont(FONT_BODY, 10)
talentButton:SetFrameStrata("HIGH")
talentButton.glow = Tex(talentButton, "BACKGROUND", "glow", 150, 60)
talentButton.glow:SetPoint("CENTER", talentButton, "CENTER", 0, 0)
talentButton.glow:SetBlendMode("ADD")
talentButton.glow:SetVertexColor(0.3, 1, 0.3, 0)
talentButton:Hide()
talentButton:SetScript("OnUpdate", function()
  if BB.talents.points > 0 then
    this.glow:SetAlpha(0.3 + 0.3 * (0.5 + 0.5 * math.sin(GetTime() * 4)))
  else
    this.glow:SetAlpha(0)
  end
end)

function BB.UpdateTalentButton()
  local tl = BB.talents
  talentButton:ClearAllPoints()
  talentButton:SetPoint("BOTTOMLEFT", spellButtons[1], "TOPLEFT", 58, 8)
  -- Talents come as cards now; the trees are a look-up on N, not a button to press.
  if true then
    talentButton:Hide()
    return
  end
  if not tl.ready then
    talentButton:Hide()
    return
  end
  if tl.points > 0 then
    talentButton.label:SetText("Talents |cff40ff40(" .. tl.points .. ")|r")
    talentButton.tip = tl.points .. " talent point" .. (tl.points == 1 and "" or "s") .. " to spend. Click, or press N."
  else
    talentButton.label:SetText("Talents")
    talentButton.tip = "Your talent trees: what your level-up cards have given you. (Key: N)"
  end
  talentButton:Show()
end

-- N opens these talents instead of the game's own window.
local origToggleTalentFrame = ToggleTalentFrame
ToggleTalentFrame = function()
  if BB.ToggleTalents() then return end
  if origToggleTalentFrame then origToggleTalentFrame() end
end

function BB.TalentLine(f)
  local verb = f[1]
  local tl = BB.talents
  if verb == "TT" then
    local i = (tonumber(f[2]) or 0) + 1
    tl.trees[i] = { name = f[3] or "", text = f[4] or "", icon = f[5] or "", background = f[6] or "" }
  elseif verb == "TD" then
    local i = (tonumber(f[2]) or 0) + 1
    tl.defs[i] = {
      tree = tonumber(f[3]) or 0, tier = tonumber(f[4]) or 1, col = tonumber(f[5]) or 1, ranks = tonumber(f[6]) or 1,
      icon = f[7] or "", name = f[8] or "", texts = Split(f[9] or "", "|"), grants = f[10] == "1",
    }
    tl.ready = true
  elseif verb == "TR" then
    local before = tl.points
    tl.points = tonumber(f[2]) or 0
    local spent = Split(f[3] or "0,0,0", ",")
    tl.spent = { tonumber(spent[1]) or 0, tonumber(spent[2]) or 0, tonumber(spent[3]) or 0 }
    tl.ranks = {}
    if f[4] and f[4] ~= "" then
      local r = Split(f[4], ",")
      for i = 1, table.getn(r) do tl.ranks[i] = tonumber(r[i]) or 0 end
    end
    BB.UpdateTalentButton()
    if talents:IsShown() then BB.LayoutTalents() end
  end
end

-- --- the Ancient's shop: defences and upgrades as cards -----------------------------------------

-- Open it at home, by the shopkeeper by the Ancient: the Shop button above the bar, B, or a
-- right-click on her (the server's SA line says when you're there).

local SHOP_SCALE = 0.66
-- The layout, from the centre: title 316, gold 278, category tabs 242, then the site tabs and
-- the first row of cards from SHOP_TOP, the second row, and Close at -342. It spans 336 up and 377
-- down; on a short screen the whole shop shrinks to stay clear of the tracker and the action bar.
local SHOP_TOP = 196
local SHOP_UP, SHOP_DOWN = 336, 377
local shop = CreateFrame("Frame", "BasinBattleShop", UIParent)
-- `here`: at home by the shopkeeper (the server's SA line), the only place the shop opens.
BB.shop = { items = {}, ready = false, sites = {}, site = 0, cat = "base", here = true, vendor = "the shopkeeper by the Ancient" }

local function AwayFromShop()
  UIErrorsFrame:AddMessage("Shopping is done at home: visit " .. BB.shop.vendor .. ".", 1, 0.82, 0.2, 1, 3)
end
shop:SetAllPoints(UIParent)
shop:SetFrameStrata("DIALOG")
shop:EnableMouse(true)
shop:Hide()
table.insert(UISpecialFrames, "BasinBattleShop")
shop:SetScript("OnShow", function()
  local half = (UIParent:GetHeight() or 768) / 2
  this:SetScale(math.min(1, (half - 76) / SHOP_UP, (half - 64) / SHOP_DOWN))
  BB.noteHolder:Hide()
end)
shop:SetScript("OnHide", function() BB.noteHolder:Show() end)

shop.dim = shop:CreateTexture(nil, "BACKGROUND")
shop.dim:SetAllPoints(shop)
shop.dim:SetTexture(0, 0, 0)
shop.dim:SetAlpha(0.66)
shop.rays = Tex(shop, "BORDER", "rays", 980, 980)
shop.rays:SetPoint("CENTER", shop, "CENTER", 0, 40)
shop.rays:SetBlendMode("ADD")
shop.rays:SetVertexColor(0.45, 1, 0.5, 0.18)
shop.halo = Tex(shop, "BORDER", "glow", 720, 300)
shop.halo:SetPoint("CENTER", shop, "CENTER", 0, 300)
shop.halo:SetBlendMode("ADD")
shop.halo:SetVertexColor(0.4, 1, 0.45, 0.3)

shop.pill = CreateFrame("Frame", nil, shop)
shop.pill:SetWidth(170)
shop.pill:SetHeight(26)
shop.pill:SetPoint("CENTER", shop, "CENTER", 0, 278)
shop.pill.bg = Tex(shop.pill, "BACKGROUND", "pill")
shop.pill.bg:SetAllPoints(shop.pill)
shop.pill.bg:SetVertexColor(0.95, 0.8, 0.42, 1)
shop.pill.text = Text(shop.pill, FONT_BODY, 12)
shop.pill.text:SetShadowOffset(0, 0)
shop.pill.text:SetPoint("CENTER", shop.pill, "CENTER", 0, 0)
shop.pill.text:SetTextColor(0.16, 0.1, 0.03)

shop.title = Text(shop, FONT_TITLE, 40, "OUTLINE")
shop.title:SetPoint("CENTER", shop, "CENTER", 0, 316)
shop.title:SetTextColor(0.75, 1, 0.6)
shop.title:SetShadowOffset(2, -2)
shop.title:SetText("The Ancient's Gifts")
shop.subtitle = Text(shop, FONT_BODY, 13)
shop.subtitle:SetPoint("CENTER", shop, "CENTER", 0, 268)
shop.subtitle:SetTextColor(0.78, 0.74, 0.66)
shop.subtitle:SetText("Bruni Ironhammer's wares, here at home: spend between waves, then back to the fight.")
shop.subtitle:Hide()

-- Three ways to spend: the base (defences, the Ancient), your soldiers, yourself.
local SHOP_CATS = {
  { key = "base", label = "Base", tip = "Defences and the Ancient: build up the base the whole party holds." },
  { key = "soldiers", label = "Soldiers", tip = "Make the Basin Defenders you hire, and the treants, tougher and deadlier." },
  { key = "hero", label = "Hero", tip = "Rings, necklaces, trinkets and cloaks that change how you play (four on offer, new ones every wave), and your weapon's enchantment." },
}
local CAT_OF = { ["Defence"] = "base", ["The Ancient"] = "base", ["Your soldiers"] = "soldiers", ["Items"] = "hero", ["Your weapon"] = "hero", ["Reforge"] = "hero" }
local ROW_OF = { ["Defence"] = 1, ["The Ancient"] = 2, ["Your soldiers"] = 1, ["Items"] = 1, ["Your weapon"] = 2, ["Reforge"] = 2 }
shop.cats = {}
for i = 1, table.getn(SHOP_CATS) do
  local c = SHOP_CATS[i]
  local b = MakeButton(shop, 120, 26, c.label, GOLD, function()
    BB.shop.cat = this.cat
    BB.LayoutShop()
  end)
  b.cat = c.key
  b.tip = c.tip
  b:SetPoint("CENTER", shop, "CENTER", (i - 2) * 132, 242)
  shop.cats[i] = b
end

shop.heads = {}
local function ShopHeading(i, label)
  local t = Text(shop, FONT_BODY, 11)
  t:SetText(Spaced(label))
  t:SetTextColor(0.7, 0.68, 0.62)
  shop.heads[i] = t
  return t
end
ShopHeading(1, "DEFENCES AT")
ShopHeading(2, "THE ANCIENT")
ShopHeading(3, "YOUR WEAPON")

shop.close = MakeButton(shop, 150, 32, "Close", { r = 0.75, g = 0.75, b = 0.75 }, function() shop:Hide() end)
shop.close:SetPoint("CENTER", shop, "CENTER", 0, -342)
shop.hint = Text(shop, FONT_BODY, 11)
shop.hint:SetPoint("TOP", shop.close, "BOTTOM", 0, -8)
shop.hint:SetTextColor(0.55, 0.53, 0.5)
shop.hint:SetText("Click a card to buy  ·  Esc to close")

-- Where the defence cards build: a tab for the Ancient and one for each bridge.
shop.tabs = {}
local function SiteTitle(name)
  local t = string.gsub(name, "^the ", "")
  t = string.gsub(t, "(%a)([%w%-]*)", function(a, b) return string.upper(a) .. b end)
  t = string.gsub(t, " Of ", " of ")
  t = string.gsub(t, " The ", " the ")
  return t
end

function BB.LayoutSiteTabs()
  local sites = BB.shop.sites
  -- Places to build (the Base tab) and to send soldiers (the Soldiers tab).
  local cat = BB.shop.cat or "base"
  if cat ~= "base" and cat ~= "soldiers" then
    for i = 1, table.getn(shop.tabs) do shop.tabs[i]:Hide() end
    return
  end
  for i = 1, table.getn(sites) do
    local tab = shop.tabs[i]
    if not tab then
      tab = MakeButton(shop, 150, 22, "", GOLD, function()
        BB.shop.site = this.site
        BB.LayoutShop()
      end)
      tab.label:SetFont(FONT_BODY, 11)
      shop.tabs[i] = tab
    end
    tab.site = i - 1
    local s = sites[i]
    local n = s.protectors + s.springs + s.defenders
    tab.label:SetText(SiteTitle(s.name) .. "  |cffbbbbbb(" .. n .. ")|r")
    tab:SetWidth(math.max(120, tab.label:GetStringWidth() + 26))
    tab:ClearAllPoints()
    if i == 1 then
      tab:SetPoint("LEFT", shop.heads[1], "RIGHT", 12, 0)
    else
      tab:SetPoint("LEFT", shop.tabs[i - 1], "RIGHT", 6, 0)
    end
    local on = (i - 1) == BB.shop.site
    if on then
      tab.color = { r = 0.55, g = 1, b = 0.5 }
      tab.fill:SetVertexColor(0.12, 0.25, 0.1, 0.95)
    else
      tab.color = GOLD
      tab.fill:SetVertexColor(0.09, 0.08, 0.07, 0.92)
    end
    tab.rim:SetVertexColor(tab.color.r, tab.color.g, tab.color.b, on and 1 or 0.6)
    tab.label:SetTextColor(tab.color.r, tab.color.g, tab.color.b)
    if cat == "soldiers" then
      tab.tip = "March Orders send soldiers to " .. s.name .. " (" .. s.defenders .. " stand there now)."
    else
      tab.tip = "Build defences at " .. s.name .. ": " .. s.protectors .. " treants, " .. s.springs .. " springs and " .. s.defenders .. " soldiers stand there."
    end
    tab:Show()
  end
  for i = table.getn(sites) + 1, table.getn(shop.tabs) do
    shop.tabs[i]:Hide()
  end
  -- One place only: no tabs needed.
  if table.getn(sites) <= 1 then
    for i = 1, table.getn(shop.tabs) do shop.tabs[i]:Hide() end
    if cat == "base" then shop.heads[1]:SetText(Spaced("DEFENCES")) end
  elseif cat == "soldiers" then
    shop.heads[1]:SetText(Spaced("SEND TO"))
  else
    shop.heads[1]:SetText(Spaced("DEFENCES AT"))
  end
end

local function SiteKind(key)
  return key == "protector" or key == "spring" or key == "defender" or key == "rifleman" or key == "cleric" or key == "march"
end

local function MakeShopCard(i)
  local holder = CreateFrame("Frame", nil, shop)
  holder:SetWidth(CARD_W)
  holder:SetHeight(CARD_H)
  holder:SetScale(SHOP_SCALE)
  holder.index = i + 100
  local function Layer(parent, layer, file, anchor)
    local t = Tex(parent, layer, file, 256 * K, 384 * K)
    t:SetPoint("TOPLEFT", anchor, "TOPLEFT", -16 * K, 16 * K)
    t:SetTexCoord(0, 1, 0, 0.75)
    return t
  end
  holder.glow = Layer(holder, "BACKGROUND", "cardglow", holder)
  holder.glow:SetBlendMode("ADD")
  local c = CreateFrame("Button", nil, holder)
  c:SetWidth(CARD_W)
  c:SetHeight(CARD_H)
  c:SetPoint("CENTER", holder, "CENTER", 0, 0)
  c:SetFrameLevel(holder:GetFrameLevel() + 1)
  c.holder = holder
  holder.card = c
  c.body = Layer(c, "BACKGROUND", "card", c)
  c.wash = Layer(c, "BORDER", "cardwash", c)
  c.border = Layer(c, "ARTWORK", "cardborder", c)
  c.tag = Tex(c, "ARTWORK", "pill", 84, 22)
  c.tag:SetPoint("TOPRIGHT", c, "TOPRIGHT", -12, -14)
  c.tagText = Text(c, FONT_BODY, 14)
  c.tagText:SetShadowOffset(0, 0)
  c.tagText:SetPoint("CENTER", c.tag, "CENTER", 0, 0)
  c.tagText:SetTextColor(0.06, 0.06, 0.06)
  c.iconGlow = Tex(c, "BORDER", "glow", 170, 170)
  c.iconGlow:SetPoint("CENTER", c, "TOP", 0, -76)
  c.iconGlow:SetBlendMode("ADD")
  c.icon = c:CreateTexture(nil, "ARTWORK")
  c.icon:SetWidth(62)
  c.icon:SetHeight(62)
  c.icon:SetPoint("CENTER", c, "TOP", 0, -76)
  c.icon:SetTexCoord(0.07, 0.93, 0.07, 0.93)
  c.iconFrame = Tex(c, "OVERLAY", "iconframe", 76, 76)
  c.iconFrame:SetPoint("CENTER", c.icon, "CENTER", 0, 0)
  c.rarity = Text(c, FONT_BODY, 13)
  c.rarity:SetPoint("TOP", c, "TOP", 0, -119)
  c.name = Text(c, FONT_TITLE, 26)
  c.name:SetPoint("TOP", c, "TOP", 0, -135)
  c.name:SetWidth(CARD_W - 16)
  c.sub = Text(c, FONT_BODY, 14)
  c.sub:SetPoint("TOP", c, "TOP", 0, -164)
  c.sub:SetTextColor(0.62, 0.6, 0.56)
  c.rule = Tex(c, "ARTWORK", "line", 180, 6)
  c.rule:SetPoint("TOP", c, "TOP", 0, -184)
  c.desc = Text(c, FONT_BODY, 14.5)
  c.desc:SetShadowOffset(1, -1)
  c.desc:SetPoint("TOP", c, "TOP", 0, -194)
  c.desc:SetWidth(CARD_W - 26)
  c.desc:SetJustifyH("CENTER")
  c.desc:SetSpacing(2)
  c.desc:SetTextColor(0.85, 0.83, 0.78)
  c.footRule = Tex(c, "ARTWORK", "line", 180, 6)
  c.footRule:SetPoint("BOTTOM", c, "BOTTOM", 0, 44)
  c.footRule:SetVertexColor(1, 1, 1, 0.12)
  c.foot = Text(c, FONT_BODY, 16)
  c.foot:SetPoint("BOTTOM", c, "BOTTOM", 0, 16)
  c.flashFrame = CreateFrame("Frame", nil, c)
  c.flashFrame:SetAllPoints(c)
  c.flashFrame:SetFrameLevel(c:GetFrameLevel() + 3)
  c.flash = Layer(c.flashFrame, "OVERLAY", "cardflash", c)
  c.flash:SetBlendMode("ADD")
  c.flash:SetAlpha(0)
  c:EnableMouse(true)
  c:RegisterForClicks("LeftButtonUp")
  c:SetScript("OnEnter", function() BB.CardHover(this, true) end)
  c:SetScript("OnLeave", function() BB.CardHover(this, false) end)
  c:SetScript("OnClick", function() BB.Buy(this.holder.item) end)
  return holder
end

shop.cards = {}

local function Gold()
  return math.floor((GetMoney() or 0) / 10000)
end

local function FillShopCard(holder, it)
  local c = holder.card
  holder.item = it
  local col = RARITY[it.rarity] or RARITY[0]
  c.color = col
  local gold = Gold()
  local can = it.cost >= 0 and gold >= it.cost
  holder.glowBase = can and 0.55 or 0.12
  holder.glow:SetVertexColor(col.r, col.g, col.b, holder.glowBase)
  c.wash:SetVertexColor(col.r, col.g, col.b, 0.32 + 0.06 * it.rarity)
  c.border:SetVertexColor(col.r, col.g, col.b, 0.8)
  c.iconGlow:SetVertexColor(col.r, col.g, col.b, can and 0.6 or 0.15)
  c.iconFrame:SetVertexColor(col.r * 0.9 + 0.1, col.g * 0.9 + 0.1, col.b * 0.9 + 0.1, 1)
  c.rule:SetVertexColor(col.r, col.g, col.b, 0.45)
  c.icon:SetTexture("Interface\\Icons\\" .. it.icon)
  c.rarity:SetText(Spaced(string.upper(it.group)))
  c.rarity:SetTextColor(col.r, col.g, col.b)
  FitTitle(c.name, it.name, { 27, 24, 22, 20, 18, 16 }, CARD_W - 22)
  c.name:SetTextColor(col.r * 0.85 + 0.15, col.g * 0.85 + 0.15, col.b * 0.85 + 0.15)
  local sub = it.count
  local site = BB.shop.sites[BB.shop.site + 1]
  if SiteKind(it.key) and site and table.getn(BB.shop.sites) > 1 then
    local here = site.protectors
    if it.key == "spring" then here = site.springs elseif it.key == "defender" then here = site.defenders end
    sub = here .. " here  ·  " .. it.count
  end
  c.sub:SetText(sub)
  c.desc:SetText(it.text)
  if it.cost < 0 then
    c.tag:SetVertexColor(0.5, 0.5, 0.5, 1)
    c.tagText:SetText(string.upper(it.note))
    c.tag:SetWidth(math.max(70, string.len(it.note) * 10 + 20))
    c.foot:SetText(it.note)
    c.foot:SetTextColor(0.6, 0.58, 0.54)
  else
    c.tag:SetWidth(96)
    c.tagText:SetText(it.cost .. " gold")
    if can then
      c.tag:SetVertexColor(1, 0.82, 0.3, 1)
      c.foot:SetText("Click to buy")
      c.foot:SetTextColor(0.4, 1, 0.35)
    else
      c.tag:SetVertexColor(0.85, 0.35, 0.3, 1)
      c.foot:SetText("Need " .. (it.cost - gold) .. " more gold")
      c.foot:SetTextColor(1, 0.45, 0.35)
    end
  end
  if can then
    c.icon:SetVertexColor(1, 1, 1)
    holder:SetAlpha(1)
  else
    c.icon:SetVertexColor(0.45, 0.45, 0.45)
    holder:SetAlpha(it.cost < 0 and 0.55 or 0.8)
  end
end

-- The open tab's cards, in two rows: Base (defences; the Ancient), Soldiers, Hero (items;
-- your weapon).
function BB.LayoutShop()
  local items = BB.shop.items
  local cat = BB.shop.cat or "base"
  local rows = { {}, {} }
  local firstOf = {}
  local headOf = {}
  for i = 1, table.getn(items) do
    local g = items[i].group
    if (CAT_OF[g] or "base") == cat then
      local row = ROW_OF[g] or 1
      table.insert(rows[row], i)
      if not firstOf[row] then
        firstOf[row] = i
        headOf[row] = g
      end
    end
  end
  for k = 1, table.getn(shop.cats) do
    local b = shop.cats[k]
    local on = b.cat == cat
    b.rim:SetVertexColor(GOLD.r, GOLD.g, GOLD.b, on and 1 or 0.45)
    b.label:SetTextColor(on and 1 or 0.6, on and 0.86 or 0.56, on and 0.5 or 0.45)
  end
  local w = CARD_W * SHOP_SCALE
  local h = CARD_H * SHOP_SCALE
  local gap = 14
  for i = 1, table.getn(items) do
    if not shop.cards[i] then shop.cards[i] = MakeShopCard(i) end
  end
  for i = 1, table.getn(shop.cards) do
    shop.cards[i]:Hide()
  end
  for r = 1, 2 do
    local n = table.getn(rows[r])
    local total = n * w + (n - 1) * gap
    local top = SHOP_TOP - (r - 1) * (h + 28)
    for k = 1, n do
      local i = rows[r][k]
      local holder = shop.cards[i]
      local x = -total / 2 + (k - 1) * (w + gap)
      holder:ClearAllPoints()
      -- Offsets on a scaled frame are in its own units.
      holder:SetPoint("TOPLEFT", shop, "CENTER", x / SHOP_SCALE, top / SHOP_SCALE)
      FillShopCard(holder, items[i])
      holder:Show()
    end
  end
  local HEADS = { ["Defence"] = "DEFENCES", ["The Ancient"] = "THE ANCIENT", ["Your soldiers"] = "YOUR SOLDIERS", ["Items"] = "ITEMS FOR YOU", ["Your weapon"] = "YOUR WEAPON", ["Reforge"] = "REFORGE" }
  for g = 1, 3 do
    local head = shop.heads[g]
    local i = firstOf[g]
    head:ClearAllPoints()
    if i then
      head:SetText(Spaced(HEADS[headOf[g]] or ""))
      head:SetPoint("BOTTOMLEFT", shop.cards[i], "TOPLEFT", 4, 6)
      head:Show()
    else
      head:Hide()
    end
  end
  shop.pill.text:SetText(Gold() .. " GOLD")
  BB.LayoutSiteTabs()
end

function BB.ToggleShop()
  if shop:IsShown() then
    shop:Hide()
    return
  end
  if not BB.shop.here then
    AwayFromShop()
    return
  end
  if not BB.shop.ready then
    Say(".bb shop")
    return
  end
  BB.OpenShop()
end

function BB.HideShop()
  shop:Hide()
end

function BB.OpenShop()
  if not BB.shop.ready then return end
  if not BB.shop.here then
    AwayFromShop()
    return
  end
  if picker:IsShown() then return end
  BB.LayoutShop()
  shop:Show()
  Play("open")
end

function BB.Buy(it)
  if not it then return end
  if it.cost < 0 then
    UIErrorsFrame:AddMessage(it.note, 1, 0.2, 0.2, 1, 2)
    return
  end
  if Gold() < it.cost then
    UIErrorsFrame:AddMessage("Not enough gold: " .. it.name .. " costs " .. it.cost .. ".", 1, 0.2, 0.2, 1, 2)
    return
  end
  if SiteKind(it.key) then
    Say(".bb buy " .. it.key .. "@" .. BB.shop.site)
  else
    Say(".bb buy " .. it.key)
  end
end

function BB.ShopLine(f)
  local verb = f[1]
  if verb == "SH" then
    local items = {}
    if f[2] and f[2] ~= "" then
      local list = Split(f[2], ";")
      for i = 1, table.getn(list) do
        local p = Split(list[i], "^")
        table.insert(items, {
          key = p[1], icon = p[2], name = p[3] or "", group = p[4] or "", count = p[5] or "",
          cost = tonumber(p[6]) or -1, rarity = tonumber(p[7]) or 0, note = p[8] or "", text = p[9] or "",
        })
      end
    end
    BB.shop.items = items
    BB.shop.ready = true
    if shop:IsShown() then BB.LayoutShop() end
    BB.UpdateShopButton()
  elseif verb == "SS" then
    local sites = {}
    if f[2] and f[2] ~= "" then
      local list = Split(f[2], ";")
      for i = 1, table.getn(list) do
        local p = Split(list[i], "^")
        table.insert(sites, { name = p[1] or "", protectors = tonumber(p[2]) or 0, springs = tonumber(p[3]) or 0, defenders = tonumber(p[4]) or 0 })
      end
    end
    BB.shop.sites = sites
    if BB.shop.site >= table.getn(sites) then BB.shop.site = 0 end
  elseif verb == "SO" then
    local site = tonumber(f[2] or "")
    if site and site < table.getn(BB.shop.sites) then BB.shop.site = site end
    if shop:IsShown() then BB.LayoutShop() else BB.OpenShop() end
  elseif verb == "SA" then
    -- At home by the shopkeeper, or away: the window only stays open at home.
    BB.shop.here = f[2] == "1"
    if f[3] and f[3] ~= "" then BB.shop.vendor = f[3] end
    if not BB.shop.here and shop:IsShown() then shop:Hide() end
    BB.UpdateShopButton()
  elseif verb == "SB" then
    -- A purchase: a flash on its card and a chime.
    Play("pick")
    for i = 1, table.getn(shop.cards) do
      local h = shop.cards[i]
      if h.item and h.item.key == f[2] and h:IsShown() then
        local c = h.card
        Tween(0.5, 0, OutCubic, function(t) c.flash:SetAlpha(0.9 * (1 - t)) end, nil, "shopflash" .. i)
      end
    end
  end
end

-- The Shop button by the Talents button: glows when you can afford something.
local shopButton = MakeButton(UIParent, 92, 20, "Shop", { r = 0.5, g = 1, b = 0.45 }, function() BB.ToggleShop() end)
shopButton.label:SetFont(FONT_BODY, 10)
shopButton:SetFrameStrata("HIGH")
shopButton.glow = Tex(shopButton, "BACKGROUND", "glow", 150, 60)
shopButton.glow:SetPoint("CENTER", shopButton, "CENTER", 0, 0)
shopButton.glow:SetBlendMode("ADD")
shopButton.glow:SetVertexColor(0.4, 1, 0.4, 0)
shopButton:Hide()

local function Affordable()
  local gold = Gold()
  local n = 0
  for i = 1, table.getn(BB.shop.items) do
    local it = BB.shop.items[i]
    if it.cost >= 0 and gold >= it.cost then n = n + 1 end
  end
  return n
end

local lastGold = -1
shopButton:SetScript("OnUpdate", function()
  local gold = Gold()
  if gold ~= lastGold then
    lastGold = gold
    BB.UpdateShopButton()
    if shop:IsShown() then BB.LayoutShop() end
  end
  if this.can then
    this.glow:SetAlpha(0.25 + 0.25 * (0.5 + 0.5 * math.sin(GetTime() * 3)))
  else
    this.glow:SetAlpha(0)
  end
end)

function BB.UpdateShopButton()
  shopButton:ClearAllPoints()
  shopButton:SetPoint("BOTTOMLEFT", spellButtons[1], "TOPLEFT", -2, 8)
  if not BB.shop.ready then
    shopButton:Hide()
    return
  end
  local n = Affordable()
  shopButton.can = n > 0 and BB.shop.here
  if BB.shop.here then
    shopButton.label:SetText("Shop |cffffd060" .. Gold() .. "g|r")
    shopButton:SetAlpha(1)
  else
    shopButton.label:SetText("|cff999999Shop (home)|r |cffffd060" .. Gold() .. "g|r")
    shopButton:SetAlpha(0.75)
  end
  if not BB.shop.here then
    shopButton.tip = "The shop is " .. BB.shop.vendor .. ": go home between waves to buy defences, soldiers, items and enchantments."
  elseif n > 0 then
    shopButton.tip = "The shop (B), here at home: defences, soldiers, items and enchantments. You can afford " .. n .. " now."
  else
    shopButton.tip = "The shop (B), here at home: defences, soldiers, items and enchantments."
  end
  shopButton:Show()
  -- Fallen away from home: nothing to spend on until you're back.
  if BB.fallenShop then
    if BB.shop.here then BB.fallenShop:Show() else BB.fallenShop:Hide() end
  end
end

-- While fallen: spend while you wait.
fallen.shop = MakeButton(fallen, 240, 34, "Spend gold while you wait", { r = 0.5, g = 1, b = 0.45 }, function() BB.OpenShop() end)
fallen.shop:SetPoint("TOP", fallen.sub, "BOTTOM", 0, -18)
fallen.shop:SetFrameStrata("HIGH")
BB.fallenShop = fallen.shop
fallen.shop.tip = "Buy defences and upgrades for the Ancient now (you fell at home): they help while you're down."

-- --- the objective tracker: what to do now -------------------------------------------------------

local goals = CreateFrame("Frame", "BasinBattleGoals", UIParent)
goals:SetWidth(250)
goals:SetHeight(160)
goals:SetPoint("TOPRIGHT", UIParent, "TOPRIGHT", -16, -210)
goals:SetFrameStrata("LOW")
goals:Hide()
goals.lines = {}
for i = 1, 3 do
  local l = {}
  l.title = Text(goals, FONT_BODY, 13, "OUTLINE")
  l.title:SetJustifyH("LEFT")
  l.title:SetWidth(240)
  l.detail = Text(goals, FONT_BODY, 11)
  l.detail:SetJustifyH("LEFT")
  l.detail:SetWidth(232)
  l.detail:SetTextColor(0.86, 0.84, 0.78)
  l.bullet = Tex(goals, "ARTWORK", "circle", 8, 8)
  goals.lines[i] = l
end

local GOAL_COLOR = {
  main = { r = 1, g = 0.82, b = 0.35 },
  warn = { r = 1, g = 0.3, b = 0.25 },
  tip = { r = 0.55, g = 0.85, b = 1 },
}

function BB.Goals(f)
  local items = {}
  if f[2] and f[2] ~= "" then items = Split(f[2], ";") end
  local y = 0
  local firstTitle = nil
  for i = 1, 3 do
    local l = goals.lines[i]
    local item = items[i]
    if item then
      local p = Split(item, "^")
      local col = GOAL_COLOR[p[1]] or GOAL_COLOR.tip
      l.title:SetText(p[2] or "")
      l.title:SetTextColor(col.r, col.g, col.b)
      l.detail:SetText(p[3] or "")
      l.bullet:SetVertexColor(col.r, col.g, col.b, 1)
      l.bullet:ClearAllPoints()
      l.bullet:SetPoint("TOPLEFT", goals, "TOPLEFT", 0, y - 4)
      l.title:ClearAllPoints()
      l.title:SetPoint("TOPLEFT", goals, "TOPLEFT", 14, y)
      l.detail:ClearAllPoints()
      l.detail:SetPoint("TOPLEFT", l.title, "BOTTOMLEFT", 2, -2)
      l.title:Show()
      l.detail:Show()
      l.bullet:Show()
      y = y - 18 - l.detail:GetHeight() - 8
      if i == 1 then firstTitle = p[2] end
    else
      l.title:Hide()
      l.detail:Hide()
      l.bullet:Hide()
    end
  end
  goals:Show()
  BB.goalsHeight = -y
  if BB.PlaceFronts then BB.PlaceFronts() end
  -- A new goal slides in.
  if firstTitle and firstTitle ~= BB.goalTitle then
    BB.goalTitle = firstTitle
    Tween(0.35, 0, OutCubic, function(t)
      goals:SetAlpha(t)
      goals:ClearAllPoints()
      goals:SetPoint("TOPRIGHT", UIParent, "TOPRIGHT", -16 + 30 * (1 - t), -210)
    end, nil, "goals")
  end
end


-- --- the fronts: where the next wave comes, what's coming now, who holds where ------------------

local FRONT_W = 250
local fronts = CreateFrame("Frame", "BasinBattleFronts", UIParent)
fronts:SetWidth(FRONT_W)
fronts:SetHeight(40)
fronts:SetFrameStrata("LOW")
fronts:Hide()
fronts.bg = fronts:CreateTexture(nil, "BACKGROUND")
fronts.bg:SetTexture("Interface\\Tooltips\\UI-Tooltip-Background")
fronts.bg:SetPoint("TOPLEFT", fronts, "TOPLEFT", -8, 6)
fronts.bg:SetPoint("BOTTOMRIGHT", fronts, "BOTTOMRIGHT", 6, -6)
fronts.bg:SetVertexColor(0, 0, 0, 0.45)
fronts.head = Text(fronts, FONT_BODY, 11, "OUTLINE")
fronts.head:SetPoint("TOPLEFT", fronts, "TOPLEFT", 0, 0)
fronts.head:SetTextColor(1, 0.82, 0.35)
fronts.rows = {}
fronts.help = MakeButton(fronts, 74, 18, "Help!", { r = 1, g = 0.35, b = 0.3 }, function() Say(".bb help") end)
fronts.help.label:SetFont(FONT_BODY, 10)
fronts.help:SetPoint("TOPRIGHT", fronts, "TOPRIGHT", 0, 3)
fronts.help.tip = "Call for help where you stand: everyone sees it."

local function FrontRow(i)
  local r = CreateFrame("Frame", nil, fronts)
  r:SetWidth(FRONT_W)
  r:SetHeight(34)
  r.bar = r:CreateTexture(nil, "BORDER")
  r.bar:SetTexture("Interface\\Tooltips\\UI-Tooltip-Background")
  r.bar:SetPoint("TOPLEFT", r, "TOPLEFT", 0, 0)
  r.bar:SetHeight(30)
  r.name = Text(r, FONT_BODY, 13, "OUTLINE")
  r.name:SetPoint("TOPLEFT", r, "TOPLEFT", 4, -2)
  r.count = Text(r, FONT_NUM, 18, "OUTLINE")
  r.count:SetPoint("TOPRIGHT", r, "TOPRIGHT", -58, 0)
  r.note = Text(r, FONT_BODY, 10)
  r.note:SetPoint("TOPLEFT", r.name, "BOTTOMLEFT", 0, -2)
  r.note:SetWidth(FRONT_W - 70)
  r.note:SetJustifyH("LEFT")
  r.hold = MakeButton(r, 50, 16, "Hold", { r = 0.55, g = 1, b = 0.5 }, function() Say(".bb hold " .. this:GetParent().site) end)
  r.hold.label:SetFont(FONT_BODY, 9)
  r.hold:SetPoint("TOPRIGHT", r, "TOPRIGHT", 0, -2)
  return r
end

local function Title(name)
  local t = string.gsub(name, "^the ", "")
  t = string.gsub(t, "(%a)([%w%-]*)", function(a, b) return string.upper(a) .. b end)
  t = string.gsub(t, " Of ", " of ")
  t = string.gsub(t, " The ", " the ")
  return t
end

-- Below the objective tracker, whatever its height.
function BB.PlaceFronts()
  fronts:ClearAllPoints()
  fronts:SetPoint("TOPRIGHT", goals, "TOPRIGHT", -6, -((BB.goalsHeight or 60) + 18))
end

function BB.Fronts(f)
  local next = tonumber(f[2]) or 0
  local items = {}
  if f[3] and f[3] ~= "" then items = Split(f[3], ";") end
  -- Bridges first, the Ancient last.
  local rows = {}
  for i = 1, table.getn(items) do
    local p = Split(items[i], "^")
    local r = {
      site = tonumber(p[1]) or 0, name = p[2] or "", planned = tonumber(p[3]) or 0, plannedElites = tonumber(p[4]) or 0,
      notes = p[5] or "", boss = p[6] or "", incoming = tonumber(p[7]) or 0, incomingElites = tonumber(p[8]) or 0,
      bossIncoming = p[9] == "1", holders = p[10] or "",
    }
    if r.site > 0 then table.insert(rows, r) end
  end
  for i = 1, table.getn(items) do
    local p = Split(items[i], "^")
    if (tonumber(p[1]) or 0) == 0 then
      table.insert(rows, {
        site = 0, name = p[2] or "", planned = tonumber(p[3]) or 0, plannedElites = tonumber(p[4]) or 0,
        notes = p[5] or "", boss = p[6] or "", incoming = tonumber(p[7]) or 0, incomingElites = tonumber(p[8]) or 0,
        bossIncoming = p[9] == "1", holders = p[10] or "",
      })
    end
  end
  -- With no bridges there's only one front: nothing to choose between.
  if table.getn(rows) <= 1 then
    fronts:Hide()
    return
  end
  local event = f[4] or ""
  if next > 0 then
    fronts.head:SetText(Spaced("WAVE " .. next .. " COMES"))
  else
    fronts.head:SetText(Spaced("FRONTS"))
  end
  if event ~= "" then
    fronts.head:SetText(fronts.head:GetText() .. "  |cffff9040" .. string.upper(event) .. "|r")
  end
  local most = 1
  for i = 1, table.getn(rows) do
    local n = rows[i].incoming
    if next > 0 then n = rows[i].planned end
    if n > most then most = n end
  end
  local me = UnitName("player") or ""
  local y = -18
  for i = 1, table.getn(rows) do
    local d = rows[i]
    local r = fronts.rows[i]
    if not r then
      r = FrontRow(i)
      fronts.rows[i] = r
    end
    r.site = d.site
    r:ClearAllPoints()
    r:SetPoint("TOPLEFT", fronts, "TOPLEFT", 0, y)
    r.name:SetText(Title(d.name))
    local n = d.incoming
    local elites = d.incomingElites
    if next > 0 then
      n = d.planned
      elites = d.plannedElites
    end
    r.count:SetText(n)
    local danger = n / most
    local bossHere = (next > 0 and d.boss ~= "") or d.bossIncoming
    if bossHere then danger = 1 end
    r.count:SetTextColor(1, 1 - 0.7 * danger, 1 - 0.8 * danger)
    r.bar:SetWidth(math.max(2, (FRONT_W - 56) * math.min(1, n / most)))
    if bossHere then
      r.bar:SetVertexColor(0.8, 0.15, 0.1, 0.45)
    else
      r.bar:SetVertexColor(1, 0.6, 0.2, 0.18 + 0.25 * danger)
    end
    local note = ""
    if bossHere then
      note = "|cffff5030" .. (d.boss ~= "" and d.boss or "The boss") .. "!|r "
    end
    if elites > 0 then note = note .. "|cffffc040" .. elites .. " elite|r  " end
    if d.notes ~= "" then note = note .. d.notes .. "  " end
    if d.holders ~= "" then
      note = note .. "|cff60ff60" .. d.holders .. " holds|r"
    elseif n > 0 then
      note = note .. "|cff999999Nobody holds it|r"
    end
    r.note:SetText(note)
    if string.find(", " .. d.holders .. ",", ", " .. me .. ",", 1, true) then
      r.hold.label:SetText("Leave")
      r.hold.tip = "Stop holding " .. d.name .. "."
    else
      r.hold.label:SetText("Hold")
      r.hold.tip = "Tell everyone you'll hold " .. d.name .. "."
    end
    r:Show()
    y = y - 22 - math.max(12, r.note:GetHeight())
  end
  for i = table.getn(rows) + 1, table.getn(fronts.rows) do
    fronts.rows[i]:Hide()
  end
  fronts:SetHeight(-y)
  BB.PlaceFronts()
  fronts:Show()
end

-- The world paused while someone chooses a card: say who, under the tracker.
local paused = CreateFrame("Frame", "BasinBattlePaused", UIParent)
paused:SetWidth(420)
paused:SetHeight(44)
paused:SetPoint("CENTER", UIParent, "CENTER", 0, 120)
paused:SetFrameStrata("HIGH")
paused:Hide()
paused.bg = paused:CreateTexture(nil, "BACKGROUND")
paused.bg:SetTexture("Interface\\Tooltips\\UI-Tooltip-Background")
paused.bg:SetAllPoints(paused)
paused.bg:SetVertexColor(0, 0, 0, 0.6)
paused.title = Text(paused, FONT_TITLE, 22, "OUTLINE")
paused.title:SetPoint("TOP", paused, "TOP", 0, -4)
paused.title:SetTextColor(0.6, 0.85, 1)
paused.title:SetText("Paused")
paused.who = Text(paused, FONT_BODY, 12)
paused.who:SetPoint("BOTTOM", paused, "BOTTOM", 0, 5)
paused.who:SetTextColor(0.9, 0.88, 0.8)
-- Waiting on your own pick with no picker on screen (closed, or lost behind something): bring
-- it back.
paused.pick = MakeButton(paused, 200, 30, "Choose your card", GOLD, function()
  Say(".bb open")
end)
paused.pick:SetPoint("TOP", paused, "BOTTOM", 0, -8)
paused.pick.tip = "The game is waiting for your pick. Click to show your cards again (or type .cards)."
paused.pick:Hide()
paused:SetScript("OnUpdate", function()
  -- Your own pick shows the picker instead.
  if picker:IsShown() then
    this:SetAlpha(0)
    this.pick:Hide()
    this.lostAt = nil
  else
    this:SetAlpha(0.75 + 0.25 * math.sin(GetTime() * 3))
    if this.mine then
      this.pick:Show()
      -- The game waits on us and our cards aren't up: ask for them again, every few seconds.
      local now = GetTime()
      if not this.lostAt then this.lostAt = now end
      if now - this.lostAt > 2 and now - (this.askedAt or 0) > 5 then
        this.askedAt = now
        Say(".bb open")
      end
    else
      this.pick:Hide()
      this.lostAt = nil
    end
  end
end)

function BB.Paused(f)
  local who = f[2] or ""
  if who == "" then
    -- Time runs again: the cooldowns pick up where they stopped.
    if BB.pausedAt then
      local held = GetTime() - BB.pausedAt
      local sb = BB.spells
      for i = 1, 12 do
        if sb.slots[i] then sb.slots[i].readyAt = sb.slots[i].readyAt + held end
      end
      sb.gcdEnd = sb.gcdEnd + held
      BB.pausedAt = nil
    end
    paused:Hide()
    return
  end
  if not BB.pausedAt then BB.pausedAt = GetTime() end
  local many = string.find(who, ",", 1, true)
  paused.who:SetText(who .. (many and " are" or " is") .. " choosing a card")
  -- Is one of them me?
  local me = UnitName("player") or ""
  paused.mine = false
  local names = Split(who, ", ")
  for i = 1, table.getn(names) do
    if names[i] == me then paused.mine = true end
  end
  paused:Show()
end

-- A ping from anyone: a call for help is loud, a hold is a note.
function BB.Ping(f)
  local help = f[2] == "1"
  local text = f[3] or ""
  if help then
    Play("danger")
    if UIErrorsFrame then UIErrorsFrame:AddMessage(text, 1, 0.3, 0.2, 1, 4) end
    BB.Toast("Interface\\Icons\\Ability_Warrior_BattleShout", "Help!", text, { r = 1, g = 0.3, b = 0.25 })
  else
    BB.Announce(text)
  end
end


-- --- the world map (M): where the enemies are, and what holds them back -------------------------

local MAP_STYLE = {
  e = { size = 7, r = 1, g = 0.2, b = 0.15 },
  E = { size = 11, r = 1, g = 0.55, b = 0.1 },
  B = { size = 18, r = 0.8, g = 0.2, b = 1 },
  t = { size = 10, r = 0.3, g = 1, b = 0.3 },
  s = { size = 10, r = 0.3, g = 0.85, b = 1 },
  d = { size = 7, r = 0.5, g = 0.9, b = 0.5 },
  p = { size = 14, r = 1, g = 0.85, b = 0.3 },
  h = { size = 20, r = 0.6, g = 1, b = 0.4 },
  g = { size = 14, r = 0.9, g = 0.2, b = 0.2 },
  o = { size = 14, r = 1, g = 0.9, b = 0.2 },
  c = { size = 14, r = 0.3, g = 0.5, b = 1 },
  P = { size = 9, r = 1, g = 1, b = 1 },
  ["!"] = { size = 26, r = 1, g = 0.1, b = 0.1, pulse = true },
}

local mapCanvas = WorldMapButton or WorldMapDetailFrame
local mapLayer, mapDots, mapLegend
if mapCanvas then
  mapLayer = CreateFrame("Frame", "BasinBattleMapLayer", mapCanvas)
  mapLayer:SetAllPoints(mapCanvas)
  mapLayer:SetFrameLevel(mapCanvas:GetFrameLevel() + 5)
  mapDots = {}
  mapLegend = Text(mapLayer, FONT_BODY, 12, "OUTLINE")
  mapLegend:SetPoint("BOTTOM", mapLayer, "BOTTOM", 0, 8)
  mapLegend:SetText("|cffff3326o|r enemies   |cffff8c1ao|r elites   |cffcc33ffo|r boss   |cff4dff4do|r treants   |cff4dd9ffo|r springs   |cffffd94do|r bridges   |cff99ff66o|r the Ancient   |cffffffffo|r heroes   |cffff1a1aO|r help!")
end

function BB.MapLine(f)
  if not mapLayer then return end
  local items = {}
  if f[2] and f[2] ~= "" then items = Split(f[2], ";") end
  local w, h = mapLayer:GetWidth(), mapLayer:GetHeight()
  local n = 0
  for i = 1, table.getn(items) do
    local p = Split(items[i], "^")
    local st = MAP_STYLE[p[1]]
    local x, y = tonumber(p[2]), tonumber(p[3])
    if st and x and y then
      n = n + 1
      local d = mapDots[n]
      if not d then
        d = Tex(mapLayer, "OVERLAY", "circle", 8, 8)
        mapDots[n] = d
      end
      -- A call for help pulses.
      local size = st.size
      if st.pulse then size = size * (0.8 + 0.4 * math.abs(math.sin(GetTime() * 5))) end
      d:SetWidth(size)
      d:SetHeight(size)
      d:SetVertexColor(st.r, st.g, st.b, 1)
      d:ClearAllPoints()
      d:SetPoint("CENTER", mapLayer, "TOPLEFT", x / 1000 * w, -y / 1000 * h)
      d:Show()
    end
  end
  for i = n + 1, table.getn(mapDots) do
    mapDots[i]:Hide()
  end
end

if WorldMapFrame then
  local mapWatch = CreateFrame("Frame", nil, WorldMapFrame)
  mapWatch:SetScript("OnShow", function() if BB.connected then Say(".bb map 1") end end)
  mapWatch:SetScript("OnHide", function() if BB.connected then Say(".bb map 0") end end)
end

-- A cast that didn't go off says why.
-- The same complaint at most once every 1.5 seconds: mashing a key shouldn't fill the screen.
BB.errorAt = {}
function BB.CastError(text)
  local now = GetTime()
  if BB.errorAt[text] and now - BB.errorAt[text] < 1.5 then return end
  BB.errorAt[text] = now
  if UIErrorsFrame then
    UIErrorsFrame:AddMessage(text, 1, 0.2, 0.2, 1, 2)
  else
    BB.Announce("|cffff4040" .. text .. "|r")
  end
end

-- Chat lines that only repeat what the screen already shows.
local NOISE = {
  "^You have gained %d+ hit points",
  "^You have gained %d+ mana",
  "^You have gained %d+ talent point",
  "^Congratulations, you have reached level",
}
if ChatFrame_OnEvent then
  local origChatFrameOnEvent = ChatFrame_OnEvent
  ChatFrame_OnEvent = function(event)
    if event == "PLAYER_LEVEL_UP" then return end
    if event == "CHAT_MSG_SYSTEM" and arg1 then
      for i = 1, table.getn(NOISE) do
        if string.find(arg1, NOISE[i]) then return end
      end
    end
    return origChatFrameOnEvent(event)
  end
end

-- --- the server's lines --------------------------------------------------------------------------

local function CardFrom(f)
  return {
    icon = f[4],
    rarity = tonumber(f[5]) or 0,
    name = f[6],
    rank = tonumber(f[7]) or 1,
    maxRank = tonumber(f[8]) or 5,
    kind = f[9],
    school = f[10],
    cooldown = f[11],
    text = f[12],
    upgrade = f[13] or "",
    tags = f[14] or "",
    slot = f[15] or "",
  }
end

function BB.Line(msg)
  local f = Split(msg, "~")
  local verb = f[1]
  if verb == "O" then
    BB.incoming = {
      id = f[2],
      kind = f[3],
      header = f[4],
      title = f[5],
      subtitle = f[6],
      n = tonumber(f[7]) or 0,
      reroll = tonumber(f[8]) or -1,
      rerollLabel = f[9],
      skip = tonumber(f[10]) or -1,
      skipLabel = f[11],
      banishes = tonumber(f[12]) or 0,
      lock = f[13] == "1",
      more = tonumber(f[14]) or 0,
      gold = tonumber(f[15]) or 0,
      cards = {},
    }
  elseif verb == "C" then
    if BB.incoming and BB.incoming.id == f[2] then
      local i = (tonumber(f[3]) or 0) + 1
      BB.incoming.cards[i] = CardFrom(f)
    end
  elseif verb == "OS" then
    if BB.incoming and BB.incoming.id == f[2] then
      BB.ShowOffer(BB.incoming)
      BB.incoming = nil
    end
  elseif verb == "OX" then
    BB.incoming = nil
    if picker:IsShown() then
      BB.ClosePicker(true)
    end
  elseif verb == "P" then
    BB.pending = tonumber(f[2]) or 0
    BB.UpdatePending()
  elseif verb == "B" then
    BB.build = {}
    if f[2] and f[2] ~= "" then
      local items = Split(f[2], ";")
      for i = 1, table.getn(items) do
        local p = Split(items[i], "^")
        table.insert(BB.build, {
          icon = p[1], rank = tonumber(p[2]) or 1, rarity = tonumber(p[3]) or 0, name = p[4],
          aura = p[5] or p[1], maxRank = tonumber(p[6]) or 5, kind = p[7] or "", school = p[8] or "",
          cooldown = p[9] or "", text = p[10] or "", next = p[11] or "",
        })
      end
    end
    if picker:IsShown() then LayoutBar() end
  elseif verb == "K" then
    local rank = tonumber(f[3]) or 1
    local col = RARITY[tonumber(f[5]) or 0] or RARITY[0]
    local sub = "New spell learned"
    if rank > 1 then sub = "Rank " .. rank end
    BB.Toast("Interface\\Icons\\" .. f[4], f[2], sub, col)
  elseif verb == "H" then
    BB.Hud(f)
  elseif verb == "S" then
    BB.Splash(f[2], f[3], f[4])
  elseif verb == "A" then
    BB.Announce(f[2])
    -- A boss turning the fight is heard as well as read.
    if f[2] and (string.find(f[2], "calls its dead", 1, true) or string.find(f[2], "is enraged", 1, true)) then
      Play("boss")
    end
    if f[2] and string.find(f[2], "is under attack", 1, true) then
      Play("danger")
    end
  elseif verb == "X" then
    BB.EnemyCast(f[2], f[3], f[4], f[5] == "1", f[6] or "", f[7] ~= "0")
  elseif verb == "Q" then
    BB.SpellLine(f)
  elseif verb == "TT" or verb == "TD" or verb == "TR" then
    BB.TalentLine(f)
  elseif verb == "MP" then
    BB.MapLine(f)
  elseif verb == "PZ" then
    BB.Paused(f)
  elseif verb == "MC" then
    BB.MyCast(f[2], f[3])
  elseif verb == "DM" then
    BB.PartyMeter(f)
  elseif verb == "DP" then
    BB.OwnMeter(f)
  elseif verb == "FR" then
    BB.Fronts(f)
  elseif verb == "BH" then
    BB.BossBar(f)
  elseif verb == "RS" then
    BB.Scoreboard(f)
  elseif verb == "DG" then
    BB.Dodged(f[2])
  elseif verb == "PG" then
    BB.Ping(f)
  elseif verb == "SH" or verb == "SO" or verb == "SB" or verb == "SS" or verb == "SA" then
    BB.ShopLine(f)
  elseif verb == "T" then
    BB.Goals(f)
  elseif verb == "E" then
    BB.CastError(f[2] or "")
  elseif verb == "V" then
    -- A Basin Battle server answered: only now are the keys set (on another realm, such as
    -- Elwynn Survival, this add-on stays asleep and leaves the keys alone).
    if not BB.connected then
      BB.connected = true
      BB.ApplyKeys()
    end
  end
end

-- --- the damage meter: the party this wave, and you since you reset it -------------------------

local METER_ROWS = 6
local meter = CreateFrame("Frame", "BasinBattleMeter", UIParent)
meter:SetWidth(FRONT_W)
meter:SetHeight(40)
meter:SetFrameStrata("LOW")
meter:SetPoint("TOPRIGHT", fronts, "BOTTOMRIGHT", 0, -16)
meter:Hide()
meter.bg = meter:CreateTexture(nil, "BACKGROUND")
meter.bg:SetTexture("Interface\\Tooltips\\UI-Tooltip-Background")
meter.bg:SetPoint("TOPLEFT", meter, "TOPLEFT", -8, 6)
meter.bg:SetPoint("BOTTOMRIGHT", meter, "BOTTOMRIGHT", 6, -6)
meter.bg:SetVertexColor(0, 0, 0, 0.55)
meter.title = Text(meter, FONT_BODY, 11, "OUTLINE")
meter.title:SetPoint("TOPLEFT", meter, "TOPLEFT", 0, 0)
meter.title:SetTextColor(GOLD.r, GOLD.g, GOLD.b)
meter.title:SetText("D A M A G E")
meter.sub = Text(meter, FONT_BODY, 10)
meter.sub:SetPoint("TOPLEFT", meter.title, "BOTTOMLEFT", 0, -3)
meter.sub:SetTextColor(0.75, 0.72, 0.65)
BB.meter = { tab = "party", party = {}, own = {}, wave = 0, secs = 0, total = 0, folded = false }

local function MeterTab(label, key, x)
  local b = MakeButton(meter, 52, 18, label, { r = 0.8, g = 0.75, b = 0.6 }, function()
    -- The open tab, clicked again, folds the meter to its title.
    if BB.meter.tab == key then
      BB.meter.folded = not BB.meter.folded
    else
      BB.meter.tab = key
      BB.meter.folded = false
    end
    BB.DrawMeter()
  end)
  b.label:SetFont(FONT_BODY, 10)
  b:SetPoint("TOPRIGHT", meter, "TOPRIGHT", x, 2)
  return b
end
meter.youTab = MeterTab("You", "own", 0)
meter.partyTab = MeterTab("Party", "party", -56)
meter.partyTab.tip = "The party's damage this wave (it starts over with each wave). Click again to fold the meter."
meter.youTab.tip = "Your damage by spell, since you last reset it. Click again to fold the meter."
meter.reset = MakeButton(meter, 52, 16, "Reset", { r = 1, g = 0.5, b = 0.35 }, function()
  Say(".bb meter reset")
end)
meter.reset.label:SetFont(FONT_BODY, 10)
meter.reset:SetPoint("TOPRIGHT", meter.youTab, "BOTTOMRIGHT", 0, -3)
meter.reset.tip = "Start your own meter over."
meter.rows = {}
for i = 1, METER_ROWS do
  local r = CreateFrame("StatusBar", nil, meter)
  r:SetHeight(15)
  r:SetPoint("TOPLEFT", meter, "TOPLEFT", 0, -40 - (i - 1) * 17)
  r:SetPoint("TOPRIGHT", meter, "TOPRIGHT", 0, -40 - (i - 1) * 17)
  r:SetStatusBarTexture(ART .. "bar")
  r:SetMinMaxValues(0, 1)
  r.back = r:CreateTexture(nil, "BACKGROUND")
  r.back:SetTexture("Interface\\Tooltips\\UI-Tooltip-Background")
  r.back:SetAllPoints(r)
  r.back:SetVertexColor(0, 0, 0, 0.35)
  r.name = Text(r, FONT_BODY, 11, "OUTLINE")
  r.name:SetPoint("LEFT", r, "LEFT", 4, 0)
  r.value = Text(r, FONT_NUM, 11, "OUTLINE")
  r.value:SetPoint("RIGHT", r, "RIGHT", -4, 0)
  r:Hide()
  meter.rows[i] = r
end

function BB.DrawMeter()
  local m = BB.meter
  local list, sub = {}, ""
  if m.tab == "party" then
    list = m.party
    sub = "Wave " .. m.wave
    meter.reset:Hide()
  else
    list = m.own
    local secs = math.max(1, m.secs)
    sub = Short(m.total) .. " · " .. Short(math.floor(m.total / secs)) .. "/sec · " .. math.floor(secs / 60) .. "m " .. math.mod(math.floor(secs), 60) .. "s"
    meter.reset:Show()
  end
  meter.sub:SetText(sub)
  meter.partyTab.label:SetTextColor(m.tab == "party" and 1 or 0.55, m.tab == "party" and 0.85 or 0.52, m.tab == "party" and 0.45 or 0.45)
  meter.youTab.label:SetTextColor(m.tab == "own" and 1 or 0.55, m.tab == "own" and 0.85 or 0.52, m.tab == "own" and 0.45 or 0.45)
  local top, sum = 0, 0
  for i = 1, table.getn(list) do
    if list[i].n > top then top = list[i].n end
    sum = sum + list[i].n
  end
  local shown = 0
  for i = 1, METER_ROWS do
    local r = meter.rows[i]
    local e = list[i]
    if e and not m.folded then
      shown = i
      local col = CLASS_COL[e.class or ""] or { r = 0.95, g = 0.7, b = 0.3 }
      r:SetStatusBarColor(col.r, col.g, col.b, 0.75)
      r:SetValue(top > 0 and e.n / top or 0)
      r.name:SetText(e.name)
      local pct = sum > 0 and math.floor(e.n * 100 / sum + 0.5) or 0
      r.value:SetText(Short(e.n) .. "  " .. pct .. "%")
      r:Show()
    else
      r:Hide()
    end
  end
  meter:SetHeight(36 + shown * 17)
  meter:Show()
end

-- `DM~wave~name^class^damage;...`
function BB.PartyMeter(f)
  local m = BB.meter
  m.wave = tonumber(f[2]) or 0
  m.party = {}
  local rows = Split(f[3] or "", ";")
  for i = 1, table.getn(rows) do
    local p = Split(rows[i], "^")
    if p[1] and p[1] ~= "" then
      table.insert(m.party, { name = p[1], class = p[2], n = tonumber(p[3]) or 0 })
    end
  end
  BB.DrawMeter()
end

-- `DP~secs~total~source^damage;...`
function BB.OwnMeter(f)
  local m = BB.meter
  m.secs = tonumber(f[2]) or 0
  m.total = tonumber(f[3]) or 0
  m.own = {}
  local rows = Split(f[4] or "", ";")
  for i = 1, table.getn(rows) do
    local p = Split(rows[i], "^")
    if p[1] and p[1] ~= "" then
      table.insert(m.own, { name = p[1], n = tonumber(p[2]) or 0 })
    end
  end
  BB.DrawMeter()
end

-- --- events --------------------------------------------------------------------------------------

local events = CreateFrame("Frame")
events:RegisterEvent("CHAT_MSG_ADDON")
events:RegisterEvent("PLAYER_ENTERING_WORLD")
events:RegisterEvent("PLAYER_DEAD")
events:RegisterEvent("PLAYER_ALIVE")
events:RegisterEvent("PLAYER_UNGHOST")
events:RegisterEvent("PLAYER_LEVEL_UP")
events:RegisterEvent("PLAYER_MONEY")
events:RegisterEvent("UPDATE_BINDINGS")
events:SetScript("OnEvent", function()
  if event == "CHAT_MSG_ADDON" then
    if arg1 == "BB" and arg2 then
      BB.Line(arg2)
    end
  elseif event == "PLAYER_ENTERING_WORLD" then
    BB.connected = false
    -- Health bars over your side (treants, springs, defenders, the party): the game turns them
    -- off on every entry. V shows the enemies' too.
    if ShowFriendNameplates then pcall(ShowFriendNameplates) end
    Say(".bb hello " .. VERSION)
  elseif event == "PLAYER_MONEY" then
    BB.UpdateGold()
  elseif event == "UPDATE_BINDINGS" then
    if BB.connected then BB.UpdateSpellBar() end
  elseif event == "PLAYER_DEAD" then
    if not BB.connected then return end
    if StaticPopup_Hide then StaticPopup_Hide("DEATH") end
    fallen:Show()
  elseif event == "PLAYER_ALIVE" or event == "PLAYER_UNGHOST" then
    fallen:Hide()
  elseif event == "PLAYER_LEVEL_UP" then
    -- The picker usually follows; when it doesn't, a toast marks the level.
    local level = arg1
    BB.levelAt = GetTime()
    local col = GOLD
    local t = CreateFrame("Frame")
    local started = GetTime()
    t:SetScript("OnUpdate", function()
      if GetTime() - started > 0.6 then
        this:SetScript("OnUpdate", nil)
        if not picker:IsShown() then
          BB.Toast("Interface\\Icons\\Spell_Holy_HolyBolt", "Level " .. level, "Stronger, and a new card", col)
        end
      end
    end)
  end
end)
