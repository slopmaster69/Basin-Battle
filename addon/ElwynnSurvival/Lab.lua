-- Elwynn Survival: the spell lab's camera (dev builds' `.es lab`, see docs/spell-lab.md).
--
-- The server sends, when a lab run starts:
--   LB n ms label
-- and this takes n screenshots, one every ms milliseconds, from now: the frames the server's
-- trace (logs/lab/<label>.txt) is timed against. The client numbers shots taken within one second
-- (WoWScrnShot_<date>_<time>_2.png ...), so a burst keeps every frame. The "Screen captured"
-- line is kept out of the frames while the burst runs.
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local lab = { left = 0, every = 0.08, next = 0, label = "" }
ES.lab = lab

local frame = CreateFrame("Frame")
frame:Hide()

local function Quiet(on)
  if not ScreenshotStatus then return end
  if on then
    ScreenshotStatus:UnregisterEvent("SCREENSHOT_SUCCEEDED")
    ScreenshotStatus:UnregisterEvent("SCREENSHOT_FAILED")
    ScreenshotStatus:Hide()
  else
    ScreenshotStatus:RegisterEvent("SCREENSHOT_SUCCEEDED")
    ScreenshotStatus:RegisterEvent("SCREENSHOT_FAILED")
  end
end

frame:SetScript("OnUpdate", function()
  local now = GetTime()
  if lab.left <= 0 then
    frame:Hide()
    Quiet(false)
    if DEFAULT_CHAT_FRAME then
      DEFAULT_CHAT_FRAME:AddMessage("Lab: " .. lab.taken .. " shots of " .. lab.label .. " saved in Screenshots.")
    end
    return
  end
  if now >= lab.next then
    Screenshot()
    lab.taken = lab.taken + 1
    lab.left = lab.left - 1
    -- Kept on the schedule from the start, not from this frame, so a slow frame doesn't push
    -- every later shot back.
    lab.next = lab.next + lab.every
    if lab.next < now then lab.next = now end
  end
end)

function ES.LabLine(f)
  lab.left = tonumber(f[2]) or 30
  lab.every = (tonumber(f[3]) or 80) / 1000
  lab.label = f[4] or ""
  lab.taken = 0
  lab.next = GetTime()
  Quiet(true)
  frame:Show()
end
