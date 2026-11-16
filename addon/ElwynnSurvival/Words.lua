-- Elwynn Survival: words in the world, not on a page. What the camp's people say comes as their
-- own say where they stand (the server's `CHAT_MSG_MONSTER_SAY`: a speech bubble, and "<name>
-- says:" in chat), and the answers a hero can give follow in chat as links (`AN`). What a hero
-- reads (a note found below, a secret come upon, the book's record of a dungeon, a run's tally:
-- `NT` of kinds note, secret, lore, run) is told in chat too, its title briefly on screen. A link
-- in chat ([Arrest Him], [The Deadmines]) sends its command when clicked, as typing it would.
-- (A wager's page and the board stay pages: `Letter.lua`.)
--
-- Lines: `AN who links` and `NT kind title place text clue links`; links are `command~label`
-- with `^` between; `{n}` breaks a line.
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES then return end

local GOLD = "|cffffd27f"
local GREY = "|cffa89f8a"
local WORDS = "|cffe8dcc0"
local CLUE = "|cffff8a6a"
local LINK = "|cff71d5ff"

-- The kinds told in chat (the rest are pages).
local TOLD = { note = 1, secret = 1, lore = 1, run = 1 }

local function Print(text)
  if DEFAULT_CHAT_FRAME then DEFAULT_CHAT_FRAME:AddMessage(text) end
end

-- `command~label^...` as links in a chat line: each a label in brackets that sends its command.
local function Links(field)
  local out = ""
  if not field or field == "" then return out end
  local items = ES.Split(field, "^")
  for i = 1, table.getn(items) do
    local l = ES.Split(items[i], "~")
    local cmd, label = l[1] or "", l[2] or l[1] or ""
    if cmd ~= "" then
      if out ~= "" then out = out .. "  " end
      out = out .. LINK .. "|Hes:" .. cmd .. "|h[" .. label .. "]|h|r"
    end
  end
  return out
end
ES.ChatLinks = Links

-- A link clicked in chat: ours (`es:<command>`) is sent as the command; any other goes on to the
-- game's own (an item's tooltip, a player's name).
local stockItemRef = SetItemRef
function SetItemRef(link, text, button)
  if link and string.sub(link, 1, 3) == "es:" then
    local cmd = string.sub(link, 4)
    if string.sub(cmd, 1, 4) == ".es " and ES.Say then
      if ES.Play then ES.Play("click") end
      ES.Say(cmd)
    end
    return
  end
  if stockItemRef then return stockItemRef(link, text, button) end
end

-- AN: the answers to what a person just said (their words came as their own say).
function ES.Answers(f)
  local links = Links(f[3])
  if links == "" then return end
  Print(GOLD .. (f[2] or "") .. ":|r  " .. links)
end

-- NT of a told kind: the title (and where it's from), its lines, the clue, the links to read on.
-- False for a kind that's a page.
function ES.Words(f)
  local kind = f[2] or ""
  if not TOLD[kind] then return false end
  local title, place = f[3] or "", f[4] or ""
  Print(GOLD .. title .. "|r" .. (place ~= "" and ("  " .. GREY .. place .. "|r") or ""))
  local text = string.gsub(f[5] or "", "{n}", "\n") .. "\n"
  for line in string.gfind(text, "([^\n]*)\n") do
    if line ~= "" then Print(WORDS .. line .. "|r") end
  end
  local clue = string.gsub(f[6] or "", "{n}", " ")
  if clue ~= "" then
    Print(CLUE .. (kind == "secret" and "Left here: " or "A clue: ") .. clue .. "|r")
  end
  local links = Links(f[7])
  if links ~= "" then Print(GREY .. "Read on:|r  " .. links) end
  if ES.Message then ES.Message(title) end
  ES.wordsTold = { kind = kind, title = title, clue = clue }
  return true
end
