-- Basin Battle: the chat sits at the bottom right, over the menu buttons and bags, clear of the
-- spell bar and its tabs on the left. (Elwynn Survival lays out its own screen, so not there.)

local CHAT_HEIGHT = 150
-- Room under the chat for the line you type, above the menu buttons.
local CHAT_LIFT = 34

local function InBasin()
  local realm = GetRealmName and GetRealmName() or ""
  return not string.find(realm, "Elwynn")
end

local placing = false
local function PlaceChat()
  if placing or not ChatFrame1 or not CharacterMicroButton or not MainMenuBar then return end
  if not InBasin() then return end
  placing = true
  local left, right = CharacterMicroButton:GetLeft(), MainMenuBar:GetRight()
  local width = 400
  if left and right and right - left > 200 then
    width = right - left - 8
  end
  ChatFrame1:ClearAllPoints()
  ChatFrame1:SetPoint("BOTTOMLEFT", CharacterMicroButton, "TOPLEFT", 4, CHAT_LIFT)
  ChatFrame1:SetWidth(width)
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
