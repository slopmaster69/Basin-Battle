-- Elwynn Survival: the skill checks, drawn. A check's ring sits on the reticle in its school's
-- colours (fire burning, frost shifting, arcane runes turning) and a thin ring of light closes on
-- it; the moment they meet is the moment to act. The server rolls each cast's timing and sends it
-- as the check begins (CK), so the ring is drawn before it's due; it vanishes the instant the
-- check is made and the outcome flashes in its place (a white-gold pop for a perfect, a star for
-- a critical, a red crack for a fail, a fizzle of smoke for an all-or-nothing miss). The add-on
-- judges the press it sees at once, as the server will, and the server's word (CR) stands.
--
--   release (Fire): hold, and let go as the closing ring meets the school ring.
--   rhythm (Arcane): a closing ring for every beat, a pip for each; press as each ring meets,
--     and let a rest (the hollow, slashed diamond) pass.
--   sweep (Frost, Cone of Cold): a frost dial; the foes in reach marked with frost sigils at
--     their bearings, a needle of cold light sweeping across them with the jet; stop it on one.
--   orb (Frost, Frozen Orb): a frost track ahead of the reticle, the foes along the orb's line
--     marked at their distance, the orb climbing it; stop it on one (not in the red at the foot).
--
-- Placement differs by school and spell: Fireball's ring on the reticle; Meteor's on the ground
-- round its circle and Arcane Explosion's round the caster's feet (on a client with
-- `SetGroundRing`); Frost's marks hovering over the foes themselves, on their nameplates.
--
-- No words but a first-time hint under the ring. Drawn on WorldFrame, so it shows with the rest
-- of the interface put away. Written for the 1.12 client's Lua 5.0: no #, no %, table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local ART = "Interface\\AddOns\\ElwynnSurvival\\art\\check\\"
local FONT = "Fonts\\FRIZQT__.TTF"
-- A padded texture's art fills the middle 70.7% of its square: turned about the centre, the
-- corners stay inside it.
local H = 0.35355
local PI2 = 6.2831853

local SCHOOL = {
  fire = {
    ring = "ring_fire", line = 0.86, size = 150,
    tint = { 1.0, 0.55, 0.16 }, glow = { 1.0, 0.78, 0.35 },
    spin = 0.35, spin2 = -0.55, sound = "Sound\\Spells\\FireBallImpactA.wav",
    fizzle = "Sound\\Spells\\Fizzle\\FizzleFireA.wav",
  },
  frost = {
    ring = "ring_frost", line = 0.84, size = 150,
    tint = { 0.55, 0.86, 1.0 }, glow = { 0.8, 0.95, 1.0 },
    spin = 0.08, spin2 = -0.12, sound = "Sound\\Spells\\FrostNova.wav",
    fizzle = "Sound\\Spells\\Fizzle\\FizzleFrostA.wav",
  },
  -- The Priest's: Penance's gold ring of light, and the same in white made shadow's violet (never the Mage's
  -- arcane on a Priest's staff: Corey).
  holy = {
    ring = "priest_penance_ring", line = 0.68, size = 160,
    tint = { 1.0, 0.9, 0.6 }, glow = { 1.0, 0.97, 0.85 },
    spin = 0.12, spin2 = -0.2, sound = "Sound\\Spells\\ArcaneMissileImpact1a.wav",
    fizzle = "Sound\\Spells\\Fizzle\\FizzleHolyA.wav",
  },
  shadow = {
    ring = "priest_ring_white", line = 0.68, size = 160,
    tint = { 0.7, 0.42, 1.0 }, glow = { 0.85, 0.7, 1.0 }, ringTint = { 0.72, 0.45, 1.0 },
    spin = 0.18, spin2 = -0.25, sound = "Sound\\Spells\\ArcaneMissileImpact1a.wav",
    fizzle = "Sound\\Spells\\Fizzle\\FizzleShadowA.wav",
  },
  arcane = {
    ring = "ring_arcane", line = 0.71, size = 170,
    tint = { 1.0, 0.36, 0.92 }, glow = { 1.0, 0.75, 1.0 },
    spin = 0.22, spin2 = -0.3, sound = "Sound\\Spells\\ArcaneMissileImpact1a.wav",
    fizzle = "Sound\\Spells\\Fizzle\\FizzleShadowA.wav",
  },
}
-- A school ring drawn in its own colours (the shadow's is Penance's ring in white, made violet).
local WHITE3 = { 1, 1, 1 }
local TICK = "Sound\\interface\\MagicClick.wav"
local CHIME = "Sound\\interface\\igTextPopupPing02.wav"
-- The closing ring's texture draws its line at 0.30 of the square: 0.849 of a turned quad's half.
local CLOSER_LINE = 0.849
-- How far out a closing ring starts (times the school ring's line), and how long before its beat
-- a rhythm's ring appears.
local START_OUT = 2.6
local LEAD = 0.75
local HINTS = {
  release = "Release as the rings meet",
  rhythm = "Press as each ring meets  -  let the hollow one pass",
  sweep = "Press again as the light crosses a mark",
  orb = "Press again as the orb reaches a mark",
}
-- Checks drawn on the ground (a client with SetGroundRing): where, and the ring's radius in yards.
local GROUND = {
  meteor = { at = "aim", radius = 6.0 },
  arcane_explosion = { at = "feet", radius = 6.0, grow = 1.0 },
}
-- Each ground texture's target line, as a share of its half-width; the closing ring's line.
local GROUND_TEX = {
  fire = { tex = "ground_fire", line = 0.84 },
  frost = { tex = "ground_frost", line = 0.80 },
  arcane = { tex = "ground_arcane", line = 0.80 },
}
local GROUND_CLOSER = 0.60
-- On the ground the ring of light starts nearer (the circle is already wide from the camera).
local GROUND_OUT = 1.6

-- The dial spreads a sweep's bearings this much wider than life, so 120 degrees fill its top half.
local DIAL_SPREAD = 1.5
local BAR = "Interface\\AddOns\\ElwynnSurvival\\art\\bar"

local check = CreateFrame("Frame", "ElwynnSurvivalCheck", WorldFrame)
check:SetWidth(1)
check:SetHeight(1)
check:SetPoint("CENTER", WorldFrame, "CENTER", 0, 0)
check:SetFrameStrata("HIGH")
check:Hide()
ES.check = check

-- --- recoloured looks ------------------------------------------------------------------------------

-- On a client with SetEffectTint (patch 0042): the Mage's arcane looks in one arcane pink, the
-- colour of the arcane rings (Corey: the ward should crackle in arcane colours, its perfect burst
-- the barrier's colour; the Arcane staff's bolt bright pink).
local ARCANE_PINK = "FF4FE6"
local TINTS = {
  ["Spells\\CounterSpell_Impact_Chest"] = ARCANE_PINK, -- the ward's crackle
  ["Spells\\arcanepower_state_chest"] = ARCANE_PINK,   -- the barrier while it's up
  ["Spells\\ArcaneExplosion_Impact_Chest"] = ARCANE_PINK, -- a perfect ward's burst
  ["Spells\\ManaBurn_Chest"] = ARCANE_PINK,            -- the ward breaking
  ["Spells\\LightningStreak_Missile"] = ARCANE_PINK,   -- the Arcane staff's bolt
  -- The swirl in the ward's raised hand (Polymorph's precast), under the names it may go by.
  ["Spells\\Polymorph_Precast_Hand"] = ARCANE_PINK,
  ["Spells\\Arcane_Precast_Hand"] = ARCANE_PINK,
  ["Spells\\ArcaneMissile_Precast_Hand"] = ARCANE_PINK,
}
if SetEffectTint then
  for model, colour in pairs(TINTS) do SetEffectTint(model, colour) end
