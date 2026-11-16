-- Elwynn Survival: the light chain under the reticle (Corey: "I wasn't able to tell if I was doing
-- my attack combo correctly?").
-- A small diamond for each blow of the weapon's set (2 to 4, a two-blade run's 4), lit as each is
-- swung; the last, the finisher, a little bigger and warm even unlit. A thin line under them
-- holds full through the blow and shrinks over the time left to press the next: while it shows,
-- the next press carries the chain on; when it's gone, the diamonds fade and the next press
-- starts again at 1. The finisher lights them all gold and they burst as it lands, and the hero's
-- hands flare with Execute's red and its special swing (the server's kit 394, played here at
-- once on our own hero as the clip crosses its impact).
-- Nothing on the ground: it reads with floor markers off. Our blows as the add-on plays them at
-- once (`PlayMine`), the server's word after (`AN`'s place in the chain, which a hero with two
-- weapons takes as it comes: the run of both hands is the server's to count).
-- Written for the 1.12 client's Lua 5.0: no #, no %, table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local ART = "Interface\\AddOns\\ElwynnSurvival\\art\\"
-- The swinger's flash (SpellVisualKit 394: Execute's cast, red on both hands, with the
-- Warrior's special swing). The server sends it to everyone else.
local FINISHER_KIT = 394
local PIP, PIP_LAST, GAP, MOST = 8, 10, 6, 6
-- The finisher's burst, and a dropped chain's fade (seconds).
local BURST, FADE = 0.45, 0.35

local ch = CreateFrame("Frame", "ElwynnSurvivalChain", WorldFrame)
ch:SetWidth(1)
ch:SetHeight(1)
ch:SetPoint("CENTER", WorldFrame, "CENTER", 0, -28)
ch:SetFrameStrata("HIGH")
ch:Hide()
ch.pips = {}
for i = 1, MOST do
  local t = ch:CreateTexture(nil, "OVERLAY")
  t:SetTexture(ART .. "diamond")
  t:SetBlendMode("ADD")
  t:Hide()
  ch.pips[i] = t
end
ch.line = ch:CreateTexture(nil, "ARTWORK")
ch.line:SetTexture(ART .. "hairline")
ch.line:SetBlendMode("ADD")
ch.line:SetHeight(2)
ch.line:SetPoint("CENTER", ch, "CENTER", 0, -8)
ch.line:Hide()
ES.chainFrame = ch

-- The row's width for `n` blows.
local function Width(n)
  return (n - 1) * (PIP + GAP) + PIP_LAST
end

-- Where blow `i` of `n` sits.
local function Lay(n)
  local w = Width(n)
  for i = 1, MOST do
    local t = ch.pips[i]
    if i <= n then
      t:ClearAllPoints()
      t:SetPoint("CENTER", ch, "CENTER", -w / 2 + PIP / 2 + (i - 1) * (PIP + GAP), 0)
      t:Show()
    else
      t:Hide()
    end
  end
end

-- Blow `b` of `o` swung: its impact and its end (times by GetTime), and the seconds after the end
-- that the next press still carries the chain on.
local function Set(b, o, impact, ends, open)
  o = math.min(o, MOST)
  b = math.min(b, o)
  local c = ES.chain
  local now = GetTime()
  if not c or c.dropAt or c.blow ~= b or c.of ~= o then
    c = { stepAt = now }
  end
  c.blow, c.of, c.impact, c.ends, c.open = b, o, impact, ends, open or 0.6
  c.fin = b == o
  ES.chain = c
  Lay(o)
  ch:Show()
end

-- The chain ends short of its finisher (a heavy attack, a feint, a stagger): the diamonds fade.
function ES.ChainDrop()
  local c = ES.chain
  if c and not c.fin and not c.dropAt then c.dropAt = GetTime() end
end

-- Our own blow just played (`PlayMine`): its place in the chain, as far as this side knows.
function ES.ChainMine(kind, step)
  local m = ES.mine
  if not m then return end
  if kind ~= "light" then
    ES.ChainDrop()
    return
  end
  -- Two weapons in hand: the server counts the run.
  if ES.chainTwo then return end
  local mv = ES.moves
  local n = mv and table.getn(mv.light) or 0
  if n < 2 then
    ES.ChainDrop()
    return
  end
  local b = math.mod(step or 0, n) + 1
  m.finisher = b == n
  Set(b, n, m.start + m.move.impact, m.start + m.move.impact + m.move.recovery, mv.comboGrace)
end

