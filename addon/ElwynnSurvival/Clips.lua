-- Elwynn Survival: trying a clip on your own hero, to see how a blow reads before naming it in
-- the content (a move's or a creature's `from`).
--
--   /esclip 17                 Attack1H, your own take on it
--   /esclip 17 1               its second take
--   /esclip 17 0 ScourgeFemale Undead female's take, on your body (the client loads it the first
--                              time: try again a moment later)
--   /esclip 57 0 OrcMale 0.5   timed so its blow lands half a second in
--
-- Models: <Race><Gender> as Human, Orc, Dwarf, NightElf, Scourge, Tauren, Gnome, Troll, Goblin
-- with Male or Female. Needs the action-combat client (HasActionCombat 10 for another model's).
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local function Say(text)
  if DEFAULT_CHAT_FRAME then DEFAULT_CHAT_FRAME:AddMessage("|cffffd27a" .. text .. "|r") end
end

-- The pieces of `/esclip <clip> [take] [model] [impact]`, or nil.
function ES.ClipArgs(msg)
  local words = {}
  for w in string.gfind(msg or "", "[^ ]+") do table.insert(words, w) end
  local clip = tonumber(words[1] or "")
  if not clip then return nil end
  local take, from, impact = tonumber(words[2] or "") or 0, nil, 0
  local i = 3
  if words[2] and not tonumber(words[2]) then
    -- No take given: the model straight after the clip.
    take, i = 0, 2
  end
  if words[i] and not tonumber(words[i]) then
    from = words[i]
    i = i + 1
  end
  impact = tonumber(words[i] or "") or 0
  return clip, take, from, impact
end

function ES.TryClip(msg)
  if not PlayUnitAnimation then
    Say("This client plays no clips (the action-combat client does).")
    return
  end
  local clip, take, from, impact = ES.ClipArgs(msg)
  if not clip then
    Say("/esclip <clip> [take] [model] [impact secs], as /esclip 17 0 ScourgeFemale")
    return
  end
  if from and not PreloadAnimationSource then
    Say("This client takes no clips from another model: it plays your own.")
    from = nil
  end
  if from then PreloadAnimationSource(from) end
  PlayUnitAnimation("player", clip, impact, "50F2EEE4", 600, take, 0, "full", from)
  Say("Clip " .. clip .. " (take " .. take .. ")" .. (from and (" from " .. from) or ", your own"))
end

SLASH_ESCLIP1 = "/esclip"
if SlashCmdList then SlashCmdList["ESCLIP"] = ES.TryClip end