end

-- --- drawing helpers ------------------------------------------------------------------------------

local function Tex(file, layer, add)
  local t = check:CreateTexture(nil, layer or "ARTWORK")
  t:SetTexture(ART .. file)
  if add ~= false then t:SetBlendMode("ADD") end
  t:SetPoint("CENTER", check, "CENTER", 0, 0)
  t:Hide()
  return t
end

-- A padded texture drawn `size` wide at `x`, `y`, turned `a` radians, `alpha`, tinted.
local function Draw(t, size, a, alpha, r, g, b, x, y)
  local c, s = math.cos(a) * H, math.sin(a) * H
  -- Corners (x right, y down): UL (-1,-1), LL (-1,1), UR (1,-1), LR (1,1), turned by a.
  t:SetTexCoord(0.5 - c + s, 0.5 - s - c, 0.5 - c - s, 0.5 - s + c, 0.5 + c + s, 0.5 + s - c, 0.5 + c - s, 0.5 + s + c)
  t:SetWidth(size)
  t:SetHeight(size)
  t:ClearAllPoints()
  t:SetPoint("CENTER", check, "CENTER", x or 0, y or 0)
  t:SetVertexColor(r or 1, g or 1, b or 1, math.max(0, math.min(1, alpha)))
  t:Show()
end

local function Clamp(v, lo, hi)
  if v < lo then return lo end
  if v > hi then return hi end
  return v
end

local function EaseOut(t) return 1 - (1 - t) * (1 - t) end

-- --- the pieces ------------------------------------------------------------------------------------

local ring = Tex("ring_fire", "ARTWORK")     -- the school ring
local ring2 = Tex("ring_fire", "ARTWORK")    -- its second, counter-turning layer (the life in it)
local halo = Tex("check_glow", "BORDER")     -- the glow behind it while the window is open
local spark = Tex("check_critical", "OVERLAY") -- the critical's spot, on the ring at the top
local closers = {}
for i = 1, 5 do closers[i] = Tex("check_closer", "OVERLAY") end
local rests = {}
for i = 1, 3 do rests[i] = Tex("pip_rest", "OVERLAY") end
local pips = {}
for i = 1, 10 do pips[i] = Tex("pip_arcane", "OVERLAY") end
-- The frost marks (a foe on the dial or the track), the needle's light and its trail, the track.
local marks = {}
for i = 1, 6 do marks[i] = Tex("sigil_frost", "OVERLAY") end
local needle = Tex("check_glow", "OVERLAY")
local needleCore = Tex("check_critical", "OVERLAY")
local trail = {}
for i = 1, 4 do trail[i] = Tex("check_glow", "ARTWORK") end
local track = check:CreateTexture(nil, "BORDER")
track:SetTexture(BAR)
track:SetBlendMode("ADD")
track:Hide()
local danger = check:CreateTexture(nil, "ARTWORK")
danger:SetTexture(BAR)
danger:SetBlendMode("ADD")
danger:Hide()
local hint = check:CreateFontString(nil, "OVERLAY")
hint:SetFont(FONT, 13, "OUTLINE")
hint:SetTextColor(1, 0.95, 0.85)
hint:Hide()

