local LrDialogs = import "LrDialogs"
local LrTasks = import "LrTasks"

local Photara = require "Photara"

LrTasks.startAsyncTask(function()
    local ok, errorMessage = LrTasks.pcall(function()
        Photara.applyVerifiedCloudPresence()
    end)
    if not ok then
        LrDialogs.message("Photara", tostring(errorMessage), "critical")
    end
end)
