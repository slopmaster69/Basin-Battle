-- Elwynn Survival: the board's second page, the beta cards. The board (N) gets two tabs: Classes
-- (the nine classes' columns, ElwynnSurvival.lua's) and Beta: five rows, one per key as ever, of
-- up to four cards any class can take for free and put on that row's key. They're there to be
-- tried out (Siege leaves them out).
--
-- The server sends them in the same TR line as the class cards, each with its column `beta` and its
-- page (the line's last field per card) 2. The class board knows no `beta` column and leaves them
-- be; this file lays them out, in the order they come, on their own page. It loads before
-- Ledger.lua, so the Ledger look dresses these cards as it does the class cards.
--
-- Written for the 1.12 client's Lua 5.0: no #, no %, no ..., table.getn, `this`.

local ES = ElwynnSurvival
if not ES or not ES.tree or not ES.boardKit then return end

local tree = ES.tree
local K = ES.boardKit
local B = { page = 1, last = nil, used = {}, resting = nil, tabs = {} }
ES.beta = B

-- Four to a row, wider than a class card and centred under the heads; each at
-- tree.cards[FIRST + (row - 1) * COLS + its place in the row].
local COLS = 4
local FIRST = 100
local CARD_W = 200
local GAP = 16
local SPAN = K.ColX(K.N) + K.W - K.ColX(1)
local WIDE = COLS * CARD_W + (COLS - 1) * GAP
local LEFT = K.ColX(1) + (SPAN - WIDE) / 2
local LOOK = { 0.35, 0.95, 0.8 }

-- --- the tabs and the page's head ----------------------------------------------------------------

local function Tab(n, label)
  local b = K.Button(tree, 96, 24, label, K.GOLD, function() B.Show(n) end)
  B.tabs[n] = b
  return b
end
Tab(2, "Beta"):SetPoint("TOPRIGHT", tree, "TOPRIGHT", -22, -16)
Tab(1, "Classes"):SetPoint("RIGHT", B.tabs[2], "LEFT", -6, 0)
B.tabs[2]:SetEnabledLook(false)

B.head = K.Text(tree, K.TITLE, 13, "OUTLINE")
B.head:SetPoint("CENTER", tree, "TOPLEFT", LEFT + WIDE / 2, -K.RowY(1) + 28)
B.head:SetTextColor(LOOK[1], LOOK[2], LOOK[3])
B.head:SetText("Beta cards")
B.head:Hide()
B.sub = K.Text(tree, K.BODY, 10)
B.sub:SetPoint("CENTER", tree, "TOPLEFT", LEFT + WIDE / 2, -K.RowY(1) + 11)
B.sub:SetTextColor(LOOK[1] * 0.8, LOOK[2] * 0.8, LOOK[3] * 0.8)
B.sub:SetText("Any class can slot these. They're here to try out.")
B.sub:Hide()

B.hintClasses = tree.hint:GetText()
B.hintBeta = "Click a card to take it (free) and put it on its key.  Hold the key a moment (or Ctrl with it) for its charged-up version."
B.blurb = "Cards of no one class: take one for free and put it on its row's key, in place of your class's card there. "
  .. "Its charged-up version works as any card's. They're here to be tried, so they may change or go; Siege leaves them out."

-- --- the cards -----------------------------------------------------------------------------------

local function Card(k)
  local b = tree.cards[FIRST + k]
  if b then return b end
  b = K.Card(FIRST + k)
  b:SetWidth(CARD_W)
  b.name:SetWidth(CARD_W - 46)
  b.ex:SetWidth(CARD_W - 46)
  b:SetScript("OnClick", function() B.Click(this) end)
  return b
end

-- A beta card clicked: yours, it goes on its key; not yet, it's taken and goes on its key.
function B.Click(b)
  local d = b.d
  if not d then return end
  ES.Play("click")
  if d.owned then
    ES.Say(".es slot " .. d.row .. " " .. d.key)
  elseif d.can then
    ES.Say(".es tree take " .. d.key)
    ES.Say(".es slot " .. d.row .. " " .. d.key)
  else
    ES.Message(d.name .. ": beta cards aren't in play here.")
  end
end

-- The TR line's beta cards (column `beta`, or page 2) laid out on the page, in the order they come.
function B.Line(f)
  B.last = f
  local parts = ES.Split(f[4] or "", "^")
  local inRow = {}
  local used = {}
  local resting = nil
  for i = 1, table.getn(parts) do
    local e = ES.Split(parts[i], "~")
    local r = K.ROW_N[e[3] or ""]
    if e[1] and e[1] ~= "" and r and (e[4] == "beta" or e[22] == "2") then
      inRow[r] = (inRow[r] or 0) + 1
      local c = inRow[r]
      if c <= COLS then
        local d = {
          key = e[1], name = e[2] or "", row = e[3], school = "beta", icon = e[5] or "", owned = e[6] == "1",
          can = e[7] == "1", on = e[8] or "", kind = e[9] or "", aimed = e[10] == "1",
          lo = tonumber(e[11] or "") or 0, hi = tonumber(e[12] or "") or 0, text = e[13] or "", facts = e[14] or "",
          exKey = e[15] or "", exName = e[16] or "", exIcon = e[17] or "", exValor = tonumber(e[18] or "") or 0,
          exText = e[19] or "", exFacts = e[20] or "", where = e[21] or "", page = 2,
        }
        local k = (r - 1) * COLS + c
        used[k] = true
        local b = Card(k)
        b:ClearAllPoints()
        b:SetPoint("TOPLEFT", tree, "TOPLEFT", LEFT + (c - 1) * (CARD_W + GAP), -K.RowY(r))
        b.d = d
        b.icon:SetTexture(K.ICONS .. d.icon)
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
          b.state:SetText("on " .. ES.ShortKey("ES_AB" .. r))
          b.state:SetTextColor(1, 0.85, 0.45)
          if not resting then resting = d end
        elseif d.owned then
          b.state:SetText("")
        elseif d.can then
          b.state:SetText("free")
          b.state:SetTextColor(0.45, 1, 0.45)
        else
          b.state:SetText("locked")
          b.state:SetTextColor(0.6, 0.45, 0.4)
        end
        ES.DrawCard(b)
      end
    end
  end
  B.used = used
  B.resting = resting
  B.Draw()
end

-- --- the page shown --------------------------------------------------------------------------------

-- What's on the board for the page that's up: its cards, its heads, its hint and the detail strip.
function B.Draw()
  local beta = B.page == 2
  for i, b in pairs(tree.cards) do
    if i > FIRST then
      if beta and B.used[i - FIRST] then b:Show() else b:Hide() end
    elseif beta then
      b:Hide()
    end
  end
  local heads = tree.heads or {}
  for i = 1, table.getn(heads) do
    if beta then heads[i]:Hide() else heads[i]:Show() end
  end
  local crests = tree.crests or {}
  for i = 1, table.getn(crests) do
    if beta then crests[i]:Hide() else crests[i]:Show() end
  end
  if beta then B.head:Show() B.sub:Show() else B.head:Hide() B.sub:Hide() end
  tree.hint:SetText(beta and B.hintBeta or B.hintClasses)
  -- The Ledger's wood on the tabs, once it's there to put on.
  for n = 1, 2 do
    local t = B.tabs[n]
    if ES.ledger and ES.ledger.Button and not t.ledger then ES.ledger.Button(t, true) end
    t:SetEnabledLook((n == 2) == beta)
  end
  if beta then
    local hovered = nil
    for i, b in pairs(tree.cards) do
      if i > FIRST and b.hover and b:IsShown() then hovered = b.d end
    end
    tree.resting = B.resting
    ES.BoardDetail(hovered or B.resting)
  end
end

-- A tab clicked: that page, drawn again from the last line (no word to the server).
function B.Show(n)
  B.page = n
  if B.last then
    ES.TreeLine(B.last)
  else
    B.Draw()
  end
end

-- After the class board has drawn the line, this page's cards (the class cards put away while it's
-- up).
local TreeLine = ES.TreeLine
function ES.TreeLine(f)
  TreeLine(f)
  B.Line(f)
end

-- Nothing hovered on the beta page: what the page is.
local BoardDetail = ES.BoardDetail
function ES.BoardDetail(d)
  BoardDetail(d)
  if not d and B.page == 2 then
    tree.regular.title:SetText("Beta cards")
    tree.regular.text:SetText(B.blurb)
  end
end