-- The outcome's pieces, and its own frame so it plays on after the ring has gone.
local pop = Tex("check_perfect", "OVERLAY")
local core = Tex("check_critical", "OVERLAY")
local burst = Tex("ring_fire", "OVERLAY")
local crack = Tex("check_fail", "OVERLAY")
local tick = Tex("check_closer", "OVERLAY")
local fizzle = Tex("check_fizzle", "OVERLAY")
local beatcore = Tex("check_critical", "OVERLAY")
-- The clarity pieces (Corey's art, Nov 5): the critical's star and a big outcome's shockwave; the
-- ring frozen where the press came with the target dimly behind it, so early (outside), late
-- (inside) and on time read at a glance; and the input to use, beside the ring for the first few.
local star = Tex("fx_starburst", "OVERLAY")
local wave = Tex("fx_shockwave", "OVERLAY")
local ghost = Tex("check_closer", "OVERLAY")
local target = Tex("ring_fire", "ARTWORK")
local icon = check:CreateTexture(nil, "OVERLAY")
icon:SetBlendMode("ADD")
icon:Hide()
-- The shockwave's line, as a share of its drawn half (the art's ring, padded for turning).
local WAVE_LINE = 0.73
-- Only the release's mouse beside its ring; the key and the stop were clutter (Corey, Nov 5).
local ICONS = { release = "input_mouse" }
local ICON_FIRST = 10
local DULL = "Sound\\Interface\\iDeselectTarget.wav"
-- The meter (Cone of Cold, after Gladius's swing meter), in Corey's frost art: the ice arc over the
-- reticle, its good zone lit in blue along the channel, the sweet spot's ice gem pulsing on it, the
-- cracked overrun past its end, and the ice-shard cursor running the channel, pointing outward.
local METER_N, METER_OVER = 40, 0
local METER_R, METER_HALF = 156, 55
-- The frame's square, drawn so its circle's centre is the reticle (its channel `METER_R` out).
local METER_FRAME = 456
local meterDots = {}
for i = 1, METER_N do meterDots[i] = Tex("check_glow", "ARTWORK") end
local meterCursor = Tex("meter_cursor", "OVERLAY")
local meterSpot = Tex("meter_spot", "OVERLAY")
local meterOver = Tex("meter_over", "ARTWORK")
local meterFrame = check:CreateTexture(nil, "BORDER")
meterFrame:SetTexture(ART .. "meter_frame")
meterFrame:SetBlendMode("ADD")
meterFrame:SetPoint("CENTER", check, "CENTER", 0, 0)
meterFrame:Hide()

local ClearGroundAll, HidePlatesAll
local function HideRing()
  if ClearGroundAll then ClearGroundAll() end
  if HidePlatesAll then HidePlatesAll() end
  ring:Hide(); ring2:Hide(); halo:Hide(); spark:Hide(); hint:Hide(); icon:Hide()
  needle:Hide(); needleCore:Hide(); track:Hide(); danger:Hide()
  for i = 1, table.getn(marks) do marks[i]:Hide() end
  for i = 1, table.getn(trail) do trail[i]:Hide() end
  for i = 1, table.getn(closers) do closers[i]:Hide() end
  for i = 1, table.getn(rests) do rests[i]:Hide() end
  for i = 1, table.getn(pips) do pips[i]:Hide() end
  for i = 1, table.getn(meterDots) do meterDots[i]:Hide() end
  meterCursor:Hide(); meterSpot:Hide(); meterOver:Hide(); meterFrame:Hide()
end

-- --- the state -------------------------------------------------------------------------------------

-- cur: the check under way (nil: none). fx: the outcome playing (nil: none).
check.cur = nil
check.fx = nil
check.pressAt = 0

local function Saved()
  ElwynnSurvivalSettings = ElwynnSurvivalSettings or {}
  ElwynnSurvivalSettings.checkHints = ElwynnSurvivalSettings.checkHints or {}
  return ElwynnSurvivalSettings.checkHints
end

-- No words beside the checks (Corey, Nov 5: cluttered); HINTS is what the skills menu says.
local function HintFor(kind)
  return nil
end

local function Made(kind)
  local h = Saved()
  h[kind] = (h[kind] or 0) + 1
  h["icon_" .. kind] = (h["icon_" .. kind] or 0) + 1
end

-- The input's icon beside the ring for the first few of each kind.
local function IconFor(kind)
  return (Saved()["icon_" .. kind] or 0) < ICON_FIRST
end

-- How the closing ring travels: share of the way in, from the share of the time.
local function Curve(name, p)
  p = Clamp(p, 0, 1.6)
  if p > 1 then return p end
  if name == "ease" then return EaseOut(p) end
  if name == "surge" then return p * p end
  if name == "stutter" then
    if p < 0.45 then return p end
    if p < 0.62 then return 0.45 end
    return 0.45 + (p - 0.62) / 0.38 * 0.55
  end
  return p
end

-- --- outcomes --------------------------------------------------------------------------------------

-- Play an outcome where the ring was: "perfect", "good", "early", "late", "full", "fail", "fizzle";
-- `beat` for one beat of a rhythm (small, the ring stays).
local function Outcome(grade, school, beat)
  local sc = SCHOOL[school] or SCHOOL.arcane
  local now = GetTime()
  if beat then
    check.beatfx = { grade = grade, at = now, school = school }
    if grade == "perfect" then PlaySoundFile(CHIME) else PlaySoundFile(TICK) end
    return
  end
  check.fx = { grade = grade, at = now, school = school }
  burst:SetTexture(ART .. sc.ring)
  if grade == "perfect" or grade == "full" then
    PlaySoundFile(sc.sound); PlaySoundFile(CHIME)
    if ShakeCamera then ShakeCamera(0.5, 0.18) end
  elseif grade == "good" then
    PlaySoundFile(sc.sound)
  elseif grade == "late" then
    PlaySoundFile(TICK)
  elseif grade == "early" then
    PlaySoundFile(DULL)
  else
    PlaySoundFile(sc.fizzle)
  end
  check:Show()
end

-- Where a release's closing ring stood at the press (its radius): outside the line early, on it in
-- the window, inside it late.
local function PressRadius(c, now)
  local sc = SCHOOL[c.school]
  local line = sc.size * sc.line / 2
  local t = now - c.start
  if t > c.flare + c.window then
    local late = (t - c.flare - c.window) / 0.35
    return line * math.min(0.84, Clamp(1 - 0.35 * late, 0.4, 1))
  end
  if t >= c.flare - c.window then return line end
  local q = Curve(c.curve, t / math.max(0.05, c.flare))
  return line * math.max(1.12, 1 + (START_OUT - 1) * (1 - Clamp(q, 0, 1)))
end

-- The check is made (or lost): its ring goes at once, its outcome plays.
local function Finish(grade)
  local c = check.cur
  if not c then return end
  check.cur = nil
  HideRing()
  Made(c.kind)
  c.judged = grade
  check.last = { key = c.key, grade = grade, at = GetTime(), school = c.school }
  local ghostR = c.kind == "release" and not c.ground and PressRadius(c, GetTime())
  local meterAt = c.kind == "meter" and (GetTime() - c.start) / math.max(0.1, c.secs)
  Outcome(grade, c.school)
  if ghostR and check.fx then check.fx.ghost = ghostR end
  if meterAt and check.fx then check.fx.meter = c; check.fx.meterAt = meterAt end
end

-- --- the server's lines ----------------------------------------------------------------------------

-- CK key kind school a b crit pattern
function ES.CheckLine(f)
  local kind, school = f[3], f[4]
  local a, b, crit = tonumber(f[5]) or 0, tonumber(f[6]) or 0, tonumber(f[7]) or 0.3
  local now = GetTime()
  -- From the press that began it, as this side saw it (the server measures from the same press),
  -- else from now.
  local start = (now - (check.pressAt or 0) < 0.6) and check.pressAt or now
  local c = { key = f[2], kind = kind, school = SCHOOL[school] and school or "arcane", start = start, crit = crit }
  if kind == "release" then
    c.flare, c.window, c.curve = a, b, f[8] or "steady"
  elseif kind == "rhythm" then
    c.w0, c.w1 = a, b
    local _, _, free, name, list = string.find(f[8] or "", "^(%d+)|([^|]*)|(.*)$")
    c.free = tonumber(free) or 0
    c.pattern = name
    c.beats = {}
    for t in string.gfind(list or "", "[^,]+") do
      local rest = string.find(t, "r") ~= nil
      table.insert(c.beats, { t = tonumber((string.gsub(t, "r", ""))) or 0, rest = rest, done = false })
    end
    c.n = table.getn(c.beats)
    c.next = c.free + 1
  elseif kind == "sweep" then
    c.secs, c.sweep, c.perfect = a, b, crit
    local _, _, way, arc, list = string.find(f[8] or "", "^(%a+)|([^|]*)|(.*)$")
    c.dir = (way == "rl") and -1 or 1
    c.arc = tonumber(arc) or 30
    c.marks = {}
    for m in string.gfind(list or "", "[^,]+") do table.insert(c.marks, tonumber(m) or 0) end
  elseif kind == "meter" then
    c.secs, c.spot, c.perfect = a, b, crit
    local _, _, way, good = string.find(f[8] or "", "^(%a+)|(.*)$")
    c.dir = (way == "rl") and -1 or 1
    c.good = tonumber(good) or 0.16
  elseif kind == "orb" then
    c.secs, c.range, c.perfect = a, b, crit
    local _, _, back, ahead, list = string.find(f[8] or "", "^([^|]*)|([^|]*)|(.*)$")
    c.backfire, c.ahead = tonumber(back) or 3, tonumber(ahead) or 1
    c.marks = {}
    for m in string.gfind(list or "", "[^,]+") do table.insert(c.marks, tonumber(m) or 0) end
  else
    return
  end
  c.hint = HintFor(kind)
  c.icon = IconFor(kind)
  c.ground = SetGroundRing and GROUND[c.key]
  if c.ground and c.ground.at == "aim" and GetAimPoint then
    local x, y, z = GetAimPoint()
    c.gx, c.gy, c.gz = x, y, z
  end
  local sc = SCHOOL[c.school]
  ring:SetTexture(ART .. sc.ring)
  ring2:SetTexture(ART .. sc.ring)
  check.cur = c
  check:Show()
end

-- CR key grade: the server's word. The same as shown: nothing more. Else (a press the add-on
-- didn't see, a beat let pass, an all-or-nothing that fizzled) it plays now.
function ES.CheckResultLine(f)
  local key, grade = f[2], f[3]
  local c = check.cur
  -- Called off (a cancel, a stagger, a stun): the ring goes quietly, nothing judged.
  if grade == "cancel" then
    if c and c.key == key then
      check.cur = nil
      HideRing()
    end
    return
  end
  if c and c.key == key then
    if c.kind == "rhythm" then
      if grade == "good" or grade == "perfect" then
        -- A press this side already showed, else one it didn't see (the lab's).
        if not c.shownBeat then
          local k = c.next
          if c.beats[k] then c.beats[k].done = true; c.beats[k].grade = grade end
          c.next = k + 1
          while c.beats[c.next] and c.beats[c.next].rest do c.next = c.next + 1 end
          Outcome(grade, c.school, true)
        end
        c.shownBeat = nil
        return
      end
      Finish(grade)
      return
    end
    Finish(grade)
    return
  end
  local l = check.last
  if l and l.key == key and GetTime() - l.at < 0.8 then
    if l.grade == grade then return end
    -- The rhythm's chain made whole after its last beat: the big one. Or the server judged a
    -- meter differently: the meter stays frozen where it stopped (and goes when the outcome does).
    l.grade = grade
    local old = check.fx
    Outcome(grade, l.school)
    if old and check.fx then
      check.fx.meter, check.fx.meterAt, check.fx.ghost = old.meter, old.meterAt, old.ghost
    end
    return
  end
  -- No ring up: shown in its own school (the server's third field; arcane from an older one), or
  -- not on the reticle at all when it's shown on the bodies (`world`: Penance's rings).
  local school = f[4]
  if school == "world" then return end
  if not SCHOOL[school or ""] then school = "arcane" end
  check.last = { key = key, grade = grade, at = GetTime(), school = school }
  Outcome(grade, school)
