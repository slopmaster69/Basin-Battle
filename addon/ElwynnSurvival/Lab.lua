-- Elwynn Survival: the spell lab's camera (dev builds' `.es lab`, see docs/spell-lab.md).
--
-- The server sends, when a lab run starts:
--   LB n ms label [cam] [secs] [hud]   (cam: behind, close or none; behind if it's missing;
--                                       hud: the hero's own bars kept in the film)
--   LW secs speed keys           (a `walk:` step: the body runs that long, that fast, that way)
-- and this takes n screenshots, one every ms milliseconds, from now: the frames the server's
-- trace (logs/lab/<label>.txt) is timed against. The client numbers shots taken within one second
-- (WoWScrnShot_<date>_<time>_2.png ...), so a burst keeps every frame. The "Screen captured"
-- line is kept out of the frames while the burst runs, and so is the whole interface (bars, chat,
-- windows: a window left open once covered the hero in every frame of a night's runs), shown
-- again a moment after the burst, when the server's video of it has ended too.
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local lab = { left = 0, every = 0.08, next = 0, label = "", show_at = nil }
ES.lab = lab

local frame = CreateFrame("Frame")
frame:Hide()

-- The aiming reticle stays in the film while the interface is hidden (Corey couldn't see the
-- Shield aimed at the friend): lifted onto the world for the burst, put back after.
local function KeepReticle(on, hud)
  local frames = { ElwynnSurvivalAim }
  -- `hud`: the health and stamina bars too (the health bar's shield and overhealth on film).
  if hud or not on then table.insert(frames, ElwynnSurvivalVitals) end
  for i = 1, table.getn(frames) do
    local f = frames[i]
    if f then
      if on then f:SetParent(WorldFrame) else f:SetParent(UIParent) end
      f:SetFrameStrata("HIGH")
    end
  end
end

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
    -- The interface back once the video's last frame is past.
    if lab.show_at and now < lab.show_at then return end
    lab.show_at = nil
    if UIParent then UIParent:Show() end
    KeepReticle(false)
    frame:Hide()
    Quiet(false)
    if DEFAULT_CHAT_FRAME and lab.taken > 0 then
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

-- The same view for every run, so runs compare: behind the hero at a set distance, nearer for a
-- scene at arm's length (`side`, `close`). The server locks the hero on to the scene, so the body
-- and the camera turn to it; turning the camera here turned the body with it, off the scene.
local function Frame(cam)
  if cam == "none" then return end
  -- View 4 is the high one (30 degrees down): from behind at a steeper angle the hero hides
  -- less of what stands in front of them.
  if ResetView and SetView then
    ResetView(4)
    SetView(4)
  end
  -- Nearer than the high view's own distance: from there the hero was a small figure in a wide
  -- field, the ability's look smaller still.
  if CameraZoomIn then
    if cam == "side" or cam == "close" then CameraZoomIn(5) else CameraZoomIn(3) end
  end
end

-- A `walk:` step. The client moves the hero and an add-on may not hold a move key, so the body
-- runs it as a dash does (the action layer's), at the pace the server says is the hero's now.
function ES.LabWalk(f)
  if ActionDash then ActionDash(tonumber(f[2]) or 1, tonumber(f[3]) or 7, f[4] or "f") end
end

function ES.LabLine(f)
  Frame(f[5] or "behind")
  lab.left = tonumber(f[2]) or 30
  lab.every = (tonumber(f[3]) or 80) / 1000
  lab.label = f[4] or ""
  lab.taken = 0
  lab.next = GetTime()
  -- A run recorded as video sends no stills (n is 0) and its video's length (secs).
  local secs = tonumber(f[6]) or 0
  if secs < lab.left * lab.every then secs = lab.left * lab.every end
  lab.show_at = lab.next + secs + 1.0
  Quiet(true)
  KeepReticle(true, f[7] == "hud")
  if UIParent then UIParent:Hide() end
  frame:Show()
end
