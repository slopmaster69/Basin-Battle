-- ElwynnSurvival: the Priest's looks drawn by the add-on (Corey's art, art/check/priest_*).
--
-- Holy Word: Salvation (`SV key x y z radius beats`): a great circle of consecrated stone on the
-- ground where the Priest stood, for everyone; on each beat a ring of golden light runs out from
-- the centre to the edge, smoothly (the server's stepped friend's ring is left off on a client
-- that draws this), and flashes as it reaches it: the edge is the line everyone times their blow
-- to (Corey). The centre's sigil flares as each ring sets out. Drawn with SetGroundRing (patch
-- 0041); a client without it keeps the server's ring.

local ES = ElwynnSurvival
local ART = "Interface\\AddOns\\ElwynnSurvival\\art\\check\\"

-- Where each texture's own line sits, as a share of the radius it's drawn at: the ground circle's
-- stone band near its edge, the rim's thin ring inside its padding.
local GROUND_LINE = 0.92
local RIM_LINE = 0.69
local SIGIL_SIZE = 1.6
-- How long a beat's flash lasts, and the circle's fade after its last beat.
local FLASH = 0.45
local FADE = 1.2

local sigils = {}

local frame = CreateFrame("Frame", "ElwynnSurvivalPriest", WorldFrame)
frame:Hide()

-- `SV key x y z radius b1,b2,b3`: the beats are seconds from now.
function ES.SigilLine(f)
  if not SetGroundRing then return end
  local key = f[2]
  local x, y, z, r = tonumber(f[3]), tonumber(f[4]), tonumber(f[5]), tonumber(f[6])
  if not (key and x and y and z and r) then return end
  local now = GetTime()
  local beats = {}
  for b in string.gfind(f[7] or "", "[^,]+") do
    local t = tonumber(b)
    if t then table.insert(beats, now + t) end
  end
  if table.getn(beats) == 0 then return end
  sigils[key] = { x = x, y = y, z = z, r = r, since = now, beats = beats }
  frame:Show()
end

local function Ring(key, s, tex, radius, a, yaw, rr, gg, bb)
  SetGroundRing(key, s.x, s.y, s.z, radius, ART .. tex, rr or 1, gg or 1, bb or 1, a, yaw)
end

local function Clear(key)
  if ClearGroundRing then
    ClearGroundRing(key .. "_ground")
    ClearGroundRing(key .. "_rim")
    ClearGroundRing(key .. "_sigil")
    ClearGroundRing(key .. "_flash")
  end
end

frame:SetScript("OnUpdate", function()
  if not SetGroundRing then sigils = {} this:Hide() return end
  local now = GetTime()
  local any = false
  for key, s in pairs(sigils) do
    local k = "es_sigil_" .. key
    local n = table.getn(s.beats)
    local last = s.beats[n]
    if now > last + FADE then
      Clear(k)
      sigils[key] = nil
    else
      any = true
      -- The circle: in over a moment, turning slowly, breathing; out after the last beat.
      local a = math.min(1, (now - s.since) / 0.3)
      if now > last then a = 1 - (now - last) / FADE end
      a = a * (0.85 + 0.1 * math.sin(now * 2.1))
      Ring(k .. "_ground", s, "priest_salvation_ground", s.r / GROUND_LINE, a, now * 0.08)
      -- The ring: from the centre out to the edge over each beat's span.
      local from, to = s.since, nil
      for i = 1, n do
        if s.beats[i] > now then to = s.beats[i] break end
        from = s.beats[i]
      end
      local glow = 0
      if to then
        local q = (now - from) / math.max(0.05, to - from)
        local rr = s.r * q
        if rr > 0.15 then
          Ring(k .. "_rim", s, "priest_salvation_rim", rr / RIM_LINE, 0.55 + 0.45 * q, now * 0.6)
        end
        glow = q * q
      elseif ClearGroundRing then
        ClearGroundRing(k .. "_rim")
      end
      -- A beat just fell: the edge flashes and swells a little, and the sigil flares as the next
      -- ring sets out from it.
      local since = now - from
      local flash = (from > s.since and since < FLASH) and (1 - since / FLASH) or 0
      Ring(k .. "_sigil", s, "priest_salvation_sigil", SIGIL_SIZE * (1 + 0.5 * flash), math.min(1, a * (0.5 + 0.3 * (1 - glow)) + flash), -now * 0.3)
      if flash > 0 then
        Ring(k .. "_flash", s, "priest_salvation_rim", (s.r * (1 + 0.12 * (1 - flash))) / RIM_LINE, flash, 0, 1, 0.95, 0.75)
      elseif ClearGroundRing then
        ClearGroundRing(k .. "_flash")
      end
    end
  end
  if not any then this:Hide() end
end)
