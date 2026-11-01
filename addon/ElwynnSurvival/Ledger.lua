-- Elwynn Survival: the Hearthside Ledger look. Ink on parchment for the camp's windows, a leather
-- binding round them, wax seals, wood buttons; over the world, brush-stroke bars in torn parchment
-- troughs, scraps of paper for the foe and the journal's goal, an ink reticle.
--
-- It loads after ElwynnSurvival.lua and dresses what that file built, by the frames' names and
-- fields, so neither file has to know much of the other: take this file out of the .toc and the
-- add-on looks as it did. What it can't reach at load (rows made later) it catches on a slow sweep.
--
-- The art is in art/ledger (art/ledger/manifest.json says what each piece is and its slices), the
-- fonts in fonts/ (IM Fell English SC for titles, IM Fell English for names, Crimson Pro for the
-- rest; all three under the SIL Open Font Licence, beside them).
--
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., math.mod, table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local L = {}
ElwynnSurvivalLedger = L
ES.ledger = L

local A = "Interface\\AddOns\\ElwynnSurvival\\art\\ledger\\"
local FONTS = "Interface\\AddOns\\ElwynnSurvival\\fonts\\"
local TITLE = FONTS .. "IMFellEnglishSC-Regular.ttf"
local SERIF = FONTS .. "IMFellEnglish-Regular.ttf"
local BODY = FONTS .. "CrimsonPro-Regular.ttf"
L.TITLE, L.SERIF, L.BODY = TITLE, SERIF, BODY

-- The palette (the ink, the parchment, the wax, the brass).
local INK = { 0.165, 0.13, 0.094 }
local SEPIA = { 0.353, 0.275, 0.188 }
local FADED = { 0.5, 0.43, 0.34 }
local WAX = { 0.604, 0.169, 0.125 }
local FROST = { 0.184, 0.373, 0.47 }
local BRASS = { 0.72, 0.537, 0.184 }
local BROWN = { 0.36, 0.22, 0.06 }
local MOSS = { 0.24, 0.38, 0.14 }
local PLUM = { 0.38, 0.2, 0.45 }
local RUST = { 0.62, 0.3, 0.08 }
local CREAM = { 0.953, 0.902, 0.784 }
L.INK, L.SEPIA, L.WAX, L.FROST, L.BRASS, L.CREAM = INK, SEPIA, WAX, FROST, BRASS, CREAM

-- --- fonts ---------------------------------------------------------------------------------------
-- The game's three faces become the Ledger's, a touch larger (Crimson and Fell sit smaller than
-- Friz Quadrata at the same size). Only the game's own faces are changed, so it's safe to apply
-- twice and leaves any other font alone.

local FACES = {
  { "MORPHEUS", TITLE, 1.04 },
  { "SKURRI", TITLE, 1.0 },
  { "FRIZQT", BODY, 1.2 },
  { "ARIALN", BODY, 1.16 },
}

local function MapFont(file, size)
  local up = string.upper(file or "")
  for i = 1, table.getn(FACES) do
    if string.find(up, FACES[i][1], 1, true) then
      return FACES[i][2], (size or 12) * FACES[i][3]
    end
  end
  return file, size
end
L.MapFont = MapFont

-- Dress one font string: its face now and whenever it's set again; `ink` for one on parchment
-- (no outline, no shadow, and its colours turned to ink).
local function Refont(fs)
  if fs.ledgerFont then return end
  fs.ledgerFont = true
  local orig = fs.SetFont
  fs.ledgerSetFont = orig
  fs.SetFont = function(self, file, size, flags)
    local f, s = MapFont(file, size)
    if self.ledgerInk then flags = "" end
    if self.ledgerFace then f = self.ledgerFace end
    return orig(self, f, s, flags)
  end
  local file, size, flags = fs:GetFont()
  if file then fs:SetFont(file, size, flags) end
end
L.Refont = Refont

-- Put a font string in one of the Ledger's faces outright (a title in Fell SC, a name in Fell).
local function Face(fs, face, size, flags)
  Refont(fs)
  fs.ledgerFace = face
  local _, s, fl = fs:GetFont()
  fs.ledgerSetFont(fs, face, size or s or 12, (fs.ledgerInk and "") or flags or fl or "")
end
L.Face = Face

-- --- ink -----------------------------------------------------------------------------------------
-- A colour picked for a dark window, as it would be written on parchment: the light neutrals ink,
-- the dim ones sepia, reds wax, golds brown, greens moss, blues frost.

local function InkColor(r, g, b)
  r, g, b = r or 1, g or 1, b or 1
  local hi = math.max(r, math.max(g, b))
  local lo = math.min(r, math.min(g, b))
  local lum = 0.3 * r + 0.59 * g + 0.11 * b
  if hi - lo < 0.16 then
    if lum > 0.72 then return INK end
    if lum > 0.52 then return SEPIA end
    return FADED
  end
  if r >= g and r >= b then
    if g > 0.7 * r and b < 0.75 * g then return BROWN end
    if g > 0.45 * r and b < 0.5 * r then return RUST end
    if b > 0.7 * r then return PLUM end
    return WAX
  end
  if g >= r and g >= b then
    if b > 0.85 * g and r < 0.7 * g then return FROST end
    return MOSS
  end
  -- Blue unless the red is well past the green (a violet).
  if r > g * 1.15 and r > 0.6 * b then return PLUM end
  return FROST
end
L.InkColor = InkColor

local function Hex(c)
  return string.format("%02x%02x%02x", math.floor(c[1] * 255 + 0.5), math.floor(c[2] * 255 + 0.5), math.floor(c[3] * 255 + 0.5))
end

local function InkCodes(text)
  if type(text) ~= "string" then return text end
  return (string.gsub(text, "|c(%x%x)(%x%x)(%x%x)(%x%x)", function(a, rr, gg, bb)
    local c = InkColor(tonumber(rr, 16) / 255, tonumber(gg, 16) / 255, tonumber(bb, 16) / 255)
    return "|c" .. a .. Hex(c)
  end))
end
L.InkCodes = InkCodes

-- Turn a font string to ink for good: its colour and any colour in its text, now and later.
local function Ink(fs, keep)
  if fs.ledgerInk or fs.ledgerKeep then return end
  Refont(fs)
  fs.ledgerInk = true
  local file, size = fs:GetFont()
  if file then fs.ledgerSetFont(fs, fs.ledgerFace or file, size, "") end
  fs:SetShadowColor(0, 0, 0, 0)
  if not keep then
    local setColor = fs.SetTextColor
    fs.ledgerRawColor = setColor
    fs.SetTextColor = function(self, r, g, b, a)
      local c = InkColor(r, g, b)
      return setColor(self, c[1], c[2], c[3], a or 1)
    end
    local r, g, b, a = fs:GetTextColor()
    fs:SetTextColor(r, g, b, a)
  end
  local setText = fs.SetText
  fs.SetText = function(self, t)
    return setText(self, InkCodes(t))
  end
  local t = fs:GetText()
  if t then fs:SetText(t) end
end
L.Ink = Ink

-- --- textures ------------------------------------------------------------------------------------

local function Tex(parent, layer, file)
  local t = parent:CreateTexture(nil, layer)
  t:SetTexture(A .. file)
  return t
end
L.Tex = Tex

-- Gone for good, whatever the code that made it does with it later.
local function Retire(t)
  if not t then return end
  t:SetTexture(nil)
  t:Hide()
  t.Show = function() end
  t.SetTexture = function() end
end
L.Retire = Retire

-- A texture whose later colours go through `fn(r, g, b, a)` first.
local function Recolor(t, fn)
  local orig = t.SetVertexColor
  t.ledgerSetColor = orig
  t.SetVertexColor = function(self, r, g, b, a)
    local nr, ng, nb, na = fn(r, g, b, a, self)
    return orig(self, nr, ng, nb, na)
  end
  local r, g, b, a = t:GetVertexColor()
  t:SetVertexColor(r or 1, g or 1, b or 1, a or 1)
end
L.Recolor = Recolor

