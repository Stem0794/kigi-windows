-- Hammerspoon: Chrome + Teams
-- Replace the entire ~/.hammerspoon/init.lua with this file.
-- Chrome is maximized in the usable desktop area, not macOS fullscreen.

local TEAMS = "Microsoft Teams"
local CHROME = "Google Chrome"
local TEAMS_ORG_MARKER = " | Orion Health | "

--------------------------------------------------
-- Application helpers
--------------------------------------------------

local function findTeamsApp()
  local app = hs.application.get(TEAMS)
  if app then return app end
  for _, candidate in ipairs(hs.application.runningApplications()) do
    local name = (candidate:name() or ""):lower()
    if name:find("teams", 1, true) and not name:find("helper", 1, true) then
      return candidate
    end
  end
  return nil
end

local function bringAppForward(name)
  local app = name == TEAMS and findTeamsApp() or hs.application.get(name)
  if not app then
    hs.application.launchOrFocus(name)
    return
  end
  app:unhide()
  app:activate(true)
end

local function maximizeWindow(win)
  if not win or not win:application() then return end
  if win:isMinimized() then win:unminimize() end

  if win:isFullScreen() then
    win:setFullScreen(false)
    hs.timer.doAfter(1, function()
      if win and win:application() then
        win:maximize(0)
        win:raise()
        win:focus()
      end
    end)
    return
  end

  win:maximize(0)
  win:raise()
  win:focus()
end

local function maximizeChrome()
  local app = hs.application.get(CHROME)
  if not app then return end
  local win = app:mainWindow()
  if not win or not win:isStandard() then
    for _, candidate in ipairs(app:allWindows()) do
      if candidate:isStandard() then
        win = candidate
        break
      end
    end
  end
  maximizeWindow(win)
end

--------------------------------------------------
-- Option + T: Teams, always front
--------------------------------------------------

hs.hotkey.bind({"alt"}, "t", function()
  bringAppForward(TEAMS)
end)

--------------------------------------------------
-- Option + B: Chrome, always front and 100%
--------------------------------------------------

hs.hotkey.bind({"alt"}, "b", function()
  bringAppForward(CHROME)
  hs.timer.doAfter(0.3, maximizeChrome)
end)

--------------------------------------------------
-- Option + C: activate existing ChatGPT Chrome tab
-- or open a new tab if none exists
--------------------------------------------------

hs.hotkey.bind({"alt"}, "c", function()
  local ok, result = hs.osascript.applescript([[
    tell application "Google Chrome"
      activate
      if (count of windows) is 0 then
        make new window
        set URL of active tab of window 1 to "https://chatgpt.com/"
        return "new"
      end if

      repeat with browserWindow in windows
        set tabNumber to 0
        repeat with browserTab in tabs of browserWindow
          set tabNumber to tabNumber + 1
          set tabURL to URL of browserTab
          if tabURL starts with "https://chatgpt.com" or tabURL starts with "https://chat.openai.com" then
            set active tab index of browserWindow to tabNumber
            set index of browserWindow to 1
            activate
            return "existing"
          end if
        end repeat
      end repeat

      tell window 1
        make new tab at end of tabs with properties {URL:"https://chatgpt.com/"}
        set active tab index to (count of tabs)
      end tell
      activate
      return "new"
    end tell
  ]])

  if not ok then
    hs.alert.show("Impossible d'activer ChatGPT dans Chrome")
    print("Erreur Option+C : " .. hs.inspect(result))
  end
end)

--------------------------------------------------
-- Cmd + Shift + Up: maximize current window, front
--------------------------------------------------

hs.hotkey.bind({"cmd", "shift"}, "up", function()
  maximizeWindow(hs.window.focusedWindow())
end)

--------------------------------------------------
-- Cmd + Shift + Left / Right: select 2 windows
--------------------------------------------------

local splitChooser

