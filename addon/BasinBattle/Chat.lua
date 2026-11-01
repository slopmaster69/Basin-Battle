-- Basin Battle: the chat sits at the bottom left, above the spell bar's switches, and keeps to
-- what people say, your loot and what needs you. (Elwynn Survival lays out its own screen, so
-- none of this runs there.)
--
-- The game's running commentary (waves counting down, who planted what, bounties, a friend back
-- on their feet) goes to the short notes under the wave tracker, which fade after a few seconds.
-- Lines the screen already shows (the pause, the next wave's plan, a banner just shown) are left
-- out of chat. Type /bbchat to see every line in chat again (and /bbchat again to tidy it).

local CHAT_WIDTH = 430
local CHAT_HEIGHT = 140
-- Room under the chat for the line you type, above the switches over the spell bar.
local CHAT_LIFT = 34
-- How far in from the left edge of the screen.
local CHAT_LEFT = 16

local BB = BasinBattle

local function InBasin()
  local realm = GetRealmName and GetRealmName() or ""
  return not string.find(realm, "Elwynn")
end

local placing = false
local function PlaceChat()
  -- (The stock frame, with its methods: a stand-in without them is left alone.)
  if placing or not ChatFrame1 or not ChatFrame1.ClearAllPoints then return end
  if not InBasin() then return end
  placing = true
  -- Above the stock bar and the Auto/Manual and Talents switches that sit on top of it.
  local bottom = 117
  if MainMenuBar and MainMenuBar:GetTop() then
    bottom = MainMenuBar:GetTop() + 30 + CHAT_LIFT
  end
  ChatFrame1:ClearAllPoints()
  ChatFrame1:SetPoint("BOTTOMLEFT", UIParent, "BOTTOMLEFT", CHAT_LEFT, bottom)
  ChatFrame1:SetWidth(CHAT_WIDTH)
  ChatFrame1:SetHeight(CHAT_HEIGHT)
  placing = false
end

-- The stock layout moves the chat whenever bars come and go; put it back each time.
if UIParent_ManageFramePositions then
  local orig = UIParent_ManageFramePositions
  UIParent_ManageFramePositions = function(a1, a2, a3)
    orig(a1, a2, a3)
    PlaceChat()
  end
end
if FCF_RestorePositionAndDimensions then
  local orig = FCF_RestorePositionAndDimensions
  FCF_RestorePositionAndDimensions = function(a1, a2, a3)
    orig(a1, a2, a3)
    PlaceChat()
  end
end

local chat = CreateFrame("Frame")
chat:RegisterEvent("PLAYER_ENTERING_WORLD")
chat:RegisterEvent("UPDATE_CHAT_WINDOWS")
chat:RegisterEvent("VARIABLES_LOADED")
chat:SetScript("OnEvent", function()
  PlaceChat()
  -- Once more a moment later, after the stock code has had its turn this frame.
  local started = GetTime()
  this:SetScript("OnUpdate", function()
    if GetTime() - started > 0.2 then
      this:SetScript("OnUpdate", nil)
      PlaceChat()
    end
  end)
end)

-- --- less clutter ------------------------------------------------------------------------------

-- Shown elsewhere already: the pause banner, the fronts panel's plan, the wave tracker's clock.
local SHOWN_ELSEWHERE = {
  "^Paused while ",
  "^Next, wave %d+:",
  "^Wave %d+ in %d+ seconds%.",
}

-- The game's commentary: to the notes under the wave tracker instead of chat.
local TO_NOTES = {
  " plants a Treant Protector",
  " raises a Healing Spring",
  " hires a ",
  " fortifies the Ancient",
  " for the soldiers%.",
  " grows stronger%.$",
  "^Thornbark rank ",
  " sends a soldier from ",
  " is claiming ",
  " joins the defence%.$",
  " is back, just as they left%.$",
  " has left%.$",
  " is back in the fight!$",
  " has fallen! Back at the Ancient",
  "^Wave %d+ bounty:",
  "a bonus level for everyone!$",
  " took too long: the first card",
  " blew up at the ",
  "^A new run begins%.$",
  "^You have learned ",
  "^Your weapon glows",
  " won't be offered again this run%.$",
  " is locked into your next offer%.$",
  " is reforged to Tier ",
  "^You put on ",
  "^Your pet has fallen%.",
  "^You've learned all this wave can teach you",
  -- The card you just took: its toast shows it, and N lists your whole build.
  "^You learn ",
  " gains |c",
  "^You gain |c",
}

-- Short "can't do that" answers: shown like any error, in red over the middle of the screen.
local AS_ERRORS = {
  "^Too far away%.$",
  "^You moved: the channel was interrupted%.$",
  "^Your Hearthstone is ready in %d+ seconds%.$",
  "^You're already at the Ancient%.$",
  "^Shopping is done at home",
  "^You need %d+ gold to reroll%.$",
  "^No banishes left%.$",
  "^No card picks are waiting%.$",
}

local showAll = false
-- Announcements the add-on drew in the last moments: the server sends the same text to chat too.
local recent = {}

local function Matches(list, text)
  for i = 1, table.getn(list) do
    if string.find(text, list[i]) then return true end
  end
  return false
end

if BB and BB.Announce then
  local origAnnounce = BB.Announce
  BB.Announce = function(text)
    if text then recent[text] = GetTime() end
    origAnnounce(text)
  end
end

local function Note(text)
  if BB and BB.Announce then
    -- Through the wrapper: the same line arriving again is then seen as already shown.
    BB.Announce(text)
  end
end

-- What to do with a system line: nil to let it into chat, or "drop", "note", "error".
local function Sort(text)
  if showAll or not text or not InBasin() then return nil end
  -- Until the add-on is talking to a Basin Battle server, everything stays in chat.
  if not (BB and BB.connected) then return nil end
  local seen = recent[text]
  if seen and GetTime() - seen < 3 then return "drop" end
  if Matches(SHOWN_ELSEWHERE, text) then return "drop" end
  if Matches(AS_ERRORS, text) then return "error" end
  if Matches(TO_NOTES, text) then return "note" end
  return nil
end

if ChatFrame_OnEvent then
  local origChatFrameOnEvent = ChatFrame_OnEvent
  ChatFrame_OnEvent = function(event)
    if event == "CHAT_MSG_SYSTEM" and arg1 then
      local what = Sort(arg1)
      if what then
        -- Every chat window that shows system lines sees the event; act on it once.
        if this == ChatFrame1 then
          if what == "note" then
            Note(arg1)
          elseif what == "error" and UIErrorsFrame then
            UIErrorsFrame:AddMessage(arg1, 1, 0.3, 0.2, 1, 3)
          end
        end
        return
      end
    end
    return origChatFrameOnEvent(event)
  end
end

SLASH_BBCHAT1 = "/bbchat"
if SlashCmdList then
  SlashCmdList["BBCHAT"] = function()
    showAll = not showAll
    if showAll then
      DEFAULT_CHAT_FRAME:AddMessage("Basin Battle: every game line shows in chat again. /bbchat to tidy it.", 1, 0.82, 0.2)
    else
      DEFAULT_CHAT_FRAME:AddMessage("Basin Battle: game news goes under the wave tracker again. /bbchat to see it all in chat.", 1, 0.82, 0.2)
    end
  end
end