end

-- --- the press, as this side sees it ---------------------------------------------------------------

local function JudgeRelease(c, now)
  local off = math.abs(now - c.start - c.flare)
  if off <= c.window * c.crit then return "perfect" end
  if off <= c.window then return "good" end
  if now - c.start < c.flare then return "early" end
  return "late"
end

local function BeatWindow(c, k)
  local span = math.max(1, c.n - c.free - 1)
  local t = Clamp((k - 1 - c.free) / span, 0, 1)
  return c.w0 + (c.w1 - c.w0) * t
end

local function JudgeBeat(c, now)
  local k = c.next
  local b = c.beats[k]
  if not b then return end
  local t = now - c.start
  if b.rest then
    -- Pressed on the rest (or before it): broken. Well after it: the press is for the next.
    if math.abs(t - b.t) <= BeatWindow(c, k) or t < b.t then return "fail" end
    k = k + 1
    b = c.beats[k]
    if not b then return end
  end
  local w = BeatWindow(c, k)
  local off = math.abs(t - b.t)
  if off > w then return "fail" end
  b.done = true
  b.grade = (off <= w * c.crit) and "perfect" or "good"
  c.next = k + 1
  while c.beats[c.next] and c.beats[c.next].rest do c.next = c.next + 1 end
  return b.grade
end

local Attack, Heavy, Ability = ES.Attack, ES.Heavy, ES.Ability
ES.Attack = function(state)
  local c = check.cur
  if state == "down" then
    check.pressAt = GetTime()
  elseif c and c.kind == "release" and ES.attackHeld then
    Finish(JudgeRelease(c, GetTime()))
  end
  return Attack(state)
end
ES.Heavy = function(state)
  if state == "down" then check.pressAt = GetTime() end
  return Heavy(state)
end
-- Right click calls a charge off: its ring goes at once, so letting go after judges nothing.
local Guard = ES.Guard
if Guard then
  ES.Guard = function(state)
    local c = check.cur
    if state == "down" and c and c.kind == "release" then
      check.cur = nil
      HideRing()
    end
    return Guard(state)
  end
end
-- Where a sweep's jet points now (degrees off the front, + left), and an orb's yards out.
local function JetAt(c, now)
  local share = Clamp((now - c.start) / math.max(0.1, c.secs), 0, 1)
  return c.dir * c.sweep / 2 - c.dir * c.sweep * share
end
local function OrbAt(c, now)
  return c.ahead + (c.range - c.ahead) * Clamp((now - c.start) / math.max(0.1, c.secs), 0, 1)
end
local function MeterAt(c, now)
  return (now - c.start) / math.max(0.1, c.secs)
end
local function JudgeStop(c, now)
  if c.kind == "meter" then
    local s = MeterAt(c, now)
    local off = math.abs(s - c.spot)
    if off <= c.perfect then return "perfect" end
    if off <= c.good then return "good" end
    if s < c.spot then return "early" end
    return "late"
  end
  if c.kind == "sweep" then
    local jet, best = JetAt(c, now), 999
    for i = 1, table.getn(c.marks) do best = math.min(best, math.abs(c.marks[i] - jet)) end
    if best <= c.perfect then return "perfect" end
    if best <= c.arc / 2 then return "good" end
    return "fail"
  end
  local at, best = OrbAt(c, now), 999
  if at <= c.backfire then return "fail" end
  for i = 1, table.getn(c.marks) do best = math.min(best, math.abs(c.marks[i] - at)) end
  if best <= c.perfect then return "perfect" end
  return "good"
end

ES.Ability = function(n, ex, state)
  local c = check.cur
  if state ~= "up" and c and (c.kind == "sweep" or c.kind == "orb" or c.kind == "meter") and GetTime() - c.start > 0.08 then
    Finish(JudgeStop(c, GetTime()))
    return Ability(n, ex, state)
  end
  if state ~= "up" and c and c.kind == "rhythm" and not ES.exHold then
    local g = JudgeBeat(c, GetTime())
    if g == "fail" then
      Finish("fail")
    elseif g then
      c.shownBeat = true
      Outcome(g, c.school, true)
    end
  end
  return Ability(n, ex, state)
end

-- --- the frame, each frame -------------------------------------------------------------------------

local function DrawRing(c, now, open, inCrit)
  local sc = SCHOOL[c.school]
  local t = now
  -- A sweep's dial is wider: its marks sit round the ring's line.
  local size = (c.kind == "sweep") and sc.size * 1.45 or sc.size
  local alpha, a2 = 0.92, 0.42
  local scale = 1
  if c.school == "fire" then
    -- Burning: a fast, uneven flicker and a little heave.
    local fl = 0.5 * math.sin(t * 13.1) + 0.3 * math.sin(t * 23.7 + 1.3) + 0.2 * math.sin(t * 37.3 + 2.1)
    alpha = 0.84 + 0.14 * fl
    a2 = 0.38 + 0.2 * (0.5 + 0.5 * math.sin(t * 9.3 + 0.7))
    scale = 1 + 0.012 * math.sin(t * 17.0)
  elseif c.school == "frost" then
    -- Shifting: a slow shimmer, two layers drifting against each other.
    alpha = 0.86 + 0.1 * math.sin(t * 2.3)
    a2 = 0.3 + 0.22 * (0.5 + 0.5 * math.sin(t * 1.4 + 2.0))
    scale = 1 + 0.008 * math.sin(t * 1.9)
  else
    -- Arcane: the runes turning, a steady breathing glow.
    alpha = 0.88 + 0.1 * math.sin(t * 4.2)
    a2 = 0.32 + 0.18 * (0.5 + 0.5 * math.sin(t * 4.2 + 1.6))
    scale = 1 + 0.022 * math.sin(t * 4.2)
  end
  if open then
    alpha = 1
    a2 = a2 + 0.35
    scale = scale * 1.05
  end
  local rt = sc.ringTint or WHITE3
  Draw(ring, size * scale, t * sc.spin, alpha, rt[1], rt[2], rt[3])
  Draw(ring2, size * scale * 1.025, t * sc.spin2 + 1.1, a2, sc.glow[1], sc.glow[2], sc.glow[3])
  if open then
    Draw(halo, size * 1.35, 0, inCrit and 0.55 or 0.32, sc.tint[1], sc.tint[2], sc.tint[3])
  else
    halo:Hide()
  end
  return size * scale * sc.line / 2
end