local function windowChoices(excludedID)
  local choices = {}
  for _, win in ipairs(hs.window.orderedWindows()) do
    local app = win:application()
    if app and app:name() ~= "Hammerspoon" and win:id() ~= excludedID
       and win:isStandard() and not win:isMinimized()
       and not win:isFullScreen() then
      local title = win:title() or ""
      local item = {
        text = app:name() .. " — " .. (title ~= "" and title or "Sans titre"),
        subText = "Fenêtre " .. tostring(win:id()),
        windowID = win:id()
      }
      local bundle = app:bundleID()
      if bundle then item.image = hs.image.imageFromAppBundle(bundle) end
      table.insert(choices, item)
    end
  end
  return choices
end

local function chooseWindow(side, excludedID, callback)
  local choices = windowChoices(excludedID)
  if #choices == 0 then
    hs.alert.show("Aucune autre fenêtre disponible")
    return
  end

  splitChooser = hs.chooser.new(function(choice)
    if not choice then return end
    local win = hs.window.get(choice.windowID)
    if not win or not win:application() then
      hs.alert.show("La fenêtre sélectionnée n'existe plus")
      return
    end
    callback(win)
  end)
  splitChooser:choices(choices)
  splitChooser:searchSubText(true)
  splitChooser:placeholderText("Choisir la fenêtre " .. side)
  splitChooser:rows(math.min(#choices, 10))
  splitChooser:show()
end

local function placeHalf(win, side, frame)
  if not win or not win:application() then return end
  local half = frame.w / 2
  win:setFrame({
    x = frame.x + (side == "right" and half or 0),
    y = frame.y,
    w = half,
    h = frame.h
  }, 0)
end

local function startSplit(firstSide)
  local focused = hs.window.focusedWindow()
  local screen = focused and focused:screen() or hs.screen.mainScreen()
  if not screen then return end
  local frame = screen:frame()
  local secondSide = firstSide == "left" and "right" or "left"

  chooseWindow(firstSide, nil, function(firstWin)
    hs.timer.doAfter(0.2, function()
      chooseWindow(secondSide, firstWin:id(), function(secondWin)
        if not firstWin:application() or not secondWin:application() then return end
        placeHalf(firstWin, firstSide, frame)
        placeHalf(secondWin, secondSide, frame)
        firstWin:raise()
        secondWin:focus()
      end)
    end)
  end)
end

hs.hotkey.bind({"cmd", "shift"}, "left", function()
  startSplit("left")
end)

hs.hotkey.bind({"cmd", "shift"}, "right", function()
  startSplit("right")
end)

--------------------------------------------------
-- Teams: meeting window on right, chat on left
-- The title-based meeting detection is a heuristic.
--------------------------------------------------

local function isTeamsWindow(win)
  if not win or not win:application() or not win:isStandard() then
    return false
  end
  return (win:application():name() or ""):lower():find("teams", 1, true) ~= nil
end

local function isTeamsChat(win)
  return isTeamsWindow(win) and (win:title() or ""):match("^Chat%s*|") ~= nil
end

local nonMeetings = {
  ["Chat"] = true,
  ["Activity"] = true,
  ["Calendar"] = true,
  ["Calls"] = true,
  ["Teams"] = true,
  ["Files"] = true,
  ["Settings"] = true,
  ["Window"] = true
}

local function isTeamsMeeting(win)
  if not isTeamsWindow(win) or isTeamsChat(win) then return false end
  local title = win:title() or ""
  local prefix = title:match("^(.-)%s*|")
  return prefix ~= nil
    and not nonMeetings[prefix]
    and title:find(TEAMS_ORG_MARKER, 1, true) ~= nil
end

local function teamsWindows()
  local chat, meeting
  for _, win in ipairs(hs.window.allWindows()) do
    if isTeamsChat(win) then
      chat = chat or win
    elseif isTeamsMeeting(win) then
      meeting = meeting or win
    end
  end
  return chat, meeting
end

local meetingID = nil
local chatID = nil

local function applyMeetingLayout(chat, meeting)
  if not chat or not meeting then return end
  local s = meeting:screen():frame()
  placeHalf(chat, "left", s)
  placeHalf(meeting, "right", s)
  chat:raise()
  meeting:focus()
end

local function synchronizeTeams()
  if meetingID then
    local oldMeeting = hs.window.get(meetingID)
    if oldMeeting and isTeamsMeeting(oldMeeting) then return end

    -- A tracked meeting was closed (or its title changed).
    local lastChat = hs.window.get(chatID)
    meetingID, chatID = nil, nil
    if not lastChat or not isTeamsWindow(lastChat) then
      lastChat = select(1, teamsWindows())
    end
    if lastChat then
      maximizeWindow(lastChat)
    end
    return
  end

  local chat, meeting = teamsWindows()
  if not chat or not meeting or chat:id() == meeting:id() then return end
  meetingID, chatID = meeting:id(), chat:id()
  applyMeetingLayout(chat, meeting)
end

-- Keep strong references to the watcher and timer.
teamsWindowFilter = hs.window.filter.new(function(win)
  return isTeamsWindow(win)
end)

local syncDebounce
teamsWindowFilter:subscribe({
  hs.window.filter.windowCreated,
  hs.window.filter.windowDestroyed,
  hs.window.filter.windowTitleChanged
}, function()
  if syncDebounce then syncDebounce:stop() end
  syncDebounce = hs.timer.doAfter(0.8, synchronizeTeams)
end)

-- Detect missed Teams events and changes when a meeting ends.
teamsWindowMonitor = hs.timer.doEvery(5, synchronizeTeams)

-- Ctrl + Cmd + Shift + T: reapply split manually
hs.hotkey.bind({"ctrl", "cmd", "shift"}, "t", function()
  local chat, meeting = teamsWindows()
  if chat and meeting then
    meetingID, chatID = meeting:id(), chat:id()
    applyMeetingLayout(chat, meeting)
  else
    hs.alert.show("Aucune réunion Teams détectée")
  end
end)

--------------------------------------------------
-- On config reload: arrange meeting, or maximize Chrome
--------------------------------------------------

hs.timer.doAfter(1.5, function()
  synchronizeTeams()
  if not meetingID then
    maximizeChrome()
  end
end)


--------------------------------------------------
-- TRACKPAD EDGE GESTURES
-- Left edge down  -> Teams
-- Right edge down -> Chrome
--------------------------------------------------

local EDGE_WIDTH = 0.20
local MIN_SWIPE = 0.22
local MAX_HORIZONTAL = 0.12
local MAX_DURATION = 1.5
local COOLDOWN = 1.0

local gestureState = nil
local lastTrigger = 0

trackpadEdgeGestures = hs.eventtap.new(
  {hs.eventtap.event.types.gesture},
  function(event)

    local touches = event:getTouches()
    if not touches then return false end

    local active = {}

    for _, t in ipairs(touches) do
      if t.touching and t.type == "indirect" then
        table.insert(active, t)
      end
    end

    -- Ignore multitouch and lifted fingers
    if #active ~= 1 then
      gestureState = nil
      return false
    end

    local t = active[1]
    local p = t.normalizedPosition
    if not p then return false end

    local now = hs.timer.secondsSinceEpoch()

    -- Start tracking a new finger
    if not gestureState
      or gestureState.id ~= t.identity then

      local action = nil

      if p.x <= EDGE_WIDTH then
        action = "t"
      elseif p.x >= 1 - EDGE_WIDTH then
        action = "b"
      end

      gestureState = {
        id = t.identity,
        x = p.x,
        y = p.y,
        time = now,
        action = action,
        triggered = false
      }

      return false
    end

    local s = gestureState

    if not s.action or s.triggered then
      return false
    end

    local vertical = s.y - p.y
    local horizontal = math.abs(p.x - s.x)
    local duration = now - s.time

    -- Cancel slow or sideways movements
    if duration > MAX_DURATION
      or horizontal > MAX_HORIZONTAL then
      s.action = nil
      return false
    end

    -- Detect downward swipe
    if vertical >= MIN_SWIPE
      and now - lastTrigger >= COOLDOWN then

      s.triggered = true
      lastTrigger = now

      local shortcut = s.action

      hs.timer.doAfter(0, function()
        hs.eventtap.keyStroke({"alt"}, shortcut)
      end)
    end

    return false
  end
)

trackpadEdgeGestures:start()