-- The finisher's flash on our own hero, as the clip crosses its impact.
function ES.ChainImpact()
  local m = ES.mine
  if not m or not m.finisher or m.flashed then return end
  m.flashed = true
  if ActionKit then ActionKit("player", FINISHER_KIT) end
end

-- `AN` for our own swing goes on `<blow>/<of>`, `<open secs>`, `<two hands 1|0>` (fields 14 to
-- 16; an older server sends none). `same`: the blow we already played.
function ES.ChainHeard(f, same)
  local _, _, b, o = string.find(f[14] or "", "^(%d+)/(%d+)$")
  b, o = tonumber(b), tonumber(o)
  if not b or not o then return end
  ES.chainTwo = f[16] == "1"
  if o < 2 or b < 1 then
    ES.ChainDrop()
    return
  end
  local now = GetTime()
  local impact = now + (tonumber(f[4]) or 0)
  local ends = impact + (tonumber(f[9]) or 0)
  local c = ES.chain
  -- (The blow we guessed, as we guessed it: its times as it began here.)
  if same and c and not c.dropAt and c.blow == b and c.of == o then
    impact, ends = c.impact, c.ends
  end
  local m = ES.mine
  if m then
    m.finisher = b == o
    -- Its impact already past here (the word came late): the flash now.
    if m.finisher and now - m.start >= m.move.impact then ES.ChainImpact() end
  end
  Set(b, o, impact, ends, tonumber(f[15]))
end

ch:SetScript("OnUpdate", function()
  local c = ES.chain
  if not c then
    this:Hide()
    return
  end
  local now = GetTime()
  -- Asleep, or a spell's skill check owns the middle of the screen: nothing drawn.
  if not ES.awake or (ES.check and ES.check:IsShown()) then
    this:SetAlpha(0)
    return
  end
  this:SetAlpha(1)
  local n = c.of
  local step = now - (c.stepAt or 0)
  -- The newest swells as it lights.
  local swell = step < 0.2 and (1 + 0.7 * (1 - step / 0.2)) or 1
  if c.fin then
    -- The finisher: every diamond gold, bursting outward as it lands, then gone.
    local t = now - c.impact
    if t >= BURST then
      ES.chain = nil
      this:Hide()
      return
    end
    for i = 1, n do
      local p = this.pips[i]
      local size = (i == n) and PIP_LAST or PIP
      local a = 1
      if t > 0 then
        size = size * (1 + 1.6 * t / BURST)
        a = 1 - t / BURST
      elseif i == n then
        size = size * swell
      end
      p:SetWidth(size)
      p:SetHeight(size)
      p:SetVertexColor(1, 0.8, 0.35, a)
    end
    this.line:Hide()
    return
  end
  -- The time to press the next ran out: dropped.
  if not c.dropAt and now >= c.ends + c.open then c.dropAt = c.ends + c.open end
  local fade = 1
  if c.dropAt then
    local t = now - c.dropAt
    if t >= FADE then
      ES.chain = nil
      this:Hide()
      return
    end
    fade = 1 - t / FADE
  end
  local waiting = now >= c.ends and not c.dropAt
  for i = 1, n do
    local p = this.pips[i]
    local size = (i == n) and PIP_LAST or PIP
    local r, g, b, a
    if i <= c.blow then
      r, g, b, a = 0.95, 0.97, 1, 0.95
      if i == c.blow then size = size * swell end
      if c.dropAt then r, g, b = 0.6, 0.6, 0.6 end
    elseif i == n then
      -- The finisher, still to come: warm.
      r, g, b, a = 1, 0.75, 0.35, 0.35
    else
      r, g, b, a = 0.8, 0.85, 0.9, 0.25
    end
    -- Waiting on the next press: the next diamond breathes.
    if waiting and i == c.blow + 1 then
      a = a + 0.35 * (0.5 + 0.5 * math.sin(now * 12))
    end
    p:SetWidth(size)
    p:SetHeight(size)
    p:SetVertexColor(r, g, b, a * fade)
  end
  -- The line: whole through the blow, shrinking over the time left to press the next.
  if c.dropAt then
    this.line:Hide()
  else
    local share, a = 1, 0.45
    if waiting then
      share = math.max(0, 1 - (now - c.ends) / math.max(0.05, c.open))
      a = 0.8
    end
    this.line:SetWidth(math.max(1, (Width(n) + 8) * share))
    this.line:SetVertexColor(0.95, 0.97, 1, a)
    this.line:Show()
  end
end)