-- A nine-slice: the art's corners at `edge` on screen, the sides and the middle stretched.
-- `rects` in the art's pixels ({x, y, w, h} for tl, t, tr, l, c, r, bl, b, br), on a `tw` x `th`
-- texture. `out` pushes it past the frame's edge. Returns the pieces (in .pieces) to colour.
local SLICE_NAMES = { "tl", "t", "tr", "l", "c", "r", "bl", "b", "br" }
local function Slice(f, file, layer, tw, th, rects, edge, out, noCenter)
  out = out or 0
  local s = { pieces = {} }
  local ex, ey = edge, edge * (rects.t[4] / rects.l[3])
  for i = 1, 9 do
    local key = SLICE_NAMES[i]
    if not (noCenter and key == "c") then
      local r = rects[key]
      local t = f:CreateTexture(nil, layer)
      t:SetTexture(A .. file)
      t:SetTexCoord(r[1] / tw, (r[1] + r[3]) / tw, r[2] / th, (r[2] + r[4]) / th)
      s[key] = t
      table.insert(s.pieces, t)
    end
  end
  local edgeY = edge * (rects.tl[4] / rects.tl[3])
  local function corner(t, point, ox, oy)
    t:SetWidth(edge)
    t:SetHeight(edgeY)
    t:SetPoint(point, f, point, ox, oy)
  end
  corner(s.tl, "TOPLEFT", -out, out)
  corner(s.tr, "TOPRIGHT", out, out)
  corner(s.bl, "BOTTOMLEFT", -out, -out)
  corner(s.br, "BOTTOMRIGHT", out, -out)
  s.t:SetPoint("TOPLEFT", s.tl, "TOPRIGHT", 0, 0)
  s.t:SetPoint("BOTTOMRIGHT", s.tr, "BOTTOMLEFT", 0, 0)
  s.b:SetPoint("TOPLEFT", s.bl, "TOPRIGHT", 0, 0)
  s.b:SetPoint("BOTTOMRIGHT", s.br, "BOTTOMLEFT", 0, 0)
  s.l:SetPoint("TOPLEFT", s.tl, "BOTTOMLEFT", 0, 0)
  s.l:SetPoint("BOTTOMRIGHT", s.bl, "TOPRIGHT", 0, 0)
  s.r:SetPoint("TOPLEFT", s.tr, "BOTTOMLEFT", 0, 0)
  s.r:SetPoint("BOTTOMRIGHT", s.br, "TOPRIGHT", 0, 0)
  if s.c then
    s.c:SetPoint("TOPLEFT", s.tl, "BOTTOMRIGHT", 0, 0)
    s.c:SetPoint("BOTTOMRIGHT", s.br, "TOPLEFT", 0, 0)
  end
  function s:SetVertexColor(r, g, b, a)
    for i = 1, table.getn(self.pieces) do self.pieces[i]:SetVertexColor(r, g, b, a) end
  end
  function s:Show() for i = 1, table.getn(self.pieces) do self.pieces[i]:Show() end end
  function s:Hide() for i = 1, table.getn(self.pieces) do self.pieces[i]:Hide() end end
  function s:SetTexture(file2)
    for i = 1, table.getn(self.pieces) do self.pieces[i]:SetTexture(A .. file2) end
  end
  return s
end
L.Slice = Slice

-- Rects for the art's own nine-slices (manifest.json).
local function Even(size, inset)
  local m = size - 2 * inset
  return {
    tl = { 0, 0, inset, inset }, t = { inset, 0, m, inset }, tr = { size - inset, 0, inset, inset },
    l = { 0, inset, inset, m }, c = { inset, inset, m, m }, r = { size - inset, inset, inset, m },
    bl = { 0, size - inset, inset, inset }, b = { inset, size - inset, m, inset }, br = { size - inset, size - inset, inset, inset },
  }
end
-- The 512 frames (leather, torn parchment) use the top-left 384 square: 64 corners, 256 sides.
local FRAME_RECTS = {
  tl = { 0, 0, 64, 64 }, t = { 64, 0, 256, 64 }, tr = { 320, 0, 64, 64 },
  l = { 0, 64, 64, 256 }, c = { 64, 64, 256, 256 }, r = { 320, 64, 64, 256 },
  bl = { 0, 320, 64, 64 }, b = { 64, 320, 256, 64 }, br = { 320, 320, 64, 64 },
}
L.FRAME_RECTS = FRAME_RECTS
local CARD_RECTS = Even(256, 48)

-- A three-slice across: caps `capPx` wide in the art (`tw` wide), drawn `h` tall; the caps keep
-- their shape at any height.
local function Strip(f, file, layer, tw, capPx, out)
  out = out or 0
  local s = { pieces = {} }
  local l = f:CreateTexture(nil, layer)
  local m = f:CreateTexture(nil, layer)
  local r = f:CreateTexture(nil, layer)
  for _, t in ipairs({ l, m, r }) do
    t:SetTexture(A .. file)
    table.insert(s.pieces, t)
  end
  l:SetTexCoord(0, capPx / tw, 0, 1)
  m:SetTexCoord(capPx / tw, 1 - capPx / tw, 0, 1)
  r:SetTexCoord(1 - capPx / tw, 1, 0, 1)
  s.l, s.m, s.r = l, m, r
  function s:Layout(h, capOverride)
    local cap = capOverride or (h * capPx / 64)
    self.l:ClearAllPoints()
    self.r:ClearAllPoints()
    self.m:ClearAllPoints()
    self.l:SetWidth(cap)
    self.r:SetWidth(cap)
    self.l:SetPoint("TOPLEFT", f, "TOPLEFT", -out, out)
    self.l:SetPoint("BOTTOMLEFT", f, "BOTTOMLEFT", -out, -out)
    self.r:SetPoint("TOPRIGHT", f, "TOPRIGHT", out, out)
    self.r:SetPoint("BOTTOMRIGHT", f, "BOTTOMRIGHT", out, -out)
    self.m:SetPoint("TOPLEFT", self.l, "TOPRIGHT", 0, 0)
    self.m:SetPoint("BOTTOMRIGHT", self.r, "BOTTOMLEFT", 0, 0)
  end
  function s:SetTexture(file2)
    for i = 1, 3 do self.pieces[i]:SetTexture(A .. file2) end
  end
  function s:SetVertexColor(r2, g2, b2, a2)
    for i = 1, 3 do self.pieces[i]:SetVertexColor(r2, g2, b2, a2) end
  end
  function s:Show() for i = 1, 3 do self.pieces[i]:Show() end end
  function s:Hide() for i = 1, 3 do self.pieces[i]:Hide() end end
  s:Layout(f:GetHeight() or 28)
  return s
end
L.Strip = Strip

