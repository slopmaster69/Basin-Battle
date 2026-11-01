-- Elwynn Survival: the death recap. Under "YOU HAVE FALLEN" and its countdown, what felled you:
-- the last blow, the two before it, and a word on answering it, so a fall teaches something.
--
-- The server sends `DE secs blows tip`: blows are lines with `^` between, the last blow first
-- ("Targorr the Dread's heavy blow, through your guard (212)"); either may be empty. `DE 0`
-- (back on your feet) clears it.
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local death = ElwynnSurvivalDown
if not death or not death.sub then return end

local BODY = "Interface\\AddOns\\ElwynnSurvival\\fonts\\CrimsonPro-Regular.ttf"
local BAND = 140
local MAX_LINES = 3

-- The band grows downward to hold the recap: its top and the title stay where they were.
death:ClearAllPoints()
death:SetPoint("TOPLEFT", UIParent, "LEFT", 0, 40 + BAND / 2)
death:SetPoint("TOPRIGHT", UIParent, "RIGHT", 0, 40 + BAND / 2)
if death.title then
  death.title:ClearAllPoints()
  death.title:SetPoint("CENTER", death, "TOP", 0, 10 - BAND / 2)
end

local function Line(size, r, g, b)
  local t = death:CreateFontString(nil, "OVERLAY")
  t:SetFont(BODY, size, "OUTLINE")
  t:SetTextColor(r, g, b)
  t:SetJustifyH("CENTER")
  t:Hide()
  return t
end

-- The blow that felled you in bone white, the ones before it fainter, the tip in gold.
local lines = {}
lines[1] = Line(15, 0.93, 0.88, 0.8)
lines[1]:SetPoint("TOP", death.sub, "BOTTOM", 0, -12)
for i = 2, MAX_LINES do
  lines[i] = Line(13, 0.62, 0.57, 0.52)
  lines[i]:SetPoint("TOP", lines[i - 1], "BOTTOM", 0, -3)
end
local tip = Line(13, 0.85, 0.68, 0.3)

local function Clear()
  for i = 1, MAX_LINES do
    lines[i]:Hide()
  end
  tip:Hide()
  death:SetHeight(BAND)
end

-- What felled you, from the death line's third and fourth fields.
function ES.Recap(blows, words)
  Clear()
  local n = 0
  if blows and blows ~= "" and ES.Split then
    local items = ES.Split(blows, "^")
    for i = 1, math.min(MAX_LINES, table.getn(items)) do
      if items[i] ~= "" then
        n = n + 1
        local text = items[i]
        if n == 1 then text = "Felled by " .. text elseif n == 2 then text = "Before it: " .. text end
        lines[n]:SetText(text)
        lines[n]:Show()
      end
    end
  end
  local below = death.sub
  if n > 0 then below = lines[n] end
  if words and words ~= "" then
    tip:ClearAllPoints()
    tip:SetPoint("TOP", below, "BOTTOM", 0, -10)
    tip:SetText(words)
    tip:Show()
  end
  -- The band grows to hold it.
  local extra = 0
  if n > 0 then extra = 22 + (n - 1) * 17 end
  if words and words ~= "" then extra = extra + 24 end
  death:SetHeight(BAND + extra)
  ES.recapShown = { lines = n, tip = words or "" }
end

function ES.RecapClear()
  Clear()
  ES.recapShown = nil
end
