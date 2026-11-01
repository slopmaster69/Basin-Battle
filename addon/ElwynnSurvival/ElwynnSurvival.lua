-- Elwynn Survival: the controls, the combat HUD, the ability keys and their board, the camp's
-- windows, building, banners, and the way back to what you dropped.
--
-- The controls: the mouse always steers in action mode (the view turns the hero, as in a
-- shooter); left mouse attacks (Shift + left: a heavy attack), right mouse raises the guard (a
-- parry just as a blow lands), mouse button 4 dodges (hold it to sprint; Shift sprints too). Five ability
-- keys: Q ranged, E close, 1 utility, 2 defense (each held a moment, or with Ctrl: its charged-up
-- EX version, for Valor) and 3 the ultimate (held a moment to unleash: the whole Valor bar). An
-- aimed one comes up on its key (the camera over the shoulder, the reticle in the middle), left
-- mouse casts it (held: charges it), right mouse or its key again calls it off. F uses what's in
-- front of you, Space jumps, R sheathes, T uses the quick item (G the next), X walks, B the bags, H
-- builds, N the board of abilities, J the journal, V the health bars over heads, Tab locks on (Tab
-- again lets go, Ctrl+Tab the next foe), Z frees the mouse. Every key can be changed on the
-- Controls card (K), mouse buttons and modifiers too, and the mouse speed set there.
--
-- The HUD: the dock bottom centre (health, stamina and Valor as wide as the five ability keys
-- under them; a pale chunk shows what a blow just took), the equipment cross on the left above
-- the chat,
-- the bar of the foe you're trading blows with, the reticle while aiming, a hit marker, the
-- crowd control on you, what F would do, healing in green, a red edge when you're hurt, the
-- kill feed and the camp top right, and icon buttons bottom right.
--
-- The mouse, two ways:
-- - On the action-combat client (benilla with this folder's patch: `HasActionCombat()`), the left
--   button is this add-on's alone (`SetActionMouse`): its binding swings or casts and it never
--   drags the camera. Action mode holds the game's own mouse-look (`MouselookStart`), let go
--   while a window is open. Aiming asks for the over-the-shoulder camera (`SetAimCamera`), and
--   the aim's ground marker (`SetAimMarker`) where the client has it.
-- - On a stock benilla, a click-catcher lies over the world (under every other window): a left
--   press swings, a right press guards and looks around. Z lifts it.
--
-- It sleeps until an Elwynn Survival server answers its hello ("ES\tV"), so on any other realm
-- it does nothing and changes no keys and no interface. Its keys are set for the session only
-- (never saved) and put back, with the interface, when you leave the world. Keys changed on the
-- card are remembered for the next session (still never written to your saved bindings); the
-- card can also switch to your own saved bindings instead, and always lists what's in effect.
--
-- The server's lines (prefix "ES", fields split by tabs) are listed in
-- crates/htl-server/src/survival/ui.rs; the add-on answers with ".es ..." chat commands.
--
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., math.mod, table.getn, `this`.

local VERSION = 6
local ART = "Interface\\AddOns\\ElwynnSurvival\\art\\"
local ICONS = "Interface\\Icons\\"
local FONT_TITLE = "Fonts\\MORPHEUS.TTF"
local FONT_BODY = "Fonts\\FRIZQT__.TTF"
local FONT_NUM = "Fonts\\ARIALN.TTF"

ElwynnSurvival = {}
local ES = ElwynnSurvival
ES.awake = false
ES.mode = "cursor"
ES.sprintHeld = false

BINDING_HEADER_ELWYNNSURVIVAL = "Elwynn Survival"
BINDING_NAME_ES_ATTACK = "Attack (cast, while aiming)"
BINDING_NAME_ES_HEAVY = "Heavy attack"
BINDING_NAME_ES_GUARD = "Raise the guard (hold); call off an aim"
BINDING_NAME_ES_DODGE = "Dodge (hold to sprint)"
BINDING_NAME_ES_AB1 = "Ability: ranged (Q)"
BINDING_NAME_ES_AB2 = "Ability: close (E)"
BINDING_NAME_ES_AB3 = "Ability: utility (1)"
BINDING_NAME_ES_AB4 = "Ability: defense (2)"
BINDING_NAME_ES_AB5 = "Ability: ultimate (3, hold)"
BINDING_NAME_ES_EX1 = "Charged up: ranged"
BINDING_NAME_ES_EX2 = "Charged up: close"
BINDING_NAME_ES_EX3 = "Charged up: utility"
BINDING_NAME_ES_EX4 = "Charged up: defense"
BINDING_NAME_ES_USE = "Use what's in front of you (place, while building)"
BINDING_NAME_ES_EAT = "Use the quick item: eat, a salve, a tonic (turn the station, while building)"
BINDING_NAME_ES_CYCLE = "Next quick item"
BINDING_NAME_ES_SHEATHE = "Sheathe / draw your weapon"
BINDING_NAME_ES_BAG = "Open the bags"
BINDING_NAME_ES_TREE = "The board of abilities"
BINDING_NAME_ES_JOURNAL = "The camp's journal"
BINDING_NAME_ES_BUILD = "Build"
BINDING_NAME_ES_ROTATE = "Turn the station you're placing"
BINDING_NAME_ES_MODE = "Free the mouse / back to action"
BINDING_NAME_ES_LOCK = "Lock on / let go"
BINDING_NAME_ES_NEXT = "Next foe (locked on)"
BINDING_NAME_ES_BARS = "Health bars over heads on / off"
BINDING_NAME_ES_JUMP = "Jump (costs stamina)"
BINDING_NAME_ES_KEYS = "Controls (this card)"

local GOLD = { r = 1.0, g = 0.82, b = 0.45 }
local RED = { r = 1.0, g = 0.35, b = 0.3 }
local GREEN = { r = 0.45, g = 1.0, b = 0.45 }
local WHITE = { r = 1, g = 1, b = 1 }
local FRAME = { r = 0.78, g = 0.64, b = 0.38 }

-- The board's columns (name, colour, what they lean to) and rows (name, what they're for).
local COLUMN_LOOK = {
  warrior = { "Warrior", 0.78, 0.61, 0.43, "Charge, break guards" },
  paladin = { "Paladin", 0.96, 0.55, 0.73, "The holy guardian" },
  hunter = { "Hunter", 0.67, 0.83, 0.45, "Traps and range" },
  rogue = { "Rogue", 1, 0.96, 0.41, "From the shadows" },
  priest = { "Priest", 0.95, 0.95, 0.95, "Light and void" },
  shaman = { "Shaman", 0.25, 0.58, 1, "Storm and totem" },
  mage = { "Mage", 0.41, 0.8, 0.94, "Fire and frost" },
  warlock = { "Warlock", 0.58, 0.51, 0.79, "Fear and fel" },
  druid = { "Druid", 1, 0.49, 0.04, "The wild shapes" },
}
local ROW_WORDS = {
  ranged = { "Ranged", "Reach the foe" },
  close = { "Close", "In the brawl" },
  utility = { "Utility", "Move, control" },
  defense = { "Defense", "Stay alive, punish" },
  ultimate = { "Ultimate", "The whole Valor bar" },
}
-- How many ability keys (the board's rows), and which is the ultimate (held to unleash).
local AB_N = 5
local ULT = 5

-- Holding the dodge button this long is a sprint, not a dodge (as the dash button in the souls games).
local HOLD_TO_SPRINT = 0.28

local SOUND = {
  open = "Sound\\Interface\\iQuestActivate.wav",
  raid = "Sound\\Interface\\RaidWarning.wav",
  done = "Sound\\Interface\\iQuestComplete.wav",
  click = "Sound\\Interface\\uChatScrollButton.wav",
  fall = "Sound\\Interface\\igQuestFailed.wav",
}

local function Play(key)
  local file = SOUND[key]
  if file then
    PlaySoundFile(file)
  end
end

-- --- helpers -------------------------------------------------------------------------------------

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

-- Commands to the server go out as chat; the dead can only whisper (to themselves).
local function Say(text)
  if UnitIsDeadOrGhost and UnitIsDeadOrGhost("player") and UnitName then
    SendChatMessage(text, "WHISPER", nil, UnitName("player"))
  else
    SendChatMessage(text, "SAY")
  end
end
-- (For the add-on's other files.)
ES.Say, ES.Split, ES.Play = Say, Split, Play

local function Tex(parent, layer, file, w, h)
  local t = parent:CreateTexture(nil, layer)
  t:SetTexture(ART .. file)
  if w then
    t:SetWidth(w)
    t:SetHeight(h)
  end
  return t
end

local function Icon(parent, layer, name, size)
  local t = parent:CreateTexture(nil, layer)
  t:SetTexture(ICONS .. name)
  -- The icons' own bevelled rim cut away.
  t:SetTexCoord(0.08, 0.92, 0.08, 0.92)
  if size then
    t:SetWidth(size)
    t:SetHeight(size)
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

local function Clamp(v, lo, hi)
  if v < lo then return lo end
  if v > hi then return hi end
  return v
end

-- Spin a texture by its coordinates (counter-clockwise for a positive angle).
local function SetRotation(t, a)
  local h = 0.35355
  local c = math.cos(a)
  local s = math.sin(a)
  local function rx(x, y) return 0.5 + x * c - y * s end
  local function ry(x, y) return 0.5 + x * s + y * c end
  t:SetTexCoord(rx(-h, -h), ry(-h, -h), rx(-h, h), ry(-h, h), rx(h, -h), ry(h, -h), rx(h, h), ry(h, h))
end

-- A bar: a dark track and a coloured fill, `w` wide.
local function MakeBar(parent, w, h, color)
  local b = CreateFrame("Frame", nil, parent)
  b:SetWidth(w)
  b:SetHeight(h)
  b.track = Tex(b, "BACKGROUND", "bar")
  b.track:SetAllPoints(b)
  b.track:SetVertexColor(0.04, 0.04, 0.04, 0.8)
  b.fill = Tex(b, "ARTWORK", "bar")
  b.fill:SetPoint("TOPLEFT", b, "TOPLEFT", 0, 0)
  b.fill:SetPoint("BOTTOMLEFT", b, "BOTTOMLEFT", 0, 0)
  b.fill:SetWidth(w)
  b.fill:SetVertexColor(color.r, color.g, color.b, 0.95)
  b.full = w
  function b:Set(share)
    self.fill:SetWidth(math.max(1, self.full * Clamp(share, 0, 1)))
  end
  return b
end

-- A vitals bar in the souls style: a framed track, the fill, and behind it a pale chunk that
-- holds what was just lost for a moment and then drains after it.
local function MakeLagBar(parent, w, h, color, chunk)
  local b = CreateFrame("Frame", nil, parent)
  b:SetWidth(w)
  b:SetHeight(h)
  -- A fine rim, a pixel and a half out: the same at any length.
  b.frame = Tex(b, "BACKGROUND", "hairline")
  b.frame:SetPoint("TOPLEFT", b, "TOPLEFT", -1.5, 1.5)
  b.frame:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", 1.5, -1.5)
  b.frame:SetVertexColor(FRAME.r * 0.75, FRAME.g * 0.75, FRAME.b * 0.75, 0.85)
  b.track = Tex(b, "BACKGROUND", "bar")
  b.track:SetAllPoints(b)
  b.track:SetVertexColor(0.03, 0.03, 0.03, 0.82)
  b.lag = Tex(b, "BORDER", "bar")
  b.lag:SetPoint("TOPLEFT", b, "TOPLEFT", 0, 0)
  b.lag:SetPoint("BOTTOMLEFT", b, "BOTTOMLEFT", 0, 0)
  b.lag:SetWidth(w)
  chunk = chunk or { r = 0.95, g = 0.8, b = 0.45 }
  b.lag:SetVertexColor(chunk.r, chunk.g, chunk.b, 0.85)
  b.fill = Tex(b, "ARTWORK", "bar")
  b.fill:SetPoint("TOPLEFT", b, "TOPLEFT", 0, 0)
  b.fill:SetPoint("BOTTOMLEFT", b, "BOTTOMLEFT", 0, 0)
  b.fill:SetWidth(w)
  b.color = color
  b.fill:SetVertexColor(color.r, color.g, color.b, 0.97)
  b.full = w
  b.share = 1
  b.lagShare = 1
  b.lagHold = 0
  function b:Set(share, instant)
    share = Clamp(share, 0, 1)
    if share < self.share and not instant then
      -- Hit: the chunk stays a moment, then drains.
      if self.lagShare < self.share then self.lagShare = self.share end
      self.lagHold = GetTime() + 0.55
    elseif share > self.lagShare or instant then
      self.lagShare = share
    end
    self.share = share
    self.fill:SetWidth(math.max(1, self.full * share))
    self.lag:SetWidth(math.max(1, self.full * self.lagShare))
  end
  function b:SetLength(w)
    if math.abs(w - self.full) < 0.5 then return end
    self.full = w
    self:SetWidth(w)
    self.fill:SetWidth(math.max(1, w * self.share))
    self.lag:SetWidth(math.max(1, w * self.lagShare))
  end
  b:SetScript("OnUpdate", function()
    if this.lagShare > this.share and GetTime() >= this.lagHold then
      this.lagShare = math.max(this.share, this.lagShare - arg1 * 0.6)
      this.lag:SetWidth(math.max(1, this.full * this.lagShare))
    end
    if this.flashUntil then
      local left = this.flashUntil - GetTime()
      if left <= 0 then
        this.flashUntil = nil
        this.fill:SetVertexColor(this.color.r, this.color.g, this.color.b, 0.97)
        this.track:SetVertexColor(0.03, 0.03, 0.03, 0.82)
      else
        local on = math.mod(math.floor(left * 10), 2) == 0
        if on then
          this.track:SetVertexColor(0.55, 0.06, 0.04, 0.9)
        else
          this.track:SetVertexColor(0.03, 0.03, 0.03, 0.82)
        end
      end
    end
  end)
  -- Too little for what was tried: the track blinks red.
  function b:Flash()
    self.flashUntil = GetTime() + 0.6
  end
  return b
end

local function MakeButton(parent, w, h, label, color, onClick)
  local b = CreateFrame("Button", nil, parent)
  b:SetWidth(w)
  b:SetHeight(h)
  b.fill = Tex(b, "BACKGROUND", "button")
  b.fill:SetAllPoints(b)
  b.fill:SetVertexColor(0.09, 0.08, 0.07, 0.92)
  b.rim = Tex(b, "ARTWORK", "buttonborder")
  b.rim:SetAllPoints(b)
  b.rim:SetVertexColor(color.r, color.g, color.b, 0.75)
  b.label = Text(b, FONT_BODY, 12)
  b.label:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.label:SetTextColor(color.r, color.g, color.b)
  b.label:SetText(label)
  b.color = color
  b:EnableMouse(true)
  b:RegisterForClicks("LeftButtonUp")
  b:SetScript("OnEnter", function()
    this.rim:SetVertexColor(this.color.r, this.color.g, this.color.b, 1)
  end)
  b:SetScript("OnLeave", function()
    this.rim:SetVertexColor(this.color.r, this.color.g, this.color.b, 0.75)
  end)
  b:SetScript("OnClick", function()
    Play("click")
    onClick(this)
  end)
  function b:SetEnabledLook(on)
    if on then
      self.label:SetTextColor(self.color.r, self.color.g, self.color.b)
      self:SetAlpha(1)
    else
      self.label:SetTextColor(0.5, 0.5, 0.5)
      self:SetAlpha(0.6)
    end
  end
  return b
end

-- An equipment slot: a dark well, the icon, a bevelled gold frame, the key that uses it and a
-- count. `Set(icon, lit, count)`.
local function MakeSlot(parent, size, key)
  local s = CreateFrame("Frame", nil, parent)
  s:SetWidth(size)
  s:SetHeight(size)
  s.well = Tex(s, "BACKGROUND", "slotwell")
  s.well:SetAllPoints(s)
  s.well:SetVertexColor(0.02, 0.02, 0.02, 0.8)
  s.icon = s:CreateTexture(nil, "ARTWORK")
  s.icon:SetPoint("TOPLEFT", s, "TOPLEFT", 5, -5)
  s.icon:SetPoint("BOTTOMRIGHT", s, "BOTTOMRIGHT", -5, 5)
  s.icon:SetTexCoord(0.08, 0.92, 0.08, 0.92)
  s.frame = Tex(s, "OVERLAY", "slot")
  s.frame:SetAllPoints(s)
  s.frame:SetVertexColor(FRAME.r, FRAME.g, FRAME.b, 0.9)
  s.key = Text(s, FONT_NUM, 10, "OUTLINE")
  s.key:SetPoint("TOPLEFT", s, "TOPLEFT", 4, -3)
  s.key:SetTextColor(0.9, 0.9, 0.9)
  s.key:SetText(key or "")
  s.count = Text(s, FONT_NUM, 13, "OUTLINE")
  s.count:SetPoint("BOTTOMRIGHT", s, "BOTTOMRIGHT", -4, 4)
  -- A width of its own: left to size itself, a count can come out as "..".
  s.count:SetWidth(size)
  s.count:SetJustifyH("RIGHT")
  function s:Set(icon, lit, count)
    if icon then
      self.icon:SetTexture(icon)
    end
    if lit then
      self.icon:SetVertexColor(1, 1, 1)
      self.frame:SetVertexColor(1, 0.86, 0.5, 1)
      self:SetAlpha(1)
    else
      self.icon:SetVertexColor(0.5, 0.5, 0.5)
      self.frame:SetVertexColor(FRAME.r * 0.75, FRAME.g * 0.75, FRAME.b * 0.75, 0.85)
      self:SetAlpha(0.85)
    end
    self.count:SetText(count or "")
  end
  return s
end

local function Fader(name, w, h, fade)
  local f = CreateFrame("Frame", name, UIParent)
  f:SetWidth(w)
  f:SetHeight(h)
  f:Hide()
  f:SetScript("OnUpdate", function()
    local left = (this.untilT or 0) - GetTime()
    if left <= 0 then
      this:Hide()
    elseif left < fade then
      this:SetAlpha(left / fade)
    end
  end)
  return f
end

-- A binding's key as a person would write it: "Ctrl+Q", "Left mouse", "Space".
local PRETTY = {
  SPACE = "Space", TAB = "Tab", ESCAPE = "Esc", ENTER = "Enter", BACKSPACE = "Backspace",
  BUTTON1 = "Left mouse", BUTTON2 = "Right mouse", BUTTON3 = "Middle mouse",
  BUTTON4 = "Mouse 4", BUTTON5 = "Mouse 5", MOUSEWHEELUP = "Wheel up", MOUSEWHEELDOWN = "Wheel down",
  NUMLOCK = "Num Lock",
}
local function Pretty(k)
  if not k or k == "" then return nil end
  local mods = ""
  while true do
    local _, _, m, rest = string.find(k, "^(%u+)%-(.+)$")
    if m == "ALT" or m == "CTRL" or m == "SHIFT" then
      mods = mods .. string.sub(m, 1, 1) .. string.lower(string.sub(m, 2)) .. "+"
      k = rest
    else
      break
    end
  end
  return mods .. (PRETTY[k] or k)
end
ES.Pretty = Pretty

-- The key an action is bound to, as a person would write it (the shortest, when it has two).
local function KeyName(action)
  if not action or not GetBindingKey then return nil end
  local a, b = GetBindingKey(action)
  if b and b ~= "" and (not a or a == "" or string.len(b) < string.len(a)) then a = b end
  if not a or a == "" then return nil end
  return Pretty(a)
end
ES.KeyName = KeyName

-- --- keys ----------------------------------------------------------------------------------------

-- The action-combat client (benilla with this folder's patch).
local function ActionClient()
  return HasActionCombat ~= nil and SetActionMouse ~= nil
end
ES.ActionClient = ActionClient

-- The action-combat client that also plays clips, hit-stops, shakes and rolls itself.
local function AnimClient()
  return ActionClient() and PlayUnitAnimation ~= nil and (tonumber(HasActionCombat()) or 0) >= 2
end
ES.AnimClient = AnimClient

-- The third round: its own action layer (clips that own the body, reactions), its own dashes and
-- lunges, the lock-on and the camera's kick.
local function ActionLayer()
  return AnimClient() and ActionDash ~= nil and (tonumber(HasActionCombat()) or 0) >= 3
end
ES.ActionLayer = ActionLayer

-- The fourth round: our own blow's weight the moment it connects (the client's own test of what
-- the blade crosses), kits and voices played here, the clips' impact keyframes told back.
local function LocalHits()
  return ActionLayer() and ActionProbe ~= nil and TakeActionMarkers ~= nil and (tonumber(HasActionCombat()) or 0) >= 4
end
ES.LocalHits = LocalHits

-- --- the weight of combat: clips played at once, hit-stop, shake, the roll -----------------------
-- On the action-combat client our own swing and dodge play the moment they're pressed (the
-- server's word follows a round trip later, and a swing we already played isn't played twice);
-- others' play when the server says. A blow that lands holds both bodies still for a moment, and
-- shakes the view of whoever is in it.

local LIGHT_TRAIL, HEAVY_TRAIL = "50F2EEE4", "78FF9A3C"
-- How long a prediction waits for the server's own word before that word plays as new.
local PREDICT_WINDOW = 0.6
-- The tumble: about the hips this high (yards), tucked with the Jump clip's shape.
local ROLL_PIVOT, ROLL_TUCK = 0.75, 38
-- The swing's whoosh, by the weapon's weight, a little before the blow lands.
local WOOSH = {
  small = { "Sound\\Item\\Weapons\\WeaponSwings\\mWooshSmall1.wav", "Sound\\Item\\Weapons\\WeaponSwings\\mWooshSmall2.wav", "Sound\\Item\\Weapons\\WeaponSwings\\mWooshSmall3.wav" },
  medium = { "Sound\\Item\\Weapons\\WeaponSwings\\mWooshMedium1.wav", "Sound\\Item\\Weapons\\WeaponSwings\\mWooshMedium2.wav", "Sound\\Item\\Weapons\\WeaponSwings\\mWooshMedium3.wav" },
  large = { "Sound\\Item\\Weapons\\WeaponSwings\\mWooshLarge1.wav", "Sound\\Item\\Weapons\\WeaponSwings\\mWooshLarge2.wav", "Sound\\Item\\Weapons\\WeaponSwings\\mWooshLarge3.wav" },
}

local function Num(i, default)
  return (ES.h and tonumber(ES.h[i])) or default
end

-- Things to do a moment from now (a whoosh as the blade comes round).
ES.timers = {}
local function After(secs, fn)
  table.insert(ES.timers, { at = GetTime() + secs, fn = fn })
end
function ES.RunTimers()
  local now = GetTime()
  local keep = {}
  for i = 1, table.getn(ES.timers) do
    local t = ES.timers[i]
    if now >= t.at then
      t.fn()
    else
      table.insert(keep, t)
    end
  end
  ES.timers = keep
end

local function Woosh(weight)
  local set = WOOSH[weight] or WOOSH.medium
  PlaySoundFile(set[math.random(1, table.getn(set))])
end

-- Free to start a swing or a dodge now, as far as this side knows.
local function Free()
  local n = ES.action and ES.action.name or "ready"
  return n == "ready" or n == "guard" or n == "eat" or n == "gather" or n == "craft" or n == "build" or n == "repair"
end

-- MV: the hero's blows, for playing our own at once: the weapon's grip, the chain and cancel
-- windows, the dodge, and each blow (clip~take~impact~recovery~lunge~stamina).
local function Move(text)
  if not text or text == "" then return nil end
  local f = Split(text, "~")
  return {
    clip = tonumber(f[1]),
    take = tonumber(f[2]) or 0,
    impact = tonumber(f[3]) or 0.3,
    recovery = tonumber(f[4]) or 0.5,
    lunge = tonumber(f[5]) or 0,
    stamina = tonumber(f[6]) or 0,
    range = tonumber(f[7]) or 2.5,
    arc = tonumber(f[8]) or 90,
  }
end

function ES.MovesLine(f)
  local light = {}
  local parts = Split(f[11] or "", "^")
  for i = 1, table.getn(parts) do
    local m = Move(parts[i])
    if m then table.insert(light, m) end
  end
  ES.moves = {
    twoHanded = f[2] == "1",
    chainAfter = tonumber(f[3]) or 0.25,
    cancelAfter = tonumber(f[4]) or 0.45,
    heavyCancel = tonumber(f[5]) or 0.65,
    comboGrace = tonumber(f[6]) or 0.6,
    lungeSecs = tonumber(f[7]) or 0.18,
    dodgeSecs = tonumber(f[8]) or 0.45,
    dodgeSpeed = tonumber(f[9]) or 16,
    dashSecs = tonumber(f[10]) or 0.25,
    light = light,
    heavy = Move(f[12]),
    running = Move(f[13]),
    bash = Move(f[14]),
    kick = Move(f[15]),
    pvp = f[16] == "1",
  }
end

-- Our own blow in hand (as played here): what it is, when it began and how long it takes.
ES.mine = nil

-- Late enough in our blow's recovery to chain the next or cut it short.
local function MineAt(share)
  local m = ES.mine
  if not m then return false end
  local t = GetTime() - m.start
  return t >= m.move.impact + m.move.recovery * share and t < m.move.impact + m.move.recovery
end

local function MineBusy()
  local m = ES.mine
  return m and GetTime() - m.start < m.move.impact + m.move.recovery
end

-- Play a blow of ours: the clip (its impact and recovery the server's), the trail, the lunge, the
-- whoosh.
local function PlayMine(kind, step, move)
  local heavy = kind == "heavy" or kind == "bash" or kind == "kick" or kind == "leap"
  local trail = heavy and HEAVY_TRAIL or LIGHT_TRAIL
  PlayUnitAnimation("player", move.clip, move.impact, trail, math.floor((move.impact + 0.2) * 1000), move.take, move.recovery)
  local mv = ES.moves
  if move.lunge > 0 and ActionDash and mv then
    local secs = mv.lungeSecs
    ActionDash(secs, move.lunge / secs, "f", math.max(0, move.impact - secs))
  end
  if kind ~= "bash" and kind ~= "kick" then
    local weight = (mv and mv.twoHanded) and "large" or "medium"
    After(math.max(0, move.impact * 0.7), function() Woosh(weight) end)
  end
  -- A heavy blow, a bash or a kick: the hero's own shout as it comes.
  if PlayUnitVocal and (kind == "heavy" or kind == "bash" or kind == "kick") then
    After(math.max(0, move.impact * 0.5), function() PlayUnitVocal("player", "exertion_crit") end)
  end
  ES.mine = { kind = kind, step = step, move = move, start = GetTime() }
  if kind == "light" then
    ES.comboNext = step + 1
    ES.comboUntil = GetTime() + move.impact + move.recovery + ((mv and mv.comboGrace) or 0.6)
  else
    ES.comboNext, ES.comboUntil = 0, nil
  end
  ES.predicted = GetTime()
  ES.predictedClip = move.clip
  ES.predictedTake = move.take
  ES.action = { name = (kind == "heavy" and "heavy") or ((kind == "bash" or kind == "kick") and kind) or "swing", start = GetTime(), total = move.impact + move.recovery }
  ES.DrawAction()
end

-- An attack pressed: which blow it is, played at once when it will surely go.
function ES.PredictSwing(kind)
  if not AnimClient() or ES.Kit() ~= "melee" or ES.upAt then return end
  local mv = ES.moves
  if not ActionLayer() or not mv then
    -- The second round's client: the first blow only.
    if not Free() then return end
    local heavy = kind == "heavy"
    local clip, impact, total, cost
    if heavy then
      clip, impact, total, cost = Num(25), Num(26), Num(27), Num(29, 999)
    else
      clip, impact, total, cost = Num(22), Num(23), Num(24), Num(28, 999)
    end
    if not clip or Num(4, 0) < cost then return end
    PlayUnitAnimation("player", clip, impact, heavy and HEAVY_TRAIL or LIGHT_TRAIL, math.floor((impact + 0.27) * 1000))
    ES.predicted = GetTime()
    ES.predictedClip = clip
    ES.action = { name = heavy and "heavy" or "swing", start = GetTime(), total = total or 1 }
    ES.DrawAction()
    return
  end
  local stamina = Num(4, 0)
  local move, step = nil, 0
  if kind == "bash" or kind == "kick" then
    if not Free() then return end
    move = (kind == "kick" or mv.twoHanded) and mv.kick or mv.bash
    if mv.twoHanded then kind = "kick" end
  elseif kind == "heavy" then
    if not Free() then return end
    move = mv.heavy
  else
    local n = table.getn(mv.light)
    if n == 0 then return end
    if MineBusy() then
      -- Late in a light blow: the next of the chain, at once; earlier, the server waits for it.
      -- (Out of a dodge or a Corey roll's landing, as out of a sprint, the chain starts after.)
      local k = ES.mine.kind
      if not (k == "light" or k == "running" or k == "roll" or k == "leap") or not MineAt(mv.chainAfter) then return end
      step = math.mod(ES.mine.step + 1, n)
    else
      if not Free() then return end
      if ES.sprintHeld and mv.running then
        kind = "running"
        move = mv.running
      elseif ES.comboUntil and GetTime() <= ES.comboUntil then
        step = math.mod(ES.comboNext or 0, n)
      end
    end
    if kind ~= "running" then
      kind = "light"
      move = mv.light[step + 1]
    end
  end
  if not move or not move.clip or stamina < math.min(move.stamina, 1) then return end
  if stamina < 1 then return end
  PlayMine(kind, step, move)
end

-- Cut our own blow short (a dodge or the guard late in its recovery).
local function CancelMine()
  if ES.mine and StopUnitAnimation then
    StopUnitAnimation("player", 0.08)
  end
  ES.mine = nil
end

-- A dodge: the tumble and the dash at once (the dash the way we're moving, or a step back).
function ES.PredictRoll()
  if not AnimClient() or ES.upAt then return end
  if Num(4, 0) < Num(30, 999) then return end
  local mv = ES.moves
  local cancel = mv and MineBusy() and MineAt((ES.mine.kind == "heavy") and mv.heavyCancel or mv.cancelAfter)
  if not Free() and not cancel then return end
  if cancel then CancelMine() end
  local secs = (mv and mv.dodgeSecs) or 0.45
  if ActionLayer() and mv then
    local style = "roll"
    if ActionMoveKeys then
      local f, r = ActionMoveKeys()
      style = ES.DodgeStyle(f, r)
    end
    ES.PlayDodge("player", secs, style)
    ActionDash(mv.dashSecs, mv.dodgeSpeed)
  else
    RollUnit("player", secs)
  end
  ES.predictedRoll = GetTime()
  ES.mine = nil
  ES.action = { name = "dodge", start = GetTime(), total = secs }
  ES.DrawAction()
end

-- Which way a dodge goes from the keys held (as the server judges it): forward tumbles, to a side
-- sways aside, back (or no keys) steps back.
function ES.DodgeStyle(f, r)
  f, r = f or 0, r or 0
  if f == 0 and r == 0 then return "back" end
  if f > 0 then return "roll" end
  if f < 0 then return "back" end
  return r > 0 and "right" or "left"
end

-- A dodge's look: the tumble forward, or the Dodge clip's sway for a step aside or back, with a
-- little dust as it ends.
local DODGE_ANIM = 30
function ES.PlayDodge(guid, secs, style)
  if style == "roll" or not LocalHits() then
    RollUnit(guid, secs, "roll", ROLL_PIVOT, ROLL_TUCK)
  else
    -- On the upper body: the legs run the quick step aside (the run sped up to the dash).
    PlayUnitAnimation(guid, DODGE_ANIM, 0, nil, nil, 0, secs, "upper")
  end
  if ActionKit and LocalHits() then
    After(secs * 0.8, function() ActionKit(guid, "dust") end)
  end
end

-- AN: a hero swings (or shoves): the clip, timed, with its trail; ours only if we didn't play the
-- same already, and with its lunge when it's new to us.
function ES.SwingLine(f)
  local guid, clip, impact, trail, ms, mine = f[2], tonumber(f[3]), tonumber(f[4]), f[5], tonumber(f[6]), f[7] == "1"
  local take, recovery, lunge, kind, step = tonumber(f[8]) or 0, tonumber(f[9]) or 0, tonumber(f[10]) or 0, f[11] or "light", tonumber(f[12]) or 0
  if not clip or not PlayUnitAnimation then return end
  -- A cast (a throw, a spell): its clip on whoever cast it, ours too (no blow of ours to track).
  if kind == "cast" then
    if mine then guid = "player" end
    PlayUnitAnimation(guid, clip, impact, nil, ms, take, recovery)
    return
  end
  if mine then
    local same = ES.predicted and GetTime() - ES.predicted < PREDICT_WINDOW and ES.predictedClip == clip and (ES.predictedTake or 0) == take
    ES.predicted = nil
    if same then return end
    -- The server's blow, not the one we guessed: it plays now.
    if ActionLayer() and ES.moves then
      PlayMine(kind, step, { clip = clip, take = take, impact = impact, recovery = recovery, lunge = lunge, stamina = 0 })
      ES.predicted = nil
      return
    end
    guid = "player"
  end
  PlayUnitAnimation(guid, clip, impact, trail, ms, take, recovery)
end

-- RO: a hero dodges: the tumble (or a step aside or back).
function ES.RollLine(f)
  local guid, secs, mine, style = f[2], tonumber(f[3]) or 0.45, f[4] == "1", f[5] or "roll"
  if not RollUnit then return end
  if mine then
    if ES.predictedRoll and GetTime() - ES.predictedRoll < PREDICT_WINDOW then
      ES.predictedRoll = nil
      return
    end
    guid = "player"
    if ActionLayer() and ES.moves then
      -- The server's dodge we didn't see coming: the dash too.
      ActionDash(ES.moves.dashSecs, ES.moves.dodgeSpeed)
    end
  end
  if ActionLayer() then
    ES.PlayDodge(guid, secs, style)
  else
    RollUnit(guid, secs)
  end
end

-- RE: a unit takes a blow: its reaction and the lean back from it (ours drops the blow in hand);
-- a knockdown plays its whole fall and rise over its time, with a cry and the dust.
local KNOCKDOWN_ANIM, STUN_ANIM = 121, 14
function ES.ReactLine(f)
  if not ReactUnit then return end
  local guid, anim, dx, dy, lean, hold = f[2], tonumber(f[3]), tonumber(f[4]) or 0, tonumber(f[5]) or 0, tonumber(f[6]) or 0, tonumber(f[7]) or 0
  local fit = tonumber(f[9]) or 0
  if not anim then return end
  if f[8] == "1" then
    guid = "player"
    if hold > 0 or fit > 0 then
      ES.mine = nil
      ES.comboUntil = nil
    end
  end
  -- A flinch takes any of the model's takes on it, a little different each time.
  ReactUnit(guid, anim, dx, dy, lean * (0.85 + math.random() * 0.3), hold, fit, "random")
  if PlayUnitVocal then
    if anim == KNOCKDOWN_ANIM then
      PlayUnitVocal(guid, "injury_crit")
    elseif anim == STUN_ANIM then
      PlayUnitVocal(guid, "injury")
    end
  end
end


-- LO: the lock-on (a guid, or 0x0 to let go).
function ES.LockLine(f)
  local g = f[2]
  if not g or g == "0x0" or g == "0X0" or g == "0" then
    ES.lockOn = nil
    if SetLockOn then SetLockOn(nil) end
  else
    ES.lockOn = g
    if SetLockOn then SetLockOn(g) end
  end
end

-- How long a landed blow holds both bodies (ms), and how hard it shakes the striker's and the
-- struck one's view (inches, seconds); a heavy one punches the camera in a little.
local STOP = { light = 60, heavy = 115, arrow = 40 }
local SHAKE_STRIKER = { light = { 1.0, 0.14 }, heavy = { 3.0, 0.26 }, arrow = { 0, 0 } }
-- Being struck shakes harder and longer than striking (play-test: it was liked, and wanted bigger).
local SHAKE_STRUCK = { light = { 4.0, 0.3 }, heavy = { 8.5, 0.42 }, arrow = { 3.2, 0.26 } }
local KICK = { heavy = { 0.07, 0.3 }, parry = { 0.05, 0.25 }, struck = { 0.06, 0.3 } }
-- Our own blow's hit-stop and shake by what it is: a bash thuds, a kick shoves, a running attack
-- lands with its weight behind it.
local STOP_FOR = { light = 60, running = 85, heavy = 115, bash = 95, kick = 90 }
local SHAKE_FOR = { light = { 1.0, 0.14 }, running = { 1.8, 0.2 }, heavy = { 3.0, 0.26 }, bash = { 2.4, 0.2 }, kick = { 2.0, 0.22 } }

-- The guard's own sound when it takes a blow: a shield's thud (wood or metal, as worn), a blade
-- or a haft ringing when a two-hander guards; a parry rings metal on metal.
local BLOCK_SOUND = {
  metal = { "Sound\\Item\\Weapons\\Shields\\MetalShieldBlock1.wav", "Sound\\Item\\Weapons\\Shields\\MetalShieldBlock2.wav", "Sound\\Item\\Weapons\\Shields\\MetalShieldBlock3.wav" },
  wood = { "Sound\\Item\\Weapons\\Shields\\WoodenShieldBlock1.wav", "Sound\\Item\\Weapons\\Shields\\WoodenShieldBlock2.wav", "Sound\\Item\\Weapons\\Shields\\WoodenShieldBlock3.wav" },
  blade = { "Sound\\Item\\Weapons\\Parry\\ParryMetalHitMetal.wav" },
  parry = { "Sound\\Item\\Weapons\\Parry\\ParryWoodHitMetalCrit.wav" },
}
-- Our own shield: wood for a buckler, metal for the rest (its icon says which).
local function OwnGuardSound()
  local t = GetInventoryItemTexture and GetInventoryItemTexture("player", 17)
  if not t then return "blade" end
  if string.find(string.lower(t), "buckler", 1, true) or string.find(string.lower(t), "wood", 1, true) then return "wood" end
  return "metal"
end
function ES.GuardSound(how, mine)
  local set
  if how == "parry" then
    set = BLOCK_SOUND.parry
  else
    set = BLOCK_SOUND[mine and OwnGuardSound() or "metal"]
  end
  PlaySoundFile(set[math.random(1, table.getn(set))])
end

-- IM: a blow lands, is blocked or is parried.
function ES.ImpactLine(f)
  local a, v, kind, how, struck, wasStruck = f[2], f[3], f[4] or "light", f[5], f[6] == "1", f[7] == "1"
  -- A guard taking a blow sounds, ours or the one ours met.
  if (how == "block" or how == "parry") and (struck or wasStruck) then
    ES.GuardSound(how, wasStruck)
  end
  if not HitStop then return end
  -- Our own blow that we already played as it connected: the rest only.
  local played = struck and ES.localHit and GetTime() - ES.localHit.at < PREDICT_WINDOW + 0.4 and string.upper(ES.localHit.guid) == string.upper(v or "")
  if played then
    ES.localHit = nil
    if how == "parry" and ShakeCamera then ShakeCamera(3.0, 0.25) end
    return
  end
  if struck then a = "player" end
  if wasStruck then v = "player" end
  local stop, shakeBy, shakeTo = 0, nil, nil
  if how == "hit" then
    stop = STOP[kind] or 60
    shakeBy, shakeTo = SHAKE_STRIKER[kind], SHAKE_STRUCK[kind]
  elseif how == "block" then
    stop = 50
    shakeBy, shakeTo = { 0.8, 0.12 }, { 1.6, 0.16 }
  elseif how == "parry" then
    stop = 150
    shakeBy, shakeTo = { 3.0, 0.25 }, { 2.0, 0.2 }
  else
    -- A blow we slipped: the blade's rush past us.
    if how == "dodge" and wasStruck and kind ~= "arrow" then Woosh("small") end
    return
  end
  if kind ~= "arrow" then HitStop(a, stop) end
  HitStop(v, stop)
  if ShakeCamera then
    if struck and shakeBy and shakeBy[1] > 0 then ShakeCamera(shakeBy[1], shakeBy[2]) end
    if wasStruck and shakeTo and shakeTo[1] > 0 then ShakeCamera(shakeTo[1], shakeTo[2]) end
  end
  if CameraKick then
    local k
    if how == "hit" and kind == "heavy" and (struck or wasStruck) then
      k = KICK.heavy
    elseif how == "parry" and (struck or wasStruck) then
      k = KICK.parry
    elseif how == "hit" and wasStruck and kind ~= "arrow" then
      k = KICK.struck
    end
    if k then CameraKick(k[1], k[2]) end
  end
end

-- Our own blow reaching its impact keyframe: what the blade crosses takes its weight here and now
-- (the server's word follows, and isn't played twice).
ES.localHit = nil
local LOCAL_KIT = { heavy = 437, bash = 83, kick = 83 }
function ES.OwnImpact()
  local m = ES.mine
  if not m or not ActionProbe then return end
  local mv = ES.moves
  local guid = ActionProbe(m.move.range or 2.5, m.move.arc or 90, mv and mv.pvp)
  if not guid then return end
  local kind = m.kind
  local weight = (kind == "heavy" or kind == "bash" or kind == "kick") and "heavy" or "light"
  local stop = STOP_FOR[kind] or STOP_FOR.light
  HitStop("player", stop)
  HitStop(guid, stop)
  local shake = SHAKE_FOR[kind] or SHAKE_FOR.light
  if ShakeCamera then ShakeCamera(shake[1], shake[2]) end
  if CameraKick and weight == "heavy" then CameraKick(KICK.heavy[1], KICK.heavy[2]) end
  local kit = LOCAL_KIT[kind]
  local last = mv and table.getn(mv.light) or 0
  if not kit and ((kind == "light" and last > 1 and m.step == last - 1) or kind == "running") then
    kit = 507
  end
  if kit and ActionKit then ActionKit(guid, kit) end
  ES.localHit = { at = GetTime(), guid = guid }
end

function ES.Markers()
  if not LocalHits() then return end
  local list = TakeActionMarkers()
  for i = 1, table.getn(list) do
    if list[i] == "impact player" then
      ES.OwnImpact()
    end
  end
end

-- The Elwynn control profile, set explicitly whatever was saved before (Basin Battle's add-on
-- saves A and D as strafes; a fresh profile has them turning), so every hero gets the same layout.
-- An empty action clears the key (the spellbook, quest log, social and talent keys, and the stock
-- action bar's, which open or cast nothing useful here). The mouse buttons are in MOUSE_KEYS.
local KEYS = {
  { "W", "MOVEFORWARD" },
  { "S", "MOVEBACKWARD" },
  { "A", "STRAFELEFT" },
  { "D", "STRAFERIGHT" },
  -- Space jumps (the game's own jump: an add-on may not call Jump(), the client blocks it; the
  -- server takes the jump's stamina); the dodge is on the mouse's side button (MOUSE_KEYS, below).
  { "SPACE", "JUMP" },
  -- Ctrl (the charged-up abilities' key) held, the hero still moves and jumps: Ctrl+S would
  -- otherwise be the game's own sound toggle. (The rest of `CTRL_TOO` below, too.)
  { "CTRL-W", "MOVEFORWARD" },
  { "CTRL-S", "MOVEBACKWARD" },
  { "CTRL-A", "STRAFELEFT" },
  { "CTRL-D", "STRAFERIGHT" },
  -- The health bars over heads, on and off (the game's own V shows only the foes').
  { "V", "ES_BARS" },
  -- The five abilities (the ultimate held), and with Ctrl the first four's EX versions.
  { "Q", "ES_AB1" },
  { "E", "ES_AB2" },
  { "1", "ES_AB3" },
  { "2", "ES_AB4" },
  { "3", "ES_AB5" },
  { "CTRL-Q", "ES_EX1" },
  { "CTRL-E", "ES_EX2" },
  { "CTRL-1", "ES_EX3" },
  { "CTRL-2", "ES_EX4" },
  { "F", "ES_USE" },
  { "R", "ES_SHEATHE" },
  { "T", "ES_EAT" },
  { "G", "ES_CYCLE" },
  { "X", "TOGGLERUN" },
  -- Run on without a key held (the game's own autorun; a mouse button on the card if you like).
  { "NUMLOCK", "TOGGLEAUTORUN" },
  { "B", "ES_BAG" },
  { "H", "ES_BUILD" },
  { "N", "ES_TREE" },
  { "J", "ES_JOURNAL" },
  -- Tab locks on and, locked, lets go; Ctrl+Tab goes to the next foe.
  { "TAB", "ES_LOCK" },
  { "CTRL-TAB", "ES_NEXT" },
  { "Z", "ES_MODE" },
  { "K", "ES_KEYS" },
  { "C", "TOGGLECHARACTER0" },
  { "M", "TOGGLEWORLDMAP" },
  -- Dead ends here.
  { "4", "" },
  { "P", "" },
  { "L", "" },
  { "O", "" },
  { "5", "" },
  { "6", "" },
  { "7", "" },
  { "8", "" },
  { "9", "" },
  { "0", "" },
  { "-", "" },
  { "=", "" },
  { "SHIFT-P", "" },
  { "SHIFT-5", "" },
  { "SHIFT-6", "" },
}
ES.KEYS = KEYS

-- Each key above is bound with Shift too (sprinting), where a Shift binding would take the key
-- otherwise, and each Ctrl one with Ctrl+Shift; these aren't.
local NO_SHIFT = { ES_KEYS = 1, ES_MODE = 1, TOGGLECHARACTER0 = 1, TOGGLEWORLDMAP = 1, ES_BAG = 1, ES_TREE = 1, ES_JOURNAL = 1, ES_BUILD = 1, ES_BARS = 1 }
-- Bound with Ctrl too (and Ctrl+Shift), where Ctrl is held for an EX version: the jump, the use
-- key, the quick item, the walk; the stock client's Ctrl chords there (the sound, the frame rate)
-- would take the key otherwise.
ES.CTRL_TOO = { JUMP = 1, ES_USE = 1, ES_SHEATHE = 1, ES_EAT = 1, ES_CYCLE = 1, TOGGLERUN = 1 }
-- The layout's version: a saved custom key or own-keys choice from before it is dropped once, so
-- every hero starts this version on the same keys (a jump left on V, a stale choice).
ES.LAYOUT_VERSION = 2

-- On the action-combat client the mouse buttons are bound too: left attacks (Shift: heavy),
-- right raises the guard, and the middle button's stock binding (both buttons' run) is cleared,
-- as the client gives it the left button's part (or it locks on, on the Controls card).
local MOUSE_KEYS = {
  { "BUTTON1", "ES_ATTACK" },
  { "SHIFT-BUTTON1", "ES_HEAVY" },
  { "CTRL-BUTTON1", "ES_ATTACK" },
  { "BUTTON2", "ES_GUARD" },
  { "SHIFT-BUTTON2", "ES_GUARD" },
  { "CTRL-BUTTON2", "ES_GUARD" },
  { "BUTTON3", "" },
  { "SHIFT-BUTTON3", "" },
  -- The souls games' dash button, on the mouse's side button: a tap dodges (on release), holding
  -- it sprints; with Shift (sprinting) or Ctrl held too. Both side buttons, as which one a client
  -- calls the fourth varies (the card shows Mouse 4).
  { "BUTTON4", "ES_DODGE" },
  { "SHIFT-BUTTON4", "ES_DODGE" },
  { "CTRL-BUTTON4", "ES_DODGE" },
  { "BUTTON5", "ES_DODGE" },
  { "SHIFT-BUTTON5", "ES_DODGE" },
  { "CTRL-BUTTON5", "ES_DODGE" },
}
ES.MOUSE_KEYS = MOUSE_KEYS

local function Settings()
  ElwynnSurvivalSettings = ElwynnSurvivalSettings or {}
  return ElwynnSurvivalSettings
end

-- The floor markers (creatures' blows coming, ground about to burn): on unless turned off in the
-- Controls card. Those up now, to take down when they're turned off.
function ES.MarksOn()
  return not (ElwynnSurvivalSettings and ElwynnSurvivalSettings.noMarks)
end
ES.marksUp = { tg = {}, mk = {} }
function ES.ClearMarks()
  for key in pairs(ES.marksUp.tg) do
    if ClearTelegraph then ClearTelegraph(key) end
  end
  for key in pairs(ES.marksUp.mk) do
    if ClearGroundMarker then ClearGroundMarker(key) end
  end
  ES.marksUp = { tg = {}, mk = {} }
end
ES.Settings = Settings

-- Whether to keep the player's own saved bindings instead of the profile (the Keys card's switch).
local function OwnKeys()
  return ElwynnSurvivalSettings and ElwynnSurvivalSettings.ownKeys
end

local function IsMouse(key)
  return string.find(key, "BUTTON", 1, true) ~= nil or string.find(key, "MOUSEWHEEL", 1, true) ~= nil
end

-- The layout in effect: the profile, the keys changed on the card in place of an action's own
-- (an ability key changed takes its EX version with Ctrl along), and the middle button's lock-on
-- if it's asked for. Each entry: key, action.
function ES.Layout()
  local custom = (ElwynnSurvivalSettings and ElwynnSurvivalSettings.custom) or {}
  local mine = {}
  -- Only actions this version has (a key saved for one since gone, the old V jump, is dropped).
  if not ES.knownActions then
    ES.knownActions = {}
    for i = 1, table.getn(KEYS) do ES.knownActions[KEYS[i][2]] = true end
    for i = 1, table.getn(MOUSE_KEYS) do ES.knownActions[MOUSE_KEYS[i][2]] = true end
    ES.knownActions[""] = nil
  end
  for action, key in pairs(custom) do
    if ES.knownActions[action] and type(key) == "string" and key ~= "" then
      mine[action] = key
    end
  end
  for n = 1, AB_N - 1 do
    local base = mine["ES_AB" .. n]
    if base and not mine["ES_EX" .. n] and not string.find(base, "-", 1, true) and not IsMouse(base) then
      mine["ES_EX" .. n] = "CTRL-" .. base
    end
  end
  local list = {}
  local function Add(key, action, shifted)
    table.insert(list, { key, action })
    if not shifted or action == "" or NO_SHIFT[action] or IsMouse(key) then return end
    if string.find(key, "^CTRL%-") then
      table.insert(list, { "CTRL-SHIFT-" .. string.sub(key, 6), action })
    elseif not string.find(key, "-", 2, true) then
      table.insert(list, { "SHIFT-" .. key, action })
      if ES.CTRL_TOO[action] then
        table.insert(list, { "CTRL-" .. key, action })
        table.insert(list, { "CTRL-SHIFT-" .. key, action })
      end
    end
  end
  local all = {}
  for i = 1, table.getn(KEYS) do table.insert(all, KEYS[i]) end
  for i = 1, table.getn(MOUSE_KEYS) do table.insert(all, MOUSE_KEYS[i]) end
  for i = 1, table.getn(all) do
    local key, action = all[i][1], all[i][2]
    if action == "" or not mine[action] then
      if key == "BUTTON3" or key == "SHIFT-BUTTON3" then
        if ElwynnSurvivalSettings and ElwynnSurvivalSettings.middleLock then action = "ES_LOCK" end
      end
      Add(key, action, not IsMouse(key))
    end
  end
  -- The changed keys last: a key taken from another action is this one's now.
  for action, key in pairs(mine) do
    Add(key, action, true)
  end
  return list
end

-- What each key did before, to put back.
ES.saved = nil

-- Each change made safely: one refused (a key the client won't take) never stops the rest of the
-- layout going on, which would leave half the keys on one profile and half on another.
local function Bind(key, action)
  if not SetBinding then return end
  if ES.saved[key] == nil then
    ES.saved[key] = (GetBindingAction and GetBindingAction(key)) or ""
  end
  if action == "" then
    pcall(SetBinding, key)
  else
    pcall(SetBinding, key, action)
  end
end

local function Unbind(key)
  if not ES.saved or ES.saved[key] == nil then return end
  local was = ES.saved[key]
  if was == "" then
    pcall(SetBinding, key)
  else
    pcall(SetBinding, key, was)
  end
  ES.saved[key] = nil
end

-- A layout from before this version: its custom keys and own-keys choice dropped, once, so every
-- hero plays this version on the same Elwynn keys (K changes them again). Whether it did.
function ES.CheckLayout()
  local set = Settings()
  if (tonumber(set.layout) or 0) >= ES.LAYOUT_VERSION then return false end
  local had = (set.custom and next(set.custom) ~= nil) or set.ownKeys
  set.custom = nil
  set.ownKeys = nil
  set.layout = ES.LAYOUT_VERSION
  return had and true or false
end

-- The mouse's speed (looking round and steering): one dial over the game's two (its Mouse Look
-- Speed and Mouse Sensitivity, whose lowest together is still quick), from a tenth of the stock
-- speed to one and a half times it; kept with your settings and put back each time you play.
ES.MOUSE_STEPS = { 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.5, 0.6, 0.75, 0.9, 1.0, 1.25, 1.5 }
function ES.CVar(name)
  if not GetCVar then return nil end
  local ok, v = pcall(GetCVar, name)
  if ok then return tonumber(v or "") end
  return nil
end
function ES.MouseSpeed()
  return (ES.CVar("cameraYawMoveSpeed") or 180) / 180 * (ES.CVar("mouseSpeed") or 1)
end
function ES.SetMouseSpeed(m)
  if not SetCVar then return end
  pcall(SetCVar, "mouseSpeed", "1")
  pcall(SetCVar, "cameraYawMoveSpeed", string.format("%.1f", 180 * m))
  pcall(SetCVar, "cameraPitchMoveSpeed", string.format("%.1f", 90 * m))
  Settings().mouse = m
end
function ES.StepMouse(dir)
  local now, steps = ES.MouseSpeed(), ES.MOUSE_STEPS
  local pick = steps[1]
  if dir > 0 then
    pick = steps[table.getn(steps)]
    for i = 1, table.getn(steps) do
      if steps[i] > now + 0.01 then pick = steps[i] break end
    end
  else
    for i = table.getn(steps), 1, -1 do
      if steps[i] < now - 0.01 then pick = steps[i] break end
    end
  end
  ES.SetMouseSpeed(pick)
end

-- Bind the layout's keys (`mouse`: the mouse buttons, else the keyboard).
local function BindLayout(mouse)
  local list = ES.Layout()
  for i = 1, table.getn(list) do
    if (IsMouse(list[i][1]) and true or false) == mouse then
      Bind(list[i][1], list[i][2])
    end
  end
end

-- The way back from your own keys: the Controls card's key and the one that frees the mouse are
-- always bound (on their Elwynn keys, if your own bindings have no key for them).
local ALWAYS_BOUND = { ES_KEYS = 1, ES_MODE = 1 }

-- Your own keys: your saved bindings, and each Elwynn Survival action you haven't given a key of
-- your own on its Elwynn key, wherever your bindings leave that key free. The card and the free
-- mouse take their keys whatever is on them.
local function BindOwnLayout()
  local list = ES.Layout()
  -- The actions put on their Elwynn keys here: only their Shift chords (sprinting) follow, and an
  -- ability's charged-up Ctrl chord only with the ability on its own Elwynn key.
  local ours = {}
  for i = 1, table.getn(list) do
    local key, action = list[i][1], list[i][2]
    if action ~= "" and string.sub(action, 1, 3) == "ES_" and not IsMouse(key) then
      local on = GetBindingAction and GetBindingAction(key)
      local free = not on or on == ""
      local shifted = string.find(key, "SHIFT-", 1, true) ~= nil
      local ex = string.find(action, "^ES_EX(%d)")
      local own = GetBindingKey and GetBindingKey(action)
      if shifted then
        if ours[action] and free then Bind(key, action) end
      elseif ex then
        local n = string.sub(action, 6)
        if ours["ES_AB" .. n] and not own and free then Bind(key, action) ours[action] = true end
      elseif not own and (free or ALWAYS_BOUND[action]) then
        Bind(key, action)
        ours[action] = true
      end
    end
  end
end

function ES.ApplyKeys()
  ES.saved = ES.saved or {}
  if OwnKeys() then
    BindOwnLayout()
    return
  end
  BindLayout(false)
end

function ES.RestoreKeys()
  if not ES.saved then return end
  local keys = {}
  for key in pairs(ES.saved) do
    table.insert(keys, key)
  end
  for i = 1, table.getn(keys) do
    Unbind(keys[i])
  end
  ES.saved = nil
end

-- The mouse buttons are the action mode's (the action-combat client only); cursor mode gives
-- them back.
function ES.ApplyMouseKeys()
  ES.saved = ES.saved or {}
  BindLayout(true)
end

function ES.RestoreMouseKeys()
  if not ES.saved then return end
  local keys = {}
  for key in pairs(ES.saved) do
    -- The side buttons (the dodge) stay: they click nothing in a window, and a roll with the
    -- mouse freed (Z) should still roll.
    local side = string.find(key, "BUTTON4", 1, true) or string.find(key, "BUTTON5", 1, true)
    if IsMouse(key) and not side then table.insert(keys, key) end
  end
  for i = 1, table.getn(keys) do
    Unbind(keys[i])
  end
end

-- The keys again after a change on the card: everything put back, then the layout anew.
function ES.Rebind()
  if not ES.awake then return end
  ES.RestoreKeys()
  ES.ApplyKeys()
  if ES.mode == "action" and ActionClient() then ES.ApplyMouseKeys() end
end

-- --- the mouse ---------------------------------------------------------------------------------------

-- The stock client's click-catcher: a left press is a swing (or a cast), a right press guards and
-- starts the game's own mouse-look, a middle press turns the camera alone; none of them reaches
-- the world's camera drag.
local catcher = CreateFrame("Frame", "ElwynnSurvivalCatcher", UIParent)
catcher:SetAllPoints(UIParent)
catcher:SetFrameStrata("BACKGROUND")
catcher:SetFrameLevel(0)
catcher:EnableMouse(true)
catcher:Hide()
ES.catcher = catcher

catcher:SetScript("OnMouseDown", function()
  if arg1 == "LeftButton" then
    if IsShiftKeyDown and IsShiftKeyDown() then
      ES.Heavy("down")
    else
      ES.Attack("down")
    end
  elseif arg1 == "RightButton" then
    ES.Look("down")
  elseif arg1 == "MiddleButton" then
    ES.Orbit("down")
  end
end)
catcher:SetScript("OnMouseUp", function()
  if arg1 == "LeftButton" then
    ES.Attack("up")
  elseif arg1 == "RightButton" then
    ES.Look("up")
  elseif arg1 == "MiddleButton" then
    ES.Orbit("up")
  end
end)
-- Hidden (Z, a window, the map): anything still held lets go, as releasing the button would.
catcher:SetScript("OnHide", function()
  if ES.attackHeld then ES.Attack("up") end
  if ES.lookHeld then ES.Look("up") end
  if ES.orbitHeld then ES.Orbit("up") end
end)

-- Action mode: the mouse steers, left attacks, right guards. Cursor mode: the mouse is the
-- game's own (click things, drag the camera where you like).
function ES.SetMode(mode)
  if not ES.awake then return end
  -- Placing a station keeps the mouse free until it's placed or cancelled.
  if mode == "action" and ES.placing then
    mode = "cursor"
  end
  ES.mode = mode
  -- Nothing held across a change of mode.
  if mode == "cursor" then
    if ES.lookHeld then ES.Look("up") end
    if ES.attackHeld then ES.Attack("up") end
    if ES.guardHeld then ES.Guard("up") end
    if ES.orbitHeld then ES.Orbit("up") end
    ES.Steer(false)
  end
  if ActionClient() then
    catcher:Hide()
    if mode == "action" then
      -- The mouse buttons are the action mode's, whichever keys are in use.
      ES.ApplyMouseKeys()
      SetActionMouse(1)
    else
      SetActionMouse(nil)
      ES.RestoreMouseKeys()
    end
  elseif mode == "action" then
    catcher:Show()
  else
    catcher:Hide()
  end
  ES.DrawMode()
end

function ES.ToggleMode()
  if not ES.awake then return end
  if ES.mode == "action" then
    ES.SetMode("cursor")
  else
    ES.CloseWindows()
    ES.SetMode("action")
  end
end

-- A window opening: on the stock client the catcher must lift for the mouse to reach it; on the
-- action-combat client the mouse-look lets go by itself while the window is up (`ES.Steer`).
local function WindowMode()
  if not ActionClient() then
    ES.SetMode("cursor")
  end
end
ES.WindowMode = WindowMode

-- The windows that want the cursor: while any is up the mouse doesn't steer.
local CURSOR_FRAMES = {
  "ElwynnSurvivalWindow", "ElwynnSurvivalKeys", "ElwynnSurvivalTree", "ElwynnSurvivalConfirm",
  "ElwynnSurvivalPlace", "ContainerFrame1", "ContainerFrame2", "ContainerFrame3",
  "ContainerFrame4", "ContainerFrame5", "CharacterFrame", "WorldMapFrame", "GameMenuFrame",
  "SpellBookFrame", "LootFrame", "MerchantFrame", "GossipFrame", "QuestFrame", "TradeFrame",
  "KeyBindingFrame", "OptionsFrame", "SoundOptionsFrame", "UIOptionsFrame", "HelpFrame",
  "StaticPopup1", "StaticPopup2", "ItemTextFrame", "MailFrame", "BankFrame", "TaxiFrame",
  "ElwynnSurvivalLetter",
}

local function CursorWanted()
  if UIParent and not UIParent:IsVisible() then return true end
  for i = 1, table.getn(CURSOR_FRAMES) do
    local f = getglobal(CURSOR_FRAMES[i])
    if f and f.IsVisible and f:IsVisible() then return true end
  end
  return false
end
ES.CursorWanted = CursorWanted

-- The mouse steering (the game's own mouse-look, held): on in action mode on the action-combat
-- client with no window up and no one typing; polled every frame, since a right button's release
-- or a window ends it.
ES.steering = false
function ES.Steer(want)
  if not MouselookStart or not MouselookStop then return end
  if want then
    if not (IsMouselooking and IsMouselooking()) then MouselookStart() end
    ES.steering = true
  elseif ES.steering then
    ES.steering = false
    if IsMouselooking and IsMouselooking() then MouselookStop() end
  end
end

-- --- the bindings' bodies ----------------------------------------------------------------------------

-- Aiming or charging an ability (the server's word, as last heard): its key.
local function Aiming()
  local a = ES.action
  if a and (a.name == "aim" or a.name == "charge") and a.ability and a.ability ~= "" then
    return a.ability
  end
  return nil
end
ES.Aiming = Aiming

function ES.Attack(state)
  if not ES.awake then return end
  if state == "down" then
    ES.attackHeld = true
    -- While aiming: the cast (held, the charge), along the reticle's line.
    if Aiming() then
      Say(".es in a" .. ES.AimLine())
      return
    end
    -- With the guard up, an attack is the shield bash (a kick with a two-handed weapon).
    if ES.guardHeld and ES.Kit() == "melee" and ActionLayer() then
      ES.PredictSwing("bash")
      Say(".es in b")
      return
    end
    ES.PredictSwing("light")
    Say(".es in a" .. ES.AimLine())
  else
    if not ES.attackHeld then return end
    ES.attackHeld = false
    Say(".es in au" .. ES.AimLine())
  end
end

-- The reticle's line (where the view's centre looks, a facing and an elevation, on the
-- action-combat client), sent with a cast or a shot so it goes where the reticle is.
function ES.AimLine()
  if not GetActionAim then return "" end
  local f, e = GetActionAim()
  if not f or not e then return "" end
  return string.format(" %.4f %.4f", f, e)
end

-- Shift + left: the heavy attack.
function ES.Heavy(state)
  if not ES.awake then return end
  if state == "down" then
    if Aiming() then
      ES.Attack("down")
      return
    end
    ES.attackHeld = true
    -- With the guard up, the kick.
    if ES.guardHeld and ES.Kit() == "melee" and ActionLayer() then
      ES.PredictSwing("kick")
      Say(".es in k")
      return
    end
    ES.PredictSwing("heavy")
    Say(".es in h" .. ES.AimLine())
  else
    ES.Attack("up")
  end
end

-- Right mouse: the guard while it's held (a parry just as a blow lands); while aiming, it calls
-- the aim off instead.
function ES.Guard(state)
  if not ES.awake then return end
  if state == "down" then
    if ES.guardHeld then return end
    ES.guardHeld = true
    -- Before a blow of ours lands the guard calls it off (a feint: its stamina comes back); late
    -- in its recovery, cuts it short.
    local mv = ES.moves
    if ES.mine and MineBusy() and GetTime() - ES.mine.start < ES.mine.move.impact then
      CancelMine()
      ES.comboUntil = nil
    elseif mv and MineBusy() and MineAt((ES.mine.kind == "heavy") and mv.heavyCancel or mv.cancelAfter) then
      CancelMine()
    end
    Say(".es in g")
  else
    if not ES.guardHeld then return end
    ES.guardHeld = false
    Say(".es in gu")
  end
end

-- The over-the-shoulder camera while an aimed ability is up (the action-combat client).
function ES.DrawAimCamera()
  local want = ES.awake and Aiming() ~= nil and ES.AimShape() ~= "self"
  if want == ES.aimCam then return end
  ES.aimCam = want
  if SetAimCamera then
    if want then
      SetAimCamera(1, 2.4, 0.75, 0.25)
    else
      SetAimCamera(nil)
    end
  end
end

-- An ability key (`n` 1 to 5, Q E 1 2 3; `ex`: its EX version, Ctrl with the key): an aimed one
-- comes up (its key again calls it off), anything else goes at once. Held a moment (`EX_HOLD`),
-- the key gives its EX version instead: an aimed one comes up at once and turns EX as the hold
-- passes; one that goes at once (a stance, a nova, a weapon art, ground around you) waits for
-- the hold or the key's release to know which. The ultimate (3) has no EX version: it goes only
-- once its key has been held that moment, so a stray press never spends the whole bar.
local EX_HOLD = 0.35
ES.exHold = nil

-- Whether there's Valor enough for slot `s`'s EX version now (a hold without it is a plain press:
-- nothing waits on a hold that can't be paid for).
function ES.ExAffordable(s)
  if not s or not s.ex then return false end
  local v = ES.valorNow and ES.valorNow() or 0
  return v + 0.001 >= (s.ex.valor or 0)
end

local function SendAbility(n, ex)
  Say(".es in ab " .. (n - 1) .. " " .. (ex and "1" or "0") .. ES.AimLine())
end

function ES.Ability(n, ex, state)
  if not ES.awake then return end
  local h = ES.exHold
  if state == "up" then
    if h and h.n == n then
      -- Let go before the hold: a waiting one goes as itself (the ultimate doesn't go at all).
      if h.ult and not h.sent then
        ES.Flash("Hold " .. (KeyName("ES_AB" .. ULT) or "3") .. " to unleash")
      elseif h.wait and not h.sent then
        SendAbility(n, false)
      end
      ES.exHold = nil
      ES.DrawAbilities()
    end
    return
  end
  if ES.placing then return end
  -- A key held down repeats nothing.
  if h and h.n == n then return end
  local s = ES.slots and ES.slots[n]
  -- A key armed for its recast, or empowered (its EX version, free): pressed, it goes at once
  -- (the server knows which it fires), with no hold to wait for.
  if ES.recasts[n] or ES.empowers[n] then
    SendAbility(n, false)
    return
  end
  if n == ULT and s and s.key ~= "" and not ex then
    ES.exHold = { n = n, at = GetTime(), wait = true, ult = true, sent = false }
    return
  end
  if ex or not s or s.key == "" or not s.ex then
    if ex and s and s.ex and not ES.ExAffordable(s) then
      ES.Flash(s.ex.name .. ": " .. (s.ex.valor or 0) .. " Valor")
    end
    SendAbility(n, ex)
    return
  end
  -- Not enough Valor for the EX version: the key is a plain press, at once.
  if not ES.ExAffordable(s) then
    SendAbility(n, false)
    return
  end
  local wait = s.shape == "self" or s.kind == "stance" or s.kind == "nova"
  ES.exHold = { n = n, at = GetTime(), wait = wait, sent = false }
  if not wait then SendAbility(n, false) end
end

-- Every frame: a key held past `EX_HOLD` sends its EX version (an aimed one only while it's
-- still the one aimed, not once a charge has begun).
function ES.StepExHold()
  local h = ES.exHold
  if not h or h.sent or GetTime() - h.at < EX_HOLD then return end
  h.sent = true
  local s = ES.slots and ES.slots[h.n]
  -- The ultimate, held long enough: unleashed.
  if h.ult then
    SendAbility(h.n, false)
    Play("click")
    ES.DrawAbilities()
    return
  end
  if not s or not s.ex then return end
  if not h.wait then
    local up = Aiming()
    if ES.attackHeld or (up and up ~= s.key) then return end
  end
  SendAbility(h.n, true)
  Play("click")
  ES.DrawAbilities()
end

-- The right mouse button on the stock client: the guard and the game's own mouse-look while it's
-- held.
function ES.Look(state)
  if not ES.awake then return end
  if state == "down" then
    ES.lookHeld = true
    ES.lookAt = GetTime()
    if MouselookStart then MouselookStart() end
    ES.Guard("down")
  else
    ES.lookHeld = false
    if MouselookStop then MouselookStop() end
    ES.Guard("up")
  end
end

-- The middle button on the stock client: the camera turns alone while it's held.
function ES.Orbit(state)
  if state == "down" then
    if ES.orbitHeld then return end
    ES.orbitHeld = true
    if ToggleMouseMove then ToggleMouseMove() end
  else
    if not ES.orbitHeld then return end
    ES.orbitHeld = false
    if ToggleMouseMove then ToggleMouseMove() end
  end
end

-- Sprint while it's held (Shift, or Space held past a tap); polled, not while typing.
function ES.Sprint(on)
  if not ES.awake or on == ES.sprintHeld then return end
  ES.sprintHeld = on
  if on then
    Say(".es in sp")
  else
    Say(".es in spu")
  end
end

-- The dodge (mouse button 4): a tap dodges as it's let go (the direction you're moving, or a step back); held, it
-- sprints.
function ES.Dodge(state)
  if not ES.awake then return end
  if state == "up" then
    if ES.spaceAt and not ES.spaceSprint then
      ES.PredictRoll()
      Say(".es in d")
    end
    ES.spaceAt = nil
    ES.spaceSprint = false
    return
  end
  -- A key bound without a key state (an old binding): dodge at once.
  if state ~= "down" then
    Say(".es in d")
    return
  end
  if not ES.spaceAt then
    ES.spaceAt = GetTime()
    ES.spaceSprint = false
  end
end

-- R: the weapon away (walk the camp with empty hands) or back in hand; while placing, turn the
-- station.
function ES.Sheathe()
  if not ES.awake then return end
  if ES.placing then
    Say(".es rot 1")
  else
    Say(".es in sh")
  end
end

-- V: a jump, which costs stamina: too winded, and the bar flashes instead.
function ES.Jump()
  local cost = Num(36, 0)
  if ES.awake and cost > 0 and Num(4, 0) < cost then
    if ElwynnSurvivalVitals and ElwynnSurvivalVitals.stamina then ElwynnSurvivalVitals.stamina:Flash() end
    return
  end
  -- The client's own jump, whichever name it has.
  if Jump then
    Jump()
  elseif JumpOrAscendStart then
    JumpOrAscendStart()
  end
end

-- Tab: lock on to the foe in front (the server picks it: the one you're fighting first); locked
-- already, a tap lets go. Ctrl+Tab (ES_NEXT) goes to the next foe in view.
function ES.Lock(state)
  if not ES.awake then return end
  if state == "up" then return end
  if ES.lockOn then
    Say(".es in unlock")
  else
    Say(".es in lock")
  end
end

function ES.NextFoe()
  if not ES.awake then return end
  -- Not locked on yet: this locks on.
  Say(".es in lock")
end

function ES.StepLockHold()
end

-- V: the health bars over heads, on and off (everyone's: foes' and friends').
function ES.Bars()
  if not ES.awake then return end
  local set = Settings()
  if set.noBars then
    set.noBars = nil
    if ShowNameplates then pcall(ShowNameplates) end
    if ShowFriendNameplates then pcall(ShowFriendNameplates) end
    ES.Message("Health bars over heads: on")
  else
    set.noBars = 1
    if HideNameplates then pcall(HideNameplates) end
    if HideFriendNameplates then pcall(HideFriendNameplates) end
    ES.Message("Health bars over heads: off (" .. (KeyName("ES_BARS") or "V") .. " brings them back)")
  end
end

-- F: use what's in front of you; while placing, put the station down.
function ES.Use()
  if not ES.awake then return end
  if ES.placing then
    Say(".es place")
  else
    Say(".es e")
  end
end

-- G: the next usable item in the bags becomes the quick item.
function ES.Cycle()
  if ES.awake then Say(".es in cy") end
end

-- T: the quick item (eat, a salve, a tonic); while placing, turn the station.
function ES.Eat()
  if not ES.awake then return end
  if ES.placing then
    Say(".es rot 1")
  else
    Say(".es in u")
  end
end

-- B: the backpack and the satchels.
function ES.Bag()
  if not ES.awake then return end
  ES.Open("bag")
end

-- N: the ability tree (the server sends it; again, it closes).
function ES.Tree()
  if not ES.awake then return end
  if ES.tree and ES.tree:IsShown() then
    ES.tree:Hide()
  else
    Say(".es tree")
  end
end

function ES.Build()
  if not ES.awake then return end
  if ES.below then
    ES.Message("Nothing is built down here.")
    return
  end
  if ES.placing then
    Say(".es cancel")
  elseif ES.win:IsShown() and ES.win.kind == "build" then
    ES.CloseWindows()
    ES.SetMode("action")
  else
    Say(".es bm")
  end
end

function ES.Rotate()
  if ES.awake and ES.placing then Say(".es rot 1") end
end

-- What's in hand: "melee" or "bow" (the server's word, as last heard: the bow while Aimed Shot
-- is drawn).
function ES.Kit()
  return (ES.h and ES.h[6]) or "melee"
end

-- --- the vitals: by the hero (bottom centre, above the abilities) or top left --------------------

local vit = CreateFrame("Frame", "ElwynnSurvivalVitals", UIParent)
vit:SetWidth(360)
vit:SetHeight(64)
vit:SetPoint("TOPLEFT", UIParent, "TOPLEFT", 30, -28)
vit:Hide()
ES.vitals = vit

-- Bars grow with what they hold, as the souls games' do.
-- In the corner the bars grow with what they hold, as the souls games' do; by the hero they're
-- the dock's width.
local HP_PER_PX = 0.75
local ST_PER_PX = 0.42
-- Tall enough to read how much is left at a glance, mid-fight.
local VIT_HEALTH, VIT_STAMINA = 13, 8
local DOCK_W = AB_N * 44 + (AB_N - 1) * 6
vit.health = MakeLagBar(vit, 200 / HP_PER_PX, VIT_HEALTH, { r = 0.66, g = 0.09, b = 0.07 })
vit.healthText = Text(vit.health, FONT_NUM, 11, "OUTLINE")
vit.healthText:SetPoint("RIGHT", vit.health, "RIGHT", -3, 0)
-- Room for "1000 / 1000" at any bar size (left to size itself, it was cut to "200 / 2...").
vit.healthText:SetWidth(120)
vit.healthText:SetJustifyH("RIGHT")
vit.healthText:SetTextColor(1, 0.92, 0.85)
vit.stamina = MakeLagBar(vit, 100 / ST_PER_PX, VIT_STAMINA, { r = 0.26, g = 0.66, b = 0.27 }, { r = 0.85, g = 0.95, b = 0.7 })

-- Lit while the guard is up, and when a swing would reach someone.
vit.guard = Icon(vit, "ARTWORK", "INV_Shield_09", 14)
vit.reach = Icon(vit, "ARTWORK", "Ability_DualWield", 14)
-- Not shown: a light for "a swing would land now" judged the distance for the player.
vit.reach:Hide()
vit.state = Text(vit, FONT_BODY, 11, "OUTLINE")
-- What a meal, a salve, a tonic or a stone is still doing, and the armor worn.
vit.effects = Text(vit, FONT_BODY, 10, "OUTLINE")
vit.effects:SetJustifyH("LEFT")
vit.effects:SetTextColor(0.75, 0.95, 0.7)

-- The two places and three sizes the Controls card offers.
local VITALS_SIZES = { 1.0, 1.25, 1.5 }
function ES.VitalsWhere()
  return (ElwynnSurvivalSettings and ElwynnSurvivalSettings.vitals) or "hero"
end
function ES.VitalsScale()
  return (ElwynnSurvivalSettings and ElwynnSurvivalSettings.vitalsScale) or 1.25
end

function ES.PlaceVitals()
  local scale = ES.VitalsScale()
  vit:SetScale(scale)
  if ES.PlaceDock then ES.PlaceDock() end
  vit:ClearAllPoints()
  vit.health:ClearAllPoints()
  vit.stamina:ClearAllPoints()
  vit.guard:ClearAllPoints()
  vit.reach:ClearAllPoints()
  vit.state:ClearAllPoints()
  vit.effects:ClearAllPoints()
  if ES.VitalsWhere() == "corner" then
    vit:SetHeight(64)
    vit:SetPoint("TOPLEFT", UIParent, "TOPLEFT", 30 / scale, -28 / scale)
    vit.health:SetPoint("TOPLEFT", vit, "TOPLEFT", 0, 0)
    vit.stamina:SetPoint("TOPLEFT", vit.health, "BOTTOMLEFT", 0, -7)
    vit.guard:SetPoint("TOPLEFT", vit.stamina, "BOTTOMLEFT", 0, -7)
    vit.reach:SetPoint("LEFT", vit.guard, "RIGHT", 5, 0)
    vit.state:SetPoint("LEFT", vit.guard, "RIGHT", 8, 0)
    vit.effects:SetPoint("TOPLEFT", vit.guard, "BOTTOMLEFT", 0, -5)
  else
    -- On the dock, over Valor and the keys, as wide as they are: health, then stamina; the guard
    -- and reach small to the left, the effects in a line over the health.
    vit:SetWidth(DOCK_W)
    vit:SetHeight(VIT_HEALTH + 4 + VIT_STAMINA)
    if ES.valor then
      vit:SetPoint("BOTTOM", ES.valor, "TOP", 0, 4)
    else
      vit:SetPoint("BOTTOM", UIParent, "BOTTOM", 0, 90 / scale)
    end
    vit.health:SetPoint("TOPLEFT", vit, "TOPLEFT", 0, 0)
    vit.stamina:SetPoint("TOPLEFT", vit.health, "BOTTOMLEFT", 0, -4)
    vit.reach:SetPoint("RIGHT", vit.health, "LEFT", -7, -3)
    vit.guard:SetPoint("RIGHT", vit.health, "LEFT", -7, -3)
    vit.state:SetPoint("BOTTOM", vit.health, "TOP", 0, 16)
    vit.effects:SetPoint("BOTTOMLEFT", vit.health, "TOPLEFT", 0, 4)
  end
  ES.DrawVitalsLength()
  if ES.PlaceTop then ES.PlaceTop() end
end
-- The bars' lengths: the dock's width by the hero, by what they hold in the corner.
function ES.DrawVitalsLength()
  local h = ES.h
  local maxhp, maxst = tonumber(h and h[3]) or 200, tonumber(h and h[5]) or 100
  if ES.VitalsWhere() == "corner" then
    vit.health:SetLength(Clamp(maxhp / HP_PER_PX, 120, 420))
    vit.stamina:SetLength(Clamp(maxst / ST_PER_PX, 100, 360))
  else
    vit.health:SetLength(DOCK_W)
    vit.stamina:SetLength(DOCK_W)
  end
end

ES.PlaceVitals()

-- EF: the effects, each name~seconds left~kind.
function ES.EffectsLine(f)
  local parts = Split(f[2] or "", "^")
  local words = {}
  for i = 1, table.getn(parts) do
    local e = Split(parts[i], "~")
    local secs = tonumber(e[2] or "")
    if e[1] and e[1] ~= "" and secs then
      table.insert(words, string.format("%s %d:%02d", e[1], math.floor(secs / 60), math.mod(secs, 60)))
    end
  end
  ES.effectText = table.concat(words, "   ")
  ES.DrawEffects()
end

function ES.DrawEffects()
  local armor = tonumber(ES.h and ES.h[33]) or 0
  local text = ES.effectText or ""
  if armor > 0 then
    text = (text ~= "" and (text .. "   ") or "") .. "Armor " .. armor .. "%"
  end
  vit.effects:SetText(text)
end

ES.hud = vit

-- The texture of the item called `name` in the backpack or the satchels, if it's there.
function ES.BagTexture(name)
  if not GetContainerItemLink or not GetContainerItemInfo then return nil end
  for bag = 0, 4 do
    local n = 16
    if GetContainerNumSlots then n = GetContainerNumSlots(bag) or 0 end
    for slot = 1, n do
      local ok, link = pcall(GetContainerItemLink, bag, slot)
      if ok and link and string.find(link, name, 1, true) then
        local tex = GetContainerItemInfo(bag, slot)
        if tex then return tex end
      end
    end
  end
  return nil
end

-- Short for a key on a slot: "LMB", "RMB", "MMB", else as written.
local function ShortKey(action)
  local k = KeyName(action)
  if not k then return "" end
  if k == "Left mouse" then return "LMB" end
  if k == "Right mouse" then return "RMB" end
  if k == "Middle mouse" then return "MMB" end
  return k
end
ES.ShortKey = ShortKey

-- --- doing something: a meal, a gather, a stagger ---------------------------------------------------

local act = CreateFrame("Frame", "ElwynnSurvivalAction", UIParent)
act:SetWidth(220)
act:SetHeight(30)
act:SetPoint("BOTTOM", UIParent, "BOTTOM", 0, 162)
act:Hide()
act.label = Text(act, FONT_BODY, 12, "OUTLINE")
act.label:SetPoint("TOP", act, "TOP", 0, 0)
act.bar = MakeBar(act, 200, 6, { r = 0.9, g = 0.9, b = 0.9 })
act.bar:SetPoint("TOP", act.label, "BOTTOM", 0, -4)
ES.actionBar = act

-- The word and colour for each thing a hero can be doing.
local LOOK = {
  ready = { 1, 1, 1, "" },
  swing = { 1, 0.55, 0.3, "" },
  heavy = { 1, 0.45, 0.2, "" },
  guard = { 0.45, 0.75, 1, "" },
  dodge = { 0.6, 1, 0.9, "" },
  aim = { 1, 0.9, 0.4, "" },
  charge = { 1, 0.8, 0.3, "" },
  cast = { 1, 0.85, 0.5, "" },
  channel = { 0.75, 0.6, 1, "" },
  stance = { 0.55, 0.85, 1, "" },
  dash = { 0.6, 1, 0.9, "" },
  eat = { 0.5, 1, 0.5, "Eating" },
  gather = { 0.5, 1, 0.5, "Gathering" },
  craft = { 1, 0.85, 0.5, "Making" },
  build = { 1, 0.85, 0.5, "Building" },
  repair = { 1, 0.85, 0.5, "Mending" },
  stagger = { 0.8, 0.4, 1, "Staggered!" },
  dead = { 0.5, 0.5, 0.5, "" },
}
-- The ones the bar shows (a cast, a channel and a stance by the ability's name).
local BARRED = { eat = 1, gather = 1, craft = 1, build = 1, repair = 1, stagger = 1, cast = 1, channel = 1, stance = 1, charge = 1 }

ES.action = { name = "ready", start = 0, total = 0, ability = "" }
ES.h = nil

-- The bar's word: the ability's name for a cast, a channel, a stance or a charge.
local function ActionWord(a, look)
  if a.ability and a.ability ~= "" and (a.name == "cast" or a.name == "channel" or a.name == "stance" or a.name == "charge") then
    local d = ES.AbilityInfo and ES.AbilityInfo(a.ability)
    return (d and d.name) or a.ability
  end
  -- Work at the camp: what it makes, builds or mends.
  if a.ability and a.ability ~= "" and (a.name == "craft" or a.name == "build" or a.name == "repair") then
    return look[4] .. " " .. a.ability
  end
  return look[4]
end

function ES.DrawAction()
  local a = ES.action
  local look = LOOK[a.name] or LOOK.ready
  vit.state:SetText(look[4])
  vit.state:SetTextColor(look[1], look[2], look[3])
  local barred = BARRED[a.name] and (a.total > 0.3 or a.name == "charge")
  -- A charge with nothing to charge (a tap) shows no bar.
  if a.name == "charge" then
    local d = ES.AbilityInfo and ES.AbilityInfo(a.ability)
    barred = d and d.hi > 0
  end
  if barred then
    act.label:SetText(ActionWord(a, look))
    act.label:SetTextColor(look[1], look[2], look[3])
    act.bar.fill:SetVertexColor(look[1], look[2], look[3], 0.95)
    act:Show()
  else
    act:Hide()
  end
  if a.name == "guard" then
    vit.guard:SetAlpha(1)
  else
    vit.guard:SetAlpha(0.25)
  end
  ES.DrawAimCamera()
  ES.DrawAim()
end

act:SetScript("OnUpdate", function()
  local a = ES.action
  if a.name == "charge" then
    -- The charge: filling to full, gold once it's past the least.
    local d = ES.AbilityInfo and ES.AbilityInfo(a.ability)
    if d and d.hi > 0 then
      local t = GetTime() - a.start
      act.bar:Set(t / d.hi)
      if t >= d.lo then
        act.bar.fill:SetVertexColor(1, 0.82, 0.3, 1)
      else
        act.bar.fill:SetVertexColor(0.7, 0.7, 0.7, 0.9)
      end
    end
  elseif a.total > 0 then
    local share = (GetTime() - a.start) / a.total
    -- What drains: a stagger, a channel and a stance.
    if a.name == "stagger" or a.name == "channel" or a.name == "stance" then
      share = 1 - share
    end
    act.bar:Set(share)
  end
end)

-- --- the reticle while aiming, and the hit marker ----------------------------------------------------
-- While an ability is aimed the camera comes in over the shoulder, and the middle of the screen
-- is where it goes (the server traces the reticle's line, as the client sends it). The ring
-- closes as a charge builds, and turns gold once it's past the least; grey while it can't go.
-- A ground ability's circle (and a cone's wedge) lies on the ground where the line meets it, and
-- a projectile's or a beam's path along it, on the clients that draw them (`SetAimMarker`).

local aim = CreateFrame("Frame", "ElwynnSurvivalAim", UIParent)
aim:SetWidth(64)
aim:SetHeight(64)
aim:SetPoint("CENTER", UIParent, "CENTER", 0, 0)
aim:SetFrameStrata("HIGH")
aim.ring = Tex(aim, "ARTWORK", "reticle", 56, 56)
aim.ring:SetPoint("CENTER", aim, "CENTER", 0, 0)
aim.dot = Tex(aim, "OVERLAY", "dot", 16, 16)
aim.dot:SetPoint("CENTER", aim, "CENTER", 0, 0)
aim.name = Text(aim, FONT_BODY, 11, "OUTLINE")
aim.name:SetPoint("TOP", aim, "BOTTOM", 0, -6)
aim.name:SetTextColor(1, 0.9, 0.6)
-- Full charge: a ring that bursts out from the reticle and fades.
aim.burst = Tex(aim, "OVERLAY", "reticle", 56, 56)
aim.burst:SetPoint("CENTER", aim, "CENTER", 0, 0)
aim.burst:Hide()
aim:Hide()
ES.aim = aim

-- Full charge, on our screen: the reticle bursts, the screen's edge flares in the school's
-- colour and the view kicks (the server plays the school's flare on the hero for everyone).
local empower = CreateFrame("Frame", "ElwynnSurvivalEmpower", UIParent)
empower:SetAllPoints(UIParent)
empower:SetFrameStrata("BACKGROUND")
empower:SetFrameLevel(2)
empower.edge = Tex(empower, "ARTWORK", "vignette")
empower.edge:SetAllPoints(empower)
empower:SetAlpha(0)
empower:Hide()
empower.flash = 0
empower:SetScript("OnUpdate", function()
  this.flash = math.max(0, this.flash - arg1 * 1.4)
  this:SetAlpha(this.flash)
  if this.flash <= 0 then this:Hide() end
end)
ES.empower = empower

local BURST_SECS = 0.45
function ES.ChargeFull(d)
  local look = d and COLUMN_LOOK[d.school or ""]
  local r, g, b = 1, 0.8, 0.3
  if look then r, g, b = look[2], look[3], look[4] end
  aim.burstAt = GetTime()
  aim.burstColor = { r, g, b }
  aim.burst:Show()
  empower.edge:SetVertexColor(r, g, b, 1)
  empower.flash = 0.55
  empower:Show()
  if ShakeCamera then ShakeCamera(1.8, 0.22) end
  local bar = ES.abar
  if d and bar and bar.slots[d.n] then bar.slots[d.n].fullAt = GetTime() end
end

-- The aimed ability's shape: "line", "circle", "cone" or "self".
function ES.AimShape()
  local d = Aiming() and ES.AbilityInfo and ES.AbilityInfo(Aiming())
  return (d and d.shape) or "line"
end

function ES.DrawAim()
  local key = ES.awake and Aiming()
  if key then
    local d = ES.AbilityInfo and ES.AbilityInfo(key)
    aim.name:SetText(d and d.name or "")
    aim.info = d
    aim:Show()
  else
    aim.info = nil
    aim:Hide()
  end
  ES.DrawAimMarker()
end

-- The marker an aimed ability gets: a ground ability's circle, a cone's wedge, and (on the eighth
-- round, which draws a strip) a projectile's or a beam's path from the feet along the view, its
-- range long and its radius either side. Nil: none (a dash, a tether, help for a friend).
function ES.AimMarkerShape(d)
  if not d then return nil end
  if d.shape == "circle" or d.shape == "cone" then return d.shape end
  if (d.kind == "projectile" or d.kind == "beam") and HasActionCombat and (tonumber(HasActionCombat()) or 0) >= 8 then
    return "line"
  end
  return nil
end

-- The ground marker under the reticle (the client patch's `SetAimMarker(shape, range, radius,
-- arc)`), off when nothing's aimed.
function ES.DrawAimMarker()
  if not SetAimMarker then return end
  local d = aim.info
  local shape = ES.AimMarkerShape(d)
  local want = shape and (shape .. d.range .. d.radius .. d.arc) or nil
  if want == ES.aimMarker then return end
  ES.aimMarker = want
  if want then
    SetAimMarker(shape, d.range, d.radius, d.arc)
  else
    SetAimMarker(nil)
  end
end

aim:SetScript("OnUpdate", function()
  local a = ES.action
  local d = this.info
  local size, r, g, b, alpha = 56, 0.9, 0.9, 0.85, 0.8
  if d and d.shape == "circle" then size = 40 end
  if a.name == "charge" and d and d.hi > 0 then
    local t = GetTime() - a.start
    local share = Clamp(t / d.hi, 0, 1)
    size = 56 - 32 * share
    if share >= 1 then
      -- Full: gold, with a slow beat, and once the burst.
      if not this.fullAt then
        this.fullAt = GetTime()
        ES.ChargeFull(d)
      end
      local beat = 0.85 + 0.15 * math.sin(GetTime() * 9)
      r, g, b, alpha = 1, 0.8, 0.3, beat
    elseif t >= d.lo then
      r, g, b = 1, 0.85, 0.4
    else
      r, g, b = 0.95, 0.95 - 0.1 * share, 0.85 - 0.45 * share
    end
  elseif d and not ES.AbilityReady(d) then
    r, g, b, alpha = 0.55, 0.55, 0.55, 0.6
  end
  if a.name ~= "charge" then this.fullAt = nil end
  if this.burstAt then
    local t = (GetTime() - this.burstAt) / BURST_SECS
    if t >= 1 then
      this.burstAt = nil
      this.burst:Hide()
    else
      local c = this.burstColor
      local grow = 40 + 150 * (1 - (1 - t) * (1 - t))
      this.burst:SetWidth(grow)
      this.burst:SetHeight(grow)
      this.burst:SetVertexColor(c[1], c[2], c[3], 1 - t)
    end
  end
  this.ring:SetWidth(size)
  this.ring:SetHeight(size)
  this.ring:SetVertexColor(r, g, b, alpha)
  this.dot:SetVertexColor(r, g, b, 0.9)
end)

local hit = Fader("ElwynnSurvivalHit", 64, 64, 0.2)
hit:SetPoint("CENTER", UIParent, "CENTER", 0, 0)
hit:SetFrameStrata("HIGH")
hit.mark = Tex(hit, "OVERLAY", "hitmark", 40, 40)
hit.mark:SetPoint("CENTER", hit, "CENTER", 0, 0)

-- A blow of yours landed: a white mark (gold and bigger for a heavy one), grey if it was blocked.
function ES.HitMark(kind, how)
  if how == "hit" then
    if kind == "heavy" then
      hit.mark:SetWidth(54)
      hit.mark:SetHeight(54)
      hit.mark:SetVertexColor(1, 0.8, 0.3, 1)
    else
      hit.mark:SetWidth(40)
      hit.mark:SetHeight(40)
      hit.mark:SetVertexColor(1, 1, 1, 0.95)
    end
  elseif how == "block" then
    hit.mark:SetWidth(30)
    hit.mark:SetHeight(30)
    hit.mark:SetVertexColor(0.6, 0.6, 0.6, 0.8)
  else
    return
  end
  hit.untilT = GetTime() + 0.28
  hit:SetAlpha(1)
  hit:Show()
end

-- --- the dock, bottom centre: the five ability keys, Valor, and (by the hero) health and stamina ---
-- One plate, one width: the five keys (Q ranged, E close, 1 utility, 2 defense, 3 ultimate), Valor over them
-- in its segments, and health and stamina over that, every bar as wide as the keys. Each key
-- shows its icon, its column's colour along its foot, its cooldown draining and its charges, a gold
-- pip while there's Valor for its EX version (and, with Ctrl held or the key held a moment, the
-- EX version itself), the key under it. The ultimate glows when the bar is full. The dock grows
-- with the health bars' size on the Controls card.

local AB_SIZE, AB_GAP = 44, 6
local dock = CreateFrame("Frame", "ElwynnSurvivalDock", UIParent)
dock:SetWidth(DOCK_W)
dock:SetHeight(AB_SIZE + 31)
dock:SetPoint("BOTTOM", UIParent, "BOTTOM", 0, 14)
dock:Hide()
ES.dock = dock
dock.plate = Tex(dock, "BACKGROUND", "dock")
dock.plate:SetVertexColor(0, 0, 0, 0.5)

local abar = CreateFrame("Frame", "ElwynnSurvivalAbilities", dock)
abar:SetWidth(DOCK_W)
abar:SetHeight(AB_SIZE)
abar:SetPoint("BOTTOM", dock, "BOTTOM", 0, 16)
ES.abar = abar
abar.slots = {}
ES.slots = {}
ES.abilities = {}
-- Keys armed for a recast (by key: the recast's look and aim, and when it closes) and keys
-- empowered (by key: when it ends), from the RC and EM lines; the recasts by their own key too,
-- for the reticle while one is aimed.
ES.recasts = {}
ES.empowers = {}
ES.recastInfo = {}

-- The quick item (T uses it, G picks the next) beside the keys: its icon from the bags and how
-- many are left (meals, when no item is picked).
dock.quick = MakeSlot(dock, 40, "T")
dock.quick:SetPoint("LEFT", abar, "RIGHT", 34, 0)
function ES.DrawQuick()
  local h, q = ES.h, dock.quick
  local quick, quickN = h and h[31] or "", tonumber(h and h[32]) or 0
  if quick == "" then
    local meals = tonumber(h and h[17]) or 0
    q:Set(ICONS .. "INV_Misc_Food_16", meals > 0, tostring(meals))
  else
    q:Set(ES.BagTexture(quick) or (ICONS .. "INV_Misc_Food_16"), quickN > 0, tostring(quickN))
  end
  q.key:SetText(ES.ShortKey("ES_EAT"))
end

-- One key's tile: a dark well, the icon, a fine rim, its column's colour along its foot, the
-- cooldown's shade, the key on a little cap under it.
local function MakeKey(parent, size)
  local b = CreateFrame("Frame", nil, parent)
  b:SetWidth(size)
  b:SetHeight(size)
  b.glow = Tex(b, "BACKGROUND", "circle", size + 26, size + 26)
  b.glow:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.glow:SetVertexColor(1, 0.78, 0.35, 0.45)
  b.glow:Hide()
  b.well = Tex(b, "BORDER", "square")
  b.well:SetAllPoints(b)
  b.well:SetVertexColor(0.02, 0.02, 0.02, 0.88)
  b.icon = b:CreateTexture(nil, "ARTWORK")
  b.icon:SetPoint("TOPLEFT", b, "TOPLEFT", 3, -3)
  b.icon:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -3, 3)
  b.icon:SetTexCoord(0.08, 0.92, 0.08, 0.92)
  b.shade = Tex(b, "ARTWORK", "bar")
  b.shade:SetPoint("TOPLEFT", b, "TOPLEFT", 3, -3)
  b.shade:SetPoint("TOPRIGHT", b, "TOPRIGHT", -3, -3)
  b.shade:SetHeight(1)
  b.shade:SetVertexColor(0, 0, 0, 0.7)
  b.shade:Hide()
  b.accent = Tex(b, "OVERLAY", "bar")
  b.accent:SetPoint("BOTTOMLEFT", b, "BOTTOMLEFT", 3, 3)
  b.accent:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -3, 3)
  b.accent:SetHeight(2)
  -- How far a held key has come toward its EX version (or the ultimate's unleashing).
  b.hold = Tex(b, "OVERLAY", "bar")
  b.hold:SetPoint("BOTTOMLEFT", b, "BOTTOMLEFT", 3, 3)
  b.hold:SetHeight(3)
  b.hold:SetVertexColor(1, 0.82, 0.4, 1)
  b.hold:Hide()
  b.rim = Tex(b, "OVERLAY", "squareline")
  b.rim:SetAllPoints(b)
  b.cd = Text(b, FONT_NUM, 16, "OUTLINE")
  b.cd:SetPoint("CENTER", b, "CENTER", 0, 0)
  b.cd:SetTextColor(1, 0.95, 0.8)
  b.count = Text(b, FONT_NUM, 11, "OUTLINE")
  b.count:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -4, 6)
  -- An empower's seconds left, small in the corner.
  b.timer = Text(b, FONT_NUM, 11, "OUTLINE")
  b.timer:SetPoint("TOPLEFT", b.icon, "TOPLEFT", 1, -1)
  b.timer:SetTextColor(1, 0.85, 0.4)
  -- The EX pip: a small gold diamond, lit with Valor enough.
  b.pip = Tex(b, "OVERLAY", "diamond", 9, 9)
  b.pip:SetPoint("TOPRIGHT", b, "TOPRIGHT", -3, -3)
  b.pip:Hide()
  b.cap = Tex(b, "BACKGROUND", "keycap", 22, 12)
  b.cap:SetPoint("TOP", b, "BOTTOM", 0, -3)
  b.cap:SetVertexColor(0.12, 0.11, 0.1, 0.92)
  b.keyText = Text(b, FONT_NUM, 10, "OUTLINE")
  b.keyText:SetPoint("CENTER", b.cap, "CENTER", 0, 0)
  b.keyText:SetTextColor(1, 0.88, 0.6)
  function b:Set(icon, lit, count)
    if icon then self.icon:SetTexture(icon) end
    if lit then
      self.icon:SetVertexColor(1, 1, 1)
      self.rim:SetVertexColor(FRAME.r, FRAME.g, FRAME.b, 0.95)
    else
      self.icon:SetVertexColor(0.42, 0.42, 0.42)
      self.rim:SetVertexColor(FRAME.r * 0.55, FRAME.g * 0.55, FRAME.b * 0.55, 0.8)
    end
    self.count:SetText(count or "")
  end
  return b
end

for n = 1, AB_N do
  local b = MakeKey(abar, AB_SIZE)
  b:SetPoint("BOTTOMLEFT", abar, "BOTTOMLEFT", (n - 1) * (AB_SIZE + AB_GAP), 0)
  b.n = n
  b:EnableMouse(true)
  b:SetScript("OnEnter", function() ES.AbilityTip(this) end)
  b:SetScript("OnLeave", function() if GameTooltip then GameTooltip:Hide() end end)
  b:SetScript("OnMouseUp", function() ES.Tree() end)
  abar.slots[n] = b
end

-- Valor, just over the keys, as wide: its segments marked, its number at its end.
local valor = CreateFrame("Frame", "ElwynnSurvivalValor", dock)
valor:SetWidth(DOCK_W)
valor:SetHeight(4)
valor:SetPoint("BOTTOMLEFT", abar, "TOPLEFT", 0, 8)
valor.bar = MakeLagBar(valor, DOCK_W, 4, { r = 1, g = 0.72, b = 0.22 }, { r = 1, g = 0.95, b = 0.75 })
valor.bar:SetPoint("TOPLEFT", valor, "TOPLEFT", 0, 0)
valor.text = Text(valor, FONT_NUM, 10, "OUTLINE")
valor.text:SetPoint("LEFT", valor, "RIGHT", 8, 0)
valor.text:SetTextColor(1, 0.82, 0.42)
valor.ticks = {}
valor.now, valor.max, valor.segment = 0, 100, 25
function ES.valorNow() return valor.now or 0 end
ES.valor = valor

-- Where the dock and the plate behind it go: the vitals over the keys (by the hero) or not (in
-- the corner), all at the size the Controls card says.
function ES.PlaceDock()
  local scale = ES.VitalsScale and ES.VitalsScale() or 1.25
  -- What you're doing, and what the use key would do, just over the dock.
  if ES.actionBar then
    ES.actionBar:ClearAllPoints()
    ES.actionBar:SetPoint("BOTTOM", dock.plate, "TOP", 0, 10)
  end
  if ES.prompt then
    ES.prompt:ClearAllPoints()
    ES.prompt:SetPoint("BOTTOM", dock.plate, "TOP", 0, 50)
  end
  dock:SetScale(scale)
  dock:ClearAllPoints()
  dock:SetPoint("BOTTOM", UIParent, "BOTTOM", 0, 14 / scale)
  dock.plate:ClearAllPoints()
  dock.plate:SetPoint("BOTTOMLEFT", dock, "BOTTOMLEFT", -14, -8)
  if ES.VitalsWhere and ES.VitalsWhere() ~= "corner" and ES.vitals then
    dock.plate:SetPoint("TOPRIGHT", ES.vitals, "TOPRIGHT", 14, 14)
  else
    dock.plate:SetPoint("TOPRIGHT", valor, "TOPRIGHT", 14, 10)
  end
end

ES.PlaceVitals()

function ES.ValorLine(f)
  local was = valor.now
  valor.now, valor.max, valor.segment = tonumber(f[2]) or 0, math.max(1, tonumber(f[3]) or 100), tonumber(f[4]) or 25
  valor.bar:Set(valor.now / valor.max, valor.now >= was)
  valor.text:SetText(string.format("%d", valor.now))
  -- The segments' marks.
  local count = math.floor(valor.max / math.max(1, valor.segment)) - 1
  for i = 1, math.max(count, table.getn(valor.ticks)) do
    local t = valor.ticks[i]
    if i <= count then
      if not t then
        t = Tex(valor.bar, "OVERLAY", "bar")
        t:SetWidth(2)
        t:SetHeight(4)
        t:SetVertexColor(0, 0, 0, 0.9)
        valor.ticks[i] = t
      end
      t:ClearAllPoints()
      t:SetPoint("LEFT", valor.bar, "LEFT", valor.bar.full * i * valor.segment / valor.max - 1, 0)
      t:Show()
    elseif t then
      t:Hide()
    end
  end
  -- A segment filled: the bar flares.
  if math.floor(valor.now / math.max(1, valor.segment)) > math.floor(was / math.max(1, valor.segment)) then
    valor.flareUntil = GetTime() + 0.5
  end
  ES.DrawAbilities()
end

valor:SetScript("OnUpdate", function()
  if this.flareUntil then
    local left = this.flareUntil - GetTime()
    if left <= 0 then
      this.flareUntil = nil
      this.bar.fill:SetVertexColor(1, 0.72, 0.22, 0.95)
    else
      this.bar.fill:SetVertexColor(1, 0.72 + 0.28 * left * 2, 0.22 + 0.6 * left * 2, 1)
    end
  end
end)

-- What an ability is, by its key (the bar's and the EX versions').
function ES.AbilityInfo(key)
  return key and (ES.abilities[key] or ES.recastInfo[key])
end

-- The key (1 to 4) an ability is on, if it is.
function ES.SlotOf(key)
  for n = 1, AB_N do
    local s = ES.slots[n]
    if s and s.key == key then return n end
  end
  return nil
end

-- Seconds before slot `s` can go again (0: now).
local function CooldownLeft(s)
  if not s or s.key == "" then return 0 end
  if s.charges > 0 then return 0 end
  return math.max(0, s.cdLeft - (GetTime() - s.at))
end

-- Free to go now, as far as this side knows: off cooldown, Valor enough.
function ES.AbilityReady(d)
  if not d then return false end
  local s = ES.slots[d.n]
  if s and CooldownLeft(s) > 0 then return false end
  return (valor.now or 0) + 0.001 >= (d.valor or 0)
end

-- AB: the five keys, each slot~ability~name~icon~kind~shape~least~full~cooldown left~cooldown~
-- charges~most~range~radius~arc~Valor~EX key~EX name~EX icon~EX Valor~EX kind~EX shape~EX range~
-- EX radius~EX arc~text~facts~EX text~EX facts~aimed~column.
function ES.AbilitiesLine(f)
  local parts = Split(f[2] or "", "^")
  ES.abilities = {}
  for n = 1, AB_N do
    local e = Split(parts[n] or "", "~")
    local s = {
      n = n, slot = e[1] or "", key = e[2] or "", name = e[3] or "", icon = e[4] or "",
      kind = e[5] or "", shape = e[6] or "line", lo = tonumber(e[7] or "") or 0, hi = tonumber(e[8] or "") or 0,
      cdLeft = tonumber(e[9] or "") or 0, cdTotal = tonumber(e[10] or "") or 0,
      charges = tonumber(e[11] or "") or 0, most = tonumber(e[12] or "") or 1,
      range = tonumber(e[13] or "") or 0, radius = tonumber(e[14] or "") or 0, arc = tonumber(e[15] or "") or 0,
      valor = tonumber(e[16] or "") or 0, text = e[26] or "", facts = e[27] or "", at = GetTime(),
      aimed = e[30] == "1", school = e[31] or "",
    }
    if (e[17] or "") ~= "" then
      s.ex = {
        n = n, key = e[17], name = e[18] or "", icon = e[19] or "", valor = tonumber(e[20] or "") or 0,
        kind = e[21] or "", shape = e[22] or "line", range = tonumber(e[23] or "") or 0,
        radius = tonumber(e[24] or "") or 0, arc = tonumber(e[25] or "") or 0, lo = s.lo, hi = s.hi,
        text = e[28] or "", facts = e[29] or "", isEx = true, school = s.school,
      }
      ES.abilities[s.ex.key] = s.ex
    end
    ES.slots[n] = s
    if s.key ~= "" then ES.abilities[s.key] = s end
  end
  ES.DrawAbilities()
  ES.DrawAim()
end

-- RC: the key in a slot (0 to 4) armed for its recast, for so many seconds: the recast's icon,
-- key, name, aim (shape, range, radius, arc, aimed) and charge; RC and the slot alone: closed.
function ES.RecastLine(f)
  local n = (tonumber(f[2] or "") or -1) + 1
  if n < 1 or n > AB_N then return end
  local secs = tonumber(f[3] or "")
  if secs then
    local r = {
      n = n, icon = f[4] or "", key = f[5] or "", name = f[6] or "", shape = f[7] or "line",
      range = tonumber(f[8] or "") or 0, radius = tonumber(f[9] or "") or 0, arc = tonumber(f[10] or "") or 0,
      aimed = f[11] == "1", lo = tonumber(f[12] or "") or 0, hi = tonumber(f[13] or "") or 0, valor = 0,
      ends = GetTime() + secs, isRecast = true,
    }
    ES.recasts[n] = r
    if r.key ~= "" then ES.recastInfo[r.key] = r end
  else
    ES.recasts[n] = nil
  end
  ES.DrawAbilities()
end

-- EM: the key in a slot (0 to 4) empowered for so many seconds (its next press is its EX version,
-- for no Valor); EM and the slot alone: spent or gone.
function ES.EmpowerLine(f)
  local n = (tonumber(f[2] or "") or -1) + 1
  if n < 1 or n > AB_N then return end
  local secs = tonumber(f[3] or "")
  if secs then
    ES.empowers[n] = GetTime() + secs
  else
    ES.empowers[n] = nil
  end
  ES.DrawAbilities()
end

local function Ctrl()
  return IsControlKeyDown and IsControlKeyDown() and true or false
end

function ES.DrawAbilities()
  local ctrl = Ctrl()
  ES.drawnCtrl = ctrl
  for n = 1, AB_N do
    local b = abar.slots[n]
    local s = ES.slots[n]
    b.keyText:SetText(ShortKey("ES_AB" .. n))
    b.glow:Hide()
    if not s or s.key == "" then
      b:Set(ICONS .. "INV_Misc_QuestionMark", false)
      b.icon:SetVertexColor(0.25, 0.25, 0.25)
      b.pip:Hide()
      b.accent:SetVertexColor(0.3, 0.3, 0.3, 0.5)
    else
      local look = COLUMN_LOOK[s.school]
      if look then
        b.accent:SetVertexColor(look[2], look[3], look[4], 0.9)
      else
        b.accent:SetVertexColor(0.6, 0.6, 0.6, 0.6)
      end
      local held = ES.exHold and ES.exHold.n == n and ES.exHold.sent and not ES.exHold.ult
      local rc, em = ES.recasts[n], ES.empowers[n]
      local d = ((ctrl or held or em) and s.ex) or s
      local ready = ES.AbilityReady(d)
      -- Empowered: its EX version, free (only its cooldown counts).
      if em and s.ex then ready = CooldownLeft(s) <= 0 end
      if rc then
        -- Armed for its recast: the recast's icon, lit (it costs nothing).
        b:Set(ICONS .. rc.icon, true, "")
      else
        b:Set(ICONS .. d.icon, ready, (s.most > 1) and tostring(s.charges) or "")
      end
      if s.ex and (valor.now or 0) + 0.001 >= s.ex.valor then
        b.pip:SetVertexColor(1, 0.8, 0.3, 1)
        b.pip:Show()
      elseif s.ex then
        b.pip:SetVertexColor(0.4, 0.4, 0.4, 0.8)
        b.pip:Show()
      else
        b.pip:Hide()
      end
      -- The ultimate with the bar full: it glows.
      b.glow:SetVertexColor(1, 0.78, 0.35, 0.45)
      if n == ULT and ready then
        b.glow:Show()
      end
      -- Empowered: it glows (and pulses, its seconds in the corner); armed for its recast, it
      -- glows green.
      if em then
        b.glow:Show()
      end
      if rc then
        b.glow:SetVertexColor(0.45, 1, 0.7, 0.6)
        b.glow:Show()
      end
      -- An aimed one up: its rim lit.
      local up = Aiming()
      if up and (up == s.key or (s.ex and up == s.ex.key) or (rc and up == rc.key)) then
        b.rim:SetVertexColor(1, 0.95, 0.6, 1)
        b.glow:Show()
      end
    end
  end
end

abar:SetScript("OnUpdate", function()
  if Ctrl() ~= ES.drawnCtrl then ES.DrawAbilities() end
  local now = GetTime()
  local h = ES.exHold
  for n = 1, AB_N do
    local b = this.slots[n]
    local s = ES.slots[n]
    local left = CooldownLeft(s)
    local rc, em = ES.recasts[n], ES.empowers[n]
    if rc then
      -- Armed for its recast: the seconds till it closes, no cooldown.
      local w = math.max(0, rc.ends - now)
      b.shade:Hide()
      if w >= 1 then
        b.cd:SetText(string.format("%d", math.ceil(w)))
      else
        b.cd:SetText(string.format("%.1f", w))
      end
      b.wasCooling = true
    elseif left > 0 and s.cdTotal > 0 then
      b.shade:SetHeight(math.max(1, (AB_SIZE - 6) * Clamp(left / s.cdTotal, 0, 1)))
      b.shade:Show()
      if left >= 1 then
        b.cd:SetText(string.format("%d", math.ceil(left)))
      else
        b.cd:SetText(string.format("%.1f", left))
      end
      b.wasCooling = true
    else
      b.shade:Hide()
      b.cd:SetText("")
      if b.wasCooling then
        -- Back: lit again.
        b.wasCooling = nil
        ES.DrawAbilities()
      end
    end
    if em then
      b.timer:SetText(string.format("%d", math.ceil(math.max(0, em - now))))
    else
      b.timer:SetText("")
    end
    -- A key held toward its EX version: the line along its foot fills.
    if h and h.n == n and not h.sent and (s and (s.ex or n == ULT)) then
      local share = Clamp((now - h.at) / EX_HOLD, 0, 1)
      b.hold:SetWidth(math.max(1, (AB_SIZE - 6) * share))
      b.hold:Show()
    else
      b.hold:Hide()
    end
    if b.fullAt then
      -- Topped out: the key flares a moment.
      local t = (now - b.fullAt) / 0.5
      if t >= 1 then
        b.fullAt = nil
        ES.DrawAbilities()
      else
        b.glow:Show()
        b.glow:SetAlpha(1 - t)
      end
    elseif (rc or em) and b.glow:IsShown() then
      b.glow:SetAlpha(0.6 + 0.35 * math.sin(now * (em and 6 or 3)))
    elseif n == ULT and b.glow:IsShown() and not Aiming() then
      b.glow:SetAlpha(0.55 + 0.35 * math.sin(now * 3))
    end
  end
end)

-- Hovered (with the mouse free): what it does, regular and charged up, in words and numbers.
function ES.AbilityTip(b)
  if not GameTooltip or not GameTooltip.SetOwner then return end
  local s = ES.slots[b.n]
  GameTooltip:SetOwner(b, "ANCHOR_TOP")
  if not s or s.key == "" then
    GameTooltip:SetText("Nothing on this key")
    GameTooltip:AddLine("Open the board (" .. (KeyName("ES_TREE") or "N") .. ") and choose a card for it.", 0.8, 0.8, 0.8, 1)
    GameTooltip:Show()
    return
  end
  local look = COLUMN_LOOK[s.school] or { "", 1, 0.82, 0.45 }
  GameTooltip:SetText(s.name, look[2], look[3], look[4])
  local row = ROW_WORDS[s.slot]
  if row then GameTooltip:AddLine(row[1] .. ((look[1] ~= "") and ("  ·  " .. look[1]) or ""), 0.6, 0.58, 0.54) end
  GameTooltip:AddLine(ES.HowText(s.slot, s.aimed, s.hi, b.n), 0.62, 0.72, 0.8, 1)
  if s.text ~= "" then GameTooltip:AddLine(s.text, 0.92, 0.9, 0.85, 1) end
  if s.facts ~= "" then GameTooltip:AddLine(s.facts, 1, 0.82, 0.5, 1) end
  local left = CooldownLeft(s)
  if left > 0 then GameTooltip:AddLine(string.format("Ready in %.0f s", math.ceil(left)), 1, 0.45, 0.4) end
  if s.ex then
    GameTooltip:AddLine(" ")
    local enough = (valor.now or 0) + 0.001 >= s.ex.valor
    GameTooltip:AddDoubleLine("Charged up: " .. s.ex.name, s.ex.valor .. " Valor", 1, 0.7, 0.35, enough and 0.5 or 1, enough and 1 or 0.45, enough and 0.5 or 0.4)
    GameTooltip:AddLine(ES.ExHowText(b.n, s.ex.valor), 0.62, 0.72, 0.8, 1)
    if s.ex.text ~= "" then GameTooltip:AddLine(s.ex.text, 0.92, 0.9, 0.85, 1) end
    if s.ex.facts ~= "" then GameTooltip:AddLine(s.ex.facts, 1, 0.82, 0.5, 1) end
  elseif s.slot == "ultimate" then
    GameTooltip:AddLine(string.format("Valor %d of %d", valor.now or 0, valor.max or 100), 1, 0.82, 0.45)
  end
  GameTooltip:Show()
end

-- --- crowd control on you, what F would do, healing --------------------------------------------------

local CC_WORDS = {
  stun = { "Stunned", 1, 0.85, 0.3, true },
  incapacitate = { "Incapacitated", 1, 0.7, 0.3, true },
  petrify = { "Frozen solid", 0.6, 0.85, 1, true },
  panic = { "Panicked", 1, 0.45, 0.8, true },
  root = { "Rooted", 0.55, 0.9, 0.45 },
  snare = { "Slowed", 0.6, 0.75, 1 },
  silence = { "Silenced", 0.8, 0.6, 1 },
  weaken = { "Weakened", 0.85, 0.6, 0.5 },
  stagger = { "Staggered", 0.8, 0.4, 1 },
  burn = { "Burning", 1, 0.5, 0.2 },
  mark = { "Marked", 0.75, 0.55, 1 },
  deathmark = { "Marked for death", 0.85, 0.35, 1 },
  onslaught = { "Onslaught", 1, 0.55, 0.3 },
  hallowed = { "Hallowed", 1, 0.9, 0.55 },
  steadfast = { "Steadfast", 1, 0.85, 0.45 },
  grace = { "Unshaken", 0.85, 0.85, 0.85 },
  stance = { "Countering melee blows", 1, 0.9, 0.5 },
}

local cc = CreateFrame("Frame", "ElwynnSurvivalCc", UIParent)
cc:SetWidth(300)
cc:SetHeight(50)
cc:SetPoint("CENTER", UIParent, "CENTER", 0, -80)
cc:Hide()
cc.big = Text(cc, FONT_TITLE, 22, "OUTLINE")
cc.big:SetPoint("TOP", cc, "TOP", 0, 0)
cc.small = Text(cc, FONT_BODY, 11, "OUTLINE")
cc.small:SetPoint("TOP", cc.big, "BOTTOM", 0, -3)
cc.list = {}
ES.cc = cc

-- CC: the crowd control on the hero, each kind~seconds left~share.
function ES.CcLine(f)
  cc.list = {}
  local parts = Split(f[2] or "", "^")
  for i = 1, table.getn(parts) do
    local e = Split(parts[i], "~")
    local secs = tonumber(e[2] or "")
    if e[1] and e[1] ~= "" and secs then
      table.insert(cc.list, { kind = e[1], untilT = GetTime() + secs, share = tonumber(e[3] or "") or 0 })
    end
  end
  if table.getn(cc.list) > 0 and ES.awake then cc:Show() else cc:Hide() end
end

cc:SetScript("OnUpdate", function()
  local now = GetTime()
  local big, small = nil, {}
  for i = 1, table.getn(this.list) do
    local c = this.list[i]
    local left = c.untilT - now
    if left > 0 then
      local w = CC_WORDS[c.kind] or { c.kind, 0.9, 0.9, 0.9 }
      local word = string.format("%s %.1f", w[1], left)
      local colored = string.format("|cff%02x%02x%02x%s|r", math.floor(w[2] * 255), math.floor(w[3] * 255), math.floor(w[4] * 255), word)
      if w[5] and not big then
        big = { word, w[2], w[3], w[4] }
      else
        table.insert(small, colored)
      end
    end
  end
  if big then
    this.big:SetText(big[1])
    this.big:SetTextColor(big[2], big[3], big[4])
  else
    this.big:SetText("")
  end
  this.small:SetText(table.concat(small, "   "))
  if not big and table.getn(small) == 0 then this:Hide() end
end)

-- NP: what the use key would do now ("NP\tverb\tname"; a bare NP: nothing in reach).
local np = CreateFrame("Frame", "ElwynnSurvivalPrompt", UIParent)
np:SetWidth(320)
np:SetHeight(26)
np:SetPoint("BOTTOM", UIParent, "BOTTOM", 0, 204)
np:Hide()
-- The words centred, the key's tile just left of them, on a soft dark band.
np.band = Tex(np, "BACKGROUND", "band")
np.band:SetPoint("TOPLEFT", np, "TOPLEFT", -40, 2)
np.band:SetPoint("BOTTOMRIGHT", np, "BOTTOMRIGHT", 40, -2)
np.band:SetVertexColor(0, 0, 0, 0.5)
np.text = Text(np, FONT_BODY, 13, "OUTLINE")
np.text:SetPoint("CENTER", np, "CENTER", 15, 0)
np.text:SetTextColor(0.95, 0.92, 0.85)
np.key = MakeSlot(np, 24, "")
np.key:SetPoint("RIGHT", np.text, "LEFT", -8, 0)
np.key.icon:Hide()
np.key.word = Text(np.key, FONT_NUM, 12, "OUTLINE")
np.key.word:SetPoint("CENTER", np.key, "CENTER", 0, 0)
np.key.word:SetTextColor(1, 0.9, 0.6)
ES.prompt = np

function ES.PromptLine(f)
  local verb, name = f[2] or "", f[3] or ""
  if verb == "" or not ES.awake then
    np:Hide()
    return
  end
  np.key.word:SetText(ShortKey("ES_USE"))
  np.text:SetText(verb .. " " .. name)
  np.text:SetWidth((np.text:GetStringWidth() or 0) + 4)
  np:SetWidth(math.max(120, 40 + (np.text:GetStringWidth() or 0)))
  np:Show()
end

-- HL: healed this much: a green number rising off the health bar.
local heal = CreateFrame("Frame", "ElwynnSurvivalHeal", UIParent)
heal:SetWidth(120)
heal:SetHeight(24)
heal:SetFrameStrata("HIGH")
heal:Hide()
heal.text = Text(heal, FONT_NUM, 18, "OUTLINE")
heal.text:SetPoint("CENTER", heal, "CENTER", 0, 0)
heal.text:SetTextColor(0.4, 1, 0.45)
heal.sum = 0
ES.heal = heal

function ES.HealLine(f)
  local n = tonumber(f[2] or "") or 0
  if n <= 0 then return end
  local now = GetTime()
  -- A tick on top of one still showing adds to it.
  if heal:IsShown() and now - (heal.at or 0) < 0.8 then
    heal.sum = heal.sum + n
  else
    heal.sum = n
    heal.born = now
  end
  heal.at = now
  heal.text:SetText("+" .. heal.sum)
  heal:SetAlpha(1)
  heal:Show()
end

heal:SetScript("OnUpdate", function()
  local t = GetTime() - (this.born or 0)
  local since = GetTime() - (this.at or 0)
  if since > 1.4 then
    this:Hide()
    return
  end
  this:ClearAllPoints()
  this:SetPoint("BOTTOM", vit.health, "TOP", 0, 4 + math.min(t, 1.4) * 18)
  if since > 0.9 then this:SetAlpha((1.4 - since) / 0.5) end
end)

-- --- the foe's bar, top centre ---------------------------------------------------------------------
-- Up top, clear of the abilities and the hero's own bars: the champion's bar when one fights
-- near, and under it the foe you're trading blows with (never the champion twice).

-- How far down the top bars start: clear of the health bars when they're top left.
local function TopY()
  return (ES.VitalsWhere() == "corner") and -118 or -38
end

local foe = CreateFrame("Frame", "ElwynnSurvivalFoe", UIParent)
foe:SetWidth(440)
foe:SetHeight(30)
foe:SetPoint("TOP", UIParent, "TOP", 0, -38)
foe:Hide()
foe.name = Text(foe, FONT_TITLE, 15, "OUTLINE")
foe.name:SetPoint("TOPLEFT", foe, "TOPLEFT", 0, 0)
foe.name:SetTextColor(0.95, 0.9, 0.82)
foe.bar = MakeLagBar(foe, 440, 9, { r = 0.62, g = 0.08, b = 0.06 })
foe.bar:SetPoint("BOTTOMLEFT", foe, "BOTTOMLEFT", 0, 0)
foe.value = Text(foe, FONT_NUM, 10, "OUTLINE")
foe.value:SetPoint("TOPRIGHT", foe, "TOPRIGHT", 0, -3)
foe.value:SetTextColor(0.8, 0.8, 0.8)
ES.foe = foe

-- The two top bars' places: the champion's first, the foe's under it (or in its place).
function ES.PlaceTop()
  local top = TopY()
  local b = ES.boss
  if b then
    b:ClearAllPoints()
    b:SetPoint("TOP", UIParent, "TOP", 0, top)
  end
  foe:ClearAllPoints()
  if b and b:IsShown() then
    foe:SetPoint("TOP", UIParent, "TOP", 0, top - 58)
  else
    foe:SetPoint("TOP", UIParent, "TOP", 0, top)
  end
end

function ES.DrawFoe()
  local h = ES.h
  local name = h and h[18] or ""
  -- The champion already has its bar: not a second one for it.
  local b = ES.boss
  if name ~= "" and b and b:IsShown() and b.who then
    local max = tonumber(h[20]) or 0
    if b.who == name or (b.most and b.most == max) then
      foe.fadeAt, foe.who = nil, nil
      foe:Hide()
      return
    end
  end
  ES.PlaceTop()
  if name == "" then
    if foe:IsShown() then
      foe.fadeAt = foe.fadeAt or (GetTime() + 0.6)
    end
    return
  end
  foe.fadeAt = nil
  foe:SetAlpha(1)
  local hp, max = tonumber(h[19]) or 0, tonumber(h[20]) or 1
  if foe.who ~= name then
    foe.who = name
    foe.name:SetText(name)
    foe.bar:Set(hp / math.max(1, max), true)
  else
    foe.bar:Set(hp / math.max(1, max))
  end
  foe.value:SetText(hp .. " / " .. max)
  foe:Show()
end

foe:SetScript("OnUpdate", function()
  if this.fadeAt then
    local left = this.fadeAt - GetTime()
    if left <= 0 then
      this.fadeAt = nil
      this.who = nil
      this:Hide()
    else
      this:SetAlpha(left / 0.6)
    end
  end
end)

-- --- hurt: the red edge -------------------------------------------------------------------------------

local hurt = CreateFrame("Frame", "ElwynnSurvivalHurt", UIParent)
hurt:SetAllPoints(UIParent)
-- Over the world and the background frames (it was under all of them), under the HUD's MEDIUM
-- strata so the bars stay readable.
hurt:SetFrameStrata("LOW")
hurt:SetFrameLevel(1)
-- The vignette's red band (its outer 40 of 256 px) in eight pieces at a fixed 150 px, so it's as
-- deep at the top and bottom as at the sides (stretched whole, the sides got 213 px and the top
-- 120 on a wide screen); no middle.
hurt.pieces = {}
do
  local E, c = 150, 40 / 256
  local cut = { { 0, c }, { c, 1 - c }, { 1 - c, 1 } }
  local X = { { "LEFT", 0, "LEFT", E }, { "LEFT", E, "RIGHT", -E }, { "RIGHT", -E, "RIGHT", 0 } }
  local Y = { { "TOP", 0, "TOP", -E }, { "TOP", -E, "BOTTOM", E }, { "BOTTOM", E, "BOTTOM", 0 } }
  for row = 1, 3 do
    for col = 1, 3 do
      if row ~= 2 or col ~= 2 then
        local t = Tex(hurt, "ARTWORK", "vignette")
        t:SetTexCoord(cut[col][1], cut[col][2], cut[row][1], cut[row][2])
        t:SetPoint("TOPLEFT", hurt, Y[row][1] .. X[col][1], X[col][2], Y[row][2])
        t:SetPoint("BOTTOMRIGHT", hurt, Y[row][3] .. X[col][3], X[col][4], Y[row][4])
        t:SetVertexColor(0.7, 0.02, 0.02, 1)
        table.insert(hurt.pieces, t)
      end
    end
  end
end
hurt:SetAlpha(0)
hurt:Hide()
hurt.flash = 0
ES.hurt = hurt

-- A blow: the edge flares by how much it took.
function ES.Hurt(share)
  hurt.flash = math.max(hurt.flash, Clamp(0.25 + share * 1.5, 0.25, 0.6))
  hurt:Show()
end

hurt:SetScript("OnUpdate", function()
  this.flash = math.max(0, this.flash - arg1 * 1.6)
  -- Low on health: a pulse that stays, deeper the nearer the end (it was a faint 0.06-0.18 shimmer
  -- at any share under 30%).
  local low = 0
  local h = ES.h
  if h then
    local share = (tonumber(h[2]) or 0) / math.max(1, tonumber(h[3]) or 1)
    if share > 0 and share < 0.3 then
      local k = 1 - share / 0.3
      low = (0.3 + 0.35 * k) * (0.75 + 0.25 * math.sin(GetTime() * 5))
    end
  end
  this:SetAlpha(math.max(this.flash, low))
end)

-- --- the corner: raid, what you dropped, the mode; the kill feed -----------------------------------------

local info = CreateFrame("Frame", "ElwynnSurvivalInfo", UIParent)
info:SetWidth(280)
info:SetHeight(170)
info:SetPoint("TOPRIGHT", UIParent, "TOPRIGHT", -30, -30)
info:Hide()
info.raid = Text(info, FONT_TITLE, 16, "OUTLINE")
info.raid:SetPoint("TOPRIGHT", info, "TOPRIGHT", 0, 0)
info.raid:SetJustifyH("RIGHT")
info.bags = Text(info, FONT_BODY, 12, "OUTLINE")
info.bags:SetPoint("TOPRIGHT", info.raid, "BOTTOMRIGHT", 0, -4)
info.cache = CreateFrame("Frame", nil, info)
info.cache:SetWidth(200)
info.cache:SetHeight(32)
info.cache:SetPoint("TOPRIGHT", info.bags, "BOTTOMRIGHT", 0, -6)
-- Siege: your gold and every camp's Hearth, under the rest.
info.siege = Text(info, FONT_BODY, 12, "OUTLINE")
info.siege:SetPoint("TOPRIGHT", info.cache, "BOTTOMRIGHT", 0, -4)
info.siege:SetJustifyH("RIGHT")
info.cache.arrow = info.cache:CreateTexture(nil, "ARTWORK")
info.cache.arrow:SetTexture("Interface\\Minimap\\MinimapArrow")
info.cache.arrow:SetWidth(30)
info.cache.arrow:SetHeight(30)
info.cache.arrow:SetPoint("RIGHT", info.cache, "RIGHT", 0, 0)
info.cache.text = Text(info.cache, FONT_BODY, 12, "OUTLINE")
info.cache.text:SetPoint("RIGHT", info.cache.arrow, "LEFT", -4, 0)
info.cache.text:SetTextColor(1, 0.85, 0.4)
info.cache:Hide()
-- (For the harness: what the corner's arrow says, if shown.)
function ES.infoCacheText() return info.cache:IsShown() and info.cache.text:GetText() or "hidden" end
info.mode = Text(info, FONT_BODY, 11, "OUTLINE")
info.mode:SetPoint("TOPRIGHT", info.cache, "BOTTOMRIGHT", 0, -6)
info.mode:SetTextColor(0.75, 0.75, 0.75)
-- The camp's journal: the goal it's on, how far along, and what to do.
info.goal = Text(info, FONT_BODY, 12, "OUTLINE")
info.goal:SetPoint("TOPRIGHT", info.mode, "BOTTOMRIGHT", 0, -6)
info.goal:SetJustifyH("RIGHT")
info.goal:SetTextColor(1, 0.86, 0.5)
info.goalText = Text(info, FONT_BODY, 10, "OUTLINE")
info.goalText:SetPoint("TOPRIGHT", info.goal, "BOTTOMRIGHT", 0, -3)
info.goalText:SetWidth(270)
info.goalText:SetJustifyH("RIGHT")
info.goalText:SetTextColor(0.85, 0.85, 0.8)

-- GL: the journal's goal: its place, how many, its title, what to do, how far along.
function ES.GoalLine(f)
  local n, total, title, text, have, need = tonumber(f[2]) or 0, tonumber(f[3]) or 0, f[4] or "", f[5] or "", tonumber(f[6]) or 0, tonumber(f[7]) or 0
  if title == "" then
    info.goal:SetText(total > 0 and "Journal done: the camp stands" or "")
    info.goalText:SetText("")
    return
  end
  local progress = need > 1 and string.format(" (%d/%d)", have, need) or ""
  info.goal:SetText(string.format("Goal %d/%d: %s%s", n, total, title, progress))
  info.goalText:SetText(ES.Keyed(text))
end

-- --- a champion's bar, across the top ---------------------------------------------------------

local boss = CreateFrame("Frame", "ElwynnSurvivalBoss", UIParent)
boss:SetWidth(600)
boss:SetHeight(40)
boss:SetPoint("TOP", UIParent, "TOP", 0, -38)
boss:Hide()
boss.name = Text(boss, FONT_TITLE, 19, "OUTLINE")
boss.name:SetPoint("TOP", boss, "TOP", 0, 0)
boss.name:SetTextColor(1, 0.88, 0.62)
boss.bar = MakeLagBar(boss, 600, 12, { r = 0.62, g = 0.07, b = 0.05 })
boss.bar:SetPoint("BOTTOMLEFT", boss, "BOTTOMLEFT", 0, 0)
boss.value = Text(boss, FONT_NUM, 10, "OUTLINE")
boss.value:SetPoint("BOTTOMRIGHT", boss.bar, "TOPRIGHT", 0, 4)
boss.value:SetTextColor(0.8, 0.8, 0.8)
-- The ends of the bar, set off like a champion's.
boss.capL = Tex(boss, "OVERLAY", "diamond", 14, 14)
boss.capL:SetPoint("CENTER", boss.bar, "LEFT", -6, 0)
boss.capL:SetVertexColor(FRAME.r, FRAME.g, FRAME.b, 0.95)
boss.capR = Tex(boss, "OVERLAY", "diamond", 14, 14)
boss.capR:SetPoint("CENTER", boss.bar, "RIGHT", 6, 0)
boss.capR:SetVertexColor(FRAME.r, FRAME.g, FRAME.b, 0.95)
ES.boss = boss

-- BS: the champion fighting near: its name, health, most health, and whether it's in its second
-- wind; nothing, none.
function ES.BossLine(f)
  local name, hp, maxhp = f[2], tonumber(f[3] or ""), tonumber(f[4] or "")
  if not name or name == "" or not hp or not maxhp then
    boss:Hide()
    boss.who, boss.most = nil, nil
    ES.DrawFoe()
    return
  end
  local fresh = boss.who ~= name
  boss.who, boss.most = name, maxhp
  boss.name:SetText(name)
  boss.value:SetText(hp .. " / " .. maxhp)
  if f[5] == "1" then
    boss.bar.color = { r = 0.85, g = 0.2, b = 0.02 }
  else
    boss.bar.color = { r = 0.62, g = 0.07, b = 0.05 }
  end
  boss.bar.fill:SetVertexColor(boss.bar.color.r, boss.bar.color.g, boss.bar.color.b, 0.97)
  boss.bar:Set(hp / math.max(1, maxhp), fresh)
  if ES.awake then boss:Show() end
  ES.DrawFoe()
end

function ES.DrawMode()
  if ES.mode == "action" then
    info.mode:SetText("")
  else
    info.mode:SetText("Mouse free: Z to fight again")
  end
end

local FEED_ROWS = 4
local feed = CreateFrame("Frame", "ElwynnSurvivalFeed", UIParent)
feed:SetWidth(300)
feed:SetHeight(FEED_ROWS * 24)
feed:SetPoint("TOPRIGHT", info, "BOTTOMRIGHT", 0, -6)
feed:Hide()
feed.rows = {}
for i = 1, FEED_ROWS do
  local r = CreateFrame("Frame", nil, feed)
  r:SetWidth(300)
  r:SetHeight(22)
  r:SetPoint("TOPRIGHT", feed, "TOPRIGHT", 0, -(i - 1) * 24)
  r.bg = Tex(r, "BACKGROUND", "band")
  r.bg:SetPoint("TOPLEFT", r, "TOPLEFT", -40, 0)
  r.bg:SetPoint("BOTTOMRIGHT", r, "BOTTOMRIGHT", 20, 0)
  r.bg:SetVertexColor(0, 0, 0, 0.6)
  r.victim = Text(r, FONT_BODY, 12, "OUTLINE")
  r.victim:SetPoint("RIGHT", r, "RIGHT", 0, 0)
  r.icon = Icon(r, "ARTWORK", "Ability_DualWield", 16)
  r.icon:SetPoint("RIGHT", r.victim, "LEFT", -6, 0)
  r.killer = Text(r, FONT_BODY, 12, "OUTLINE")
  r.killer:SetPoint("RIGHT", r.icon, "LEFT", -6, 0)
  r:Hide()
  feed.rows[i] = r
end
feed.entries = {}
ES.feed = feed

function ES.DrawFeed()
  local now = GetTime()
  local live = {}
  for i = 1, table.getn(feed.entries) do
    local e = feed.entries[i]
    if e.untilT > now then
      table.insert(live, e)
    end
  end
  feed.entries = live
  local me = UnitName and UnitName("player")
  for i = 1, FEED_ROWS do
    local r = feed.rows[i]
    local e = live[i]
    if e then
      r.killer:SetText(e.killer)
      r.victim:SetText(e.victim)
      if e.victim == me then
        r.victim:SetTextColor(1, 0.35, 0.3)
      else
        r.victim:SetTextColor(0.95, 0.9, 0.85)
      end
      if e.killer == me then
        r.killer:SetTextColor(1, 0.85, 0.4)
      else
        r.killer:SetTextColor(0.95, 0.9, 0.85)
      end
      if e.killer == "" then
        r.icon:SetTexture(ICONS .. "INV_Misc_Bone_HumanSkull_01")
      else
        r.icon:SetTexture(ICONS .. "Ability_DualWield")
      end
      local left = e.untilT - now
      r:SetAlpha(Clamp(left / 0.8, 0, 1))
      r:Show()
    else
      r:Hide()
    end
  end
end

function ES.Feed(killer, victim)
  table.insert(feed.entries, 1, { killer = killer or "", victim = victim or "", untilT = GetTime() + 7 })
  while table.getn(feed.entries) > FEED_ROWS do
    table.remove(feed.entries)
  end
  ES.DrawFeed()
end

feed:SetScript("OnUpdate", function()
  if table.getn(feed.entries) > 0 then
    ES.DrawFeed()
  end
end)

function ES.DrawHud()
  local h = ES.h
  if not h then return end
  local hp, maxhp = tonumber(h[2]) or 0, tonumber(h[3]) or 1
  local st, maxst = tonumber(h[4]) or 0, tonumber(h[5]) or 1
  ES.DrawVitalsLength()
  -- A blow taken: the red edge.
  if ES.lastHp and hp < ES.lastHp then
    ES.Hurt((ES.lastHp - hp) / math.max(1, maxhp))
  end
  ES.lastHp = hp
  vit.health:Set(hp / math.max(1, maxhp))
  vit.healthText:SetText(hp .. " / " .. maxhp)
  vit.stamina:Set(st / math.max(1, maxst))
  local reach = tonumber(h[16]) or 0
  if reach > 0 then
    vit.reach:SetAlpha(1)
  else
    vit.reach:SetAlpha(0.25)
  end
  info.bags:SetText("")
  ES.bar.bag.count:SetText((h[8] or "?") .. "/" .. (h[9] or "?"))
  ES.DrawQuick()
  ES.DrawAimCamera()
  ES.DrawAim()
  ES.DrawFoe()
  local cd = tonumber(h[10]) or -1
  -- Your supplies first; else the nearest of Elwynn's places not yet found.
  local pd = tonumber(h[38] or "") or -1
  if cd >= 0 then
    info.cache:Show()
    info.cache.text:SetText("Your supplies: " .. cd .. " yd")
    info.cache.text:SetTextColor(1, 0.85, 0.4)
    SetRotation(info.cache.arrow, math.rad(tonumber(h[11]) or 0))
  elseif pd >= 0 and h[37] and h[37] ~= "" then
    info.cache:Show()
    info.cache.text:SetText("Unexplored: " .. h[37] .. " (" .. pd .. " yd)")
    info.cache.text:SetTextColor(0.6, 0.85, 1)
    SetRotation(info.cache.arrow, math.rad(tonumber(h[39]) or 0))
  elseif ES.below and h[40] and h[40] ~= "" then
    -- Below, with nothing else to point to: which way north is.
    info.cache:Show()
    info.cache.text:SetText("North")
    info.cache.text:SetTextColor(0.85, 0.85, 0.85)
    SetRotation(info.cache.arrow, math.rad(tonumber(h[40]) or 0))
  else
    info.cache:Hide()
  end
  local phase, secs, left = h[12], tonumber(h[13]) or 0, tonumber(h[14]) or 0
  if ES.siegeHud then
    ES.DrawSiege()
  elseif ES.arena then
    ES.DrawArena()
  elseif phase == "warning" then
    info.raid:SetText(((h[34] and h[34] ~= "") and h[34] or "The raid") .. " in " .. secs .. "s")
    info.raid:SetTextColor(1, 0.4, 0.3)
  elseif phase == "active" then
    info.raid:SetText("Raiders left: " .. left)
    info.raid:SetTextColor(1, 0.4, 0.3)
  elseif h[15] == "1" then
    info.raid:SetText("At the camp: resting")
    info.raid:SetTextColor(0.5, 1, 0.5)
  else
    info.raid:SetText("")
  end
end

-- --- the Proving Ring (AA: wave, foes standing, seconds to the next or -1, the camp's best) --------
-- In the corner where the raid's count goes: the wave and who's left of it, or the next one's
-- countdown.

function ES.ArenaLine(f)
  if not f[2] or f[2] == "" then
    ES.arena = nil
    info.raid:SetText("")
    return
  end
  local next = tonumber(f[4] or "") or -1
  ES.arena = {
    wave = tonumber(f[2]) or 0,
    left = tonumber(f[3]) or 0,
    nextAt = next >= 0 and (GetTime() + next) or nil,
    best = tonumber(f[5] or "") or 0,
  }
  ES.DrawArena()
end

function ES.DrawArena()
  local a = ES.arena
  if not a then return end
  if a.nextAt then
    local secs = math.max(0, math.ceil(a.nextAt - GetTime()))
    info.raid:SetText("Wave " .. (a.wave + 1) .. " in " .. secs .. "s")
    info.raid:SetTextColor(1, 0.82, 0.3)
  else
    info.raid:SetText("Wave " .. a.wave .. ": " .. a.left .. (a.left == 1 and " foe" or " foes") .. " left")
    info.raid:SetTextColor(1, 0.4, 0.3)
  end
end

local ringClock = CreateFrame("Frame", nil, info)
ringClock.t = 0
ringClock:SetScript("OnUpdate", function()
  if not ES.arena or not ES.arena.nextAt then return end
  this.t = this.t + (arg1 or 0)
  if this.t < 0.2 then return end
  this.t = 0
  ES.DrawArena()
end)

-- --- Siege (SG: phase, wave, waves, secs, gold, income, camps, coming, result) ----------------------
-- In the corner where the raid's count goes: the wave and its countdown; under it, your gold and
-- each camp's Hearth (yours marked, the one you send to arrowed).

function ES.SiegeLine(f)
  if not f[2] or f[2] == "" then
    ES.siegeHud = nil
    info.siege:SetText("")
    if not ES.arena then info.raid:SetText("") end
    return
  end
  local secs = tonumber(f[5] or "") or 0
  ES.siegeHud = {
    phase = f[2],
    wave = tonumber(f[3] or "") or 0,
    waves = tonumber(f[4] or "") or 0,
    at = GetTime() + secs,
    gold = tonumber(f[6] or "") or 0,
    income = tonumber(f[7] or "") or 0,
    camps = (f[8] and f[8] ~= "") and Split(f[8], "^") or {},
    coming = (f[9] and f[9] ~= "") and Split(f[9], "^") or {},
    result = f[10] or "",
    ready = (f[11] and f[11] ~= "") and Split(f[11], "~") or nil,
  }
  ES.DrawSiege()
end

function ES.DrawSiege()
  local h = ES.siegeHud
  if not h then return end
  local secs = math.max(0, math.ceil(h.at - GetTime()))
  if h.phase == "countdown" then
    info.raid:SetText("The siege begins in " .. secs .. "s")
    info.raid:SetTextColor(1, 0.82, 0.3)
  elseif h.phase == "over" then
    info.raid:SetText(h.result .. ": the hall in " .. secs .. "s")
    info.raid:SetTextColor(1, 0.82, 0.3)
  elseif h.wave == 0 then
    info.raid:SetText("Wave 1 in " .. secs .. "s")
    info.raid:SetTextColor(1, 0.82, 0.3)
  else
    info.raid:SetText("Wave " .. h.wave .. " of " .. h.waves .. ": next in " .. secs .. "s")
    info.raid:SetTextColor(1, 0.4, 0.3)
  end
  local lines = { "|cffffd24a" .. h.gold .. " gold|r  |cffbfb6a4(+" .. h.income .. " a wave)|r" }
  for i = 1, table.getn(h.camps) do
    local c = Split(h.camps[i], "~")
    local hp, most = tonumber(c[2] or "") or 0, tonumber(c[3] or "") or 1
    local name = c[1] or "?"
    local mark = ""
    if c[5] == "1" then mark = " |cff7fff7f(yours)|r" elseif c[6] == "1" then mark = " |cffff9f5a(your sends)|r" end
    local state
    if c[4] == "1" then
      state = "|cff8a8a8afallen|r"
    else
      local share = math.floor(hp / math.max(1, most) * 100 + 0.5)
      local col = share > 60 and "|cffd8d2c4" or (share > 30 and "|cffffc04a" or "|cffff6b5a")
      state = col .. share .. "%|r"
    end
    table.insert(lines, name .. mark .. "  " .. state)
  end
  if table.getn(h.coming) > 0 then
    local words = {}
    for i = 1, table.getn(h.coming) do
      local c = Split(h.coming[i], "~")
      local n = tonumber(c[2] or "") or 1
      table.insert(words, (n > 1 and (n .. " ") or "") .. (c[1] or "?"))
    end
    table.insert(lines, "|cffff6b5aComing: " .. table.concat(words, ", ") .. "|r")
  end
  -- The ready vote: everyone ready (F at their Hearth), the next wave comes at once.
  local r = h.ready
  if h.phase == "running" and r and tonumber(r[2] or "") and tonumber(r[2]) > 0 then
    if r[3] == "1" then
      table.insert(lines, "|cff7fff7fReady|r |cffbfb6a4(" .. (r[1] or "0") .. " of " .. r[2] .. "; F at the Hearth takes it back)|r")
    else
      table.insert(lines, "|cffbfb6a4Ready early? F at your Hearth (" .. (r[1] or "0") .. " of " .. r[2] .. " ready)|r")
    end
  end
  info.siege:SetText(table.concat(lines, "\n"))
end

ES.siegeClock = CreateFrame("Frame", nil, info)
ES.siegeClock.t = 0
ES.siegeClock:SetScript("OnUpdate", function()
  if not ES.siegeHud then return end
  this.t = this.t + (arg1 or 0)
  if this.t < 0.2 then return end
  this.t = 0
  ES.DrawSiege()
end)

-- --- messages, flashes, banners, falling ------------------------------------------------------------

-- Low over the middle, above the foe's and a champion's bars, clear of the banners at the top.
local msg = Fader("ElwynnSurvivalMessage", 640, 24, 0.5)
msg:SetPoint("BOTTOM", UIParent, "BOTTOM", 0, 236)
msg.text = Text(msg, FONT_BODY, 14, "OUTLINE")
msg.text:SetPoint("CENTER", msg, "CENTER", 0, 0)
msg.text:SetWidth(620)
msg.text:SetTextColor(1, 0.9, 0.65)

function ES.Message(text)
  msg.text:SetText(ES.Keyed and ES.Keyed(text) or text)
  msg.untilT = GetTime() + 2.6 + math.min(4, string.len(text or "") / 40)
  msg:SetAlpha(1)
  msg:Show()
end

-- A parry: a word over the hero, big and brief.
local flash = Fader("ElwynnSurvivalFlash", 300, 40, 0.5)
flash:SetPoint("CENTER", UIParent, "CENTER", 0, 70)
flash.text = Text(flash, FONT_TITLE, 30, "OUTLINE")
flash.text:SetPoint("CENTER", flash, "CENTER", 0, 0)
flash.text:SetTextColor(1, 0.85, 0.35)

function ES.Flash(text)
  flash.text:SetText(text)
  flash.untilT = GetTime() + 0.9
  flash:SetAlpha(1)
  flash:Show()
end

-- A blow on its way that no guard stops: a red mark over the hero for its wind-up (dodge it).
local danger = Fader("ElwynnSurvivalDanger", 120, 70, 0.15)
danger:SetPoint("CENTER", UIParent, "CENTER", 0, 125)
danger.text = Text(danger, FONT_TITLE, 52, "OUTLINE")
danger.text:SetPoint("CENTER", danger, "CENTER", 0, 0)
danger.text:SetTextColor(0.95, 0.12, 0.08)
danger.text:SetText("!")
danger.sub = Text(danger, FONT_BODY, 11, "OUTLINE")
danger.sub:SetPoint("TOP", danger.text, "BOTTOM", 0, 2)
danger.sub:SetTextColor(1, 0.55, 0.45)
danger.sub:SetText("no guard stops it")
ES.danger = danger

-- A creature winding up a blow no guard stops: its weapons red (and pulsing) for the wind-up.
-- False where the client can't, or for a hazard (no creature).
local DANGER_RED = "FF2418"
function ES.WeaponGlow(guid, secs)
  if not SetUnitWeaponGlow or not guid or guid == "" then return false end
  SetUnitWeaponGlow(guid, DANGER_RED, math.max(0.3, (secs or 0.8) + 0.1))
  return true
end

-- Tells: a blow coming at you reads on the striker itself, with the floor markers on or off. Its
-- wind-up glows (amber: a guard stops it, red: nothing does, from the TG line); a moment before
-- the window to answer it opens, the glow flashes bright: raise the guard now for a parry, with a
-- ping, or roll now for an unblockable blow. The stroke into the blow wooshes, heavier for
-- a longer wind-up. ES.tells[key] is the wind-up's number, so a blow called off (its TG gone
-- before it lands, or a new one for the same key) plays nothing more.
ES.tells, ES.tellN = {}, 0
-- The flash comes this long before the window opens: about a reaction's time, so the press lands
-- inside the window, not after it.
ES.TELL_LEAD = 0.12
ES.TELL_FLASH = "FFF6D8"
-- The cue to raise the guard: a ping, not a weapon's sound. It used to be a blade's ring, the very
-- sound of a guard meeting a blow, so the warning sounded like a parry had already happened.
ES.TELL_RING = "Sound\\Interface\\MapPing.wav"
function ES.Tell(key, guid, secs, kind, parry, roll)
  ES.tellN = ES.tellN + 1
  local n = ES.tellN
  ES.tells[key] = n
  secs = secs or 0.8
  local window = (kind == "block") and (parry or 0.25) or (roll or 0.32)
  local flashAt = math.max(0, secs - window - ES.TELL_LEAD)
  After(flashAt, function()
    if ES.tells[key] ~= n then return end
    if SetUnitWeaponGlow then SetUnitWeaponGlow(guid, ES.TELL_FLASH, math.max(0.15, secs - flashAt + 0.05)) end
    if kind == "block" then PlaySoundFile(ES.TELL_RING) end
  end)
  After(math.max(0, secs - 0.22), function()
    if ES.tells[key] ~= n then return end
    Woosh((secs >= 1.0 and "large") or (secs >= 0.5 and "medium") or "small")
  end)
end

function ES.Danger(secs)
  danger.untilT = GetTime() + math.max(0.3, secs or 0.8)
  danger:SetAlpha(1)
  danger:Show()
end

local banner = Fader("ElwynnSurvivalBanner", 700, 110, 0.8)
banner:SetPoint("TOP", UIParent, "TOP", 0, -150)
banner.bg = Tex(banner, "BACKGROUND", "band", 760, 110)
banner.bg:SetPoint("CENTER", banner, "CENTER", 0, 0)
banner.bg:SetVertexColor(0.35, 0.05, 0.03, 0.85)
banner.title = Text(banner, FONT_TITLE, 34, "OUTLINE")
banner.title:SetPoint("CENTER", banner, "CENTER", 0, 12)
banner.title:SetTextColor(1, 0.82, 0.45)
banner.sub = Text(banner, FONT_BODY, 14, "OUTLINE")
banner.sub:SetPoint("TOP", banner.title, "BOTTOM", 0, -6)

-- The game's own sting for a place found, in the hero's race's voice (the Human one by default,
-- as in Elwynn): Sound\Interface\<Race>Exploration.wav.
ES.EXPLORE_RACES = { Human = 1, Orc = 1, Undead = 1, Tauren = 1, Troll = 1, NightElf = 1, Gnome = 1, Dwarf = 1 }
function ES.ExploreSound()
  local _, race = nil, nil
  if UnitRace then _, race = UnitRace("player") end
  if race == "Scourge" then race = "Undead" end
  if not race or not ES.EXPLORE_RACES[race] then race = "Human" end
  PlaySoundFile("Sound\\Interface\\" .. race .. "Exploration.wav")
end

function ES.Banner(title, sub)
  -- A place found or a journal goal done is quiet news, told the way the game tells it: a yellow
  -- line at the top in its own font, like a quest's progress. No banner, no horn (those are for
  -- what's coming at you). The server's titles start "Discovered: " and "Done: " (explore.rs,
  -- journal.rs).
  local t = title or ""
  if string.find(t, "^Discovered") or string.find(t, "^Done: ") then
    local line
    if string.find(t, "^Done: ") then
      line = string.sub(t, 7) .. " completed."
    else
      -- Just the place: how many are found is for the journal, not the moment.
      line = t
      ES.ExploreSound()
    end
    if UIErrorsFrame then
      UIErrorsFrame:AddMessage(line, 1.0, 0.82, 0.0)
    else
      ES.Message(line)
    end
    return
  end
  banner.title:SetText(title)
  banner.sub:SetText(ES.Keyed(sub or ""))
  banner.untilT = GetTime() + 4.5
  banner:SetAlpha(1)
  banner:Show()
  if string.find(t, "^Everyone Ready") or string.find(t, "^Victory") then
    Play("done")
  else
    Play("raid")
  end
end

-- A text's key hints, "(H)", as the keys really are: the server writes the Elwynn keys, and a
-- key moved on the Controls card is shown where it went.
ES.HINT_KEYS = { H = "ES_BUILD", B = "ES_BAG", N = "ES_TREE", J = "ES_JOURNAL", F = "ES_USE", T = "ES_EAT",
  G = "ES_CYCLE", R = "ES_SHEATHE", K = "ES_KEYS", Z = "ES_MODE", Tab = "ES_LOCK", V = "ES_BARS", X = "TOGGLERUN", Space = "JUMP" }
function ES.Keyed(text)
  if not text or text == "" or not string.find(text, "(", 1, true) then return text end
  local out = string.gsub(text, "%((%a+)%)", function(k)
    local action = ES.HINT_KEYS[k]
    local now = action and KeyName(action)
    if now and now ~= k then return "(" .. now .. ")" end
    return "(" .. k .. ")"
  end)
  return out
end

-- Falling: a dark band across the screen, the words in red, and when you're up again.
local death = CreateFrame("Frame", "ElwynnSurvivalDown", UIParent)
death:SetHeight(140)
death:SetPoint("LEFT", UIParent, "LEFT", 0, 40)
death:SetPoint("RIGHT", UIParent, "RIGHT", 0, 40)
death:SetFrameStrata("HIGH")
death.bg = Tex(death, "BACKGROUND", "band")
death.bg:SetAllPoints(death)
death.bg:SetVertexColor(0, 0, 0, 0.85)
death.title = Text(death, FONT_TITLE, 44, "OUTLINE")
death.title:SetPoint("CENTER", death, "CENTER", 0, 10)
death.title:SetTextColor(0.72, 0.08, 0.06)
death.title:SetText("YOU HAVE FALLEN")
death.sub = Text(death, FONT_BODY, 13, "OUTLINE")
death.sub:SetPoint("TOP", death.title, "BOTTOM", 0, -10)
death.sub:SetTextColor(0.8, 0.75, 0.7)
death:Hide()
ES.upAt = nil

function ES.DrawDeath()
  if not ES.upAt then
    death:Hide()
    return
  end
  local left = ES.upAt - GetTime()
  if not death:IsShown() then
    death.shownAt = GetTime()
    death:Show()
  end
  -- Fading in over the first moments.
  death:SetAlpha(Clamp((GetTime() - (death.shownAt or 0)) / 0.7, 0, 1))
  death.sub:SetText(string.format("Back on your feet in %d", math.max(0, math.ceil(left))))
end

-- --- windows: the chest, a station's work, the build menu, the weapon rack, the journal -------------
-- One window, in two shapes: a slot grid (the chest) and a list of rows with icons (everything
-- else). Below either, a detail strip says what the hovered thing is and does; pages when there's
-- more than fits.

local WIN_W = 600
local BODY_TOP = 86
local LIST_ROWS = 8
local ROW_H = 50
local GRID_COLS = 10
local GRID_ROWS = 5
local SLOT = 46
local SLOT_GAP = 6
local DETAIL_H = 64
local FOOT_H = 46
local PANEL_TINT = { r = 0.13, g = 0.11, b = 0.09 }

-- A window's body and gilded frame (the panel art fills the frame it's given).
local function Panel(f)
  -- Solid under the panel's art, so the HUD behind never shows through a window.
  if not f.solid then
    f.solid = Tex(f, "BACKGROUND", "bar")
    f.solid:SetPoint("TOPLEFT", f, "TOPLEFT", 8, -8)
    f.solid:SetPoint("BOTTOMRIGHT", f, "BOTTOMRIGHT", -8, 8)
    f.solid:SetVertexColor(0.05, 0.045, 0.04, 0.9)
  end
  f.panel = Tex(f, "BACKGROUND", "panel")
  f.panel:SetAllPoints(f)
  f.panel:SetVertexColor(PANEL_TINT.r, PANEL_TINT.g, PANEL_TINT.b, 0.97)
  f.panelFrame = Tex(f, "BORDER", "panelframe")
  f.panelFrame:SetAllPoints(f)
  f.panelFrame:SetVertexColor(FRAME.r, FRAME.g, FRAME.b, 0.85)
end
ES.Panel = Panel

-- A thin gold rule across a window, `y` from its top.
local function Divider(f, y)
  local d = Tex(f, "ARTWORK", "divider")
  d:SetHeight(8)
  d:SetPoint("TOPLEFT", f, "TOPLEFT", 16, -y)
  d:SetPoint("TOPRIGHT", f, "TOPRIGHT", -16, -y)
  d:SetVertexColor(FRAME.r, FRAME.g, FRAME.b, 0.7)
  return d
end

local win = CreateFrame("Frame", "ElwynnSurvivalWindow", UIParent)
win:SetWidth(WIN_W)
win:SetHeight(BODY_TOP + LIST_ROWS * ROW_H + DETAIL_H + FOOT_H)
win:SetPoint("LEFT", UIParent, "LEFT", 70, 30)
win:SetFrameStrata("DIALOG")
win:EnableMouse(true)
Panel(win)

-- The header: the window's icon in a slot, its title and a line on what it's for.
win.iconSlot = MakeSlot(win, 46)
win.iconSlot:SetPoint("TOPLEFT", win, "TOPLEFT", 18, -18)
win.icon = win.iconSlot.icon
win.title = Text(win, FONT_TITLE, 22, "OUTLINE")
win.title:SetPoint("TOPLEFT", win.iconSlot, "TOPRIGHT", 12, 2)
win.title:SetJustifyH("LEFT")
win.title:SetTextColor(GOLD.r, GOLD.g, GOLD.b)
win.note = Text(win, FONT_BODY, 11)
win.note:SetPoint("TOPLEFT", win.title, "BOTTOMLEFT", 0, -3)
win.note:SetWidth(WIN_W - 130)
win.note:SetJustifyH("LEFT")
win.note:SetTextColor(0.78, 0.76, 0.72)
win.close = MakeButton(win, 26, 26, "x", WHITE, function()
  ES.CloseWindows()
  ES.SetMode("action")
end)
win.close:SetPoint("TOPRIGHT", win, "TOPRIGHT", -14, -14)
Divider(win, BODY_TOP - 10)

-- The detail strip: what the hovered (or first) thing is.
win.detailRule = Divider(win, 0)
win.detailName = Text(win, FONT_BODY, 13)
win.detailName:SetJustifyH("LEFT")
win.detailName:SetTextColor(GOLD.r, GOLD.g, GOLD.b)
win.detail = Text(win, FONT_BODY, 11)
win.detail:SetJustifyH("LEFT")
win.detail:SetJustifyV("TOP")
win.detail:SetWidth(WIN_W - 44)
win.detail:SetTextColor(0.86, 0.84, 0.8)
win.hint = Text(win, FONT_BODY, 10)
win.hint:SetJustifyH("LEFT")
win.hint:SetTextColor(0.62, 0.6, 0.56)

-- The foot: an extra action on the left (Bank all), pages in the middle.
win.extra = MakeButton(win, 130, 28, "", GOLD, function(b)
  if b.cmd then Say(b.cmd) end
end)
win.extra:SetPoint("BOTTOMLEFT", win, "BOTTOMLEFT", 18, 14)
win.prev = MakeButton(win, 34, 26, "<", WHITE, function() ES.Page(-1) end)
win.next = MakeButton(win, 34, 26, ">", WHITE, function() ES.Page(1) end)
win.pageText = Text(win, FONT_BODY, 11)
win.pageText:SetPoint("BOTTOM", win, "BOTTOM", 0, 22)
win.pageText:SetTextColor(0.8, 0.8, 0.8)
win.prev:SetPoint("RIGHT", win.pageText, "LEFT", -12, 0)
win.next:SetPoint("LEFT", win.pageText, "RIGHT", 12, 0)
win.prev:Hide()
win.next:Hide()

local function ShowDetail(name, text, hint)
  win.detailName:SetText(name or "")
  win.detail:SetText(text or "")
  win.hint:SetText(hint or "")
end

-- What the strip shows with nothing hovered.
local function RestDetail()
  local d = win.restDetail
  if d then
    ShowDetail(d[1], d[2], d[3])
  else
    ShowDetail("", "", "")
  end
end

-- The rows: an icon in its slot, the name (and a tag beside it), a line under it (a cost, a
-- reward), and up to two buttons.
win.rows = {}
for i = 1, LIST_ROWS do
  local r = CreateFrame("Frame", nil, win)
  r:SetWidth(WIN_W - 36)
  r:SetHeight(ROW_H - 4)
  r:SetPoint("TOPLEFT", win, "TOPLEFT", 18, -BODY_TOP - (i - 1) * ROW_H)
  r.band = Tex(r, "BACKGROUND", "rowband")
  r.band:SetAllPoints(r)
  r.band:SetVertexColor(1, 0.86, 0.5, 0)
  r.slot = MakeSlot(r, 42)
  r.slot:SetPoint("LEFT", r, "LEFT", 2, 0)
  r.name = Text(r, FONT_BODY, 13)
  r.name:SetPoint("TOPLEFT", r.slot, "TOPRIGHT", 10, -3)
  r.name:SetJustifyH("LEFT")
  r.tag = Text(r, FONT_BODY, 10)
  r.tag:SetPoint("LEFT", r.name, "RIGHT", 8, 0)
  r.tag:SetTextColor(0.62, 0.6, 0.56)
  r.cost = Text(r, FONT_BODY, 11)
  r.cost:SetPoint("TOPLEFT", r.name, "BOTTOMLEFT", 0, -5)
  r.cost:SetJustifyH("LEFT")
  r.cost:SetWidth(WIN_W - 250)
  r.cost:SetTextColor(0.8, 0.78, 0.74)
  r.go = MakeButton(r, 96, 28, "", GOLD, function(b)
    if type(b.cmd) == "function" then b.cmd() elseif b.cmd then Say(b.cmd) end
  end)
  r.go:SetPoint("RIGHT", r, "RIGHT", -6, 0)
  r.alt = MakeButton(r, 72, 28, "", GOLD, function(b)
    if b.cmd then Say(b.cmd) end
  end)
  r.alt:SetPoint("RIGHT", r.go, "LEFT", -6, 0)
  -- Hovered: lit, and the strip says what it is.
  r:EnableMouse(true)
  r:SetScript("OnEnter", function()
    this.band:SetVertexColor(1, 0.86, 0.5, 0.12)
    ShowDetail(this.detailName, this.tip, this.hint)
  end)
  r:SetScript("OnLeave", function()
    this.band:SetVertexColor(1, 0.86, 0.5, this.chosen and 0.07 or 0)
    RestDetail()
  end)
  r:Hide()
  win.rows[i] = r
end

-- The grid: a slot each, its count, a gold mark on gear; a click takes or puts on.
win.slots = {}
local gridW = GRID_COLS * SLOT + (GRID_COLS - 1) * SLOT_GAP
for i = 1, GRID_COLS * GRID_ROWS do
  local col = math.mod(i - 1, GRID_COLS)
  local row = math.floor((i - 1) / GRID_COLS)
  local b = CreateFrame("Button", nil, win)
  b:SetWidth(SLOT)
  b:SetHeight(SLOT)
  b:SetPoint("TOPLEFT", win, "TOPLEFT", (WIN_W - gridW) / 2 + col * (SLOT + SLOT_GAP), -BODY_TOP - row * (SLOT + SLOT_GAP))
  b.well = Tex(b, "BACKGROUND", "slotwell")
  b.well:SetAllPoints(b)
  b.well:SetVertexColor(0.02, 0.02, 0.02, 0.85)
  b.icon = b:CreateTexture(nil, "ARTWORK")
  b.icon:SetPoint("TOPLEFT", b, "TOPLEFT", 5, -5)
  b.icon:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -5, 5)
  b.icon:SetTexCoord(0.08, 0.92, 0.08, 0.92)
  b.frame = Tex(b, "OVERLAY", "slot")
  b.frame:SetAllPoints(b)
  b.frame:SetVertexColor(FRAME.r * 0.8, FRAME.g * 0.8, FRAME.b * 0.8, 0.85)
  b.gear = Tex(b, "OVERLAY", "diamond", 10, 10)
  b.gear:SetPoint("TOPLEFT", b, "TOPLEFT", 3, -3)
  b.gear:SetVertexColor(GOLD.r, GOLD.g, GOLD.b, 1)
  b.count = Text(b, FONT_NUM, 13, "OUTLINE")
  b.count:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -5, 4)
  b:EnableMouse(true)
  b:RegisterForClicks("LeftButtonUp", "RightButtonUp")
  b:SetScript("OnEnter", function()
    if not this.key then return end
    this.frame:SetVertexColor(1, 0.86, 0.5, 1)
    ShowDetail(this.detailName, this.tip, this.hint)
  end)
  b:SetScript("OnLeave", function()
    this.frame:SetVertexColor(FRAME.r * 0.8, FRAME.g * 0.8, FRAME.b * 0.8, 0.85)
    RestDetail()
  end)
  b:SetScript("OnClick", function()
    if not this.key then return end
    Play("click")
    if this.gearKind ~= "0" then
      Say(".es equip " .. this.key)
    elseif arg1 == "RightButton" then
      Say(".es take " .. this.key .. " " .. this.n)
    elseif IsShiftKeyDown and IsShiftKeyDown() then
      Say(".es take " .. this.key .. " 5")
    else
      Say(".es take " .. this.key .. " 1")
    end
  end)
  b:Hide()
  win.slots[i] = b
end

win:Hide()
ES.win = win
-- Escape closes it, as it does the game's own windows.
if UISpecialFrames then
  table.insert(UISpecialFrames, "ElwynnSurvivalWindow")
end
win:SetScript("OnHide", function()
  -- Hidden with the whole interface (the map): it's still open.
  if this:IsShown() then return end
  if ES.awake and not ES.placing and not ES.confirm:IsShown() then
    Say(".es close")
    ES.SetMode("action")
  end
end)

-- Lay the window out for a list (`rows` of them) or the grid, with the strip and foot below.
local function Shape(grid)
  local body = grid and (GRID_ROWS * (SLOT + SLOT_GAP)) or (LIST_ROWS * ROW_H)
  win:SetHeight(BODY_TOP + body + DETAIL_H + FOOT_H)
  local y = BODY_TOP + body + 2
  win.detailRule:ClearAllPoints()
  win.detailRule:SetPoint("TOPLEFT", win, "TOPLEFT", 16, -y)
  win.detailRule:SetPoint("TOPRIGHT", win, "TOPRIGHT", -16, -y)
  win.detailName:ClearAllPoints()
  win.detailName:SetPoint("TOPLEFT", win, "TOPLEFT", 22, -y - 10)
  win.detail:ClearAllPoints()
  win.detail:SetPoint("TOPLEFT", win.detailName, "BOTTOMLEFT", 0, -4)
  win.hint:ClearAllPoints()
  win.hint:SetPoint("TOPRIGHT", win, "TOPRIGHT", -22, -y - 12)
end

-- A cost as the server sends it (`name:need:have`, `;` between): each input, red when short
-- (with what there is of it).
local function CostText(cost)
  if not cost or cost == "" then return "" end
  local parts = Split(cost, ";")
  local words = {}
  for i = 1, table.getn(parts) do
    local f = Split(parts[i], ":")
    local name, need, have = f[1] or "?", tonumber(f[2] or "") or 0, tonumber(f[3] or "") or 0
    if have >= need then
      table.insert(words, "|cffd8d2c4" .. need .. " " .. name .. "|r")
    else
      table.insert(words, "|cffff6b5a" .. need .. " " .. name .. " (" .. have .. ")|r")
    end
  end
  return table.concat(words, "|cff7a746a,  |r")
end
ES.CostText = CostText

-- One row: `o` holds icon, name, tag, sub, go and cmd (the button), alt and altCmd, can (the
-- button's lit), detail (the strip's text), hint, chosen (lit a little).
local function Row(i, o)
  local r = win.rows[i]
  if not r then return end
  r.slot:Set(ICONS .. (o.icon and o.icon ~= "" and o.icon or "INV_Misc_QuestionMark"), o.lit ~= false)
  r.slot.count:SetText(o.count or "")
  r.name:SetText(o.name or "")
  r.tag:SetText(o.tag or "")
  r.cost:SetText(o.sub or "")
  r.tip = o.detail
  r.hint = o.hint
  r.detailName = o.detailName or o.plainName or o.name
  r.chosen = o.chosen
  r.band:SetVertexColor(1, 0.86, 0.5, o.chosen and 0.07 or 0)
  if o.go then
    r.go.label:SetText(o.go)
    r.go.cmd = o.cmd
    r.go:SetEnabledLook(o.can ~= false)
    r.go:Show()
  else
    r.go:Hide()
  end
  if o.alt then
    r.alt.label:SetText(o.alt)
    r.alt.cmd = o.altCmd
    r.alt:Show()
  else
    r.alt:Hide()
  end
  r:Show()
end

local WINDOW_ICONS = {
  chest = "INV_Misc_Bag_08",
  journal = "INV_Misc_Book_09",
  station = "INV_Hammer_15",
  rack = "INV_Sword_04",
  build = "INV_Misc_Note_01",
}

local function OpenWindow(kind, title, note, icon, grid)
  win.kind = kind
  win.title:SetText(title)
  win.note:SetText(note or "")
  win.iconSlot:Set(ICONS .. (icon or WINDOW_ICONS[kind] or "INV_Misc_QuestionMark"), true)
  for i = 1, LIST_ROWS do
    win.rows[i]:Hide()
  end
  for i = 1, table.getn(win.slots) do
    win.slots[i]:Hide()
  end
  Shape(grid)
  win.restDetail = nil
  ShowDetail("", "", "")
  win.extra:Hide()
  win.prev:Hide()
  win.next:Hide()
  win.pageText:SetText("")
  if not win:IsShown() then
    Play("open")
  end
  win:Show()
  WindowMode()
end

function ES.CloseWindows()
  if win:IsShown() then
    win:Hide()
  end
  if ES.letter and ES.letter:IsShown() then ES.letter:Hide() end
  if ES.tree then ES.tree:Hide() end
  ES.confirm:Hide()
end

local function Items(list)
  if list and list ~= "" then
    return Split(list, "^")
  end
  return {}
end

-- Pages: each window's own, and how to draw it again on another.
ES.pages = {}
ES.redraw = {}
function ES.Page(step)
  local kind = win.kind
  if not kind or not ES.redraw[kind] then return end
  ES.pages[kind] = (ES.pages[kind] or 1) + step
  ES.redraw[kind]()
end

-- Which page to draw of `count` things `per` page (kept in range), and the first index on it.
local function PageOf(kind, count, per)
  local pages = math.max(1, math.ceil(count / per))
  ES.pages[kind] = Clamp(ES.pages[kind] or 1, 1, pages)
  if pages > 1 then
    win.pageText:SetText(ES.pages[kind] .. " / " .. pages)
    win.prev:Show()
    win.next:Show()
    win.prev:SetEnabledLook(ES.pages[kind] > 1)
    win.next:SetEnabledLook(ES.pages[kind] < pages)
  end
  return (ES.pages[kind] - 1) * per
end

-- The strip's resting text: the first row's, or a word for an empty window.
local function Rest(name, text, hint)
  win.restDetail = { name, text, hint }
  RestDetail()
end

-- --- Elwynn on the world map (PM: tally, then key~x~y~kind~found~name~text~icon^...) -----------
-- Marks on the world map's Elwynn for every place worth going to, lit once the camp has found it,
-- each with what's there on its tooltip; a tally of places, chests and secrets along the top. The
-- map hides UIParent while it's up, so the marks live on the map's own frames.

-- (One local for it all: the main chunk is near Lua's limit of locals.)
local Pins = { size = 20 }
-- The zone's rect on the map (WorldMapArea: left, right are world y; top, bottom world x).
Pins.elwynn = { left = 1535.4166, right = -1935.4166, top = -7939.583, bottom = -10254.166 }
Pins.icons = {
  camp = "INV_BannerPVP_02",
  town = "INV_Misc_Note_02",
  horde = "INV_BannerPVP_01",
  foes = "INV_Sword_04",
  sanctum = "Spell_Holy_PrayerOfHealing",
  lair = "INV_Misc_Bone_HumanSkull_01",
  landmark = "INV_Misc_Map_01",
  arena = "Ability_Warrior_BattleShout",
  dungeon = "INV_Misc_Key_03",
  fishing = "Trade_Fishing",
  mine = "Trade_Mining",
  herbs = "Trade_Herbalism",
}
Pins.words = {
  camp = "Your camp", town = "A town", horde = "Held by the Horde", foes = "Hostile ground",
  sanctum = "A sanctum: its trial teaches the camp new cards", lair = "A lair",
  landmark = "A landmark", arena = "The Proving Ring", dungeon = "A way below",
  fishing = "Fishing waters", mine = "A mine", herbs = "Herbs grow here",
}

ES.pins = {}
ES.pinFrames = {}

function ES.PinsLine(f)
  local t = Split(f[2] or "", "~")
  ES.pinTally = t
  ES.pins = {}
  local items = Items(f[3])
  for i = 1, table.getn(items) do
    local p = Split(items[i], "~")
    table.insert(ES.pins, { key = p[1], x = tonumber(p[2]) or 0, y = tonumber(p[3]) or 0, kind = p[4] or "", found = p[5] == "1", name = p[6] or "", text = p[7] or "", icon = p[8] or "" })
  end
  ES.DrawPins()
end

-- Where a world spot falls on the shown map (0..1), or nil off it.
-- (Only ever drawn on Elwynn's own map, so its rect is all it takes.)
function Pins.Spot(x, y)
  local u = (Pins.elwynn.left - y) / (Pins.elwynn.left - Pins.elwynn.right)
  local v = (Pins.elwynn.top - x) / (Pins.elwynn.top - Pins.elwynn.bottom)
  if u < 0 or u > 1 or v < 0 or v > 1 then return nil end
  return u, v
end

function Pins.Tooltip()
  return WorldMapTooltip or GameTooltip
end

function Pins.Frame(i)
  local b = ES.pinFrames[i]
  if b then return b end
  local parent = WorldMapButton or WorldMapDetailFrame
  b = CreateFrame("Button", nil, parent)
  b:SetWidth(Pins.size)
  b:SetHeight(Pins.size)
  b:SetFrameLevel(parent:GetFrameLevel() + 5)
  b.ring = b:CreateTexture(nil, "BACKGROUND")
  b.ring:SetTexture("Interface\\Minimap\\UI-Minimap-Background")
  b.ring:SetAllPoints(b)
  b.ring:SetVertexColor(0, 0, 0, 0.8)
  b.icon = b:CreateTexture(nil, "ARTWORK")
  b.icon:SetPoint("TOPLEFT", b, "TOPLEFT", 3, -3)
  b.icon:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -3, 3)
  b.icon:SetTexCoord(0.08, 0.92, 0.08, 0.92)
  b.check = b:CreateTexture(nil, "OVERLAY")
  b.check:SetTexture("Interface\\Buttons\\UI-CheckBox-Check")
  b.check:SetWidth(14)
  b.check:SetHeight(14)
  b.check:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", 5, -5)
  b:SetScript("OnEnter", function()
    local p = this.pin
    if not p then return end
    local tip = Pins.Tooltip()
    tip:SetOwner(this, "ANCHOR_RIGHT")
    tip:SetText(p.name, 1, 0.82, 0)
    tip:AddLine(Pins.words[p.kind] or "", 0.6, 0.8, 1)
    if p.text ~= "" then tip:AddLine(p.text, 1, 1, 1, 1) end
    if p.found then
      tip:AddLine("Found", 0.3, 1, 0.3)
    else
      tip:AddLine("Not found yet: go there", 0.7, 0.7, 0.7)
    end
    tip:Show()
  end)
  b:SetScript("OnLeave", function() Pins.Tooltip():Hide() end)
  ES.pinFrames[i] = b
  return b
end

function ES.DrawPins()
  if not WorldMapDetailFrame then return end
  local shown = WorldMapFrame and WorldMapFrame:IsVisible() and GetMapInfo and GetMapInfo() == "Elwynn"
  local w, h = WorldMapDetailFrame:GetWidth(), WorldMapDetailFrame:GetHeight()
  local used = 0
  if shown then
    for i = 1, table.getn(ES.pins) do
      local p = ES.pins[i]
      local u, v = Pins.Spot(p.x, p.y)
      if u then
        used = used + 1
        local b = Pins.Frame(used)
        b.pin = p
        b:ClearAllPoints()
        b:SetPoint("CENTER", WorldMapDetailFrame, "TOPLEFT", u * w, -v * h)
        local icon = p.icon ~= "" and p.icon or (Pins.icons[p.kind] or "INV_Misc_QuestionMark")
        b.icon:SetTexture("Interface\\Icons\\" .. icon)
        if p.found then
          b.icon:SetVertexColor(1, 1, 1)
          b.check:Show()
        else
          b.icon:SetVertexColor(0.55, 0.55, 0.55)
          b.check:Hide()
        end
        b:Show()
      end
    end
  end
  for i = used + 1, table.getn(ES.pinFrames) do
    ES.pinFrames[i]:Hide()
  end
  -- The tally along the top of the map.
  if not ES.pinText and WorldMapButton then
    ES.pinText = Text(WorldMapButton, FONT_BODY, 13, "OUTLINE", "OVERLAY")
    ES.pinText:SetPoint("TOP", WorldMapDetailFrame, "TOP", 0, -6)
  end
  if ES.pinText then
    local t = ES.pinTally or {}
    if shown and t[2] then
      -- (Secrets aren't counted here: they're found, not ticked off.)
      ES.pinText:SetText(string.format("Elwynn: %s of %s places found   %s of %s chests opened", t[1] or 0, t[2] or 0, t[3] or 0, t[4] or 0))
      ES.pinText:Show()
    else
      ES.pinText:Hide()
    end
  end
end

-- --- the journal (GJ: title~state~what to do~reward, state 0 done, 1 now, 2 ahead; J opens it) ---

function ES.JournalLine(f)
  ES.journal = Items(f[2])
  if win:IsShown() and win.kind == "journal" then
    ES.DrawJournal()
  end
end

function ES.Journal()
  if not ES.awake then return end
  if win:IsShown() and win.kind == "journal" then
    ES.CloseWindows()
    ES.SetMode("action")
    return
  end
  -- It opens on the page of the goal it's on.
  ES.pages.journal = nil
  local items = ES.journal or {}
  for i = 1, table.getn(items) do
    if Split(items[i], "~")[2] == "1" then
      ES.pages.journal = math.ceil(i / LIST_ROWS)
    end
  end
  ES.DrawJournal()
end

function ES.DrawJournal()
  OpenWindow("journal", "The Camp's Journal", "What the camp has done and what comes next. Every step pays into the camp's chest.")
  local items = ES.journal or {}
  local first = PageOf("journal", table.getn(items), LIST_ROWS)
  Rest("Nothing written yet.", "", "")
  for i = 1, math.min(LIST_ROWS, table.getn(items) - first) do
    local f = Split(items[first + i], "~")
    local title, state, text, reward = f[1] or "?", f[2], f[3] or "", f[4] or ""
    local n = first + i
    local o = { icon = "INV_Misc_Note_01", plainName = n .. ". " .. title, detail = text }
    o.sub = reward ~= "" and ("Into the chest: " .. reward) or ""
    if state == "0" then
      o.name = "|cff8fbf7f" .. n .. ". " .. title .. "|r"
      o.tag = "|cff8fbf7fDone|r"
      o.lit = false
    elseif state == "1" then
      o.name = "|cffffd27f" .. n .. ". " .. title .. "|r"
      o.tag = "|cffffd27fNow|r"
      o.icon = "INV_Misc_Book_09"
      o.chosen = true
      Rest(o.plainName, text, "")
    else
      o.name = "|cffb0aaa0" .. n .. ". " .. title .. "|r"
      o.lit = false
    end
    Row(i, o)
  end
end
ES.redraw.journal = function() ES.DrawJournal() end

-- --- the chest (S: item~name~count~gear~text~icon, gear 1 held, 2 worn) -----------------------------

function ES.Chest(list)
  OpenWindow("chest", "Camp Storage", "What everyone banks for the camp. Click a thing to take one, Shift-click for five, right-click for all of it; click gear to put it on.", nil, true)
  ES.chestList = list
  local items = Items(list)
  local per = GRID_COLS * GRID_ROWS
  local first = PageOf("chest", table.getn(items), per)
  if table.getn(items) == 0 then
    Rest("The chest is empty.", "Bring timber, meat and copper back from the land and use the chest to bank them.", "")
  else
    Rest("Hover over something", "to see what it is and does.", "")
  end
  local shown = math.min(per, table.getn(items) - first)
  -- The room left: empty wells.
  for i = shown + 1, per do
    local b = win.slots[i]
    b.key, b.detailName, b.tip, b.hint = nil, nil, nil, nil
    b.icon:Hide()
    b.count:SetText("")
    b.gear:Hide()
    b.well:SetVertexColor(0.02, 0.02, 0.02, 0.4)
    b.frame:SetAlpha(0.2)
    b:Show()
  end
  for i = 1, shown do
    local f = Split(items[first + i], "~")
    local b = win.slots[i]
    local key, name, n, gear, text, icon = f[1], f[2] or "?", tonumber(f[3]) or 0, f[4] or "0", f[5] or "", f[6] or ""
    b.key, b.n, b.gearKind = key, n, gear
    b.well:SetVertexColor(0.02, 0.02, 0.02, 0.85)
    b.frame:SetAlpha(1)
    b.icon:Show()
    b.icon:SetTexture(ICONS .. (icon ~= "" and icon or "INV_Misc_QuestionMark"))
    b.count:SetText(n > 1 and n or "")
    b.detailName = name .. (n > 1 and ("  x" .. n) or "")
    b.tip = text
    if gear == "1" then
      b.gear:Show()
      b.hint = "Click: take it up (what you hold goes in the chest)"
    elseif gear == "2" then
      b.gear:Show()
      b.hint = "Click: wear it (what you wear goes in the chest)"
    else
      b.gear:Hide()
      b.hint = "Click: 1   Shift-click: 5   Right-click: all"
    end
    b:Show()
  end
  win.extra.label:SetText("Bank all you carry")
  win.extra.cmd = ".es bank"
  win.extra:Show()
end
ES.redraw.chest = function() ES.Chest(ES.chestList) end

-- --- a station's work (W: station, name, then recipe~output~cost~can~text~icon; its icon) ---------

function ES.Station(name, list, icon)
  local note = "What it needs comes out of your bags first, then the camp's chest."
  if ES.siege then
    note = "Your class's other cards, for gold. A card bought goes on its key if that key is free; N swaps cards on the board."
  end
  OpenWindow("station", name, note, icon ~= "" and icon or nil)
  local items = Items(list)
  if table.getn(items) == 0 then
    Rest(ES.siege and "You know every card of your class." or "Nothing to make here yet.", "", "")
  end
  for i = 1, math.min(LIST_ROWS, table.getn(items)) do
    local f = Split(items[i], "~")
    local can = f[4] == "1"
    Row(i, {
      icon = f[6],
      name = f[2] or "?",
      sub = CostText(f[3]),
      go = ES.siege and "Learn" or "Make",
      cmd = ".es craft " .. (f[1] or ""),
      can = can,
      detail = f[5] or "",
      hint = can and "" or (ES.siege and "Not enough gold yet" or "Short of something: see what's red"),
    })
    if i == 1 then Rest(f[2] or "?", f[5] or "", "") end
  end
end

-- --- the weapon rack (AR: key~name~kind~held~text~icon) ----------------------------------------------

function ES.Rack(list)
  OpenWindow("rack", "Weapon Rack", "Take up anything to try it. Two-handed weapons guard with themselves, no shield.")
  ES.rackList = list
  local items = Items(list)
  local first = PageOf("rack", table.getn(items), LIST_ROWS)
  for i = 1, math.min(LIST_ROWS, table.getn(items) - first) do
    local f = Split(items[first + i], "~")
    local held = f[4] == "1"
    Row(i, {
      icon = f[6],
      name = f[2] or "?",
      tag = f[3] or "",
      sub = f[5] or "",
      go = held and "In hand" or "Take up",
      cmd = (not held) and (".es arm " .. (f[1] or "")) or nil,
      can = not held,
      chosen = held,
      detail = f[5] or "",
    })
    if i == 1 then Rest(f[2] or "?", f[5] or "", "") end
  end
end
ES.redraw.rack = function() ES.Rack(ES.rackList) end

-- --- the build menu (BM: key~name~cost~can~built~text~icon) ----------------------------------------

function ES.BuildMenu(list)
  if ES.siege then
    OpenWindow("build", "Build", "Defences stand on your camp's ground round its Hearth, paid for in gold.")
  else
    OpenWindow("build", "Build", "Stations and defences stand in the meadow by the camp. Materials come out of your bags first, then the camp's chest.")
  end
  ES.buildList = list
  local items = Items(list)
  local first = PageOf("build", table.getn(items), LIST_ROWS)
  for i = 1, math.min(LIST_ROWS, table.getn(items) - first) do
    local f = Split(items[first + i], "~")
    local built = tonumber(f[5]) or 0
    local can = f[4] == "1"
    Row(i, {
      icon = f[7],
      name = f[2] or "?",
      tag = built > 0 and (built .. " built") or "",
      sub = CostText(f[3]),
      go = "Place",
      cmd = ".es build " .. (f[1] or ""),
      can = can,
      detail = f[6] or "",
      hint = can and "" or "Short of something: see what's red",
    })
    if i == 1 then Rest(f[2] or "?", f[6] or "", "") end
  end
end
ES.redraw.build = function() ES.BuildMenu(ES.buildList) end

-- --- the Smuggler's Logbook at camp (BK: open, pages key~name~dungeon, ring, siege, dungeons) ------
-- The book's first page lists the Proving Ring, Siege and every dungeon; a dungeon opened shows its
-- pages (its first lamp, then each one lit), and a way back.

function ES.Book(open, list, ring, siege, dungeons)
  ES.bookData = { open or "", list or "", ring or "", siege or "", dungeons or "" }
  if ES.bookDungeon and not string.find("^" .. (dungeons or "") , "%^" .. ES.bookDungeon .. "~") then
    ES.bookDungeon = nil
  end
  ES.BookDraw()
end
ES.redraw.book = function() if ES.bookData then ES.BookDraw() end end

function ES.BookOpen(key)
  ES.bookDungeon = key
  ES.pages.book = 1
  ES.BookDraw()
end

function ES.BookDraw()
  local d = ES.bookData
  local dl = Items(d[5])
  -- (A server from before the other dungeons: the Deadmines' pages, as they were.)
  if table.getn(dl) == 0 then
    ES.BookFlat(d[1], d[2], d[3], d[4])
    return
  end
  OpenWindow("book", "The Smuggler's Logbook", "A smuggler's log of every dark place under Azeroth the Brotherhood ever sold a map of. Every page is a place: read one and you're there. What you carry goes with you; the camp's chest stays here.", "INV_Misc_Book_09")
  local pick = ES.bookDungeon
  if pick then
    local info
    for i = 1, table.getn(dl) do
      local f = Split(dl[i], "~")
      if f[1] == pick then info = f end
    end
    if not info then
      ES.bookDungeon = nil
      ES.BookDraw()
      return
    end
    Row(1, {
      icon = "INV_Misc_Book_09",
      name = "Back to the book",
      sub = "The Proving Ring, Siege and every dungeon",
      go = "Back",
      cmd = function() ES.BookOpen(nil) end,
      detail = "The logbook's first page.",
    })
    if info[4] ~= "1" then
      Rest(info[2] or "?", "Written in a cipher: hold the camp against " .. (info[5] or "1") .. " raid(s) and the scouts will have read these pages.", "")
      return
    end
    Rest(info[2] or "?", info[9] or "", "")
    -- (A server that keeps the book's record of notes sends their tallies. It sends the secrets'
    -- too, but they aren't shown: a secret is found down there, not ticked off in the book.)
    local top = 1
    if info[11] then
      top = 2
      local read, notes = tonumber(info[12]) or 0, tonumber(info[13]) or 0
      Row(2, {
        icon = "INV_Misc_Note_02",
        lit = read > 0,
        name = "Notes",
        tag = read .. " of " .. notes .. " read",
        sub = "The pages you've found down there",
        go = "Read",
        cmd = ".es lore " .. pick,
        detail = "Every note you've found here, to read again, and what they hint at.",
      })
    end
    -- (And of the Bookmaker's Wager: the level open, the time to beat.)
    if info[14] then
      top = 3
      local most, limit = tonumber(info[14]) or 1, tonumber(info[15]) or 0
      Row(3, {
        icon = "INV_Misc_Coin_02",
        lit = most > 1,
        name = "The Bookmaker's Wager",
        tag = "Open to +" .. most,
        sub = string.format("%d:%02d to beat, the stakes rising", math.floor(limit / 60), math.mod(limit, 60)),
        go = "Wager",
        cmd = ".es wager " .. pick,
        detail = "A run against the clock, the dungeon fresh: win in time to raise the stakes (tougher foes, better pay), and get your names on the Bookmaker's Board.",
      })
    end
    local pages = {}
    local items = Items(d[2])
    for i = 1, table.getn(items) do
      local f = Split(items[i], "~")
      if (f[3] or "deadmines") == pick then table.insert(pages, f) end
    end
    local first = PageOf("book", table.getn(pages), LIST_ROWS - top)
    for i = 1, LIST_ROWS - top do
      local f = pages[first + i]
      if not f then break end
      local n = first + i
      Row(i + top, {
        icon = n == 1 and "INV_Misc_Map_01" or "Spell_Fire_Fire",
        name = f[2] or "?",
        tag = "Page " .. n,
        sub = n == 1 and "Where every run begins" or "A lamp you've lit below",
        go = "Go down",
        cmd = ".es travel " .. (f[1] or ""),
        detail = n == 1 and (info[9] ~= "" and info[9] or "The way in.") or "Wake at this lamp, as you left it.",
      })
    end
    return
  end
  -- The first page: the ring, Siege, then the dungeons.
  local rows = {}
  if d[3] ~= "" then
    local r = Split(d[3], "~")
    local best = tonumber(r[3] or "") or 0
    table.insert(rows, {
      icon = "Ability_Warrior_BattleShout",
      name = r[2] or "The Proving Ring",
      tag = best > 0 and ("Best: wave " .. best) or "",
      sub = "Endless waves, more and tougher every time",
      go = "Go",
      cmd = ".es travel " .. (r[1] or "arena"),
      detail = "A ring of braziers on the field south of Fargodeep: the Horde comes in waves that never end, a champion every fifth. Fall and you get up at the camp, nothing lost; the banner takes you home.",
    })
  end
  if d[4] ~= "" then
    local g = Split(d[4], "~")
    table.insert(rows, {
      icon = "INV_BannerPVP_02",
      name = g[2] or "The Siege",
      tag = "1v1 to 1v1v1v1",
      sub = "Camps against camps in Arathi Basin",
      go = "Go",
      cmd = ".es travel " .. (g[1] or "siege"),
      detail = (g[3] or "") .. " You play a fresh hero of your class there; your own waits here.",
    })
  end
  for i = 1, table.getn(dl) do
    local f = Split(dl[i], "~")
    local key = f[1]
    local opened = f[4] == "1"
    local lit = tonumber(f[7] or "") or 0
    local lamps = tonumber(f[6] or "") or 0
    table.insert(rows, {
      icon = (f[3] and f[3] ~= "") and f[3] or "INV_Misc_Map_01",
      lit = opened,
      name = f[2] or "?",
      tag = opened and (lit .. " of " .. lamps .. " lamps") or "In cipher",
      sub = f[8] or "",
      go = "Open",
      cmd = function() ES.BookOpen(key) end,
      detail = opened and ((f[8] or "") .. " Its pages: the way in, and every lamp you've lit.") or ("Hold the camp against " .. (f[5] or "1") .. " raid(s) and the scouts will have read it."),
    })
  end
  Rest("The way in", "Every dungeon starts at its first page; light the lamps as you go and they're pages too. There aren't many lamps down there.", "")
  local first = PageOf("book", table.getn(rows), LIST_ROWS)
  for i = 1, LIST_ROWS do
    local o = rows[first + i]
    if not o then break end
    Row(i, o)
  end
end

-- The Deadmines' pages alone (a server from before the other dungeons).

function ES.BookFlat(open, list, ring, siege)
  OpenWindow("book", "The Smuggler's Logbook", "A Defias smuggler's log of the Deadmines. Every page is a place: read one and you're there. What you carry goes with you; the camp's chest stays here.", "INV_Misc_Book_09")
  local items = Items(list)
  -- The Proving Ring's page, scrawled on the inside cover: there from the start.
  local n = 0
  if ring and ring ~= "" then
    local r = Split(ring, "~")
    local best = tonumber(r[3] or "") or 0
    n = 1
    Row(1, {
      icon = "Ability_Warrior_BattleShout",
      name = r[2] or "The Proving Ring",
      tag = best > 0 and ("Best: wave " .. best) or "",
      sub = "Endless waves, more and tougher every time",
      go = "Go",
      cmd = ".es travel " .. (r[1] or "arena"),
      detail = "A ring of braziers on the field south of Fargodeep: the Horde comes in waves that never end, a champion every fifth. Fall and you get up at the camp, nothing lost; the banner takes you home.",
    })
  end
  -- Siege's page, beside it: there from the start too.
  if siege and siege ~= "" then
    local g = Split(siege, "~")
    n = n + 1
    Row(n, {
      icon = "INV_BannerPVP_02",
      name = g[2] or "The Siege",
      tag = "1v1 to 1v1v1v1",
      sub = "Camps against camps in Arathi Basin",
      go = "Go",
      cmd = ".es travel " .. (g[1] or "siege"),
      detail = (g[3] or "") .. " You play a fresh hero of your class there; your own waits here.",
    })
  end
  if open ~= "1" then
    Rest("Written in a cipher", "Hold the camp against its first raid and the scouts will have read the rest of it.", "")
    return
  end
  Rest("The way in", "The first page is the smugglers' landing, at the back of the mine: in from the sea at Ironclad Cove, out through the front door. Light the lamps as you go and they're pages here.", "")
  for i = 1, math.min(LIST_ROWS - n, table.getn(items)) do
    local f = Split(items[i], "~")
    Row(n + i, {
      icon = i == 1 and "INV_Misc_Map_01" or "Spell_Fire_Fire",
      name = f[2] or "?",
      tag = "Page " .. i,
      sub = i == 1 and "In from the sea: where every run begins" or "A lamp you've lit below",
      go = "Go down",
      cmd = ".es travel " .. (f[1] or ""),
      detail = i == 1 and "The smugglers' landing in Ironclad Cove's sea tunnel." or "Wake at this lamp, as you left it.",
    })
  end
end

-- --- the Proving Ring's banner (AW: wave, in a wave, seconds to the next, the camp's best, heroes) --

function ES.ArenaBanner(f)
  local wave, fighting, secs = tonumber(f[2] or "") or 0, f[3] == "1", tonumber(f[4] or "") or 0
  local best, heroes = tonumber(f[5] or "") or 0, tonumber(f[6] or "") or 1
  OpenWindow("ring", "The Proving Ring", "They come in waves that never end. Beat one and you get your breath back; every fifth brings a champion, and his wave pays into the camp's chest.", "Ability_Warrior_BattleShout")
  local now
  if fighting then
    now = "Wave " .. wave .. " is on you"
  elseif wave == 0 then
    now = "The first wave in " .. secs .. " seconds"
  else
    now = "Wave " .. wave .. " beaten: the next in " .. secs .. " seconds"
  end
  Row(1, {
    icon = "Ability_Warrior_BattleShout",
    name = "Raise the banner",
    tag = "Wave " .. (wave + 1),
    sub = fighting and "Beat this wave first" or "The next wave, now",
    go = "Call",
    cmd = ".es arena next",
    can = not fighting,
    detail = "No waiting: the next wave comes in at once.",
  })
  Row(2, {
    icon = "INV_Misc_Book_09",
    name = "Home to the camp",
    sub = "Your run ends here",
    go = "Go home",
    cmd = ".es travel camp",
    detail = "Back by the book at camp, with all you carry.",
  })
  Rest(now, "The camp's best: " .. (best > 0 and ("wave " .. best) or "none yet") .. ". In the ring: " .. heroes .. (heroes == 1 and " hero." or " heroes."), "")
end

-- --- Siege's muster banner (SL: open, status, secs, then key~name~text~seats~heroes~mine) ----------

function ES.Muster(f)
  OpenWindow("muster", "The Siege Muster", "Join a way to play: once its seats are full it begins, or start it with whoever's waiting. Every match starts the same: your class's Q and E, the plain kit.", "INV_BannerPVP_02")
  local ways = Items(f[5])
  local mine = nil
  for i = 1, table.getn(ways) do
    local w = Split(ways[i], "~")
    local seats = tonumber(w[4] or "") or 2
    local who = w[5] or ""
    local n = 0
    if who ~= "" then n = table.getn(Split(who, ", ")) end
    local me = w[6] == "1"
    if me then mine = w end
    Row(i, {
      icon = (seats > 2 and seats ~= 4) and "Ability_Warrior_BattleShout" or "INV_BannerPVP_02",
      name = w[2] or "?",
      tag = n .. " of " .. seats,
      sub = who ~= "" and who or (w[3] or ""),
      go = me and "Leave" or "Join",
      cmd = me and ".siege leave" or (".siege join " .. (w[1] or "")),
      chosen = me,
      detail = w[3] or "",
    })
  end
  local n = table.getn(ways)
  Row(n + 1, {
    icon = "Ability_Warrior_Charge",
    name = "Begin now",
    sub = "With whoever's waiting with you (two at least)",
    go = "Begin",
    cmd = ".siege start",
    can = mine ~= nil,
    detail = "Camps with nobody in them aren't played.",
  })
  Row(n + 2, {
    icon = "INV_Misc_Book_09",
    name = "Home to the camp",
    sub = "Back to Goldshire, your own hero again",
    go = "Go home",
    cmd = ".es travel camp",
    detail = "Leaving a match leaves your camp to hold without you.",
  })
  local secs = tonumber(f[4] or "") or 0
  Rest(f[3] or "", secs > 0 and (secs .. " seconds") or "", "")
end

-- --- a camp's Mercenary Ledger (SB: open, gold, income, sent to, wave, then sends) ------------------
-- Each send: key~name~cost~income~from~can~why~text~mob.

-- The ledger's top line: the purse, and how many creatures may still go with the next wave.
function ES.LedgerSub(gold, income, capField)
  local cap = Split(capField or "", "~")
  local sub = "Gold " .. gold .. ", +" .. income .. " a wave"
  if cap[2] and tonumber(cap[2]) and tonumber(cap[2]) > 0 then
    sub = sub .. "; sent " .. (cap[1] or "0") .. " of " .. cap[2] .. " for the next wave"
  end
  return sub
end

function ES.Ledger(f)
  ES.ledgerF = f
  local gold, income, to, wave = f[3] or "0", f[4] or "0", f[5] or "", tonumber(f[6] or "") or 0
  OpenWindow("ledger", "Mercenary Ledger", "Pay gold to send creatures at a rival: they join the rival's next wave, and each send raises your income (paid every wave) for the rest of the match. Only so many may go each wave.", "INV_Misc_Book_11")
  Row(1, {
    icon = "Ability_Hunter_SniperShot",
    name = "Sending to " .. (to ~= "" and to or "nobody"),
    sub = ES.LedgerSub(gold, income, f[8]),
    go = "Next",
    cmd = ".siege target",
    detail = "Your sends march on this camp. Next: the next rival camp still standing.",
  })
  local sends = Items(f[7])
  local per = LIST_ROWS - 1
  local first = PageOf("ledger", table.getn(sends), per)
  for i = 1, math.min(per, table.getn(sends) - first) do
    local x = Split(sends[first + i], "~")
    local can = x[6] == "1"
    local from = tonumber(x[5] or "") or 1
    local sub = "+" .. (x[4] or "0") .. " income"
    if from > wave + 1 then sub = sub .. ", from wave " .. from end
    Row(i + 1, {
      icon = "Ability_Hunter_BeastCall",
      name = x[2] or "?",
      tag = (x[3] or "?") .. " gold",
      sub = sub,
      go = "Send",
      cmd = ".siege send " .. (x[1] or ""),
      can = can,
      detail = x[8] or "",
      hint = (not can) and (x[7] or "") or "",
    })
  end
  Rest("Gold " .. gold, "Income " .. income .. " a wave, paid as each wave begins. Felling foes and the Blacksmith's veins pay too; felling a rival takes a share of theirs.", "")
end
ES.redraw.ledger = function() if ES.ledgerF then ES.Ledger(ES.ledgerF) end end

-- --- a lamp below (LP: key, name, draughts, most, the lamps lit, the wares, plunder) -----------------

function ES.Lamp(f)
  local name, have, most = f[3] or "Lamp", tonumber(f[4] or "") or 0, tonumber(f[5] or "") or 0
  local lamps, wares, plunder = Items(f[6]), Items(f[7]), tonumber(f[8] or "") or 0
  OpenWindow("lamp", name, "Resting heals you, fills your Draughts and brings the Brotherhood back to their posts (but never a boss). You wake here if you fall.", "Spell_Fire_Fire")
  local n = 1
  Row(n, {
    icon = "INV_Drink_04",
    name = "Rest at the lamp",
    tag = "Draughts " .. have .. " / " .. most,
    sub = "Whole again, Draughts full; the mine stirs",
    go = "Rest",
    cmd = ".es rest",
    detail = "Every foe you've beaten (but the bosses) is back at its post when you rest.",
  })
  n = n + 1
  Row(n, {
    icon = "INV_Misc_Book_09",
    name = "Home to the camp",
    sub = "By the smugglers' way: you'll be back by the book",
    go = "Go home",
    cmd = ".es travel camp",
    detail = "What you carry goes with you. Your Draughts stay below for next time.",
  })
  for i = 1, table.getn(lamps) do
    if n >= LIST_ROWS then break end
    local l = Split(lamps[i], "~")
    n = n + 1
    Row(n, { icon = "Spell_Fire_Fire", name = l[2] or "?", sub = "A lamp you've lit", go = "Travel", cmd = ".es travel " .. (l[1] or ""), detail = "Make your way there by the paths you've opened." })
  end
  for i = 1, table.getn(wares) do
    if n >= LIST_ROWS then break end
    local w = Split(wares[i], "~")
    local can = w[4] == "1"
    n = n + 1
    Row(n, {
      icon = w[5],
      name = w[2] or "?",
      tag = (w[3] or "?") .. " Plunder",
      sub = "From the lamp's quartermaster",
      go = "Buy",
      cmd = ".es buy " .. (w[1] or ""),
      can = can,
      detail = "Paid in Defias Plunder (you have " .. plunder .. ").",
      hint = can and "" or "Not enough plunder",
    })
  end
  Rest(name, "Defias Plunder: " .. plunder .. ". It's lost if you fall twice before you win your sack back.", "")
end

-- --- placing a station --------------------------------------------------------------------------------
-- While placing, the mouse is free (turn and look around to choose the spot); the station stands
-- in front of you.

local place = CreateFrame("Frame", "ElwynnSurvivalPlacing", UIParent)
place:SetWidth(520)
place:SetHeight(96)
place:SetPoint("BOTTOM", UIParent, "BOTTOM", 0, 200)
place:SetFrameStrata("DIALOG")
place:EnableMouse(true)
place.bg = Tex(place, "BACKGROUND", "banner", 520, 96)
place.bg:SetPoint("CENTER", place, "CENTER", 0, 0)
place.bg:SetVertexColor(0.1, 0.1, 0.1, 0.85)
place.title = Text(place, FONT_BODY, 15, "OUTLINE")
place.title:SetPoint("TOP", place, "TOP", 0, -10)
place.title:SetTextColor(1, 0.82, 0.45)
place.state = Text(place, FONT_BODY, 12, "OUTLINE")
place.state:SetPoint("TOP", place.title, "BOTTOM", 0, -4)
place.keys = Text(place, FONT_BODY, 11)
place.keys:SetPoint("TOP", place.state, "BOTTOM", 0, -3)
place.keys:SetTextColor(0.8, 0.8, 0.8)
place.keys:SetText("")
place.ok = MakeButton(place, 80, 22, "Place", GREEN, function() Say(".es place") end)
place.ok:SetPoint("BOTTOM", place, "BOTTOM", -90, 6)
place.turn = MakeButton(place, 80, 22, "Rotate", GOLD, function() Say(".es rot 1") end)
place.turn:SetPoint("BOTTOM", place, "BOTTOM", 0, 6)
place.cancel = MakeButton(place, 80, 22, "Cancel", WHITE, function() Say(".es cancel") end)
place.cancel:SetPoint("BOTTOM", place, "BOTTOM", 90, 6)
place:Hide()

-- BP: name, cost, why not (empty when it can go). Just "BP": not placing.
function ES.Placing(f)
  if not f[2] or f[2] == "" then
    if ES.placing then
      ES.placing = false
      place:Hide()
      if ClearGroundMarker then ClearGroundMarker("place") end
      ES.SetMode("action")
    end
    return
  end
  -- The keys as they are now (the card may have moved them).
  place.keys:SetText("It stands in front of you, inside the ring: hold right mouse to turn.   "
    .. (KeyName("ES_USE") or "F") .. ": place   " .. (KeyName("ES_SHEATHE") or "R") .. ": rotate   " .. (KeyName("ES_BUILD") or "H") .. ": cancel")
  -- A ring on the ground where it would go: green, it can; red, it can't.
  local at = Split(f[5] or "", "~")
  local x, y, z, r = tonumber(at[1] or ""), tonumber(at[2] or ""), tonumber(at[3] or ""), tonumber(at[4] or "")
  if SetGroundMarker and x and y and z and r then
    SetGroundMarker("place", x, y, z, r, 600, (f[4] and f[4] ~= "") and true or nil)
  end
  if not ES.placing then
    ES.placing = true
    win:Hide()
    ES.SetMode("cursor")
  end
  place.title:SetText(f[2] .. "  (" .. (f[3] or "") .. ")")
  if f[4] and f[4] ~= "" then
    place.state:SetText(f[4])
    place.state:SetTextColor(RED.r, RED.g, RED.b)
    place.ok:SetEnabledLook(false)
  else
    place.state:SetText("It can go here.")
    place.state:SetTextColor(GREEN.r, GREEN.g, GREEN.b)
    place.ok:SetEnabledLook(true)
  end
  place:Show()
end

-- --- asking first --------------------------------------------------------------------------------------

local confirm = CreateFrame("Frame", "ElwynnSurvivalConfirm", UIParent)
confirm:SetWidth(460)
confirm:SetHeight(140)
confirm:SetPoint("CENTER", UIParent, "CENTER", 0, 120)
confirm:SetFrameStrata("DIALOG")
confirm:EnableMouse(true)
Panel(confirm)
confirm.bg = confirm.panel
confirm.bg:SetAllPoints(confirm)
confirm.bg:SetVertexColor(0.15, 0.08, 0.06, 0.96)
confirm.text = Text(confirm, FONT_BODY, 13)
confirm.text:SetPoint("TOP", confirm, "TOP", 0, -18)
confirm.text:SetWidth(420)
confirm.yes = MakeButton(confirm, 120, 26, "Ring it", RED, function()
  Say(".es " .. (confirm.what or "raid") .. " yes")
  confirm:Hide()
  ES.SetMode("action")
end)
confirm.yes:SetPoint("BOTTOMRIGHT", confirm, "BOTTOM", -8, 14)
confirm.no = MakeButton(confirm, 120, 26, "Not yet", WHITE, function()
  Say(".es " .. (confirm.what or "raid") .. " no")
  confirm:Hide()
  ES.SetMode("action")
end)
confirm.no:SetPoint("BOTTOMLEFT", confirm, "BOTTOM", 8, 14)
confirm:Hide()
ES.confirm = confirm

-- --- the minimal interface ----------------------------------------------------------------------------
-- The stock interface's action bar (with its menu buttons), minimap, portraits and cast bar go
-- away while awake: each is parented to a hidden holder (so nothing it shows can catch a click)
-- and put back as it was on leaving. What stays reachable is on the icon bar bottom right.

local HIDDEN = { "MainMenuBar", "MinimapCluster", "PlayerFrame", "TargetFrame", "CastingBarFrame" }
local holder = CreateFrame("Frame", "ElwynnSurvivalHolder", UIParent)
holder:Hide()
ES.parents = nil

function ES.HideChrome()
  if ES.parents then return end
  ES.parents = {}
  for i = 1, table.getn(HIDDEN) do
    local f = getglobal(HIDDEN[i])
    if f and f.SetParent then
      ES.parents[HIDDEN[i]] = (f.GetParent and f:GetParent()) or UIParent
      f:SetParent(holder)
    end
  end
  -- Health bars over everyone's heads in place of the portraits (unless V has turned them off).
  if ElwynnSurvivalSettings and ElwynnSurvivalSettings.noBars then
    if HideNameplates then pcall(HideNameplates) end
    if HideFriendNameplates then pcall(HideFriendNameplates) end
  else
    if ShowNameplates then pcall(ShowNameplates) end
    if ShowFriendNameplates then pcall(ShowFriendNameplates) end
  end
  -- The chat stays where it is, for talking, and fades between lines (the client lays it out
  -- again as it likes, so it isn't moved).
  if ChatFrame1 and ChatFrame1.SetTimeVisible then
    pcall(ChatFrame1.SetTimeVisible, ChatFrame1, 15)
  end
end

function ES.RestoreChrome()
  if not ES.parents then return end
  for name, parent in pairs(ES.parents) do
    local f = getglobal(name)
    if f then
      f:SetParent(parent)
      -- Shown again under a shown parent: a hide and show makes sure it's drawn (and its own
      -- OnShow lays it out again). The cast bar stays hidden until a cast shows it.
      if f:IsShown() and name ~= "CastingBarFrame" then
        f:Hide()
        f:Show()
      end
    end
  end
  ES.parents = nil
  if HideFriendNameplates then pcall(HideFriendNameplates) end
  if ChatFrame1 and ChatFrame1.SetTimeVisible then
    pcall(ChatFrame1.SetTimeVisible, ChatFrame1, 120)
  end
end

local function Open(what)
  if what == "bag" then
    -- The backpack and the four satchels together.
    if OpenAllBags then OpenAllBags() elseif ToggleBackpack then ToggleBackpack() end
  elseif what == "character" then
    if ToggleCharacter then ToggleCharacter("PaperDollFrame") end
  elseif what == "map" then
    if ToggleWorldMap then ToggleWorldMap() end
  elseif what == "menu" then
    if ToggleGameMenu then
      ToggleGameMenu()
    elseif GameMenuFrame then
      if GameMenuFrame:IsShown() then HideUIPanel(GameMenuFrame) else ShowUIPanel(GameMenuFrame) end
    end
  elseif what == "keys" then
    ES.ToggleKeys()
  end
end
ES.Open = Open

-- The icon bar, bottom right: each opens its window; hover for its name and key.
local BAR_ICON = 34
local BAR = {
  { "bag", "Bag", "INV_Misc_Bag_08", "" },
  { "character", "Character", "INV_Helmet_01", "C" },
  { "map", "Map", "INV_Misc_Map_01", "M" },
  { "keys", "Controls", "INV_Misc_Key_03", "" },
  { "menu", "Game Menu", "INV_Misc_Gear_01", "Esc" },
}
local bar = CreateFrame("Frame", "ElwynnSurvivalBar", UIParent)
bar:SetWidth(table.getn(BAR) * (BAR_ICON + 6))
bar:SetHeight(BAR_ICON)
bar:SetPoint("BOTTOMRIGHT", UIParent, "BOTTOMRIGHT", -16, 14)
bar:Hide()
ES.bar = bar
for i = 1, table.getn(BAR) do
  local what, name, icon, key = BAR[i][1], BAR[i][2], BAR[i][3], BAR[i][4]
  local b = CreateFrame("Button", nil, bar)
  b:SetWidth(BAR_ICON)
  b:SetHeight(BAR_ICON)
  b:SetPoint("LEFT", bar, "LEFT", (i - 1) * (BAR_ICON + 6), 0)
  b.well = Tex(b, "BACKGROUND", "slotwell")
  b.well:SetAllPoints(b)
  b.well:SetVertexColor(0.02, 0.02, 0.02, 0.85)
  b.icon = Icon(b, "ARTWORK", icon)
  b.icon:SetPoint("TOPLEFT", b, "TOPLEFT", 4, -4)
  b.icon:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -4, 4)
  b.frame = Tex(b, "OVERLAY", "slot")
  b.frame:SetAllPoints(b)
  b.frame:SetVertexColor(FRAME.r, FRAME.g, FRAME.b, 0.8)
  b.count = Text(b, FONT_NUM, 10, "OUTLINE")
  b.count:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -3, 3)
  -- The old word, kept for anything that reads it; the icon says it now.
  b.label = Text(b, FONT_NUM, 10)
  b.label:SetText(name)
  b.label:Hide()
  b.what = what
  b.name = name
  b.key = key
  b:EnableMouse(true)
  b:RegisterForClicks("LeftButtonUp")
  b:SetScript("OnEnter", function()
    this.frame:SetVertexColor(1, 0.88, 0.55, 1)
    if GameTooltip and GameTooltip.SetOwner then
      GameTooltip:SetOwner(this, "ANCHOR_TOPLEFT")
      local k = this.key
      local action = ({ bag = "ES_BAG", character = "TOGGLECHARACTER0", map = "TOGGLEWORLDMAP", keys = "ES_KEYS" })[this.what]
      if action then k = KeyName(action) or "" end
      if k and k ~= "" then
        GameTooltip:SetText(this.name .. "  |cffaaaaaa(" .. k .. ")|r")
      else
        GameTooltip:SetText(this.name)
      end
      GameTooltip:Show()
    end
  end)
  b:SetScript("OnLeave", function()
    this.frame:SetVertexColor(FRAME.r, FRAME.g, FRAME.b, 0.8)
    if GameTooltip and GameTooltip.Hide then GameTooltip:Hide() end
  end)
  b:SetScript("OnClick", function()
    Play("click")
    Open(this.what)
  end)
  bar[what] = b
end

-- --- the board (N): five rows, one per key, nine cards in each --------------------------------------
-- Each row is a key (Q ranged, E close, 1 utility, 2 defense, 3 ultimate) and holds nine cards, one
-- from each class; click one to put it on its row's key. Hover a card for what it does, regular
-- and charged up (its EX version: hold the key a moment, or Ctrl).

local BOARD_COLS = { "warrior", "paladin", "hunter", "rogue", "priest", "shaman", "mage", "warlock", "druid" }
local BOARD_N = 9
local BOARD_ROWS = { "ranged", "close", "utility", "defense", "ultimate" }
local CARD_W, CARD_H, CARD_GAP = 112, 50, 5
local BOARD_LABEL = 100
local BOARD_W = 24 + BOARD_LABEL + BOARD_N * CARD_W + (BOARD_N - 1) * CARD_GAP + 24
local BOARD_TOP = 84
local BOARD_DETAIL = 128
local BOARD_H = BOARD_TOP + AB_N * (CARD_H + CARD_GAP) + BOARD_DETAIL + 74

local tree = CreateFrame("Frame", "ElwynnSurvivalTree", UIParent)
tree:SetWidth(BOARD_W)
tree:SetHeight(BOARD_H)
tree:SetPoint("CENTER", UIParent, "CENTER", 0, 20)
tree:SetFrameStrata("DIALOG")
tree:EnableMouse(true)
tree:Hide()
Panel(tree)
ES.tree = tree
if UISpecialFrames then
  table.insert(UISpecialFrames, "ElwynnSurvivalTree")
end
tree.title = Text(tree, FONT_TITLE, 22, "OUTLINE")
tree.title:SetPoint("TOP", tree, "TOP", 0, -14)
tree.title:SetTextColor(1, 0.82, 0.45)
tree.title:SetText("Abilities")
tree.points = Text(tree, FONT_BODY, 10)
tree.points:SetPoint("TOP", tree, "TOP", 0, -42)
tree.points:SetTextColor(0.75, 0.73, 0.68)
tree.close = MakeButton(tree, 90, 24, "Close", WHITE, function() tree:Hide() end)
tree.close:SetPoint("BOTTOMRIGHT", tree, "BOTTOMRIGHT", -16, 14)
tree.full = MakeButton(tree, 100, 24, "Dev: all", GOLD, function() Say(".es tree full") end)
tree.full:SetPoint("BOTTOMLEFT", tree, "BOTTOMLEFT", 16, 14)
tree.reset = MakeButton(tree, 100, 24, "Dev: reset", GOLD, function() Say(".es tree reset") end)
tree.reset:SetPoint("LEFT", tree.full, "RIGHT", 8, 0)
tree.hint = Text(tree, FONT_BODY, 10)
tree.hint:SetPoint("BOTTOM", tree, "BOTTOM", 0, 46)
tree.hint:SetTextColor(0.65, 0.63, 0.58)
tree.hint:SetText("Click a card to put it on its key.  Hold the key a moment (or Ctrl with it) for its charged-up version.")

-- The column heads.
local function ColX(c)
  return 24 + BOARD_LABEL + (c - 1) * (CARD_W + CARD_GAP)
end
local function RowY(r)
  return BOARD_TOP + (r - 1) * (CARD_H + CARD_GAP)
end
for c = 1, BOARD_N do
  local look = COLUMN_LOOK[BOARD_COLS[c]]
  local head = Text(tree, FONT_TITLE, 13, "OUTLINE")
  head:SetPoint("CENTER", tree, "TOPLEFT", ColX(c) + CARD_W / 2, -BOARD_TOP + 28)
  head:SetTextColor(look[2], look[3], look[4])
  head:SetText(look[1])
  local sub = Text(tree, FONT_BODY, 8)
  sub:SetPoint("CENTER", tree, "TOPLEFT", ColX(c) + CARD_W / 2, -BOARD_TOP + 12)
  sub:SetTextColor(look[2] * 0.75, look[3] * 0.75, look[4] * 0.75)
  sub:SetText(look[5])
end
-- The row heads: the key and what the row is for.
tree.rowKeys = {}
for r = 1, AB_N do
  local words = ROW_WORDS[BOARD_ROWS[r]]
  local y = RowY(r)
  local cap = Tex(tree, "ARTWORK", "keycap", 30, 22)
  cap:SetPoint("TOPLEFT", tree, "TOPLEFT", 24, -y - (CARD_H - 22) / 2)
  cap:SetVertexColor(0.16, 0.14, 0.12, 0.95)
  local k = Text(tree, FONT_NUM, 13, "OUTLINE")
  k:SetPoint("CENTER", cap, "CENTER", 0, 0)
  k:SetTextColor(1, 0.88, 0.6)
  tree.rowKeys[r] = k
  local name = Text(tree, FONT_TITLE, 13, "OUTLINE")
  name:SetPoint("LEFT", tree, "TOPLEFT", 24 + 38, -y - CARD_H / 2 + 8)
  name:SetWidth(BOARD_LABEL - 40)
  name:SetJustifyH("LEFT")
  name:SetTextColor(0.95, 0.9, 0.8)
  name:SetText(words[1])
  local sub = Text(tree, FONT_BODY, 8)
  sub:SetPoint("LEFT", tree, "TOPLEFT", 24 + 38, -y - CARD_H / 2 - 9)
  sub:SetWidth(BOARD_LABEL - 36)
  sub:SetJustifyH("LEFT")
  sub:SetTextColor(0.6, 0.58, 0.54)
  sub:SetText(words[2])
  if r > 1 then
    local rule = Tex(tree, "BORDER", "divider")
    rule:SetHeight(6)
    rule:SetPoint("TOPLEFT", tree, "TOPLEFT", 20, -y + CARD_GAP / 2 + 3)
    rule:SetPoint("TOPRIGHT", tree, "TOPRIGHT", -20, -y + CARD_GAP / 2 + 3)
    rule:SetVertexColor(FRAME.r, FRAME.g, FRAME.b, 0.18)
  end
end

-- The detail strip under the rows: the card hovered, regular on the left, charged up on the right.
local dy = RowY(AB_N + 1) + 4
tree.detail = CreateFrame("Frame", nil, tree)
tree.detail:SetPoint("TOPLEFT", tree, "TOPLEFT", 20, -dy)
tree.detail:SetPoint("TOPRIGHT", tree, "TOPRIGHT", -20, -dy)
tree.detail:SetHeight(BOARD_DETAIL)
tree.detail.bg = Tex(tree.detail, "BACKGROUND", "square")
tree.detail.bg:SetAllPoints(tree.detail)
tree.detail.bg:SetVertexColor(0, 0, 0, 0.35)
local half = (BOARD_W - 40) / 2
local function DetailColumn(x)
  local col = {}
  col.title = Text(tree.detail, FONT_TITLE, 15, "OUTLINE")
  col.title:SetPoint("TOPLEFT", tree.detail, "TOPLEFT", x + 12, -10)
  col.title:SetJustifyH("LEFT")
  col.tag = Text(tree.detail, FONT_NUM, 11, "OUTLINE")
  col.tag:SetPoint("TOPRIGHT", tree.detail, "TOPLEFT", x + half - 12, -12)
  col.tag:SetJustifyH("RIGHT")
  col.how = Text(tree.detail, FONT_BODY, 10)
  col.how:SetPoint("TOPLEFT", tree.detail, "TOPLEFT", x + 12, -32)
  col.how:SetWidth(half - 24)
  col.how:SetJustifyH("LEFT")
  col.how:SetTextColor(0.62, 0.72, 0.8)
  col.text = Text(tree.detail, FONT_BODY, 11)
  col.text:SetPoint("TOPLEFT", tree.detail, "TOPLEFT", x + 12, -50)
  col.text:SetWidth(half - 24)
  col.text:SetJustifyH("LEFT")
  col.text:SetTextColor(0.9, 0.88, 0.82)
  col.facts = Text(tree.detail, FONT_NUM, 11)
  col.facts:SetPoint("TOPLEFT", col.text, "BOTTOMLEFT", 0, -5)
  col.facts:SetWidth(half - 24)
  col.facts:SetJustifyH("LEFT")
  col.facts:SetTextColor(1, 0.82, 0.5)
  return col
end
tree.regular = DetailColumn(0)
tree.charged = DetailColumn(half)
tree.split = Tex(tree.detail, "ARTWORK", "bar")
tree.split:SetWidth(1)
tree.split:SetPoint("TOP", tree.detail, "TOP", 0, -10)
tree.split:SetPoint("BOTTOM", tree.detail, "BOTTOM", 0, 10)
tree.split:SetVertexColor(FRAME.r, FRAME.g, FRAME.b, 0.3)

tree.cards = {}
local function BoardCard(i)
  local b = tree.cards[i]
  if b then return b end
  b = CreateFrame("Button", nil, tree)
  b:SetWidth(CARD_W)
  b:SetHeight(CARD_H)
  b:SetFrameLevel(tree:GetFrameLevel() + 2)
  b.plate = Tex(b, "BACKGROUND", "square")
  b.plate:SetAllPoints(b)
  b.rim = Tex(b, "OVERLAY", "squareline")
  b.rim:SetAllPoints(b)
  b.accent = Tex(b, "ARTWORK", "bar")
  b.accent:SetPoint("TOPLEFT", b, "TOPLEFT", 6, -2)
  b.accent:SetPoint("TOPRIGHT", b, "TOPRIGHT", -6, -2)
  b.accent:SetHeight(2)
  b.well = Tex(b, "BORDER", "square", 32, 32)
  b.well:SetPoint("LEFT", b, "LEFT", 6, -1)
  b.well:SetVertexColor(0, 0, 0, 0.7)
  b.icon = b:CreateTexture(nil, "ARTWORK")
  b.icon:SetWidth(28)
  b.icon:SetHeight(28)
  b.icon:SetPoint("CENTER", b.well, "CENTER", 0, 0)
  b.icon:SetTexCoord(0.08, 0.92, 0.08, 0.92)
  -- The name on up to two lines, the EX version's under it.
  b.name = Text(b, FONT_BODY, 9, "OUTLINE")
  b.name:SetPoint("TOPLEFT", b, "TOPLEFT", 42, -6)
  b.name:SetWidth(CARD_W - 46)
  b.name:SetHeight(22)
  b.name:SetJustifyH("LEFT")
  b.name:SetJustifyV("TOP")
  b.ex = Text(b, FONT_BODY, 8)
  b.ex:SetPoint("TOPLEFT", b, "TOPLEFT", 42, -29)
  b.ex:SetWidth(CARD_W - 46)
  b.ex:SetHeight(10)
  b.ex:SetJustifyH("LEFT")
  b.ex:SetTextColor(1, 0.7, 0.35)
  b.state = Text(b, FONT_NUM, 8, "OUTLINE")
  b.state:SetPoint("BOTTOMRIGHT", b, "BOTTOMRIGHT", -5, 3)
  b.state:SetJustifyH("RIGHT")
  b:EnableMouse(true)
  b:RegisterForClicks("LeftButtonUp")
  b:SetScript("OnEnter", function() ES.BoardDetail(this.d) this.hover = true ES.DrawCard(this) end)
  b:SetScript("OnLeave", function() this.hover = nil ES.DrawCard(this) ES.BoardDetail(tree.resting) end)
  b:SetScript("OnClick", function() ES.TreeClick(this) end)
  tree.cards[i] = b
  return b
end

function ES.DrawCard(b)
  local d = b.d
  if not d then return end
  local look = COLUMN_LOOK[d.school] or COLUMN_LOOK.warrior
  b.accent:SetVertexColor(look[2], look[3], look[4], d.owned and 0.9 or 0.3)
  if d.on ~= "" then
    b.plate:SetVertexColor(0.2, 0.16, 0.09, 0.95)
    b.rim:SetVertexColor(1, 0.85, 0.45, 1)
  elseif b.hover then
    b.plate:SetVertexColor(0.13, 0.12, 0.1, 0.95)
    b.rim:SetVertexColor(look[2], look[3], look[4], 0.9)
  else
    b.plate:SetVertexColor(0.07, 0.065, 0.06, 0.92)
    b.rim:SetVertexColor(FRAME.r * 0.6, FRAME.g * 0.6, FRAME.b * 0.6, 0.55)
  end
end

-- How a card is used, in words, with the key it's on (or would go on).
function ES.HowText(row, aimed, hi, n)
  local key = KeyName("ES_AB" .. n) or "its key"
  if row == "ultimate" then
    if aimed then
      return "Hold " .. key .. " to bring it up, then " .. ((hi > 0) and "hold the attack to charge it and let go" or "attack") .. " to unleash it."
    end
    return "Hold " .. key .. " a moment to unleash it."
  end
  if aimed then
    if hi > 0 then
      return "Press " .. key .. " to aim; hold " .. (KeyName("ES_ATTACK") or "attack") .. " to charge, let go to cast."
    end
    return "Press " .. key .. " to aim; " .. (KeyName("ES_ATTACK") or "attack") .. " to cast."
  end
  return "Press " .. key .. "."
end

function ES.ExHowText(n, valor)
  local key = KeyName("ES_AB" .. n) or "its key"
  local ex = KeyName("ES_EX" .. n)
  local alt = (ex and ex ~= "") and (" (or " .. ex .. ")") or ""
  return "Hold " .. key .. " a moment" .. alt .. "."
end

local ROW_N = { ranged = 1, close = 2, utility = 3, defense = 4, ultimate = 5 }

function ES.BoardDetail(d)
  local r, c = tree.regular, tree.charged
  if not d then
    r.title:SetText("Choose a card for each key")
    r.title:SetTextColor(0.95, 0.9, 0.8)
    r.tag:SetText("")
    r.how:SetText("")
    r.text:SetText("Two keys to deal damage (one to reach the foe, one for the brawl), one for getting in, out or through, one for staying alive, and an ultimate that takes the whole Valor bar. Any card goes with any other.")
    r.facts:SetText("")
    c.title:SetText("")
    c.tag:SetText("")
    c.how:SetText("")
    c.text:SetText("")
    c.facts:SetText("")
    return
  end
  local look = COLUMN_LOOK[d.school] or COLUMN_LOOK.warrior
  local n = ROW_N[d.row] or 1
  r.title:SetText(d.name)
  r.title:SetTextColor(look[2], look[3], look[4])
  r.tag:SetText(look[1] .. "  ·  " .. (ROW_WORDS[d.row] and ROW_WORDS[d.row][1] or d.row))
  r.tag:SetTextColor(0.6, 0.58, 0.54)
  r.how:SetText(ES.HowText(d.row, d.aimed, d.hi, n))
  r.text:SetText(d.text)
  if not d.owned and d.where and d.where ~= "" then
    r.facts:SetText("|cffff9966Not yet learned. " .. d.where .. ".|r  " .. d.facts)
  else
    r.facts:SetText(d.facts)
  end
  if d.exKey ~= "" then
    c.title:SetText("Charged up: " .. d.exName)
    c.title:SetTextColor(1, 0.7, 0.35)
    c.tag:SetText(d.exValor .. " Valor")
    c.tag:SetTextColor(1, 0.82, 0.45)
    c.how:SetText(ES.ExHowText(n, d.exValor))
    c.text:SetText(d.exText)
    c.facts:SetText(d.exFacts)
  elseif d.row == "ultimate" then
    c.title:SetText("No charged-up version")
    c.title:SetTextColor(0.6, 0.58, 0.54)
    c.tag:SetText("")
    c.how:SetText("")
    c.text:SetText("An ultimate takes the whole Valor bar: save it for this, or spend it on charged-up versions as you go.")
    c.facts:SetText("")
  else
    c.title:SetText("")
    c.tag:SetText("")
    c.how:SetText("")
    c.text:SetText("")
    c.facts:SetText("")
  end
end

-- TR: the board: dev, what the count says, then per card key~name~row~column~icon~owned~can
-- unlock~on key~kind~aimed~least charge~full charge~text~facts~EX key~EX name~EX icon~EX Valor~EX
-- text~EX facts.
function ES.TreeLine(f)
  local dev = f[2] == "1"
  tree.points:SetText(f[3] or "")
  local parts = Split(f[4] or "", "^")
  local used = {}
  for r = 1, AB_N do
    tree.rowKeys[r]:SetText(ShortKey("ES_AB" .. r))
  end
  tree.resting = nil
  for i = 1, table.getn(parts) do
    local e = Split(parts[i], "~")
    local r, c = ROW_N[e[3] or ""], nil
    for k = 1, BOARD_N do
      if BOARD_COLS[k] == e[4] then c = k end
    end
    if e[1] and e[1] ~= "" and r and c then
      local d = {
        key = e[1], name = e[2] or "", row = e[3], school = e[4], icon = e[5] or "", owned = e[6] == "1",
        can = e[7] == "1", on = e[8] or "", kind = e[9] or "", aimed = e[10] == "1",
        lo = tonumber(e[11] or "") or 0, hi = tonumber(e[12] or "") or 0, text = e[13] or "", facts = e[14] or "",
        exKey = e[15] or "", exName = e[16] or "", exIcon = e[17] or "", exValor = tonumber(e[18] or "") or 0,
        exText = e[19] or "", exFacts = e[20] or "", where = e[21] or "",
      }
      local slot = (r - 1) * BOARD_N + c
      used[slot] = true
      local b = BoardCard(slot)
      b:ClearAllPoints()
      b:SetPoint("TOPLEFT", tree, "TOPLEFT", ColX(c), -RowY(r))
      b.d = d
      b.icon:SetTexture(ICONS .. d.icon)
      if d.owned then
        b.icon:SetVertexColor(1, 1, 1)
        b.name:SetTextColor(0.95, 0.92, 0.85)
      else
        b.icon:SetVertexColor(0.4, 0.4, 0.4)
        b.name:SetTextColor(0.55, 0.53, 0.5)
      end
      b.name:SetText(d.name)
      b.ex:SetText(d.exName ~= "" and ("EX " .. d.exName) or ((d.row == "ultimate") and "Whole Valor bar" or ""))
      if d.on ~= "" then
        b.state:SetText("on " .. ShortKey("ES_AB" .. r))
        b.state:SetTextColor(1, 0.85, 0.45)
        if not tree.resting then tree.resting = d end
      elseif d.owned then
        b.state:SetText("")
      elseif d.can then
        b.state:SetText("unlock")
        b.state:SetTextColor(0.45, 1, 0.45)
      else
        b.state:SetText("locked")
        b.state:SetTextColor(0.6, 0.45, 0.4)
      end
      ES.DrawCard(b)
      b:Show()
    end
  end
  for slot, b in pairs(tree.cards) do
    if not used[slot] then b:Hide() end
  end
  if dev then tree.full:Show() tree.reset:Show() else tree.full:Hide() tree.reset:Hide() end
  local hovered = nil
  for _, b in pairs(tree.cards) do
    if b.hover and b:IsShown() then hovered = b.d end
  end
  ES.BoardDetail(hovered or tree.resting)
  if not tree:IsShown() then
    tree:Show()
    Play("open")
  end
end

function ES.TreeClick(b)
  local d = b.d
  if not d then return end
  Play("click")
  if d.owned then
    Say(".es slot " .. d.row .. " " .. d.key)
  elseif d.can then
    Say(".es tree take " .. d.key)
  elseif d.where and d.where ~= "" then
    ES.Message(d.name .. " is learned out in Elwynn: " .. d.where .. ".")
  else
    ES.Message(d.name .. " isn't unlocked yet.")
  end
end

-- --- the controls card ------------------------------------------------------------------------------
-- What each control is right now: the key read back from the game's bindings (so it says what a
-- press does, whatever set it). Click a key to change it: the next key or mouse button pressed,
-- with Shift, Ctrl or Alt as held, takes its place (remembered for the next session, and never
-- written to your own saved bindings). Below: your own saved bindings instead, the middle
-- button's lock-on, and where and how big the health bars are.

local CONTROLS = {
  { "Move forward", "MOVEFORWARD" },
  { "Move back", "MOVEBACKWARD" },
  { "Strafe left", "STRAFELEFT" },
  { "Strafe right", "STRAFERIGHT" },
  { "Look around (turns the hero)", nil, "The mouse" },
  { "Attack (again: the next blow)", "ES_ATTACK" },
  { "Heavy attack", "ES_HEAVY" },
  { "Guard (just in time: parry)", "ES_GUARD" },
  { "Shield bash / kick", nil, "guardbash" },
  { "Running attack", nil, "running" },
  { "Dodge (hold: sprint)", "ES_DODGE" },
  { "Sprint", nil, "Shift (hold)" },
  { "Jump", "JUMP" },
  { "Lock on / let go", "ES_LOCK" },
  { "Next foe (locked on)", "ES_NEXT" },
  { "Free the mouse", "ES_MODE" },
  { "Ranged ability", "ES_AB1" },
  { "Close ability", "ES_AB2" },
  { "Utility ability", "ES_AB3" },
  { "Defense ability", "ES_AB4" },
  { "Ultimate (hold)", "ES_AB5" },
  { "Charged-up version (Valor)", nil, "ex" },
  { "Aimed ability: cast / cancel", nil, "cast" },
  { "Use / place", "ES_USE" },
  { "Sheathe / draw", "ES_SHEATHE" },
  { "Quick item", "ES_EAT" },
  { "Next quick item", "ES_CYCLE" },
  { "Walk / run", "TOGGLERUN" },
  { "Autorun", "TOGGLEAUTORUN" },
  { "Bags", "ES_BAG" },
  { "Ability board", "ES_TREE" },
  { "Journal", "ES_JOURNAL" },
  { "Build", "ES_BUILD" },
  { "Character", "TOGGLECHARACTER0" },
  { "Map", "TOGGLEWORLDMAP" },
  { "Health bars over heads", "ES_BARS" },
  { "Controls (this card)", "ES_KEYS" },
}
ES.CONTROLS = CONTROLS
local CARD_ROWS = 19
local CARD_COL = 400

-- What a row's fixed words are, from the keys in effect.
local function FixedText(what)
  if what == "guardbash" then
    return (KeyName("ES_GUARD") or "Guard") .. " + " .. (KeyName("ES_ATTACK") or "attack")
  elseif what == "running" then
    return (KeyName("ES_ATTACK") or "Attack") .. " while sprinting"
  elseif what == "ex" then
    -- "Ctrl + the key" when every EX key is that; else each one.
    local same = true
    for n = 1, AB_N - 1 do
      local a, x = GetBindingKey and GetBindingKey("ES_AB" .. n), GetBindingKey and GetBindingKey("ES_EX" .. n)
      if not a or not x or x ~= "CTRL-" .. a then same = false end
    end
    if same then return "Hold its key (or Ctrl + it)" end
    local t = {}
    for n = 1, AB_N - 1 do table.insert(t, KeyName("ES_EX" .. n) or "-") end
    return table.concat(t, " ")
  elseif what == "cast" then
    return (KeyName("ES_ATTACK") or "Attack") .. " / " .. (KeyName("ES_GUARD") or "guard")
  end
  return what
end

function ES.ControlKey(i)
  local c = CONTROLS[i]
  if c[3] then return FixedText(c[3]) end
  local a = KeyName(c[2]) or "not bound"
  if c[4] then return a .. " / " .. (KeyName(c[4]) or "not bound") end
  return a
end

-- The card: a margin all round (clear of the frame's border), the keys in two columns (a label,
-- and on the right the key, which a click changes; the few that follow from others are plain
-- grey words), then the settings, each a line saying what it is with its value on a button.
local card = CreateFrame("Frame", "ElwynnSurvivalKeys", UIParent)
card.M, card.ROW_H, card.KEY_W = 40, 20, 190
card.KEYS_TOP, card.SET_ROW_H = 108, 30
card.SET_COL = (2 * CARD_COL + 20) / 3
card:SetWidth(2 * CARD_COL + 20 + 2 * card.M)
card:SetHeight(card.KEYS_TOP + CARD_ROWS * card.ROW_H + 52 + 3 * card.SET_ROW_H + 54 + card.M)
card:SetPoint("CENTER", UIParent, "CENTER", 0, 0)
card:SetFrameStrata("DIALOG")
card:EnableMouse(true)
Panel(card)
card.bg = card.panel
card.bg:SetAllPoints(card)
card.bg:SetVertexColor(0.12, 0.1, 0.08, 0.96)
card.title = Text(card, FONT_TITLE, 22, "OUTLINE")
-- Below the frame's inner rim, not on it.
card.title:SetPoint("TOP", card, "TOP", 0, -card.M - 10)
card.title:SetTextColor(1, 0.82, 0.45)
card.title:SetText("Controls")
card.hint = Text(card, FONT_BODY, 11)
card.hint:SetPoint("TOP", card.title, "BOTTOM", 0, -6)
card.hint:SetTextColor(0.75, 0.75, 0.75)
card.rows = {}
for i = 1, table.getn(CONTROLS) do
  local col = (i > CARD_ROWS) and 1 or 0
  local row = i - col * CARD_ROWS
  local x = card.M + col * (CARD_COL + 20)
  local y = -card.KEYS_TOP - (row - 1) * card.ROW_H
  local changeable = CONTROLS[i][2] and not CONTROLS[i][4]
  local l = Text(card, FONT_BODY, 12)
  l:SetPoint("LEFT", card, "TOPLEFT", x, y - card.ROW_H / 2)
  l:SetWidth(CARD_COL - card.KEY_W - 10)
  l:SetHeight(card.ROW_H)
  l:SetJustifyH("LEFT")
  l:SetTextColor(0.85, 0.85, 0.85)
  local b = CreateFrame("Button", nil, card)
  b:SetWidth(card.KEY_W)
  b:SetHeight(card.ROW_H - 2)
  b:SetPoint("TOPRIGHT", card, "TOPLEFT", x + CARD_COL, y)
  b.text = Text(b, FONT_BODY, 12)
  b.text:SetPoint("RIGHT", b, "RIGHT", -4, 0)
  b.text:SetWidth(card.KEY_W - 8)
  b.text:SetJustifyH("RIGHT")
  b.band = Tex(b, "BACKGROUND", "rowband")
  b.band:SetAllPoints(b)
  b.band:SetVertexColor(1, 0.85, 0.5, 0)
  b.i = i
  b.fixed = not changeable
  if changeable then
    b.text:SetTextColor(1, 0.82, 0.45)
    b:EnableMouse(true)
    b:RegisterForClicks("LeftButtonUp")
    b:SetScript("OnEnter", function() if not OwnKeys() then this.band:SetVertexColor(1, 0.85, 0.5, 0.18) end end)
    b:SetScript("OnLeave", function() this.band:SetVertexColor(1, 0.85, 0.5, 0) end)
    b:SetScript("OnClick", function() ES.CaptureKey(this.i) end)
  else
    -- Follows from other keys (or is the mouse itself): words, not a key to click.
    b.text:SetTextColor(0.6, 0.6, 0.6)
  end
  card.rows[i] = { l, b.text, b }
end

-- The settings: three columns of lines, each its name and its value (a click changes it).
card.setTop = card.KEYS_TOP + CARD_ROWS * card.ROW_H + 18
card.setHead = Text(card, FONT_TITLE, 16, "OUTLINE")
card.setHead:SetPoint("TOPLEFT", card, "TOPLEFT", card.M, -card.setTop)
card.setHead:SetTextColor(1, 0.82, 0.45)
card.setHead:SetText("Settings")
card.setNames = {}
-- A setting's line at column `col` (1-3), row `row` (1-3): its name, and where its value goes.
function card.Setting(col, row, name)
  local x = card.M + (col - 1) * card.SET_COL
  local y = -card.setTop - 30 - (row - 1) * card.SET_ROW_H
  local t = Text(card, FONT_BODY, 12)
  t:SetPoint("LEFT", card, "TOPLEFT", x, y - 12)
  t:SetJustifyH("LEFT")
  t:SetTextColor(0.85, 0.85, 0.85)
  t:SetText(name)
  table.insert(card.setNames, t)
  return x + card.SET_COL - 16, y
end
function card.Place(b, rightX, y)
  b:ClearAllPoints()
  b:SetPoint("TOPRIGHT", card, "TOPLEFT", rightX, y)
end

card.switch = MakeButton(card, 130, 24, "", GOLD, function()
  local set = Settings()
  if OwnKeys() then
    set.ownKeys = nil
  else
    set.ownKeys = 1
  end
  ES.Rebind()
  ES.DrawKeys()
  ES.DrawAbilities()
  ES.DrawQuick()
end)
card.Place(card.switch, card.Setting(1, 1, "Key layout"))
-- Only while there are changed keys to put back (and the Elwynn layout is the one in use).
card.reset = MakeButton(card, 130, 24, "Put them back", GOLD, function()
  if OwnKeys() or not (ElwynnSurvivalSettings and ElwynnSurvivalSettings.custom and next(ElwynnSurvivalSettings.custom)) then return end
  Settings().custom = nil
  ES.Rebind()
  ES.DrawKeys()
  ES.DrawAbilities()
  ES.DrawQuick()
end)
card.resetRight, card.resetY = card.Setting(1, 2, "Keys you changed")
card.Place(card.reset, card.resetRight, card.resetY)
card.resetNone = Text(card, FONT_BODY, 12)
card.resetNone:SetPoint("RIGHT", card, "TOPLEFT", card.resetRight - 4, card.resetY - 12)
card.resetNone:SetTextColor(0.6, 0.6, 0.6)
card.middle = MakeButton(card, 130, 24, "", GOLD, function()
  local set = Settings()
  if set.middleLock then set.middleLock = nil else set.middleLock = 1 end
  ES.Rebind()
  ES.DrawKeys()
end)
card.Place(card.middle, card.Setting(1, 3, "Middle mouse"))
-- The mouse's speed: its value between slower and faster.
card.mouseRight, card.mouseY = card.Setting(2, 1, "Mouse speed")
card.faster = MakeButton(card, 26, 24, "+", GOLD, function()
  ES.StepMouse(1)
  ES.DrawKeys()
end)
card.Place(card.faster, card.mouseRight, card.mouseY)
card.mouseText = Text(card, FONT_BODY, 12)
card.mouseText:SetPoint("RIGHT", card.faster, "LEFT", -6, 0)
card.mouseText:SetWidth(46)
card.mouseText:SetJustifyH("CENTER")
card.mouseText:SetTextColor(1, 0.82, 0.45)
card.slower = MakeButton(card, 26, 24, "-", GOLD, function()
  ES.StepMouse(-1)
  ES.DrawKeys()
end)
card.slower:SetPoint("RIGHT", card.mouseText, "LEFT", -6, 0)
-- The floor markers: a creature's blow coming (its wedge or ring), ground about to burn, holy
-- ground. Off, none are drawn (those up now go at once).
card.marks = MakeButton(card, 130, 24, "", GOLD, function()
  local set = Settings()
  if set.noMarks then set.noMarks = nil else set.noMarks = 1 end
  if set.noMarks then ES.ClearMarks() end
  ES.DrawKeys()
end)
card.Place(card.marks, card.Setting(2, 2, "Floor markers"))
card.where = MakeButton(card, 130, 24, "", GOLD, function()
  local set = Settings()
  if ES.VitalsWhere() == "corner" then set.vitals = "hero" else set.vitals = "corner" end
  ES.PlaceVitals()
  ES.DrawKeys()
end)
card.Place(card.where, card.Setting(3, 1, "Health bar"))
card.size = MakeButton(card, 130, 24, "", GOLD, function()
  local set = Settings()
  local now, nextSize = ES.VitalsScale(), VITALS_SIZES[1]
  for i = 1, table.getn(VITALS_SIZES) do
    if VITALS_SIZES[i] > now + 0.01 then nextSize = VITALS_SIZES[i] break end
  end
  set.vitalsScale = nextSize
  ES.PlaceVitals()
  ES.DrawKeys()
end)
card.Place(card.size, card.Setting(3, 2, "Health bar size"))

-- Under the settings: how the keys work now (and, on a stock client, what it's missing).
card.profile = Text(card, FONT_BODY, 11)
card.profile:SetPoint("BOTTOMLEFT", card, "BOTTOMLEFT", card.M, card.M + 34)
card.profile:SetWidth(2 * CARD_COL + 20 - 110)
card.profile:SetJustifyH("LEFT")
card.profile:SetTextColor(0.75, 0.75, 0.75)
card.client = Text(card, FONT_BODY, 11)
card.client:SetPoint("BOTTOMLEFT", card, "BOTTOMLEFT", card.M, card.M)
card.client:SetWidth(2 * CARD_COL + 20 - 110)
card.client:SetJustifyH("LEFT")
card.client:SetTextColor(0.75, 0.75, 0.75)
card.close = MakeButton(card, 90, 26, "Close", WHITE, function() card:Hide() end)
card.close:SetPoint("BOTTOMRIGHT", card, "BOTTOMRIGHT", -card.M, card.M)
card:Hide()
ES.card = card
if UISpecialFrames then
  table.insert(UISpecialFrames, "ElwynnSurvivalKeys")
end

-- Changing a key: the next press (a key or a mouse button, with its modifiers) takes its place.
local capture = CreateFrame("Frame", "ElwynnSurvivalCapture", card)
capture:SetAllPoints(card)
capture:SetFrameStrata("FULLSCREEN_DIALOG")
capture:EnableMouse(true)
if capture.EnableKeyboard then capture:EnableKeyboard(true) end
capture.bg = Tex(capture, "BACKGROUND", "band")
capture.bg:SetAllPoints(capture)
capture.bg:SetVertexColor(0, 0, 0, 0.75)
capture.text = Text(capture, FONT_TITLE, 18, "OUTLINE")
capture.text:SetPoint("CENTER", capture, "CENTER", 0, 14)
capture.text:SetTextColor(1, 0.85, 0.5)
capture.sub = Text(capture, FONT_BODY, 12)
capture.sub:SetPoint("TOP", capture.text, "BOTTOM", 0, -8)
capture.sub:SetTextColor(0.8, 0.8, 0.8)
capture.sub:SetText("Shift, Ctrl and Alt count as held. Esc: leave it. Backspace: back to the Elwynn key.")
capture:Hide()
ES.capture = capture

local MOUSE_NAMES = { LeftButton = "BUTTON1", RightButton = "BUTTON2", MiddleButton = "BUTTON3", Button4 = "BUTTON4", Button5 = "BUTTON5" }
local MODIFIER_KEYS = { SHIFT = 1, CTRL = 1, ALT = 1, LSHIFT = 1, RSHIFT = 1, LCTRL = 1, RCTRL = 1, LALT = 1, RALT = 1 }

local function Chord(key)
  local mods = ""
  if IsAltKeyDown and IsAltKeyDown() then mods = mods .. "ALT-" end
  if IsControlKeyDown and IsControlKeyDown() then mods = mods .. "CTRL-" end
  if IsShiftKeyDown and IsShiftKeyDown() then mods = mods .. "SHIFT-" end
  return mods .. key
end

function ES.CaptureKey(i)
  if OwnKeys() then
    ES.Message("Your own keys are in use: change them in the game's Key Bindings, or switch back to the Elwynn keys.")
    return
  end
  local c = CONTROLS[i]
  capture.action = c[2]
  capture.label = c[1]
  capture.text:SetText("Press the new key for " .. c[1])
  capture:Show()
end

-- Put `chord` on the action being changed (nil: back to the Elwynn key).
function ES.SetKey(action, chord)
  local set = Settings()
  set.custom = set.custom or {}
  local taken
  if chord then
    local list = ES.Layout()
    for i = 1, table.getn(list) do
      if list[i][1] == chord and list[i][2] ~= "" and list[i][2] ~= action then taken = list[i][2] end
    end
  end
  set.custom[action] = chord
  -- An ability key moved: its EX version follows with Ctrl unless that was changed too.
  ES.Rebind()
  ES.DrawKeys()
  ES.DrawAbilities()
  ES.DrawQuick()
  if taken then
    local name = getglobal("BINDING_NAME_" .. taken) or taken
    ES.Message(Pretty(chord) .. " was " .. name .. "'s: that has no key now.")
  end
end

local function Captured(key)
  local action = capture.action
  capture:Hide()
  if not action then return end
  ES.SetKey(action, Chord(key))
end

capture:SetScript("OnKeyDown", function()
  local key = arg1
  if not key or MODIFIER_KEYS[key] then return end
  if key == "ESCAPE" then
    this:Hide()
    return
  end
  if key == "BACKSPACE" then
    local action = this.action
    this:Hide()
    if action then ES.SetKey(action, nil) end
    return
  end
  Captured(key)
end)
capture:SetScript("OnMouseDown", function()
  local key = MOUSE_NAMES[arg1]
  if key then Captured(key) end
end)

function ES.DrawKeys()
  for i = 1, table.getn(CONTROLS) do
    card.rows[i][1]:SetText(CONTROLS[i][1])
    card.rows[i][2]:SetText(ES.ControlKey(i))
  end
  -- Only a stock client needs telling what it's missing.
  if AnimClient() or ActionClient() then
    card.client:SetText("")
  else
    card.client:SetText("Stock client: run scripts/play-survival.sh (or update the Windows folder) for the steering mouse and the over-the-shoulder aim.")
  end
  local own = OwnKeys()
  if own then
    -- What has no key at all (its Elwynn key was taken by one of yours): bind it in Key Bindings.
    local none = 0
    for i = 1, table.getn(CONTROLS) do
      local action = CONTROLS[i][2]
      if action and string.sub(action, 1, 3) == "ES_" and not (GetBindingKey and GetBindingKey(action)) then none = none + 1 end
    end
    local gap = (none > 0) and string.format(" %d of its actions have no key yet.", none) or ""
    card.hint:SetText("Your own key bindings: change them in the game's Key Bindings (Elwynn Survival's are under its heading).")
    card.profile:SetText("The Elwynn keys fill in whatever yours leave free." .. gap)
    card.switch.label:SetText("Your own")
  else
    card.hint:SetText("Click a key to change it. The plain ones follow from other keys.")
    card.profile:SetText("Keys you change are kept for next time. Your own saved bindings come back when you leave Elwynn.")
    card.switch.label:SetText("Elwynn layout")
  end
  -- Putting changed keys back: only when there are some, in the Elwynn layout.
  local changed = ElwynnSurvivalSettings and ElwynnSurvivalSettings.custom and next(ElwynnSurvivalSettings.custom) ~= nil
  if changed and not own then
    card.reset:Show()
    card.resetNone:SetText("")
  else
    card.reset:Hide()
    card.resetNone:SetText(own and "(Elwynn layout only)" or "None")
  end
  card.middle.label:SetText((ElwynnSurvivalSettings and ElwynnSurvivalSettings.middleLock) and "Locks on" or "Hold to look")
  card.where.label:SetText((ES.VitalsWhere() == "corner") and "Top left" or "By the hero")
  card.size.label:SetText(string.format("%d%%", math.floor(ES.VitalsScale() * 100 + 0.5)))
  card.marks.label:SetText(ES.MarksOn() and "Shown" or "Hidden")
  card.mouseText:SetText(string.format("%d%%", math.floor(ES.MouseSpeed() * 100 + 0.5)))
end

function ES.ToggleKeys()
  if card:IsShown() then
    card:Hide()
  else
    ES.DrawKeys()
    card:Show()
  end
end

-- --- every frame: sprint, held keys, the death screen -------------------------------------------------

local function Typing()
  return ChatFrameEditBox and ChatFrameEditBox:IsVisible()
end

-- Walking with the bow drawn (Aimed Shot's charge): the server lets the legs walk, and the
-- upper body holds the draw (HoldBow) meanwhile, on the clients with an action layer.
local HOLD_BOW = 109
function ES.BowWalk()
  local want = false
  if ActionLayer() and ActionMoveKeys and ES.action and ES.action.name == "charge" and ES.Kit() == "bow" then
    local f, r = ActionMoveKeys()
    want = (f ~= 0 or r ~= 0)
  end
  if want and not ES.bowWalk then
    ES.bowWalk = true
    PlayUnitAnimation("player", HOLD_BOW, 0, nil, nil, 0, 60, "upper")
  elseif not want and ES.bowWalk then
    ES.bowWalk = nil
    if StopUnitAnimation then StopUnitAnimation("player", 0.12) end
  end
end

local poll = CreateFrame("Frame")
poll:SetScript("OnUpdate", function()
  if not ES.awake then return end
  ES.RunTimers()
  ES.Markers()
  ES.BowWalk()
  ES.StepExHold()
  ES.StepLockHold()
  local typing = Typing()
  -- Space held past a tap: a sprint.
  if ES.spaceAt and not ES.spaceSprint and GetTime() - ES.spaceAt >= HOLD_TO_SPRINT then
    ES.spaceSprint = true
  end
  local shift = IsShiftKeyDown and IsShiftKeyDown()
  ES.Sprint(((shift or ES.spaceSprint) and not typing) and true or false)
  if typing then
    if ES.guardHeld then ES.Guard("up") end
    if ES.attackHeld then ES.Attack("up") end
    ES.spaceAt = nil
    ES.spaceSprint = false
  end
  -- The mouse steers in action mode, unless a window wants the cursor or the dead lie still.
  if ActionClient() then
    ES.Steer(ES.mode == "action" and not typing and not CursorWanted() and not ES.placing)
  end
  -- A look ended outside the catcher (the stock client).
  if ES.lookHeld and IsMouselooking and not IsMouselooking() and GetTime() - (ES.lookAt or 0) > 0.3 then
    ES.Look("up")
  end
  ES.DrawDeath()
end)

-- --- the server's lines -----------------------------------------------------------------------------

function ES.Wake()
  if ES.awake then return end
  ES.awake = true
  local reset = ES.CheckLayout()
  ES.ApplyKeys()
  -- The mouse speed chosen on the Controls card, as it was (a new build's settings, or the
  -- game's own sliders, don't change it).
  local m = tonumber(ElwynnSurvivalSettings and ElwynnSurvivalSettings.mouse or "")
  if m and m > 0 then ES.SetMouseSpeed(m) end
  ES.HideChrome()
  vit:Show()
  info:Show()
  feed:Show()
  if ES.boss.who then ES.boss:Show() end
  bar:Show()
  hurt:Show()
  dock:Show()
  ES.PlaceVitals()
  ES.DrawAction()
  ES.DrawAbilities()
  ES.SetMode("action")
  -- The camera stops short of other bodies instead of entering them.
  if AnimClient() and SetCameraUnitCollision then SetCameraUnitCollision(1) end
  ES.Message(string.format("%s attacks, %s guards, %s dodges; abilities on %s %s %s %s, the ultimate on %s (hold it). %s lists every key.",
    KeyName("ES_ATTACK") or "Left mouse", KeyName("ES_GUARD") or "right mouse", KeyName("ES_DODGE") or "Space",
    KeyName("ES_AB1") or "-", KeyName("ES_AB2") or "-", KeyName("ES_AB3") or "-", KeyName("ES_AB4") or "-", KeyName("ES_AB5") or "-",
    KeyName("ES_KEYS") or "The key icon"))
  if reset then
    if DEFAULT_CHAT_FRAME then
      DEFAULT_CHAT_FRAME:AddMessage("|cffffd24aElwynn Survival:|r this version starts everyone on the Elwynn keys; " .. (KeyName("ES_KEYS") or "K") .. " shows and changes them.")
    end
  end
end

function ES.Sleep()
  if ES.lookHeld and MouselookStop then MouselookStop() end
  ES.Steer(false)
  -- Whatever holds the look now lets go, so the next screen (a logout's) has its cursor.
  if IsMouselooking and IsMouselooking() and MouselookStop then MouselookStop() end
  if SetAimMarker then SetAimMarker(nil) end
  if ClearGroundMarker then ClearGroundMarker() end
  if ClearTelegraph then ClearTelegraph() end
  ES.aimMarker = nil
  if ES.orbitHeld and ToggleMouseMove then ToggleMouseMove() end
  if ES.sprintHeld then Say(".es in spu") end
  ES.lookHeld, ES.guardHeld, ES.attackHeld, ES.orbitHeld, ES.sprintHeld = nil, nil, nil, nil, false
  ES.spaceAt, ES.spaceSprint = nil, false
  if SetActionMouse then SetActionMouse(nil) end
  if SetAimCamera then SetAimCamera(nil) end
  if SetCameraUnitCollision then SetCameraUnitCollision(nil) end
  ES.predicted, ES.predictedRoll = nil, nil
  ES.mine, ES.comboUntil, ES.moves, ES.lockOn = nil, nil, nil, nil
  ES.timers = {}
  if SetLockOn then SetLockOn(nil) end
  ES.aimCam = nil
  ES.RestoreKeys()
  ES.RestoreChrome()
  ES.awake = false
  ES.placing = false
  ES.upAt = nil
  ES.lastHp = nil
  ES.h = nil
  catcher:Hide()
  vit:Hide()
  act:Hide()
  info:Hide()
  feed:Hide()
  boss:Hide()
  foe:Hide()
  hurt:Hide()
  win:Hide()
  place:Hide()
  confirm:Hide()
  death:Hide()
  bar:Hide()
  card:Hide()
  aim:Hide()
  hit:Hide()
  dock:Hide()
  ES.cc:Hide()
  ES.prompt:Hide()
  ES.heal:Hide()
  ES.tree:Hide()
  ES.capture:Hide()
  ES.slots, ES.abilities = {}, {}
  ES.action = { name = "ready", start = 0, total = 0, ability = "" }
end

function ES.Line(line)
  local f = Split(line, "\t")
  local verb = f[1]
  if verb == "V" then
    ES.Wake()
    return
  end
  if not ES.awake then return end
  if verb == "H" then
    ES.h = f
    ES.DrawHud()
    ES.DrawEffects()
  elseif verb == "EF" then
    ES.EffectsLine(f)
  elseif verb == "GL" then
    ES.GoalLine(f)
  elseif verb == "BS" then
    ES.BossLine(f)
  elseif verb == "WN" then
    -- A blow no guard stops: the creature's weapon goes red where the client can show it, else
    -- the red mark over the hero.
    local secs = tonumber(f[2] or "")
    if not ES.WeaponGlow(f[3], secs) then ES.Danger(secs) end
  elseif verb == "WG" then
    ES.WeaponGlow(f[2], tonumber(f[3] or ""))
  elseif verb == "GJ" then
    ES.JournalLine(f)
  elseif verb == "PM" then
    ES.PinsLine(f)
  elseif verb == "A" then
    local elapsed = tonumber(f[3]) or 0
    ES.action = { name = f[2] or "ready", start = GetTime() - elapsed, total = tonumber(f[4]) or 0, ability = f[5] or "" }
    ES.DrawAction()
    ES.DrawAbilities()
  elseif verb == "AB" then
    ES.AbilitiesLine(f)
  elseif verb == "RC" then
    ES.RecastLine(f)
  elseif verb == "EM" then
    ES.EmpowerLine(f)
  elseif verb == "VA" then
    ES.ValorLine(f)
  elseif verb == "CC" then
    ES.CcLine(f)
  elseif verb == "TR" then
    ES.TreeLine(f)
  elseif verb == "NP" then
    ES.PromptLine(f)
  elseif verb == "HL" then
    ES.HealLine(f)
  elseif verb == "ST" then
    -- A swing called off (a feint): ours went at once; another's lets go now.
    if f[3] == "1" then
      if ES.mine then
        if StopUnitAnimation then StopUnitAnimation("player", 0.1) end
        ES.mine = nil
      end
    elseif StopUnitAnimation and f[2] then
      StopUnitAnimation(f[2], 0.1)
    end
  elseif verb == "TG" then
    -- A creature's blow (or a hero's wind-up) coming: its shape, reach and timing on the ground
    -- under it; a strip's width last. (Floor markers off in the Controls card: none.)
    local key = "es" .. (f[2] or "")
    -- The striker itself lights up for its wind-up (its weapon, or a beast's whole body: a bear's
    -- paw is small), amber for a blow a guard stops and red for one it doesn't, so a blow reads
    -- with the floor markers off too.
    if f[3] and f[3] ~= "" and SetUnitWeaponGlow and (f[8] == "block" or f[8] == "unblockable") then
      local secs = tonumber(f[7]) or 0.8
      SetUnitWeaponGlow(f[3], (f[8] == "block") and "FFB030" or DANGER_RED, math.max(0.3, secs + 0.05))
    end
    -- Its tells, for a blow meant for us (the flash to answer it, the woosh); gone, none.
    if f[3] and f[3] ~= "" and f[10] == "1" and (f[8] == "block" or f[8] == "unblockable") then
      ES.Tell(key, f[3], tonumber(f[7]), f[8], tonumber(f[9]), tonumber(f[12]))
    else
      -- Gone, or no longer meant for us: nothing more from it.
      ES.tells[key] = nil
    end
    if f[3] and SetTelegraph and ES.MarksOn() then
      SetTelegraph(key, f[3], f[4] or "wedge", tonumber(f[5]) or 2, tonumber(f[6]) or 90,
        tonumber(f[7]) or 1, f[8] or "block", tonumber(f[9]) or 0.25, f[10] == "1", tonumber(f[11]) or 0)
      ES.marksUp.tg[key] = 1
    elseif ClearTelegraph then
      ClearTelegraph(key)
      ES.marksUp.tg[key] = nil
    end
  elseif verb == "TA" then
    -- A shape laid on the ground at a spot (a sky-fall's ring, a strip): gone as a TG.
    if SetTelegraphAt and ES.MarksOn() then
      local key = "es" .. (f[2] or "")
      SetTelegraphAt(key, tonumber(f[3]) or 0, tonumber(f[4]) or 0, tonumber(f[5]) or 0,
        tonumber(f[6]) or 0, f[7] or "ring", tonumber(f[8]) or 2, tonumber(f[9]) or 360, tonumber(f[10]) or 1,
        f[11] or "unblockable", tonumber(f[12]) or 0, f[13] == "1")
      ES.marksUp.tg[key] = 1
    end
  elseif verb == "MK" then
    -- A blow coming down there: its circle on the ground (the client lays it), or gone.
    local key = "es" .. (f[2] or "")
    if SetGroundMarker and f[3] and ES.MarksOn() then
      -- A friend's ground (holy ground) green, a warning red.
      local hostile = 1
      if f[8] == "1" then hostile = nil end
      SetGroundMarker(key, tonumber(f[3]) or 0, tonumber(f[4]) or 0, tonumber(f[5]) or 0, tonumber(f[6]) or 1, tonumber(f[7]) or 1, hostile)
      ES.marksUp.mk[key] = 1
    elseif ClearGroundMarker then
      ClearGroundMarker(key)
      ES.marksUp.mk[key] = nil
    end
  elseif verb == "DA" then
    -- A dash of ours (Charge, Intercept): the body runs it here, as a dodge's.
    if ActionDash then ActionDash(tonumber(f[2]) or 0.3, tonumber(f[3]) or 20, "f") end
  elseif verb == "T" then
    ES.Message(f[2] or "")
  elseif verb == "NT" then
    if ES.Letter then ES.Letter(f) end
  elseif verb == "WT" then
    if ES.WagerLine then ES.WagerLine(f) end
  elseif verb == "DP" then
    if ES.OwnMeter then ES.OwnMeter(f) end
  elseif verb == "DM" then
    if ES.FightMeter then ES.FightMeter(f) end
  elseif verb == "FL" then
    ES.Flash(f[2] or "")
  elseif verb == "WD" then
    vit.stamina:Flash()
  elseif verb == "HT" then
    ES.HitMark(f[3], f[4])
  elseif verb == "AN" then
    ES.SwingLine(f)
  elseif verb == "IM" then
    ES.ImpactLine(f)
  elseif verb == "RO" then
    ES.RollLine(f)
  elseif verb == "RE" then
    ES.ReactLine(f)
  elseif verb == "MV" then
    ES.MovesLine(f)
  elseif verb == "LO" then
    ES.LockLine(f)
  elseif verb == "PI" then
    -- The server times our round trip by this.
    Say(".es po " .. (f[2] or "0"))
  elseif verb == "KF" then
    ES.Feed(f[2], f[3])
  elseif verb == "BN" then
    ES.Banner(f[2] or "", f[3] or "")
  elseif verb == "S" then
    ES.Chest(f[2])
  elseif verb == "W" then
    ES.Station(f[3] or "", f[4], f[5] or "")
  elseif verb == "AR" then
    ES.Rack(f[2])
  elseif verb == "BK" then
    ES.Book(f[2], f[3], f[4], f[5], f[6])
  elseif verb == "AA" then
    ES.ArenaLine(f)
  elseif verb == "AW" then
    ES.ArenaBanner(f)
  elseif verb == "LP" then
    ES.Lamp(f)
  elseif verb == "WR" then
    ES.below = f[2] == "below"
    ES.siege = f[2] == "siege"
    -- The Bookmaker's clock is only below.
    if not ES.below and ES.WagerLine then ES.WagerLine({}) end
  elseif verb == "SG" then
    ES.SiegeLine(f)
  elseif verb == "SL" then
    if f[2] == "1" or (win:IsShown() and win.kind == "muster") then ES.Muster(f) end
  elseif verb == "SB" then
    if f[2] == "1" or (win:IsShown() and win.kind == "ledger") then ES.Ledger(f) end
  elseif verb == "BM" then
    ES.BuildMenu(f[2])
  elseif verb == "BP" then
    ES.Placing(f)
  elseif verb == "CF" then
    confirm.what = f[2]
    confirm.text:SetText(f[3] or "")
    confirm:Show()
    Play("raid")
    WindowMode()
  elseif verb == "DE" then
    local secs = tonumber(f[2]) or 0
    if secs > 0 then
      ES.upAt = GetTime() + secs
      ES.CloseWindows()
      Play("fall")
    else
      ES.upAt = nil
      ES.lastHp = nil
      if StaticPopup_Hide then StaticPopup_Hide("DEATH") end
    end
  end
end

-- --- events --------------------------------------------------------------------------------------------

local events = CreateFrame("Frame")
events:RegisterEvent("CHAT_MSG_ADDON")
events:RegisterEvent("WORLD_MAP_UPDATE")
events:RegisterEvent("PLAYER_ENTERING_WORLD")
events:RegisterEvent("PLAYER_LEAVING_WORLD")
events:RegisterEvent("PLAYER_LOGOUT")
events:RegisterEvent("PLAYER_DEAD")
events:SetScript("OnEvent", function()
  if event == "CHAT_MSG_ADDON" then
    if arg1 == "ES" and arg2 then
      ES.Line(arg2)
    end
  elseif event == "PLAYER_ENTERING_WORLD" then
    -- Asleep until an Elwynn Survival server answers. "ac": this client aims over the shoulder.
    -- ac9: a creature's swing trails its last stroke into the blow (not the whole wind-up).
    if LocalHits() and SetTelegraphAt and (tonumber(HasActionCombat()) or 0) >= 9 then
      Say(".es hello " .. VERSION .. " ac9")
    elseif LocalHits() and SetTelegraphAt and (tonumber(HasActionCombat()) or 0) >= 8 then
      Say(".es hello " .. VERSION .. " ac8")
    elseif LocalHits() and SetTelegraph and (tonumber(HasActionCombat()) or 0) >= 7 then
      Say(".es hello " .. VERSION .. " ac7")
    elseif LocalHits() and SetUnitWeaponGlow and (tonumber(HasActionCombat()) or 0) >= 6 then
      Say(".es hello " .. VERSION .. " ac6")
    elseif LocalHits() and SetGroundMarker and (tonumber(HasActionCombat()) or 0) >= 5 then
      Say(".es hello " .. VERSION .. " ac5")
    elseif LocalHits() then
      Say(".es hello " .. VERSION .. " ac4")
    elseif ActionLayer() then
      Say(".es hello " .. VERSION .. " ac3")
    elseif AnimClient() then
      Say(".es hello " .. VERSION .. " ac2")
    elseif ActionClient() then
      Say(".es hello " .. VERSION .. " ac")
    else
      Say(".es hello " .. VERSION)
    end
  elseif event == "WORLD_MAP_UPDATE" then
    if ES.awake then ES.DrawPins() end
  elseif event == "PLAYER_LEAVING_WORLD" or event == "PLAYER_LOGOUT" then
    if ES.awake or event == "PLAYER_LEAVING_WORLD" then ES.Sleep() end
  elseif event == "PLAYER_DEAD" then
    -- Getting up is the server's: no release button.
    if ES.awake and StaticPopup_Hide then
      StaticPopup_Hide("DEATH")
    end
  end
end)