-- Parchment tiled across a frame (the fill's 512 art at `scale` screen pixels per art pixel).
local function Parchment(f, layer, inset, scale)
  inset = inset or 0
  scale = scale or 0.75
  local t = Tex(f, layer or "BACKGROUND", "parchment_fill")
  t:SetPoint("TOPLEFT", f, "TOPLEFT", inset, -inset)
  t:SetPoint("BOTTOMRIGHT", f, "BOTTOMRIGHT", -inset, inset)
  t.ledgerScale = scale
  -- Stretched, not tiled (cropped to the frame's shape so the fibres keep theirs).
  local function fit()
    local w, h = (f:GetWidth() or 256) - 2 * inset, (f:GetHeight() or 256) - 2 * inset
    if w <= 0 or h <= 0 then return end
    if w >= h then
      t:SetTexCoord(0, 1, 0, h / w)
    else
      t:SetTexCoord(0, w / h, 0, 1)
    end
  end
  fit()
  t.Fit = fit
  return t
end
L.Parchment = Parchment

-- --- pieces --------------------------------------------------------------------------------------

-- A wood button (MakeButton's): stained plate, brass rim, cream letters in Fell SC; brighter
-- under the mouse; a faded parchment plate when it can't be used.
local function Button(b, small)
  if not b or b.ledger then return end
  b.ledger = true
  Retire(b.fill)
  Retire(b.rim)
  b.plate = Strip(b, "button_normal", "BACKGROUND", 256, 24, 1)
  b.plate:Layout((b:GetHeight() or 28) + 2)
  if b.label then
    b.label.ledgerKeep = true
    Refont(b.label)
    b.label.ledgerInk = nil
    Face(b.label, TITLE, small and 13 or 15, "")
    b.label:SetShadowColor(0, 0, 0, 0.8)
    b.label:SetShadowOffset(0, -1)
    local setColor = b.label.ledgerRawColor or b.label.SetTextColor
    b.label.SetTextColor = function(self, r, g, b2, a)
      if self.ledgerOff then
        return setColor(self, SEPIA[1], SEPIA[2], SEPIA[3], 0.85)
      end
      -- A button's own colour (green to place, red to ring) as a warm wash on the cream.
      local c = CREAM
      if r and g and b2 and math.abs(r - g) + math.abs(g - b2) > 0.4 then
        c = { 0.5 * CREAM[1] + 0.5 * r, 0.5 * CREAM[2] + 0.5 * g, 0.5 * CREAM[3] + 0.5 * b2 }
      end
      return setColor(self, c[1], c[2], c[3], a or 1)
    end
    local r, g, b2 = b.label:GetTextColor()
    b.label:SetTextColor(r, g, b2)
  end
  b.ledgerLook = function(self)
    if self.ledgerOff then
      self.plate:SetTexture("button_disabled")
    elseif self.ledgerDown then
      self.plate:SetTexture("button_pressed")
    elseif self.ledgerOver then
      self.plate:SetTexture("button_hover")
    else
      self.plate:SetTexture("button_normal")
    end
  end
  local enter, leave = b:GetScript("OnEnter"), b:GetScript("OnLeave")
  b:SetScript("OnEnter", function()
    if enter then enter() end
    this.ledgerOver = true
    this:ledgerLook()
  end)
  b:SetScript("OnLeave", function()
    if leave then leave() end
    this.ledgerOver = nil
    this.ledgerDown = nil
    this:ledgerLook()
  end)
  b:SetScript("OnMouseDown", function()
    this.ledgerDown = true
    this:ledgerLook()
  end)
  b:SetScript("OnMouseUp", function()
    this.ledgerDown = nil
    this:ledgerLook()
  end)
  if b.SetEnabledLook then
    local look = b.SetEnabledLook
    b.SetEnabledLook = function(self, on)
      look(self, on)
      self.ledgerOff = not on
      self:SetAlpha(1)
      if self.label then
        local r, g, b2 = self.label:GetTextColor()
        self.label:SetTextColor(r, g, b2)
      end
      self:ledgerLook()
    end
  end
end
L.Button = Button

-- The ink-drawn close mark for a little "x" button.
local function CloseButton(b)
  if not b or b.ledger then return end
  b.ledger = true
  Retire(b.fill)
  Retire(b.rim)
  if b.label then b.label:SetText("") b.label:Hide() b.label.Show = function() end end
  b.mark = Tex(b, "ARTWORK", "close_ink")
  b.mark:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.mark:SetWidth(30)
  b.mark:SetHeight(30)
  local enter, leave = b:GetScript("OnEnter"), b:GetScript("OnLeave")
  b:SetScript("OnEnter", function()
    if enter then enter() end
    this.mark:SetTexture(A .. "close_ink_hover")
  end)
  b:SetScript("OnLeave", function()
    if leave then leave() end
    this.mark:SetTexture(A .. "close_ink")
  end)
end
L.CloseButton = CloseButton

-- A count on a little parchment disc (shown only while there's a count).
local function CountPlate(holder, fs, size)
  if fs.ledgerPlate then return end
  local plate = Tex(holder, "ARTWORK", "count_plate")
  plate:SetWidth(size or 20)
  plate:SetHeight(size or 20)
  plate:SetPoint("CENTER", fs, "CENTER", 0, 0)
  plate:Hide()
  fs.ledgerPlate = plate
  Ink(fs)
  fs:SetTextColor(1, 1, 1)
  -- Over the disc: the string's own layer goes above the plate's.
  if fs.SetDrawLayer then fs:SetDrawLayer("OVERLAY") end
  local setText = fs.SetText
  fs.SetText = function(self, t)
    setText(self, t)
    local s = t and tostring(t) or ""
    if s ~= "" then
      local w = (self:GetStringWidth() or 8) + 10
      plate:SetWidth(math.max(size or 20, w))
      plate:Show()
    else
      plate:Hide()
    end
  end
  fs:SetText(fs:GetText())
end
L.CountPlate = CountPlate

-- A MakeSlot written into a ledger: the icon in an inked square, no dark well; dimmed (and drained
-- of colour) when it isn't lit.
local function InkSlot(s)
  if not s or s.ledger then return end
  s.ledger = true
  Retire(s.well)
  s.frame:SetTexture(A .. "icon_frame_ink")
  Recolor(s.frame, function(r, g, b, a)
    if r and r > 0.95 then return INK[1], INK[2], INK[3], 1 end
    return SEPIA[1], SEPIA[2], SEPIA[3], 0.7
  end)
  s.icon:ClearAllPoints()
  s.icon:SetPoint("TOPLEFT", s, "TOPLEFT", 4, -4)
  s.icon:SetPoint("BOTTOMRIGHT", s, "BOTTOMRIGHT", -4, 4)
  if type(s.key) == "table" then Ink(s.key) end
  if s.count then CountPlate(s, s.count, 18) end
  local set = s.Set
  s.Set = function(self, icon, lit, count)
    set(self, icon, lit, count)
    if self.icon.SetDesaturated then self.icon:SetDesaturated(not lit) end
    self:SetAlpha(1)
    if not lit then self.icon:SetVertexColor(0.85, 0.8, 0.7) end
  end
end
L.InkSlot = InkSlot

-- A MakeSlot over the world: a leather pocket, the icon in it, a brass frame.
local function PocketSlot(s, inset)
  if not s or s.ledger then return end
  s.ledger = true
  inset = inset or 6
  s.well:SetTexture(A .. "pocket_filled")
  Recolor(s.well, function(r, g, b, a) return 1, 1, 1, ((a or 1) < 0.5) and 0.55 or 1 end)
  s.icon:ClearAllPoints()
  s.icon:SetPoint("TOPLEFT", s, "TOPLEFT", inset, -inset)
  s.icon:SetPoint("BOTTOMRIGHT", s, "BOTTOMRIGHT", -inset, inset)
  -- A thin inked rim in brass, not the thick brass-cornered frame: the icon shows whole.
  s.frame:SetTexture(A .. "icon_frame_ink")
  Recolor(s.frame, function(r, g, b, a)
    if r and r > 0.95 then return CREAM[1], CREAM[2], CREAM[3], 1 end
    return BRASS[1], BRASS[2], BRASS[3], math.min(1, (a or 1) + 0.1)
  end)
  if type(s.key) == "table" then
    Refont(s.key)
    s.key:SetTextColor(CREAM[1], CREAM[2], CREAM[3])
  end
end
L.PocketSlot = PocketSlot

-- A divider (the old gold rule) as a fine ink vine.
local function Vine(t)
  t:SetTexture(A .. "divider_vine")
  t:SetHeight(16)
  Recolor(t, function(r, g, b, a) return 1, 1, 1, 0.9 end)
end

-- A leather-bound sheet in place of a Panel()'d window: parchment inside, the leather cover's
-- stitched border round it with brass corners.
local function Sheet(f, edge)
  if not f or f.ledgerSheet then return end
  f.ledgerSheet = true
  Retire(f.solid)
  Retire(f.panel)
  Retire(f.panelFrame)
  edge = edge or 44
  f.parchment = Parchment(f, "BACKGROUND", 6, 0.8)
  f.binding = Slice(f, "leather_frame", "BORDER", 512, 512, FRAME_RECTS, edge, 10, true)
  -- The parchment's own torn rim, just inside the leather.
  f.torn = Slice(f, "parchment_frame", "BACKGROUND", 512, 512, FRAME_RECTS, 34, -4, true)
  f.torn:SetVertexColor(1, 1, 1, 0.55)
  local setH, setW = f.SetHeight, f.SetWidth
  f.SetHeight = function(self, h) setH(self, h) self.parchment.Fit() end
  f.SetWidth = function(self, w) setW(self, w) self.parchment.Fit() end
end
L.Sheet = Sheet

-- Every region and child of `f`, depth first.
local function Walk(f, fn, depth)
  depth = depth or 0
  if depth > 12 then return end
  local regions = { f:GetRegions() }
  for i = 1, table.getn(regions) do fn(regions[i], f) end
  local kids = { f:GetChildren() }
  for i = 1, table.getn(kids) do
    fn(kids[i], f)
    Walk(kids[i], fn, depth + 1)
  end
end
L.Walk = Walk

local function IsFont(r) return r.GetObjectType and r:GetObjectType() == "FontString" end
local function IsTex(r) return r.GetObjectType and r:GetObjectType() == "Texture" end
local function TexIs(r, name)
  if not IsTex(r) then return false end
  local t = r:GetTexture()
  if not t then return false end
  t = string.lower(t)
  local tail = "\\art\\" .. name
  return string.sub(t, -string.len(tail)) == tail
end
L.TexIs = TexIs

-- Everything on a sheet in ink, the rules as vines.
local function InkAll(f)
  Walk(f, function(r)
    if IsFont(r) then
      if not r.ledgerKeep then Ink(r) end
    elseif TexIs(r, "divider") then
      Vine(r)
    end
  end)
end
L.InkAll = InkAll

-- --- the windows ---------------------------------------------------------------------------------

local BUILD_ART = {
  ["cooking fire"] = "build_cooking_fire", ["smithing anvil"] = "build_smithing_anvil",
  ["anvil"] = "build_smithing_anvil", ["tanning rack"] = "build_tanning_rack",
  ["cauldron"] = "build_cauldron", ["workbench"] = "build_workbench", ["palisade"] = "build_palisade",
  ["barricade"] = "build_barricade", ["spike pit"] = "build_spike_pit", ["militia post"] = "build_militia_post",
  ["watchtower"] = "build_watchtower", ["brazier"] = "build_brazier", ["healing well"] = "build_healing_well",
  ["forge"] = "build_forge", ["lumber camp"] = "build_lumber_camp",
}
L.BUILD_ART = BUILD_ART

-- The drawing for a station's name, if the Ledger has one.
local function BuildArt(name)
  if not name then return nil end
  local n = string.lower(string.gsub(name, "|c%x%x%x%x%x%x%x%x", ""))
  n = string.gsub(n, "|r", "")
  if BUILD_ART[n] then return BUILD_ART[n] end
  for k, v in pairs(BUILD_ART) do
    if string.find(n, k, 1, true) then return v end
  end
  return nil
end
L.BuildArt = BuildArt

local RES_ART = {
  timber = "res_timber", copper = "res_copper", ["raw meat"] = "res_meat", meat = "res_meat",
  roast = "res_roast", stone = "res_stone", hide = "res_hide", bronze = "res_bronze", gold = "res_gold",
}
local function ResArt(name)
  local n = string.lower(name or "")
  for k, v in pairs(RES_ART) do
    if string.find(n, k, 1, true) then return v end
  end
  return nil
end
L.ResArt = ResArt

local function SkinWindow()
  local win = ElwynnSurvivalWindow
  if not win or win.ledger then return end
  win.ledger = true
  Sheet(win)
  -- The title in Fell SC, the line under it in Fell, the rest in Crimson; all in ink (last, below,
  -- so what keeps its own colour is marked first).
  -- The header clear of the binding: the icon, title and line under it come in a little.
  win.iconSlot:ClearAllPoints()
  win.iconSlot:SetPoint("TOPLEFT", win, "TOPLEFT", 24, -26)
  win.title:ClearAllPoints()
  win.title:SetPoint("TOPLEFT", win.iconSlot, "TOPRIGHT", 12, 0)
  Face(win.title, TITLE, 24)
  win.title:SetTextColor(1, 1, 1)
  Face(win.note, SERIF, 12)
  win.note:SetTextColor(0.7, 0.7, 0.7)
  win.note:SetHeight(26)
  win.close:ClearAllPoints()
  win.close:SetPoint("TOPRIGHT", win, "TOPRIGHT", -24, -24)
  -- The hovered thing's words: two lines at most (the rest is cut with an ellipsis).
  Face(win.detail, BODY, 13)
  win.detail:SetHeight(30)
  Face(win.detailName, SERIF, 17)
  win.detailName:SetTextColor(1, 1, 1)
  InkSlot(win.iconSlot)
  -- A sprig under the title.
  win.sprig = Tex(win, "ARTWORK", "header_sprig_left")
  win.sprig:SetWidth(64)
  win.sprig:SetHeight(32)
  win.sprig:SetPoint("LEFT", win.title, "RIGHT", 6, -2)
  win.sprig:SetVertexColor(1, 1, 1, 0.85)
  CloseButton(win.close)
  Button(win.extra)
  Button(win.prev, true)
  Button(win.next, true)
  -- The rows: an ochre wash under the mouse, ink slots, wood buttons.
  for i = 1, table.getn(win.rows) do
    local r = win.rows[i]
    r.band:SetTexture("Interface\\AddOns\\ElwynnSurvival\\art\\bar")
    Recolor(r.band, function(rr, g, b, a) return 0.6, 0.42, 0.16, math.min(0.3, (a or 0) * 1.8) end)
    InkSlot(r.slot)
    Face(r.name, SERIF, 16)
    -- The line under the name stays one line, clear of the buttons (long words go to the strip).
    r.cost:SetWidth(310)
    r.cost:SetHeight(16)
    r.tag:SetWidth(150)
    r.tag:SetHeight(14)
    r.tag:SetJustifyH("LEFT")
    Button(r.go)
    Button(r.alt)
    -- A rule under each row, as a ledger's lines.
    r.rule = Tex(r, "BORDER", "divider_vine")
    r.rule:SetHeight(10)
    r.rule:SetPoint("BOTTOMLEFT", r, "BOTTOMLEFT", 40, -6)
    r.rule:SetPoint("BOTTOMRIGHT", r, "BOTTOMRIGHT", -40, -6)
    r.rule:SetVertexColor(1, 1, 1, 0.35)
  end
  -- The chest's grid: leather pockets, the count on a disc, gear marked with a seal.
  for i = 1, table.getn(win.slots) do
    local b = win.slots[i]
    b.well:SetTexture(A .. "pocket_filled")
    Recolor(b.well, function(r, g, bb, a, t)
      b.ledgerFull = (a or 1) >= 0.6
      if b.ledgerOver and b.ledgerFull then
        t:SetTexture(A .. "pocket_selected")
        return 1, 1, 1, 1
      elseif b.ledgerFull then
        t:SetTexture(A .. "pocket_filled")
        return 1, 1, 1, 1
      end
      t:SetTexture(A .. "pocket_empty")
      return 1, 1, 1, 0.7
    end)
    b.icon:ClearAllPoints()
    b.icon:SetPoint("TOPLEFT", b, "TOPLEFT", 9, -9)
    b.icon:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -9, 9)
    -- Under the mouse the pocket itself lights (brass and a warm glow round the item).
    Retire(b.frame)
    local pocket = b
    Recolor(b.frame, function(r, g, bb, a)
      local over = r and r > 0.95
      if pocket.ledgerOver ~= over then
        pocket.ledgerOver = over
        pocket.well:SetVertexColor(1, 1, 1, pocket.ledgerFull and 1 or 0.4)
      end
      return 1, 1, 1, 0
    end)
    b.gear:SetTexture(A .. "seal_red")
    b.gear:SetWidth(16)
    b.gear:SetHeight(16)
    Recolor(b.gear, function() return 1, 1, 1, 1 end)
    b.count:ClearAllPoints()
    b.count:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -6, 5)
    b.count.ledgerKeep = true
    Face(b.count, BODY, 15, "OUTLINE")
    b.count:SetTextColor(CREAM[1], CREAM[2], CREAM[3])
  end
  -- The chest's lid over the window while it's the chest.
  win.lid = Tex(win, "ARTWORK", "chest_header")
  win.lid:SetWidth(360)
  win.lid:SetHeight(90)
  win.lid:SetPoint("BOTTOM", win, "TOP", 0, -22)
  win.lid:Hide()
  win.rack = Tex(win, "ARTWORK", "rack_board")
  win.rack:SetWidth(300)
  win.rack:SetHeight(76)
  win.rack:SetTexCoord(0, 1, 0.2, 0.8)
  win.rack:SetPoint("BOTTOM", win, "TOP", 0, -20)
  win.rack:Hide()
  InkAll(win)
end

-- What each window wears on top, by its kind.
local function DressWindow()
  local win = ElwynnSurvivalWindow
  if not win or not win.ledger then return end
  if win.kind == "chest" then win.lid:Show() else win.lid:Hide() end
  if win.kind == "rack" then win.rack:Show() else win.rack:Hide() end
  -- A station's own drawing in place of its icon, where the Ledger has one.
  for i = 1, table.getn(win.rows) do
    local r = win.rows[i]
    local art = r:IsShown() and (win.kind == "build" or win.kind == "station") and BuildArt(r.name:GetText())
    if win.kind == "station" then art = nil end
    if art then
      r.slot.icon:SetTexture(A .. art)
      r.slot.icon:SetTexCoord(0, 1, 0, 1)
      r.slot.icon:ClearAllPoints()
      r.slot.icon:SetPoint("TOPLEFT", r.slot, "TOPLEFT", -4, 4)
      r.slot.icon:SetPoint("BOTTOMRIGHT", r.slot, "BOTTOMRIGHT", 4, -4)
      r.slot.frame:SetAlpha(0)
    else
      r.slot.icon:SetTexCoord(0.08, 0.92, 0.08, 0.92)
      r.slot.icon:ClearAllPoints()
      r.slot.icon:SetPoint("TOPLEFT", r.slot, "TOPLEFT", 4, -4)
      r.slot.icon:SetPoint("BOTTOMRIGHT", r.slot, "BOTTOMRIGHT", -4, 4)
      r.slot.frame:SetAlpha(1)
    end
  end
  -- The line under each name as wide as the buttons leave it.
  for i = 1, table.getn(win.rows) do
    local r = win.rows[i]
    if r:IsShown() then
      r.cost:SetWidth((r.alt and r.alt:IsShown()) and 300 or 378)
    end
  end
  -- The window's own icon: the station's drawing too.
  local art = BuildArt(win.title:GetText())
  if win.kind == "build" then art = "build_workbench" end
  if art then
    win.iconSlot.icon:SetTexture(A .. art)
    win.iconSlot.icon:SetTexCoord(0, 1, 0, 1)
    win.iconSlot.frame:SetAlpha(0)
  else
    win.iconSlot.icon:SetTexCoord(0.08, 0.92, 0.08, 0.92)
    win.iconSlot.frame:SetAlpha(1)
  end
end
L.DressWindow = DressWindow

local function Wrap(name, after)
  local orig = ES[name]
  if not orig then return end
  -- Every argument through (the book takes five: its dungeons come last).
  ES[name] = function(a, b, c, d, e, f, g, h)
    local r = orig(a, b, c, d, e, f, g, h)
    after(a, b, c, d, e, f, g, h)
    return r
  end
end
L.Wrap = Wrap

local function SkinConfirm()
  local c = ElwynnSurvivalConfirm
  if not c or c.ledger then return end
  c.ledger = true
  Sheet(c, 36)
  c.seal = Tex(c, "ARTWORK", "seal_ribbon")
  c.seal:SetWidth(64)
  c.seal:SetHeight(32)
  c.seal:SetPoint("TOP", c, "TOP", 0, 14)
  Face(c.text, SERIF, 16)
  Button(c.yes)
  Button(c.no)
  InkAll(c)
end

local function SkinCard()
  local c = ElwynnSurvivalKeys
  if not c or c.ledger then return end
  c.ledger = true
  Sheet(c)
  -- Room for the two notes under the keys, which the larger type would push into the last row.
  c:SetHeight((c:GetHeight() or 400) + 26)
  Face(c.title, TITLE, 26)
  c.title:SetTextColor(1, 1, 1)
  for i = 1, table.getn(c.rows) do
    local row = c.rows[i]
    -- The key on the right in wax, as a stamp.
    row[2]:SetTextColor(1, 0.3, 0.3)
    if row[3] and row[3].band then
      row[3].band:SetTexture("Interface\\AddOns\\ElwynnSurvival\\art\\bar")
      Recolor(row[3].band, function(r, g, b, a) return 0.6, 0.42, 0.16, math.min(0.3, (a or 0) * 1.6) end)
    end
  end
  for _, k in ipairs({ "switch", "reset", "middle", "where", "size", "close" }) do
    Button(c[k], true)
  end
  InkAll(c)
  for i = 1, table.getn(c.rows) do c.rows[i][2]:SetTextColor(1, 0.3, 0.3) end
  local cap = ElwynnSurvivalCapture
  if cap then
    if cap.bg then
      cap.bg:SetTexture(A .. "parchment_fill")
      Recolor(cap.bg, function() return 0.42, 0.33, 0.22, 0.92 end)
    end
    if cap.text then Face(cap.text, TITLE, 22) end
  end
end

local function SkinPlacing()
  local p = ElwynnSurvivalPlacing
  if not p or p.ledger then return end
  p.ledger = true
  p.bg:SetTexture(A .. "torn_banner")
  p.bg:SetWidth(640)
  p.bg:SetHeight(150)
  Recolor(p.bg, function() return 1, 1, 1, 0.97 end)
  Face(p.title, TITLE, 19)
  p.title:SetTextColor(1, 1, 1)
  Button(p.ok, true)
  Button(p.turn, true)
  Button(p.cancel, true)
  InkAll(p)
end

-- --- the HUD -------------------------------------------------------------------------------------

-- A MakeLagBar (or MakeBar) as a brush stroke in a torn parchment trough.
local function InkBar(b, color, lagColor, thin)
  if not b or b.ledger then return end
  b.ledger = true
  if b.frame then Retire(b.frame) end
  local h = b:GetHeight() or 10
  b.trough = Strip(b, thin and "bar_frame_thin" or "bar_frame", "OVERLAY", 512, thin and 24 or 32, 0)
  -- The trough's ink line sits a little outside the bar.
  local pad = thin and 3 or 4
  b.trough.l:ClearAllPoints()
  b.trough.r:ClearAllPoints()
  b.trough.l:SetWidth(thin and 8 or 12)
  b.trough.r:SetWidth(thin and 8 or 12)
  b.trough.l:SetPoint("TOPLEFT", b, "TOPLEFT", -pad - 2, pad)
  b.trough.l:SetPoint("BOTTOMLEFT", b, "BOTTOMLEFT", -pad - 2, -pad)
  b.trough.r:SetPoint("TOPRIGHT", b, "TOPRIGHT", pad + 2, pad)
  b.trough.r:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", pad + 2, -pad)
  b.track:SetTexture(A .. "parchment_fill")
  b.track:SetTexCoord(0, 1, 0.4, 0.45)
  Recolor(b.track, function(r, g, bb, a)
    -- The flash (too little stamina): the trough's paper goes red.
    if r and r > 0.4 and g < 0.2 then return 0.95, 0.45, 0.35, 0.95 end
    -- The empty part dark, so what's left stands out.
    return 0.2, 0.16, 0.12, 0.9
  end)
  b.fill:SetTexture(A .. "bar_brush_fill")
  if color then
    b.color = { r = color[1], g = color[2], b = color[3] }
    local orig = b.fill.SetVertexColor
    b.fill.SetVertexColor = function(self, r, g, bb, a)
      -- The bar's own colour shifts (a champion's second wind) go through as ink of that hue.
      local c = (r and (math.abs(r - b.color.r) + math.abs(g - b.color.g) > 0.05)) and { r, g, bb } or color
      return orig(self, c[1], c[2], c[3], 1)
    end
    b.fill:SetVertexColor(color[1], color[2], color[3], 1)
  end
  if b.lag then
    b.lag:SetTexture(A .. "bar_brush_fill")
    local lc = lagColor or { 0.93, 0.82, 0.55 }
    Recolor(b.lag, function() return lc[1], lc[2], lc[3], 0.95 end)
  end
end
L.InkBar = InkBar

-- A scrap of paper behind a HUD piece.
local function Scrap(f, file, w, h, ox, oy, layer)
  local t = Tex(f, layer or "BACKGROUND", file)
  t:SetWidth(w)
  t:SetHeight(h)
  t:SetPoint("CENTER", f, "CENTER", ox or 0, oy or 0)
  return t
end
L.Scrap = Scrap

-- Cream lettering with a soft dark shadow, for text straight over the world.
local function Cream(fs, face, size)
  Refont(fs)
  -- Over the world the plainer face reads better than Fell's rough one, with an outline to stand
  -- off a busy background.
  if face == SERIF then face = BODY end
  if face then Face(fs, face, size, "OUTLINE") end
  fs:SetShadowColor(0, 0, 0, 0.9)
  fs:SetShadowOffset(1, -1)
end

-- Bright enough to judge at a glance over a dark trough.
local HEALTH = { 0.86, 0.14, 0.09 }
local STAMINA = { 0.5, 0.8, 0.26 }

local function SkinHud()
  local vit = ElwynnSurvivalVitals
  if vit and not vit.ledger then
    vit.ledger = true
    InkBar(vit.health, HEALTH, { 0.95, 0.8, 0.5 })
    InkBar(vit.stamina, STAMINA, { 0.85, 0.9, 0.62 }, true)
    Refont(vit.healthText)
    vit.healthText:SetTextColor(CREAM[1], CREAM[2], CREAM[3])
    Cream(vit.state, TITLE, 15)
    Cream(vit.effects, SERIF, 13)
  end

  local act = ElwynnSurvivalAction
  if act and not act.ledger then
    act.ledger = true
    InkBar(act.bar, nil, nil, true)
    act.bar.fill:SetTexture(A .. "bar_brush_fill")
    Cream(act.label, SERIF, 16)
  end

  local foe = ElwynnSurvivalFoe
  if foe and not foe.ledger then
    foe.ledger = true
    -- (No paper scrap behind the foe's bar: over the world it hid what you were fighting.)
    InkBar(foe.bar, HEALTH, { 0.85, 0.6, 0.4 })
    Cream(foe.name, TITLE, 18)
    foe.name:ClearAllPoints()
    foe.name:SetPoint("BOTTOMLEFT", foe.bar, "TOPLEFT", 4, 3)
    Cream(foe.value, BODY, 12)
  end

  local boss = ElwynnSurvivalBoss
  if boss and not boss.ledger then
    boss.ledger = true
    boss.banner = Scrap(boss, "torn_banner", 760, 116, 0, -8)
    InkBar(boss.bar, { 0.56, 0.07, 0.05 }, { 0.85, 0.6, 0.4 })
    Ink(boss.name)
    Face(boss.name, TITLE, 24)
    Ink(boss.value)
    for _, k in ipairs({ "capL", "capR" }) do
      local t = boss[k]
      t:SetTexture(A .. "seal_red")
      t:SetWidth(24)
      t:SetHeight(24)
      Recolor(t, function() return 1, 1, 1, 1 end)
    end
  end

  local bar = ElwynnSurvivalBar
  if bar and not bar.ledger then
    bar.ledger = true
    for _, k in ipairs({ "bag", "character", "map", "keys", "menu", "help" }) do
      local b = bar[k]
      if b then PocketSlot(b, 3) end
    end
  end

  local aim = ElwynnSurvivalAim
  if aim and not aim.ledger then
    aim.ledger = true
    aim.ring:SetTexture(A .. "reticle_ring")
    aim.dot:SetTexture(A .. "reticle_dot")
    aim.dot:SetWidth(10)
    aim.dot:SetHeight(10)
    aim.burst:SetTexture(A .. "reticle_charge")
    Cream(aim.name, TITLE, 15)
  end
  local hit = ElwynnSurvivalHit
  if hit and hit.mark and not hit.ledger then
    hit.ledger = true
    hit.mark:SetTexture(A .. "hitmark_ink")
  end
  -- (The hurt and empowered edges keep their own vignette: it fills the screen to its edges,
  -- where the paper one left borders uncovered.)

  local cc = ElwynnSurvivalCc
  if cc and not cc.ledger then
    cc.ledger = true
    Cream(cc.big, TITLE, 24)
    Cream(cc.small, SERIF, 14)
    -- The big word stamped in a rough ink box of its own colour.
    local box = CreateFrame("Frame", nil, cc)
    box:SetHeight(36)
    box:SetPoint("CENTER", cc.big, "CENTER", 0, 0)
    cc.box = box
    local pieces = Slice(box, "stamp_frame", "BACKGROUND", 128, 64,
      { tl = { 0, 8, 12, 20 }, t = { 12, 8, 104, 20 }, tr = { 116, 8, 12, 20 },
        l = { 0, 28, 12, 8 }, c = { 12, 28, 104, 8 }, r = { 116, 28, 12, 8 },
        bl = { 0, 36, 12, 20 }, b = { 12, 36, 104, 20 }, br = { 116, 36, 12, 20 } }, 10, 0, true)
    box.pieces = pieces
    box:Hide()
    local setText, setColor = cc.big.SetText, cc.big.SetTextColor
    cc.big.SetText = function(self, t)
      setText(self, t)
      if t and t ~= "" then
        box:SetWidth((self:GetStringWidth() or 100) + 30)
        box:Show()
      else
        box:Hide()
      end
    end
    cc.big.SetTextColor = function(self, r, g, b, a)
      setColor(self, r, g, b, a)
      pieces:SetVertexColor(r, g, b, 0.9)
    end
  end

  local np = ElwynnSurvivalPrompt
  if np and not np.ledger then
    np.ledger = true
    np.band:SetTexture(A .. "scrap_note_c")
    np.band:ClearAllPoints()
    np.band:SetPoint("TOPLEFT", np, "TOPLEFT", -12, 4)
    np.band:SetPoint("BOTTOMRIGHT", np, "BOTTOMRIGHT", 12, -4)
    Recolor(np.band, function() return 1, 1, 1, 0.85 end)
    Ink(np.text)
    Face(np.text, SERIF, 16)
    np.text:SetTextColor(1, 1, 1)
    local k = np.key
    Retire(k.well)
    k.frame:SetTexture(A .. "seal_red")
    Recolor(k.frame, function() return 1, 1, 1, 1 end)
    k:SetWidth(30)
    k:SetHeight(30)
    Refont(k.word)
    Face(k.word, TITLE, 14)
    k.word:SetTextColor(CREAM[1], CREAM[2], CREAM[3])
  end

  local heal = ElwynnSurvivalHeal
  if heal and not heal.ledger then
    heal.ledger = true
    Cream(heal.text, SERIF, 22)
  end

  local info = ElwynnSurvivalInfo
  if info and not info.ledger then
    info.ledger = true
    Cream(info.raid, TITLE, 19)
    Cream(info.bags, SERIF, 14)
    Cream(info.mode, SERIF, 13)
    if info.cache then Cream(info.cache.text, SERIF, 14) end
    -- The journal's goal on a parchment card that grows with its words (any length), a red wax
    -- seal with its ribbon pinned to the card's corner.
    local note = CreateFrame("Frame", nil, info)
    note:SetWidth(300)
    note:SetPoint("TOPRIGHT", info.goal, "TOPRIGHT", 16, 9)
    -- The goal a little further under the line above it, so the card clears that line.
    info.goal:ClearAllPoints()
    info.goal:SetPoint("TOPRIGHT", info.mode, "BOTTOMRIGHT", 0, -16)
    note:SetFrameLevel(math.max(0, info:GetFrameLevel() - 1))
    note.card = Slice(note, "detail_card", "BACKGROUND", 256, 256, CARD_RECTS, 26, 0)
    note.card:SetVertexColor(1, 1, 1, 0.97)
    note.seal = Tex(note, "ARTWORK", "seal_ribbon")
    note.seal:SetWidth(76)
    note.seal:SetHeight(38)
    note.seal:SetPoint("CENTER", note, "TOPLEFT", 12, -8)
    info.note = note
    Ink(info.goal)
    Face(info.goal, SERIF, 15)
    Ink(info.goalText)
    Face(info.goalText, BODY, 13)
    info.goalText:SetWidth(262)
    -- As tall as the title and the words under it, plus the card's margins.
    local function Fit()
      local h1 = info.goal:GetHeight() or 16
      local text = info.goalText:GetText()
      local h2 = (text and text ~= "") and ((info.goalText:GetHeight() or 0) + 3) or 0
      note:SetHeight(math.max(46, h1 + h2 + 22))
      -- The kill feed under whichever ends lower: the corner's block or the card.
      local feed = ElwynnSurvivalFeed
      if feed then
        local nb, ib = note:IsShown() and note:GetBottom(), info:GetBottom()
        feed:ClearAllPoints()
        if nb and ib and nb < ib then
          feed:SetPoint("TOPRIGHT", note, "BOTTOMRIGHT", -16, -8)
        else
          feed:SetPoint("TOPRIGHT", info, "BOTTOMRIGHT", 0, -6)
        end
      end
    end
    note.Fit = Fit
    local setText = info.goal.SetText
    info.goal.SetText = function(self, t)
      setText(self, t)
      if t and t ~= "" then note:Show() else note:Hide() end
      Fit()
    end
    local setWords = info.goalText.SetText
    info.goalText.SetText = function(self, t)
      setWords(self, t)
      Fit()
    end
    info.goal:SetText(info.goal:GetText())
  end

  local feed = ElwynnSurvivalFeed
  if feed and not feed.ledger then
    feed.ledger = true
    for i = 1, table.getn(feed.rows) do
      local r = feed.rows[i]
      r.bg:SetTexture(A .. "feed_strip")
      r.bg:ClearAllPoints()
      r.bg:SetPoint("TOPLEFT", r, "TOPLEFT", 60, 3)
      r.bg:SetPoint("BOTTOMRIGHT", r, "BOTTOMRIGHT", 14, -3)
      Recolor(r.bg, function() return 1, 1, 1, 0.95 end)
      Ink(r.killer)
      Face(r.killer, SERIF, 14)
      Ink(r.victim)
      Face(r.victim, SERIF, 14)
    end
  end

  local msg = ElwynnSurvivalMessage
  if msg and msg.text and not msg.ledger then
    msg.ledger = true
    Cream(msg.text, SERIF, 18)
  end
  local flash = ElwynnSurvivalFlash
  if flash and flash.text and not flash.ledger then
    flash.ledger = true
    Cream(flash.text, TITLE, 34)
  end
  local danger = ElwynnSurvivalDanger
  if danger and danger.text and not danger.ledger then
    danger.ledger = true
    danger.splat = Tex(danger, "BACKGROUND", "ink_splat_a")
    danger.splat:SetWidth(96)
    danger.splat:SetHeight(96)
    danger.splat:SetPoint("CENTER", danger.text, "CENTER", 0, 0)
    danger.splat:SetVertexColor(0.55, 0.04, 0.03, 0.85)
    Cream(danger.text, TITLE, 56)
    danger.text:SetTextColor(CREAM[1], CREAM[2], CREAM[3])
    Cream(danger.sub, SERIF, 13)
  end

  local banner = ElwynnSurvivalBanner
  if banner and banner.bg and not banner.ledger then
    banner.ledger = true
    banner.bg:SetTexture(A .. "torn_banner")
    banner.bg:SetWidth(700)
    banner.bg:SetHeight(140)
    Recolor(banner.bg, function() return 1, 1, 1, 0.98 end)
    Ink(banner.title)
    Face(banner.title, TITLE, 40)
    banner.title:SetTextColor(1, 1, 1)
    Ink(banner.sub)
    Face(banner.sub, SERIF, 17)
    banner.sub:SetTextColor(0.7, 0.7, 0.7)
    banner.seal = Tex(banner, "ARTWORK", "seal_ribbon")
    banner.seal:SetWidth(72)
    banner.seal:SetHeight(36)
    banner.seal:SetPoint("BOTTOM", banner.title, "TOP", 0, 4)
  end

  local death = ElwynnSurvivalDown
  if death and death.bg and not death.ledger then
    death.ledger = true
    death.bg:SetTexture(A .. "torn_banner")
    death.bg:ClearAllPoints()
    death.bg:SetPoint("CENTER", death, "CENTER", 0, 0)
    death.bg:SetWidth(900)
    death.bg:SetHeight(210)
    Recolor(death.bg, function() return 0.92, 0.85, 0.75, 1 end)
    death.shade = Tex(death, "BACKGROUND", "paper_vignette")
    death.shade:SetPoint("TOPLEFT", UIParent, "TOPLEFT", 0, 0)
    death.shade:SetPoint("BOTTOMRIGHT", UIParent, "BOTTOMRIGHT", 0, 0)
    death.shade:SetVertexColor(0.12, 0.05, 0.03, 0.95)
    death.shade:SetDrawLayer("BACKGROUND")
    death.splat = Tex(death, "BORDER", "ink_splat_b")
    death.splat:SetWidth(260)
    death.splat:SetHeight(260)
    death.splat:SetPoint("CENTER", death, "CENTER", -260, 6)
    death.splat:SetVertexColor(0.5, 0.05, 0.03, 0.38)
    Ink(death.title)
    Face(death.title, TITLE, 50)
    death.title:SetTextColor(1, 0.2, 0.2)
    Ink(death.sub)
    Face(death.sub, SERIF, 17)
  end
end
L.SkinHud = SkinHud

-- --- the dock: the ability keys on a leather strap ------------------------------------------------
-- Each key a parchment roundel in a brass bezel (the icon seen through it), its key on a red wax
-- seal under it, its cooldown an ink wash; a seal at its shoulder while there's Valor for its
-- charged-up version (grey while there isn't). Valor itself a gold brush stroke.

local ROUNDEL, KEY_ICON = 56, 34

local function SkinKey(b)
  if not b or b.ledger then return end
  b.ledger = true
  Retire(b.rim)
  Retire(b.accent)
  if b.cap then
    b.cap:SetTexture(A .. "seal_red")
    b.cap:SetWidth(22)
    b.cap:SetHeight(22)
    b.cap:ClearAllPoints()
    b.cap:SetPoint("CENTER", b, "BOTTOM", 0, -6)
    b.cap:SetDrawLayer("OVERLAY")
    Recolor(b.cap, function() return 1, 1, 1, 1 end)
  end
  if b.keyText then
    b.keyText:ClearAllPoints()
    b.keyText:SetPoint("CENTER", b.cap or b, "CENTER", 0, 0)
    Face(b.keyText, TITLE, 12, "")
    b.keyText:SetTextColor(CREAM[1], CREAM[2], CREAM[3])
    b.keyText:SetShadowColor(0, 0, 0, 0.7)
    b.keyText:SetShadowOffset(0, -1)
  end
  b.icon:ClearAllPoints()
  b.icon:SetWidth(KEY_ICON)
  b.icon:SetHeight(KEY_ICON)
  b.icon:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.well:ClearAllPoints()
  b.well:SetWidth(KEY_ICON)
  b.well:SetHeight(KEY_ICON)
  b.well:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.roundel = Tex(b, "OVERLAY", "ability_roundel")
  b.roundel:SetWidth(ROUNDEL)
  b.roundel:SetHeight(ROUNDEL)
  b.roundel:SetPoint("CENTER", b, "CENTER", 0, 0)
  -- Dimmed with the icon while it can't go.
  local set = b.Set
  b.Set = function(self, icon, lit, count)
    set(self, icon, lit, count)
    self.roundel:SetVertexColor(lit and 1 or 0.72, lit and 1 or 0.68, lit and 1 or 0.62, 1)
  end
  if b.glow then
    b.glow:SetTexture(A .. "ability_roundel_glow")
    b.glow:SetWidth(ROUNDEL + 22)
    b.glow:SetHeight(ROUNDEL + 22)
  end
  -- The cooldown: an ink wash down over the icon, as tall as what's left.
  if b.shade then
    b.shade:SetTexture(A .. "parchment_fill")
    b.shade:ClearAllPoints()
    b.shade:SetPoint("TOPLEFT", b.icon, "TOPLEFT", 0, 0)
    b.shade:SetPoint("TOPRIGHT", b.icon, "TOPRIGHT", 0, 0)
    Recolor(b.shade, function() return 0.16, 0.11, 0.07, 0.78 end)
    local setH = b.shade.SetHeight
    local full = (b:GetHeight() or 44) - 6
    b.shade.SetHeight = function(self, h)
      return setH(self, math.max(1, (h or 1) * KEY_ICON / math.max(1, full)))
    end
  end
  if b.cd then
    Face(b.cd, SERIF, 20, "OUTLINE")
    b.cd:SetTextColor(CREAM[1], CREAM[2], CREAM[3])
  end
  if b.count then
    b.count:ClearAllPoints()
    b.count:SetPoint("BOTTOMRIGHT", b.icon, "BOTTOMRIGHT", 2, -1)
    Face(b.count, BODY, 13, "OUTLINE")
  end
  if b.pip then
    b.pip:SetTexture(A .. "seal_red")
    b.pip:SetWidth(16)
    b.pip:SetHeight(16)
    b.pip:ClearAllPoints()
    b.pip:SetPoint("CENTER", b, "TOPRIGHT", -2, -2)
    Recolor(b.pip, function(r, g, bb, a, t)
      -- Lit (gold): a red seal; not yet (grey): a grey one.
      if r and r > 0.9 then t:SetTexture(A .. "seal_red") else t:SetTexture(A .. "seal_grey") end
      return 1, 1, 1, 1
    end)
  end
  if b.hold then
    b.hold:ClearAllPoints()
    b.hold:SetPoint("BOTTOMLEFT", b.icon, "BOTTOMLEFT", 0, -1)
    b.hold:SetHeight(3)
    local setW = b.hold.SetWidth
    local full = (b:GetWidth() or 44) - 6
    b.hold.SetWidth = function(self, w)
      return setW(self, math.max(1, (w or 1) * KEY_ICON / math.max(1, full)))
    end
  end
end
L.SkinKey = SkinKey

local function SkinDock()
  local dock, abar = ElwynnSurvivalDock, ElwynnSurvivalAbilities
  if not dock or not abar or dock.ledger then return end
  dock.ledger = true
  -- The leather strap behind the keys alone (the bars above sit on the world).
  if dock.plate then Retire(dock.plate) end
  local strap = CreateFrame("Frame", nil, dock)
  strap:SetPoint("TOPLEFT", abar, "TOPLEFT", -24, 4)
  strap:SetPoint("BOTTOMRIGHT", abar, "BOTTOMRIGHT", 24, -16)
  strap:SetFrameLevel(math.max(0, abar:GetFrameLevel() - 1))
  local l = Tex(strap, "BACKGROUND", "action_strap")
  local m = Tex(strap, "BACKGROUND", "action_strap")
  local r = Tex(strap, "BACKGROUND", "action_strap")
  l:SetTexCoord(0, 64 / 512, 0, 1)
  m:SetTexCoord(64 / 512, 192 / 512, 0, 1)
  r:SetTexCoord(192 / 512, 256 / 512, 0, 1)
  l:SetWidth(36)
  r:SetWidth(36)
  l:SetPoint("TOPLEFT", strap, "TOPLEFT", 0, 0)
  l:SetPoint("BOTTOMLEFT", strap, "BOTTOMLEFT", 0, 0)
  r:SetPoint("TOPRIGHT", strap, "TOPRIGHT", 0, 0)
  r:SetPoint("BOTTOMRIGHT", strap, "BOTTOMRIGHT", 0, 0)
  m:SetPoint("TOPLEFT", l, "TOPRIGHT", 0, 0)
  m:SetPoint("BOTTOMRIGHT", r, "BOTTOMLEFT", 0, 0)
  dock.strap = strap
  for i = 1, table.getn(abar.slots or {}) do SkinKey(abar.slots[i]) end
  local valor = ElwynnSurvivalValor
  if valor and valor.bar then
    InkBar(valor.bar, { 0.8, 0.56, 0.12 }, { 0.98, 0.9, 0.62 }, true)
    if valor.text then
      Face(valor.text, SERIF, 13)
      valor.text:SetTextColor(1, 0.86, 0.5)
      -- Its number just past the bar's end.
      valor.text:ClearAllPoints()
      valor.text:SetPoint("LEFT", valor, "RIGHT", 8, 0)
    end
  end
end
L.SkinDock = SkinDock

-- --- the board (N): five keys, nine classes -------------------------------------------------------
-- A ledger's table: each class heads its column by its woodcut crest, each key its row on a wax
-- seal; each card a ledger cell (washed under the mouse, sealed when it's on
-- a key, hatched while locked), the hovered card written out on a clean card below.

local CLASS_CREST = {
  warrior = "crest_warrior", paladin = "crest_paladin", hunter = "crest_hunter", rogue = "crest_rogue",
  priest = "crest_priest", shaman = "crest_shaman", mage = "crest_mage", warlock = "crest_warlock",
  druid = "crest_druid",
}
local CLASS_NAMES = {
  Warrior = "warrior", Paladin = "paladin", Hunter = "hunter", Rogue = "rogue", Priest = "priest",
  Shaman = "shaman", Mage = "mage", Warlock = "warlock", Druid = "druid",
}
local ROW_GLYPH = {
  Ranged = "slot_primary", Close = "slot_area", Utility = "slot_mobility", Defense = "slot_counter",
  Ultimate = "slot_ultimate",
}
L.CLASS_CREST, L.ROW_GLYPH = CLASS_CREST, ROW_GLYPH

-- A class's colour as ink: the same hue, dark enough to read on parchment (the priest's white,
-- plain ink).
local function DarkInk(r, g, b)
  r, g, b = r or 1, g or 1, b or 1
  local hi = math.max(r, math.max(g, b))
  local lo = math.min(r, math.min(g, b))
  if hi - lo < 0.16 then return InkColor(r, g, b) end
  local k = 0.5 / math.max(0.01, hi)
  return { r * k, g * k, b * k }
end
L.DarkInk = DarkInk

-- A font string on parchment whose colours keep their hue (a class's name), darkened to ink.
local function InkHue(fs)
  Ink(fs, true)
  if fs.ledgerHue then return end
  fs.ledgerHue = true
  local setColor = fs.SetTextColor
  fs.SetTextColor = function(self, r, g, b, a)
    local c = DarkInk(r, g, b)
    return setColor(self, c[1], c[2], c[3], a or 1)
  end
  local r, g, b, a = fs:GetTextColor()
  fs:SetTextColor(r, g, b, a)
end
L.InkHue = InkHue

local CELL_RECTS = {
  tl = { 0, 0, 16, 32 }, t = { 16, 0, 96, 32 }, tr = { 112, 0, 16, 32 },
  l = { 0, 32, 16, 64 }, c = { 16, 32, 96, 64 }, r = { 112, 32, 16, 64 },
  bl = { 0, 96, 16, 32 }, b = { 16, 96, 96, 32 }, br = { 112, 96, 16, 32 },
}

local function SkinBoardCard(b)
  if b.ledger then return end
  b.ledger = true
  Retire(b.plate)
  Retire(b.rim)
  Retire(b.well)
  b.cell = Slice(b, "cell_normal", "BACKGROUND", 128, 128, CELL_RECTS, 8, 0)
  b.inkFrame = Tex(b, "OVERLAY", "icon_frame_ink")
  b.inkFrame:SetWidth(34)
  b.inkFrame:SetHeight(34)
  b.inkFrame:SetPoint("CENTER", b.icon, "CENTER", 0, 0)
  b.inkFrame:SetVertexColor(INK[1], INK[2], INK[3], 0.9)
  -- The column says the class: no rule of its colour on every card.
  Retire(b.accent)
  -- The key it's on: a red seal in the corner with the key pressed into it.
  b.seal = Tex(b, "OVERLAY", "seal_red")
  b.seal:SetWidth(22)
  b.seal:SetHeight(22)
  b.seal:SetPoint("CENTER", b, "TOPRIGHT", -4, -4)
  b.seal:Hide()
  b.sealKey = b:CreateFontString(nil, "OVERLAY")
  b.sealKey:SetFont(TITLE, 11, "")
  b.sealKey:SetPoint("CENTER", b.seal, "CENTER", 0, 0)
  b.sealKey:SetTextColor(CREAM[1], CREAM[2], CREAM[3])
  b.sealKey:SetShadowColor(0, 0, 0, 0.6)
  b.sealKey:SetShadowOffset(0, -1)
  b.sealKey:Hide()
  -- The name on up to two lines at the top; one line at the foot: the charged-up version's name
  -- when it's yours, else whether it can be unlocked.
  Ink(b.name)
  Face(b.name, BODY, 10.5)
  b.name:ClearAllPoints()
  b.name:SetPoint("TOPLEFT", b, "TOPLEFT", 42, -6)
  b.name:SetWidth((b:GetWidth() or 112) - 50)
  b.name:SetHeight(24)
  Ink(b.ex)
  Face(b.ex, BODY, 9.5)
  b.ex:ClearAllPoints()
  b.ex:SetPoint("BOTTOMLEFT", b, "BOTTOMLEFT", 42, 6)
  b.ex:SetWidth((b:GetWidth() or 112) - 48)
  b.ex:SetHeight(12)
  Ink(b.state)
  Face(b.state, SERIF, 10.5)
  b.state:ClearAllPoints()
  b.state:SetPoint("BOTTOMLEFT", b, "BOTTOMLEFT", 42, 5)
  b.state:SetJustifyH("LEFT")
end

local function DrawCellLook(b)
  local d = b.d
  if not d or not b.cell then return end
  local file = "cell_normal"
  if d.on ~= "" then
    file = "cell_equipped"
  elseif not d.owned then
    file = "cell_locked"
  elseif b.hover then
    file = "cell_hover"
  end
  if b.hover and d.on == "" and d.owned then file = "cell_hover" end
  b.cell:SetTexture(file)
  b.cell:SetVertexColor(1, 1, 1, (d.owned or d.can) and 1 or 0.85)
  if b.icon.SetDesaturated then b.icon:SetDesaturated(not d.owned) end
  -- One line at the foot: the EX version's name, or (not yours yet) whether it can be had.
  local st = b.state:GetText() or ""
  if d.on ~= "" then
    b.seal:Show()
    b.sealKey:SetText((string.gsub(st, "^on ", "")))
    b.sealKey:Show()
    b.state:Hide()
    b.ex:Show()
  else
    b.seal:Hide()
    b.sealKey:Hide()
    if d.owned then
      b.state:Hide()
      b.ex:Show()
    else
      b.ex:Hide()
      if st == "locked" then b.state:SetTextColor(0.55, 0.55, 0.55) else b.state:SetTextColor(0.45, 1, 0.45) end
      b.state:Show()
    end
  end
end

local function SkinBoard()
  local tree = ElwynnSurvivalTree
  if not tree or tree.ledger then return end
  tree.ledger = true
  Sheet(tree, 40)
  tree.title:ClearAllPoints()
  tree.title:SetPoint("TOP", tree, "TOP", 0, -22)
  Face(tree.title, TITLE, 25)
  tree.title:SetTextColor(1, 1, 1)
  Face(tree.points, SERIF, 13)
  -- The count up in the corner, clear of the title and the class heads.
  tree.points:ClearAllPoints()
  tree.points:SetPoint("TOPLEFT", tree, "TOPLEFT", 30, -24)
  tree.points:SetJustifyH("LEFT")
  Button(tree.close, true)
  Button(tree.full, true)
  Button(tree.reset, true)
  Face(tree.hint, SERIF, 12)
  -- The detail strip: a clean card, its split an ink line.
  if tree.detail then
    Retire(tree.detail.bg)
    tree.detail.card = Slice(tree.detail, "detail_card", "BACKGROUND", 256, 256, CARD_RECTS, 30, 4)
    tree.detail.card:SetVertexColor(1, 1, 1, 0.95)
  end
  if tree.split then Recolor(tree.split, function() return INK[1], INK[2], INK[3], 0.35 end) end
  for _, col in ipairs({ tree.regular, tree.charged }) do
    if col then
      InkHue(col.title)
      Face(col.title, SERIF, 18)
      Ink(col.tag)
      Ink(col.how)
      Ink(col.text)
      Face(col.text, BODY, 13)
      Ink(col.facts)
    end
  end
  -- The heads: a crest over each class, a seal and a glyph by each key.
  local regions = { tree:GetRegions() }
  for i = 1, table.getn(regions) do
    local r = regions[i]
    if IsFont(r) then
      local t = r:GetText() or ""
      if CLASS_NAMES[t] then
        InkHue(r)
        Face(r, TITLE, 15)
        local crest = Tex(tree, "ARTWORK", CLASS_CREST[CLASS_NAMES[t]])
        crest:SetWidth(26)
        crest:SetHeight(26)
        crest:SetPoint("RIGHT", r, "LEFT", -3, 1)
        local cr, cg, cb = r:GetTextColor()
        crest:SetVertexColor(cr, cg, cb, 0.95)
      elseif t == "Reach the foe" or t == "In the brawl" or t == "Move, control" or t == "Stay alive, punish"
        or t == "The whole Valor bar" then
        -- A row's line: kept clear of the first card.
        r:SetWidth(58)
      elseif ROW_GLYPH[t] then
        Ink(r)
        Face(r, TITLE, 14)
        r:SetWidth(64)
        r:SetTextColor(1, 1, 1)
      end
    elseif TexIs(r, "keycap") then
      r:SetTexture(A .. "seal_red")
      r:SetWidth(30)
      r:SetHeight(30)
      Recolor(r, function() return 1, 1, 1, 1 end)
    end
  end
  for i = 1, table.getn(tree.rowKeys or {}) do
    local k = tree.rowKeys[i]
    k.ledgerKeep = true
    Face(k, TITLE, 15, "")
    k:SetTextColor(CREAM[1], CREAM[2], CREAM[3])
    k:SetShadowColor(0, 0, 0, 0.6)
  end
  InkAll(tree)
end
L.SkinBoard = SkinBoard

-- While the bar over the dock names what you're doing, the word over the health doesn't repeat it.
Wrap("DrawAction", function()
  local vit, act = ElwynnSurvivalVitals, ElwynnSurvivalAction
  if vit and vit.state and act then
    vit.state:SetAlpha(act:IsShown() and 0 or 1)
  end
end)

Wrap("DrawCard", function(b)
  if b and b.ledger then DrawCellLook(b) end
end)
Wrap("TreeLine", function()
  local tree = ElwynnSurvivalTree
  if not tree then return end
  SkinBoard()
  for _, b in pairs(tree.cards or {}) do
    if not b.ledger then
      SkinBoardCard(b)
      -- What TreeLine just set, again through the ink.
      if b.d then
        b.name:SetText(b.name:GetText())
        if b.d.owned then b.name:SetTextColor(1, 1, 1) else b.name:SetTextColor(0.55, 0.53, 0.5) end
        b.ex:SetTextColor(1, 0.7, 0.35)
        local r, g, bb = b.state:GetTextColor()
        b.state:SetTextColor(r, g, bb)
        ES.DrawCard(b)
      end
    end
  end
end)

-- --- Elwynn's places on the world map -------------------------------------------------------------
-- Each place an icon in a small parchment roundel, a red seal with a pressed check once the camp
-- has found it; the tally along the top in Fell.

local function SkinPin(b)
  if not b or b.ledger then return end
  b.ledger = true
  if b.ring then
    b.ring:SetTexture(A .. "effect_frame")
    b.ring:SetDrawLayer("OVERLAY")
    Recolor(b.ring, function() return 1, 1, 1, 1 end)
    b.ring:ClearAllPoints()
    b.ring:SetPoint("TOPLEFT", b, "TOPLEFT", -3, 3)
    b.ring:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", 3, -3)
  end
  if b.check then
    b.check:SetTexture(A .. "seal_check")
    b.check:SetWidth(15)
    b.check:SetHeight(15)
  end
  if b.icon then
    local setColor = b.icon.SetVertexColor
    b.icon.SetVertexColor = function(self, r, g, bb, a)
      -- Not found yet: drained to a sepia print.
      if self.SetDesaturated then self:SetDesaturated(r and r < 0.9) end
      if r and r < 0.9 then return setColor(self, 0.85, 0.75, 0.6, a or 1) end
      return setColor(self, r, g, bb, a)
    end
  end
end

Wrap("DrawPins", function()
  for i = 1, table.getn(ES.pinFrames or {}) do
    local b = ES.pinFrames[i]
    if not b.ledger then
      SkinPin(b)
      -- What DrawPins set, again through the skin.
      if b.pin and b.icon then
        if b.pin.found then b.icon:SetVertexColor(1, 1, 1) else b.icon:SetVertexColor(0.55, 0.55, 0.55) end
      end
    end
  end
  if ES.pinText and not ES.pinText.ledgerFont then
    Refont(ES.pinText)
    Face(ES.pinText, SERIF, 16)
    ES.pinText:SetTextColor(CREAM[1], CREAM[2], CREAM[3])
  end
end)

-- --- the sweep ------------------------------------------------------------------------------------
-- Every frame the add-on made, by its name: each font in the Ledger's faces. Rows and marks made
-- after load are caught on the next sweep.

local function RefontTree(f)
  Walk(f, function(r)
    if IsFont(r) then Refont(r) end
  end)
end

-- The add-on's named frames, found once (and again on waking): the sweep walks only these.
local named = nil
function L.FindFrames()
  named = {}
  for name, f in pairs(getfenv(0)) do
    if type(name) == "string" and string.sub(name, 1, 14) == "ElwynnSurvival" and type(f) == "table"
      and type(f.GetObjectType) == "function" and type(f.GetRegions) == "function" then
      table.insert(named, f)
    end
  end
end

function L.Sweep()
  if not named then L.FindFrames() end
  for i = 1, table.getn(named) do
    RefontTree(named[i])
  end
end

function L.Apply()
  SkinWindow()
  SkinBoard()
  SkinDock()
  SkinConfirm()
  SkinCard()
  SkinPlacing()
  SkinHud()
  L.Sweep()
end

-- The windows' extras, after the add-on draws them.
Wrap("Chest", DressWindow)
Wrap("Station", DressWindow)
Wrap("BuildMenu", DressWindow)
Wrap("Rack", DressWindow)
Wrap("DrawJournal", DressWindow)
Wrap("Book", DressWindow)
Wrap("Lamp", DressWindow)
Wrap("Muster", DressWindow)
Wrap("Ledger", DressWindow)
-- Waking: anything made since load is dressed, and the sweep learns the new frames.
Wrap("Wake", function()
  L.FindFrames()
  L.Apply()
end)

L.Apply()

local sweeper = CreateFrame("Frame", "ElwynnSurvivalLedgerSweep", UIParent)
sweeper.t = 0
sweeper:SetScript("OnUpdate", function()
  this.t = this.t + (arg1 or 0)
  if this.t < 1.5 then return end
  this.t = 0
  if ES.awake then L.Sweep() end
end)
