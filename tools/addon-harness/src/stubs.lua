-- Stand-ins for the stock interface the add-on leans on.
SAID = {}
UIParent = CreateFrame("Frame", "UIParent")
UIParent:SetAllPoints()
WorldFrame = UIParent
DEFAULT_CHAT_FRAME = { AddMessage = function(self, t) table.insert(SAID, "chat: " .. tostring(t)) end }
ChatFrame1 = DEFAULT_CHAT_FRAME
function ChatFrame_OnEvent(e) end
UISpecialFrames = {}
function ToggleTalentFrame() end
function StaticPopup_Hide() end
WorldMapFrame = CreateFrame("Frame", "WorldMapFrame", UIParent)
WorldMapFrame:SetWidth(1000) WorldMapFrame:SetHeight(700) WorldMapFrame:SetPoint("CENTER", UIParent, "CENTER", 0, 0)
WorldMapFrame:Hide()
WorldMapButton = CreateFrame("Button", "WorldMapButton", WorldMapFrame)
WorldMapButton:SetWidth(1002) WorldMapButton:SetHeight(668) WorldMapButton:SetPoint("CENTER", WorldMapFrame, "CENTER", 0, 0)
WorldMapDetailFrame = WorldMapButton
for i = 1, 12 do
  local b = CreateFrame("CheckButton", "ActionButton" .. i, UIParent)
  b:SetWidth(36) b:SetHeight(36) b:SetPoint("BOTTOMLEFT", UIParent, "BOTTOMLEFT", 40 * i, 40)
end
GameTooltip = CreateFrame("GameTooltip", "GameTooltip", UIParent)
local realSend = SendChatMessage
function SendChatMessage(msg, kind, lang, target) table.insert(SAID, "send: " .. tostring(msg)) end
HOVER = nil
function MouseIsOver(frame)
  if HOVER == frame then return 1 end
  return nil
end
local realUnitName = UnitName
function UnitName(unit)
  if unit == "player" then return "Probe" end
  if realUnitName then return realUnitName(unit) end
  return nil
end
-- The stock frames the minimal interface hides, and the windows its bar opens.
MainMenuBar = CreateFrame("Frame", "MainMenuBar", UIParent)
PlayerFrame = CreateFrame("Frame", "PlayerFrame", UIParent)
MinimapCluster = CreateFrame("Frame", "MinimapCluster", UIParent)
OPENED = {}
function ToggleBackpack() table.insert(OPENED, "backpack") end
function ToggleCharacter(t) table.insert(OPENED, "character " .. tostring(t)) end
function ToggleWorldMap() table.insert(OPENED, "map") end
function ToggleGameMenu() table.insert(OPENED, "menu") end
function ToggleHelpFrame() table.insert(OPENED, "help") end
ChatFrameEditBox = CreateFrame("EditBox", "ChatFrameEditBox", UIParent)
ChatFrameEditBox:Hide()
UIErrorsFrame = { AddMessage = function(self, t) table.insert(SAID, "error frame: " .. tostring(t)) end }