-- A closing ring at `r` (pixels, its line's radius), `alpha`, in `col`.
local function DrawCloser(t, r, alpha, col, spinAt)
  local size = 2 * r / CLOSER_LINE
  Draw(t, size, spinAt, alpha, col[1], col[2], col[3])
end

-- A ring on the ground: the school's (turning, flickering as its school does) and the ring of light
-- closing on it, `q` of the way in (0 out wide, 1 on the line); `open` brightens it.
local function GroundRing(key, c, tex, radius, rgba, yaw)
  local tpath = ART .. tex
  if c.ground.at == "feet" then
    SetGroundRingOnUnit(key, "player", radius, tpath, rgba[1], rgba[2], rgba[3], rgba[4], yaw)
  else
    -- Meteor's circle follows the reticle while it's held; the school's ring takes the place of
    -- the plain aim circle (left in place, the two drew over each other).
    if SetAimMarker and ES.aimMarker then SetAimMarker(nil) end
    if GetAimPoint then
      local x, y, z = GetAimPoint()
      if x then c.gx, c.gy, c.gz = x, y, z end
    end
    if c.gx then SetGroundRing(key, c.gx, c.gy, c.gz, radius, tpath, rgba[1], rgba[2], rgba[3], rgba[4], yaw) end
  end
end

local function DrawGround(c, now, q, open, closerCol, grow)
  local g = GROUND_TEX[c.school] or GROUND_TEX.arcane
  local sc = SCHOOL[c.school]
  local radius = c.ground.radius + (grow or 0)
  local a = 0.9
  if c.school == "fire" then
    -- A little quieter on the ground than on the reticle: it spans the view at Meteor's size.
    a = 0.7 + 0.14 * (0.5 * math.sin(now * 13.1) + 0.3 * math.sin(now * 23.7 + 1.3) + 0.2 * math.sin(now * 37.3))
  elseif c.school == "frost" then
    a = 0.85 + 0.12 * math.sin(now * 2.3)
  else
    a = 0.86 + 0.12 * math.sin(now * 4.2)
  end
  if open then a = 1 end
  local line = radius * g.line
  GroundRing("es_check_ring", c, g.tex, radius * (open and 1.04 or 1), { 1, 1, 1, a }, now * sc.spin)
  GroundRing("es_check_ring2", c, g.tex, radius * 1.03, { sc.glow[1], sc.glow[2], sc.glow[3], open and 0.7 or 0.32 }, now * sc.spin2 + 1.1)
  if q then
    local L = line * (1 + (GROUND_OUT - 1) * (1 - Clamp(q, 0, 1)))
    local col = closerCol or sc.tint
    GroundRing("es_check_closer", c, "check_closer", L / GROUND_CLOSER, { col[1], col[2], col[3], 0.4 + 0.6 * Clamp(q, 0, 1) }, 0)
  else
    ClearGroundRing("es_check_closer")
  end
end

local function ClearGround()
  if ClearGroundRing then
    ClearGroundRing("es_check_ring")
    ClearGroundRing("es_check_ring2")
    ClearGroundRing("es_check_closer")
  end
end
ClearGroundAll = ClearGround

-- The foes' nameplates now (the client's anonymous WorldFrame children with a plate's border), with
-- their screen centres.
local PLATE_BORDER = "Interface\\Tooltips\\Nameplate-Border"
local function Plates()
  local out = {}
  if not WorldFrame or not WorldFrame.GetChildren then return out end
  local kids = { WorldFrame:GetChildren() }
  for i = 1, table.getn(kids) do
    local f = kids[i]
    if f and f.GetRegions and not f:GetName() and f:IsShown() then
      local border = f:GetRegions()
      if border and border.GetTexture and border:GetTexture() == PLATE_BORDER then
        local x, y = f:GetCenter()
        if x then table.insert(out, { f = f, x = x, y = y }) end
      end
    end
  end
  return out
end

-- The frost sigils over the foes: each mark matched to a plate (a sweep's by bearing, left to
-- right; an orb's by distance, near to far), brightening and swelling as the frost reaches it.
local plateSigils = {}
for i = 1, 6 do
  local t = WorldFrame:CreateTexture(nil, "OVERLAY")
  t:SetTexture(ART .. "sigil_frost")
  t:SetBlendMode("ADD")
  t:Hide()
  plateSigils[i] = t
end
local function HidePlateSigils()
  for i = 1, table.getn(plateSigils) do plateSigils[i]:Hide() end
end
HidePlatesAll = HidePlateSigils

local function DrawPlateSigils(c, nears)
  local plates = Plates()
  local n = table.getn(c.marks)
  if n == 0 or table.getn(plates) ~= n then
    HidePlateSigils()
    return false
  end
  -- Order the marks and the plates the same way.
  local order = {}
  for i = 1, n do order[i] = i end
  if c.kind == "sweep" then
    table.sort(order, function(a, b) return c.marks[a] > c.marks[b] end)
    table.sort(plates, function(a, b) return a.x < b.x end)
  else
    table.sort(order, function(a, b) return c.marks[a] < c.marks[b] end)
    table.sort(plates, function(a, b) return a.y < b.y end)
  end
  local now = GetTime()
  for k = 1, table.getn(plateSigils) do
    local t = plateSigils[k]
    local pl, mi = plates[k], order[k]
    if pl and mi then
      local near = nears[mi] or 0
      local size = 34 + 26 * near
      local cs, sn = math.cos(now * 0.6) * H, math.sin(now * 0.6) * H
      t:SetTexCoord(0.5 - cs + sn, 0.5 - sn - cs, 0.5 - cs - sn, 0.5 - sn + cs, 0.5 + cs + sn, 0.5 + sn - cs, 0.5 + cs - sn, 0.5 + sn + cs)
      t:SetWidth(size)
      t:SetHeight(size)
      t:ClearAllPoints()
      t:SetPoint("BOTTOM", pl.f, "TOP", 0, 2 + 6 * near)
      t:SetVertexColor(1, 1, 1, 0.55 + 0.45 * near)
      t:Show()
    else
      t:Hide()
    end
  end
  return true
end

local StepPips

local function StepRelease(c, now)
  local t = now - c.start
  local open = math.abs(t - c.flare) <= c.window
  local inCrit = math.abs(t - c.flare) <= c.window * c.crit
  local sc = SCHOOL[c.school]
  local q = Curve(c.curve, t / math.max(0.05, c.flare))
  if c.ground then
    if t > c.flare + c.window then
      DrawGround(c, now, nil, false)
    else
      DrawGround(c, now, Clamp(q, 0, 1), open, open and { 1, 1, 1 } or sc.tint)
    end
    ring:Hide(); ring2:Hide(); halo:Hide(); closers[1]:Hide()
    local near = Clamp(1 - math.abs(t - c.flare) / (c.window * 3), 0, 1)
    local blaze = inCrit and 1 or near * 0.6
    Draw(spark, 22 + 30 * blaze, now * 1.5, 0.35 + 0.65 * blaze, 1, 1, 1, 0, 0)
    if c.hint then
      hint:SetText(c.hint)
      hint:ClearAllPoints()
      hint:SetPoint("TOP", check, "CENTER", 0, -40)
      hint:Show()
    end
    return
  end
  local line = DrawRing(c, now, open, inCrit)
  if t > c.flare + c.window then
    -- Past its moment: the ring sinks inside and fades; the school ring dims.
    local late = (t - c.flare - c.window) / 0.35
    DrawCloser(closers[1], line * Clamp(1 - 0.35 * late, 0.4, 1), 0.7 * (1 - late), { 0.6, 0.6, 0.6 }, 0)
    ring:SetVertexColor(0.55, 0.55, 0.6, 0.6)
  else
    local r = line * (1 + (START_OUT - 1) * (1 - Clamp(q, 0, 1)))
    DrawCloser(closers[1], r, 0.35 + 0.65 * Clamp(q, 0, 1), open and { 1, 1, 1 } or sc.tint, 0)
  end
  -- The critical's spot: a star on the ring's line at the top, blazing in the very middle.
  local near = Clamp(1 - math.abs(t - c.flare) / (c.window * 3), 0, 1)
  local blaze = inCrit and 1 or near * 0.6
  Draw(spark, 22 + 30 * blaze, now * 1.5, 0.35 + 0.65 * blaze, 1, 1, 1, 0, line)
  if c.hint then
    hint:SetText(c.hint)
    hint:ClearAllPoints()
    hint:SetPoint("TOP", check, "CENTER", 0, -line - 34)
    hint:Show()
  end
end

local function StepRhythm(c, now)
  local t = now - c.start
  local sc = SCHOOL[c.school]
  -- Open: the next pressed beat's window.
  local nb = c.beats[c.next]
  local open = nb and not nb.rest and math.abs(t - nb.t) <= BeatWindow(c, c.next)
  if c.ground then
    -- The ring round the feet grows a yard a beat, as the bursts do; the next beat's ring of light
    -- closes on it.
    local done = 0
    for k = 1, c.n do if c.beats[k].done then done = done + 1 end end
    local q
    if nb then q = 1 - Clamp((nb.t - t) / LEAD, 0, 1) end
    DrawGround(c, now, q, open, nil, (c.ground.grow or 0) * math.max(0, done - 1))
    ring:Hide(); ring2:Hide(); halo:Hide()
    for i = 1, table.getn(closers) do closers[i]:Hide() end
    for k = 1, c.n do
      local b = c.beats[k]
      if k <= c.free and not b.done and t >= b.t then
        b.done = true
        b.grade = "free"
        check.beatfx = { grade = "free", at = now, school = c.school }
        PlaySoundFile(TICK)
      end
    end
    StepPips(c, now, 40)
    if c.hint then
      hint:SetText(c.hint)
      hint:ClearAllPoints()
      hint:SetPoint("TOP", check, "CENTER", 0, -80)
      hint:Show()
    end
    return
  end
  local line = DrawRing(c, now, open, false)
  -- The closing rings: each beat's, from LEAD before it to its moment.
  local used, usedRest = 0, 0
  for k = 1, c.n do
    local b = c.beats[k]
    local until_ = b.t - t
    if not b.done and until_ <= LEAD and until_ > -0.12 then
      local q = 1 - Clamp(until_ / LEAD, 0, 1)
      if b.rest then
        usedRest = usedRest + 1
        if rests[usedRest] then
          -- A rest: the hollow diamond swells in the middle and dims as it passes.
          Draw(rests[usedRest], 40 + 50 * q, 0, 0.35 + 0.6 * q, 1, 1, 1)
        end
      else
        used = used + 1
        if closers[used] then
          local free = k <= c.free
          local col = free and { sc.tint[1] * 0.7, sc.tint[2] * 0.7, sc.tint[3] * 0.7 } or sc.tint
          DrawCloser(closers[used], line * (1 + (START_OUT - 1) * (1 - q)), (free and 0.45 or 0.55) + 0.45 * q, col, 0)
        end
      end
    end
    -- A free beat goes by itself as it falls.
    if k <= c.free and not b.done and t >= b.t then
      b.done = true
      b.grade = "free"
      check.beatfx = { grade = "free", at = now, school = c.school }
      PlaySoundFile(TICK)
    end
  end
  for i = used + 1, table.getn(closers) do closers[i]:Hide() end
  for i = usedRest + 1, table.getn(rests) do rests[i]:Hide() end
  StepPips(c, now, line + 30)
  if c.hint then
    hint:SetText(c.hint)
    hint:ClearAllPoints()
    hint:SetPoint("TOP", check, "CENTER", 0, -line - 54)
    hint:Show()
  end
end

-- The pips: one a beat, `below` pixels under the reticle; lit as they go, the next one breathing.
StepPips = function(c, now, below)
  local gap = 30
  local x0 = -(c.n - 1) * gap / 2
  for k = 1, table.getn(pips) do
    local p = pips[k]
    local b = c.beats[k]
    if b then
      if b.rest then p:SetTexture(ART .. "pip_rest") else p:SetTexture(ART .. "pip_arcane") end
      local size, alpha, r, g, bl = 28, 0.3, 0.75, 0.6, 0.8
      if b.done then
        if b.grade == "perfect" then
          size, alpha, r, g, bl = 38, 1, 1, 1, 1
        elseif b.rest then
          size, alpha = 28, 0.5
        else
          size, alpha, r, g, bl = 34, 0.95, 1, 1, 1
        end
        -- Lit just now: it pops.
        local since = now - (b.litAt or now)
        if not b.litAt then b.litAt = now; since = 0 end
        size = size * (1 + 0.6 * Clamp(1 - since / 0.25, 0, 1))
      elseif k == c.next then
        local beat = 0.5 + 0.5 * math.sin(now * 9)
        size, alpha = 30 + 5 * beat, 0.55 + 0.35 * beat
      end
      Draw(p, size, 0, alpha, r, g, bl, x0 + (k - 1) * gap, -below)
    else
      p:Hide()
    end
  end
end

-- A mark on the dial or the track: a frost sigil, brightening and swelling as the light nears it.
local function DrawMark(t, x, y, near)
  local size = 30 + 16 * near
  Draw(t, size, GetTime() * 0.4, 0.55 + 0.45 * near, 1, 1, 1, x, y)
end

local function StepSweep(c, now)
  local line = DrawRing(c, now, false, false)
  local jet = JetAt(c, now)
  local best = 999
  local nears = {}
  for i = 1, table.getn(c.marks) do
    local d = math.abs(c.marks[i] - jet)
    nears[i] = (d <= c.perfect) and 1 or Clamp(1 - d / (c.arc / 2), 0, 1)
  end
  DrawPlateSigils(c, nears)
  for i = 1, table.getn(marks) do
    local m = c.marks[i]
    if m then
      local phi = math.rad(m * DIAL_SPREAD)
      local d = math.abs(m - jet)
      best = math.min(best, d)
      local near = Clamp(1 - d / (c.arc / 2), 0, 1)
      if d <= c.perfect then near = 1 end
      DrawMark(marks[i], -line * math.sin(phi), line * math.cos(phi), near)
    else
      marks[i]:Hide()
    end
  end
  -- The needle: the jet's light on the dial, its trail fading behind it.
  local sc = SCHOOL.frost
  local hot = best <= c.perfect
  for k = 1, table.getn(trail) do
    local back = jet + c.dir * k * 4
    local phi = math.rad(back * DIAL_SPREAD)
    Draw(trail[k], 34 - 4 * k, 0, 0.5 - 0.1 * k, sc.tint[1], sc.tint[2], sc.tint[3], -line * math.sin(phi), line * math.cos(phi))
  end
  local phi = math.rad(jet * DIAL_SPREAD)
  local x, y = -line * math.sin(phi), line * math.cos(phi)
  Draw(needle, hot and 58 or 44, 0, 1, hot and 1 or sc.tint[1], hot and 1 or sc.tint[2], 1, x, y)
  Draw(needleCore, hot and 40 or 26, now * 3, hot and 1 or 0.7, 1, 1, 1, x, y)
  if c.hint then
    hint:SetText(c.hint)
    hint:ClearAllPoints()
    hint:SetPoint("TOP", check, "CENTER", 0, -line - 30)
    hint:Show()
  end
end

-- The orb's track: from the foot (the caster) up through the reticle to its reach.
local TRACK_FOOT, TRACK_TOP = -120, 150
local function TrackY(c, yd) return TRACK_FOOT + (TRACK_TOP - TRACK_FOOT) * Clamp(yd / math.max(1, c.range), 0, 1) end

-- The meter at share `at` of its run (nil: no cursor), faded by `fade`.
local function MeterXY(c, sh, r)
  local ang = math.rad((-METER_HALF + 2 * METER_HALF * sh) * c.dir)
  return (r or METER_R) * math.sin(ang), (r or METER_R) * math.cos(ang), ang
end
local function DrawMeter(c, at, fade)
  local now = GetTime()
  meterFrame:SetWidth(METER_FRAME)
  meterFrame:SetHeight(METER_FRAME)
  meterFrame:SetVertexColor(1, 1, 1, 0.95 * fade)
  meterFrame:Show()
  -- The good zone lit along the channel.
  for i = 1, METER_N do
    local t = meterDots[i]
    local sh = (i - 0.5) / METER_N
    if math.abs(sh - c.spot) <= c.good then
      local x, y = MeterXY(c, sh)
      Draw(t, 22, 0, 0.95 * fade, 0.45, 0.8, 1, x, y)
    else
      t:Hide()
    end
  end
  -- The sweet spot's gem, breathing.
  local sx, sy = MeterXY(c, c.spot)
  Draw(meterSpot, 52 * (1 + 0.1 * math.sin(now * 6)), 0, fade, 1, 1, 1, sx, sy)
  -- The overrun past the end, along the arc.
  local ox, oy, oang = MeterXY(c, 1.1)
  Draw(meterOver, 96, oang, 0.9 * fade, 1, 1, 1, ox, oy)
  if at then
    local x, y, ang = MeterXY(c, Clamp(at, 0, 1.12))
    Draw(meterCursor, 100, ang, fade, 1, 1, 1, x, y)
    -- A glow under the shard, so it reads over the jet.
    Draw(halo, 60, 0, 0.6 * fade, 0.7, 0.9, 1, x, y)
  else
    meterCursor:Hide()
  end
end

local function StepMeter(c, now)
  DrawMeter(c, MeterAt(c, now), 1)
end

local function StepOrb(c, now)
  local sc = SCHOOL.frost
  local len = TRACK_TOP - TRACK_FOOT
  track:SetWidth(5)
  track:SetHeight(len)
  track:ClearAllPoints()
  track:SetPoint("BOTTOM", check, "CENTER", 0, TRACK_FOOT)
  track:SetVertexColor(sc.tint[1], sc.tint[2], sc.tint[3], 0.55 + 0.15 * math.sin(now * 2.3))
  track:Show()
  local dl = TrackY(c, c.backfire) - TRACK_FOOT
  danger:SetWidth(9)
  danger:SetHeight(math.max(2, dl))
  danger:ClearAllPoints()
  danger:SetPoint("BOTTOM", check, "CENTER", 0, TRACK_FOOT)
  danger:SetVertexColor(1, 0.2, 0.15, 0.7 + 0.25 * math.sin(now * 8))
  danger:Show()
  local at = OrbAt(c, now)
  local nears = {}
  for i = 1, table.getn(c.marks) do
    local d = math.abs(c.marks[i] - at)
    nears[i] = (d <= c.perfect) and 1 or Clamp(1 - d / 3, 0, 1)
  end
  DrawPlateSigils(c, nears)
  for i = 1, table.getn(marks) do
    local m = c.marks[i]
    if m then
      local near = Clamp(1 - math.abs(m - at) / 3, 0, 1)
      if math.abs(m - at) <= c.perfect then near = 1 end
      DrawMark(marks[i], 0, TrackY(c, m), near)
    else
      marks[i]:Hide()
    end
  end
  local y = TrackY(c, at)
  for k = 1, table.getn(trail) do
    Draw(trail[k], 34 - 5 * k, 0, 0.5 - 0.1 * k, sc.tint[1], sc.tint[2], sc.tint[3], 0, TrackY(c, at - k * 0.6))
  end
  -- The orb: a frost star spinning in a ball of light.
  Draw(needle, 60, 0, 1, sc.tint[1], sc.tint[2], 1, 0, y)
  Draw(needleCore, 40, now * 5, 1, 1, 1, 1, 0, y)
  ring:Hide(); ring2:Hide(); halo:Hide()
  if c.hint then
    hint:SetText(c.hint)
    hint:ClearAllPoints()
    hint:SetPoint("TOP", check, "CENTER", 0, TRACK_FOOT - 14)
    hint:Show()
  end
end

-- A beat's own flash: a tick of light round the ring (a star for a critical), the ring staying.
local function StepBeatFx(now)
  local f = check.beatfx
  if not f then return end
  local sc = SCHOOL[f.school] or SCHOOL.arcane
  local t = (now - f.at) / 0.28
  if t >= 1 then
    check.beatfx = nil
    tick:Hide()
    beatcore:Hide()
    return
  end
  local line = sc.size * sc.line / 2
  local col = (f.grade == "free") and { sc.tint[1] * 0.8, sc.tint[2] * 0.8, sc.tint[3] * 0.8 } or { 1, 0.9, 1 }
  DrawCloser(tick, line * (1 + 0.35 * EaseOut(t)), (f.grade == "free" and 0.6 or 1) * (1 - t), col, 0)
  if f.grade == "perfect" then
    Draw(beatcore, 40 + 120 * EaseOut(t), t * 1.2, 1 - t, 1, 1, 1)
  else
    beatcore:Hide()
  end
end

-- The outcome, playing out where the ring was.
local function StepFx(now)
  local f = check.fx
  if not f then return end
  local sc = SCHOOL[f.school] or SCHOOL.arcane
  local g = f.grade
  local age = now - f.at
  local line = sc.size * sc.line / 2
  local dur = ((g == "perfect" or g == "full") and 0.7) or (g == "good" and 0.55) or ((g == "fail" or g == "fizzle") and 0.7) or (f.ghost and 0.6) or 0.3
  local t = age / dur
  if f.meter then
    -- The meter frozen where the press came, fading: how close it was to the sweet spot (a fail's
    -- cursor is in the overrun, drawn below).
    if t < 1 then DrawMeter(f.meter, g ~= "fail" and f.meterAt or nil, 1 - t) else
      for i = 1, table.getn(meterDots) do meterDots[i]:Hide() end
      meterCursor:Hide(); meterSpot:Hide(); meterOver:Hide(); meterFrame:Hide()
    end
  end
  if t >= 1 then
    check.fx = nil
    pop:Hide(); core:Hide(); burst:Hide(); crack:Hide(); tick:Hide(); fizzle:Hide()
    star:Hide(); wave:Hide(); ghost:Hide(); target:Hide()
    return
  end
  local e = EaseOut(t)
  if g == "perfect" or g == "full" or g == "good" then
    -- The school ring bursts outward and away; the pop flares white-gold (tinted to the school
    -- for a perfect); a critical's star blazes over it.
    Draw(burst, sc.size * (1 + 0.75 * e), age * 1.4, (1 - t) * 0.95, 1, 1, 1)
    local top = (g == "perfect" or g == "full")
    local tint = top and { 1, 1, 0.92 } or { 0.5 + 0.5 * sc.glow[1], 0.5 + 0.5 * sc.glow[2], 0.5 + 0.5 * sc.glow[3] }
    Draw(pop, (top and 300 or 240) * (0.55 + 0.75 * e), age * 0.9, (1 - t) * (1 - t) * 1.0, tint[1], tint[2], tint[3])
    if top then
      Draw(core, 60 + 260 * e, -age * 2.2, (1 - t), 1, 1, 1)
      -- The critical's star (Corey's): blazing over it all, turning as it fades.
      Draw(star, 110 + 320 * e, age * 0.7, (1 - t) * (1 - t), 1, 0.95, 0.8)
    else
      core:Hide(); star:Hide()
    end
    -- The shockwave out from the ring's line: white-gold for a critical, the school's for a perfect.
    local wc = top and { 1, 0.9, 0.65 } or sc.tint
    Draw(wave, (2 * line / WAVE_LINE) * (1 + 1.2 * e), 0, (1 - t) * 0.95, wc[1], wc[2], wc[3])
    ghost:Hide(); target:Hide()
    crack:Hide(); fizzle:Hide(); tick:Hide()
  elseif g == "late" or g == "early" then
    -- A tick of light: the ring's line pulsing out once, the school's colour (grey, early).
    local col = (g == "late") and sc.tint or { 0.62, 0.62, 0.66 }
    if f.ghost then
      -- A release off its moment: the target dimly where it was, and the closing ring frozen where
      -- the press came, outside it (early, a cold grey) or inside it (late, amber).
      col = (g == "late") and { 1.0, 0.68, 0.25 } or { 0.62, 0.7, 0.88 }
      target:SetTexture(ART .. sc.ring)
      Draw(target, sc.size, 0, (1 - t) * 0.5, 1, 1, 1)
      DrawCloser(ghost, f.ghost, (1 - t * t) * 0.95, col, 0)
      tick:Hide()
    else
      DrawCloser(tick, line * (1 + 0.45 * e), (1 - t) * 0.95, col, 0)
      ghost:Hide(); target:Hide()
    end
    pop:Hide(); core:Hide(); burst:Hide(); crack:Hide(); fizzle:Hide(); star:Hide(); wave:Hide()
  elseif f.meter then
    -- The meter's own failure (Corey: not the ring's red circle): the cursor stuck in the cracked
    -- overrun, which flares red and shakes as the meter fades.
    local shake = (1 - t) * 6 * math.sin(age * 60)
    local ox, oy, oang = MeterXY(f.meter, 1.1)
    Draw(meterOver, 96 * (1 + 0.25 * (1 - t)), oang, 1 - t * t, 1, 0.35 + 0.3 * t, 0.3 + 0.3 * t, ox + shake, oy)
    local cx, cy, cang = MeterXY(f.meter, 1.1)
    Draw(meterCursor, 100, cang, 1 - t, 1, 0.5, 0.5, cx + shake, cy)
    pop:Hide(); core:Hide(); burst:Hide(); tick:Hide(); star:Hide(); wave:Hide(); ghost:Hide(); target:Hide(); crack:Hide(); fizzle:Hide()
  else
    -- Failed: the red crack bursts apart with a jolt; a fizzle puffs out of the middle and dies.
    local shake = (1 - t) * 5 * math.sin(age * 70)
    Draw(crack, sc.size * (1.05 + 0.3 * e), 0, (1 - t) * (g == "fizzle" and 1 or 0.9), 1, 0.35, 0.3, shake, 0)
    local frame = math.min(11, math.floor(t * 12))
    local col, row = math.mod(frame, 4), math.floor(frame / 4)
    fizzle:SetTexCoord(col / 4, (col + 1) / 4, row / 4, (row + 1) / 4)
    local fs = (g == "fizzle") and 210 or 150
    fizzle:SetWidth(fs)
    fizzle:SetHeight(fs)
    fizzle:ClearAllPoints()
    fizzle:SetPoint("CENTER", check, "CENTER", 0, 0)
    fizzle:SetVertexColor(0.85, 0.8, 0.95, 1)
    fizzle:Show()
    pop:Hide(); core:Hide(); burst:Hide(); tick:Hide(); star:Hide(); wave:Hide(); ghost:Hide(); target:Hide()
  end
