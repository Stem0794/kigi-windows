with timeout of 3 seconds
    tell application id "com.google.Chrome"
        if (count of windows) is 0 then
            make new window
            set URL of active tab of window 1 to "https://chatgpt.com/"
            activate
            return 2
        end if
        repeat with browserWindow in windows
            set tabURLs to URL of tabs of browserWindow
            repeat with tabNumber from 1 to count of tabURLs
                set tabURL to item tabNumber of tabURLs
                if tabURL is "https://chatgpt.com" or tabURL starts with "https://chatgpt.com/" or tabURL is "https://chat.openai.com" or tabURL starts with "https://chat.openai.com/" then
                    set active tab index of browserWindow to tabNumber
                    set minimized of browserWindow to false
                    set index of browserWindow to 1
                    activate
                    return 1
                end if
            end repeat
        end repeat
        tell window 1
            make new tab at end of tabs with properties {URL:"https://chatgpt.com/"}
            set active tab index to (count of tabs)
            set minimized to false
        end tell
        activate
        return 2
    end tell
end timeout