end

-- The input beside the ring (the first few of each kind): the mouse for a release, its button
-- brightening as the moment nears; a key for a rhythm, tapping on each beat; a stop for a sweep or
-- an orb, breathing.
local function DrawIcon(c, now)
  local sc = SCHOOL[c.school]
  local line = sc.size * sc.line / 2
  local t = now - c.start
  local x, y, size, k = 0, -78, 30, 0.6
  if (c.kind == "release" or c.kind == "rhythm") and not c.ground then
    x, y = line + 34, 0
  elseif c.ground then
    x, y = 70, -50
  end
  if c.kind == "release" then
    k = 0.4 + 0.6 * Clamp(1 - math.abs(t - c.flare) / math.max(0.25, c.flare * 0.6), 0, 1)
  elseif c.kind == "rhythm" then
    local nb = c.beats[c.next]
    if nb and not nb.rest then
      local d = math.abs(t - nb.t)
      if d < 0.14 then
        size = size * (1 + 0.3 * (1 - d / 0.14))
        k = 1
      end
    end
  else
    k = 0.55 + 0.35 * (0.5 + 0.5 * math.sin(now * 6))
  end
  icon:SetTexture(ART .. ICONS[c.kind])
  icon:SetWidth(size)
  icon:SetHeight(size)
  icon:ClearAllPoints()
  icon:SetPoint("CENTER", check, "CENTER", x, y)
  icon:SetVertexColor(1, 1, 1, k)
  icon:Show()
end

check:SetScript("OnUpdate", function()
  local now = GetTime()
  local c = check.cur
  if c then
    if c.kind == "release" then
      StepRelease(c, now)
      -- Long past its moment with no word: let it go quietly.
      if now - c.start > c.flare + 3 then check.cur = nil; HideRing() end
    elseif c.kind == "rhythm" then
      StepRhythm(c, now)
      if c.beats[c.n] and now - c.start > c.beats[c.n].t + 2 then check.cur = nil; HideRing() end
    elseif c.kind == "sweep" then
      StepSweep(c, now)
      -- Let run the whole way: nothing stopped, it goes quietly.
      if now - c.start > c.secs + 0.15 then check.cur = nil; HideRing() end
    elseif c.kind == "orb" then
      StepOrb(c, now)
      if now - c.start > c.secs + 0.15 then check.cur = nil; HideRing() end
    elseif c.kind == "meter" then
      StepMeter(c, now)
      -- Into the overrun with no word: the server's fail comes; quietly gone if it doesn't.
      if now - c.start > c.secs * 1.08 + 0.6 then check.cur = nil; HideRing() end
    end
  end
  if c and check.cur == c and c.icon and ICONS[c.kind] then DrawIcon(c, now) else icon:Hide() end
  StepBeatFx(now)
  StepFx(now)
  if not check.cur and not check.fx and not check.beatfx then this:Hide() end
end)

-- OB key x y z x2 y2 z2 secs model scale: an orb set rolling (Frozen Orb), drawn rolling and
-- spinning flat on a client with PlayGroundOrb (patch 0043); OX key: it stopped or was spent.
function ES.OrbLine(f)
  if not PlayGroundOrb then return end
  local n = function(i) return tonumber(f[i]) or 0 end
  PlayGroundOrb("es_orb_" .. (f[2] or ""), f[10] or "", n(3), n(4), n(5), n(6), n(7), n(8), n(9), tonumber(f[11]) or 1, 240)
end
function ES.OrbGoneLine(f)
  if StopGroundOrb then StopGroundOrb("es_orb_" .. (f[2] or "")) end
end

-- A lab run starts fresh: nothing left over from the last.
function ES.CheckClear()
  check.cur = nil
  check.fx = nil
  check.beatfx = nil
  HideRing()
  pop:Hide(); core:Hide(); burst:Hide(); crack:Hide(); tick:Hide(); fizzle:Hide(); beatcore:Hide()
end
