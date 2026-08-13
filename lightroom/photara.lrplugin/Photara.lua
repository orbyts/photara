local LrApplication = import "LrApplication"
local LrApplicationView = import "LrApplicationView"
local LrBinding = import "LrBinding"
local LrDialogs = import "LrDialogs"
local LrDevelopController = import "LrDevelopController"
local LrExportSession = import "LrExportSession"
local LrFileUtils = import "LrFileUtils"
local LrFunctionContext = import "LrFunctionContext"
local LrPathUtils = import "LrPathUtils"
local LrProgressScope = import "LrProgressScope"
local LrTasks = import "LrTasks"
local LrView = import "LrView"

local bind = LrView.bind
local M = {}

local function shellQuote(value)
    return "'" .. tostring(value):gsub("'", "'\\''") .. "'"
end

local function readFile(path)
    local file = io.open(path, "r")
    if not file then return "" end
    local contents = file:read("*a") or ""
    file:close()
    return contents
end

local function removeFile(path)
    if path and path ~= "" then pcall(os.remove, path) end
end

local function jsonEscape(value)
    return '"' .. tostring(value):gsub('\\', '\\\\'):gsub('"', '\\"')
        :gsub('\n', '\\n'):gsub('\r', '\\r'):gsub('\t', '\\t') .. '"'
end

local function jsonEncode(value)
    local kind = type(value)
    if kind == "nil" then return "null" end
    if kind == "boolean" or kind == "number" then return tostring(value) end
    if kind == "string" then return jsonEscape(value) end
    if kind ~= "table" then error("Cannot encode " .. kind .. " as JSON") end
    local count, maximum, array = 0, 0, true
    for key in pairs(value) do
        count = count + 1
        if type(key) ~= "number" or key < 1 or key % 1 ~= 0 then array = false
        else maximum = math.max(maximum, key) end
    end
    if array and maximum == count then
        local parts = {}
        for index = 1, maximum do parts[index] = jsonEncode(value[index]) end
        return "[" .. table.concat(parts, ",") .. "]"
    end
    local parts = {}
    for key, item in pairs(value) do
        table.insert(parts, jsonEscape(key) .. ":" .. jsonEncode(item))
    end
    table.sort(parts)
    return "{" .. table.concat(parts, ",") .. "}"
end

local function writeFile(path, contents)
    local file, message = io.open(path, "wb")
    if not file then error("Could not write " .. path .. ": " .. tostring(message)) end
    file:write(contents)
    file:close()
end

local function scalarSettingsEqual(left, right)
    for key, value in pairs(left or {}) do
        if type(value) ~= "table" and right[key] ~= value then return false end
    end
    for key, value in pairs(right or {}) do
        if type(value) ~= "table" and left[key] ~= value then return false end
    end
    return true
end

local function regularFileExists(path)
    local file = io.open(path, "rb")
    if not file then return false end
    file:close()
    return true
end

local function runPhotara(arguments)
    local config = dofile(_PLUGIN.path .. "/Config.lua")
    local temp = LrPathUtils.getStandardFilePath("temp")
    local token = tostring(os.time()) .. "-" .. tostring(math.random(100000, 999999))
    local outputPath = LrPathUtils.child(temp, "photara-" .. token .. ".lua")
    local errorPath = LrPathUtils.child(temp, "photara-" .. token .. ".err")
    local inner = "eval \"$(" .. shellQuote(config.apogee_path) .. ")\"; " ..
        shellQuote(config.photara_path) .. " " .. arguments ..
        " > " .. shellQuote(outputPath) .. " 2> " .. shellQuote(errorPath)
    local status = LrTasks.execute("/bin/zsh -lc " .. shellQuote(inner))
    if status ~= 0 then
        local message = readFile(errorPath)
        removeFile(outputPath)
        removeFile(errorPath)
        message = tostring(message or ""):gsub("^%s+", ""):gsub("%s+$", "")
        message = message:gsub('^Error: Configuration%("', ""):gsub('"%)$', "")
        local exitCode = status
        if status > 255 and status % 256 == 0 then exitCode = status / 256 end
        if message == "" then
            message = "The Photara command returned no diagnostic details."
        end
        error(
            "Photara could not complete this action (command exit " .. tostring(exitCode) .. ").\n\n" ..
            message .. "\n\nNo Lightroom metadata was changed.",
            0
        )
    end
    local ok, result = pcall(dofile, outputPath)
    local parseError = ok and nil or result
    removeFile(outputPath)
    removeFile(errorPath)
    if not ok then error("Could not read Photara response: " .. tostring(parseError), 0) end
    return result
end

local function projectItems(projects)
    local items = {}
    table.sort(projects, function(left, right)
        return left.display_name < right.display_name
    end)
    for _, project in ipairs(projects) do
        table.insert(items, {
            title = project.display_name .. "  (" .. project.slug .. ")",
            value = project.slug,
        })
    end
    return items
end

local function tableCount(values)
    local count = 0
    for _ in pairs(values or {}) do count = count + 1 end
    return count
end

local function archiveRelativePath(path)
    local normalized = tostring(path or ""):gsub("\\", "/")
    local lower = string.lower(normalized)
    local marker = "/images/"
    local start = string.find(lower, marker, 1, true)
    if not start then return nil end
    return string.sub(normalized, start + #marker)
end

local function chooseProject(context, photoCount)
    local items = projectItems(context.projects or {})
    if #items == 0 then
        error("No Photara projects exist. Initialize a project with the Photara CLI first.")
    end
    return LrFunctionContext.callWithContext("Photara choose project", function(functionContext)
        local properties = LrBinding.makePropertyTable(functionContext)
        properties.project = items[1].value
        local factory = LrView.osFactory()
        local result = LrDialogs.presentModalDialog({
            title = "Photara — Selected Shoot",
            actionVerb = "Continue",
            cancelVerb = "Cancel",
            contents = factory:column({
                bind_to_object = properties,
                spacing = factory:control_spacing(),
                factory:static_text({
                    title = photoCount and (tostring(photoCount) .. " selected photo(s)")
                        or "Reconcile imported selections",
                    font = "<system/bold>",
                }),
                factory:row({
                    factory:static_text({ title = "Project:", width = 90 }),
                    factory:popup_menu({
                        value = bind("project"),
                        items = items,
                        width = 360,
                    }),
                }),
                factory:static_text({
                    title = "People, location, and scene come from the selected Photara project.",
                    width = 460,
                    height_in_lines = 2,
                }),
            }),
        })
        if result ~= "ok" then return nil end
        return properties.project
    end)
end

local function promptForPost(defaultValue)
    return LrFunctionContext.callWithContext("Photara choose post", function(functionContext)
        local properties = LrBinding.makePropertyTable(functionContext)
        properties.post = defaultValue or "package-a"
        local factory = LrView.osFactory()
        local result = LrDialogs.presentModalDialog({
            title = "Photara — Edit Comparison",
            actionVerb = "Continue",
            cancelVerb = "Cancel",
            contents = factory:column({
                bind_to_object = properties,
                spacing = factory:control_spacing(),
                factory:static_text({
                    title = "Prepare neutral Lightroom sources",
                    font = "<system/bold>",
                }),
                factory:row({
                    factory:static_text({ title = "Post:", width = 90 }),
                    factory:edit_field({
                        value = bind("post"),
                        width_in_chars = 32,
                    }),
                }),
                factory:static_text({
                    title = "Photara will use Lightroom Reset + Adobe Color, export a neutral TIFF, then restore the authored edits.",
                    width = 460,
                    height_in_lines = 3,
                }),
            }),
        })
        if result ~= "ok" then return nil end
        local post = tostring(properties.post or ""):gsub("^%s+", ""):gsub("%s+$", "")
        if post == "" then return nil end
        return post
    end)
end

local function keywordKey(keywordPath)
    return table.concat(keywordPath.path or {}, "\31")
end

local function keywordLabel(keywordPath)
    return table.concat(keywordPath.path or {}, " | ")
end

local function confirmPlan(plan, photoCount)
    local people = {}
    for _, keyword in ipairs(plan.people_keywords or {}) do
        table.insert(people, keywordLabel(keyword))
    end
    local iptc = plan.managed_iptc
    local message = table.concat({
        "Selected photos: " .. tostring(photoCount),
        "Project: " .. tostring(plan.project.display_name),
        "People: " .. (#people > 0 and table.concat(people, ", ") or "None"),
        "Scene: " .. tostring(iptc.scene),
        "Location: " .. table.concat({ iptc.sublocation, iptc.city, iptc.state_province }, ", "),
        "",
        "Photara will update only managed IPTC fields and people keywords, then reconcile managed collection trees.",
    }, "\n")
    return LrDialogs.confirm("Apply Photara project?", message, "Apply", "Cancel") == "ok"
end

local function childSetByName(catalog, parent, name)
    local sets = parent and parent:getChildCollectionSets() or catalog:getChildCollectionSets()
    for _, set in ipairs(sets or {}) do
        if set:getName() == name then return set end
    end
    return nil
end

local function childCollectionByName(parent, name)
    for _, collection in ipairs(parent:getChildCollections() or {}) do
        if collection:getName() == name then return collection end
    end
    return nil
end

local function ensureSet(catalog, parent, name)
    local existing = childSetByName(catalog, parent, name)
    if existing then return existing end
    local created = nil
    catalog:withWriteAccessDo("Photara: create collection set " .. name, function()
        created = catalog:createCollectionSet(name, parent, true)
    end)
    LrTasks.yield()
    return created or childSetByName(catalog, parent, name)
end

local function ensureSetPath(catalog, parent, path)
    for _, name in ipairs(path or {}) do
        parent = ensureSet(catalog, parent, name)
        if not parent then error("Could not create collection set " .. tostring(name)) end
    end
    return parent
end

local function keywordChild(catalog, parent, name)
    local children = parent and parent:getChildren() or catalog:getKeywords()
    for _, keyword in ipairs(children or {}) do
        if keyword:getName() == name then return keyword end
    end
    return nil
end

local function ensureKeyword(catalog, parent, name)
    local existing = keywordChild(catalog, parent, name)
    if existing then return existing end
    local created = nil
    catalog:withWriteAccessDo("Photara: create keyword " .. name, function()
        created = catalog:createKeyword(name, {}, true, parent, true)
    end)
    LrTasks.yield()
    return created or keywordChild(catalog, parent, name)
end

local function ensureKeywordPath(catalog, path)
    local parent = nil
    for _, name in ipairs(path or {}) do
        parent = ensureKeyword(catalog, parent, name)
        if not parent then error("Could not create keyword " .. tostring(name)) end
    end
    return parent
end

local function collectKeywordTree(keyword, output)
    if not keyword then return end
    table.insert(output, keyword)
    for _, child in ipairs(keyword:getChildren() or {}) do
        collectKeywordTree(child, output)
    end
end

local function photoHasKeyword(photo, expected)
    for _, keyword in ipairs(photo:getRawMetadata("keywords") or {}) do
        if keyword.localIdentifier == expected.localIdentifier then return true end
    end
    return false
end

local function sdkRule(rule)
    if rule.field == "job-identifier" then
        return { criteria = "jobIdentifier", operation = "==", value = rule.value }
    elseif rule.field == "file-type" then
        return { criteria = "fileFormat", operation = "==", value = string.upper(rule.value) }
    elseif rule.field == "keyword" then
        return { criteria = "keywords", operation = "all", value = rule.value }
    end
    error("Unsupported Photara collection rule field: " .. tostring(rule.field))
end

local function searchDescription(collection)
    local description = { combine = "intersect" }
    for _, rule in ipairs(collection.rules or {}) do
        table.insert(description, sdkRule(rule))
    end
    return description
end

local function ensureSmartCollection(catalog, parent, collection)
    local description = searchDescription(collection)
    local existing = childCollectionByName(parent, collection.name)
    if existing then
        if not existing:isSmartCollection() then
            error("A non-smart collection already occupies Photara path: " .. collection.name)
        end
        catalog:withWriteAccessDo("Photara: update " .. collection.name, function()
            existing:setSearchDescription(description)
        end)
        return existing
    end
    local created = nil
    catalog:withWriteAccessDo("Photara: create " .. collection.name, function()
        created = catalog:createSmartCollection(collection.name, description, parent, true)
    end)
    LrTasks.yield()
    return created or childCollectionByName(parent, collection.name)
end

local function reconcileCollections(catalog, plan, progress)
    for index, tree in ipairs(plan.collection_trees or {}) do
        local parent = ensureSetPath(catalog, nil, tree.path)
        for _, collection in ipairs(tree.smart_collections or {}) do
            local collectionParent = ensureSetPath(
                catalog,
                parent,
                collection.collection_set_path or {}
            )
            ensureSmartCollection(catalog, collectionParent, collection)
        end
        progress:setCaption("Collection tree " .. tostring(index) .. "/" .. tostring(#plan.collection_trees))
        progress:setPortionComplete(index, #plan.collection_trees)
        LrTasks.yield()
    end
end

local function applyMetadata(catalog, photos, plan, projectKeyword, peopleKeywords, progress)
    local iptc = plan.managed_iptc
    catalog:withWriteAccessDo("Photara: apply project metadata", function()
        for index, photo in ipairs(photos) do
            photo:setRawMetadata("jobIdentifier", iptc.job_identifier)
            photo:setRawMetadata("scene", iptc.scene)
            photo:setRawMetadata("location", iptc.sublocation)
            photo:setRawMetadata("city", iptc.city)
            photo:setRawMetadata("stateProvince", iptc.state_province)
            photo:setRawMetadata("country", iptc.country_region)
            photo:setRawMetadata("isoCountryCode", iptc.iso_country_code)
            if iptc.creator then photo:setRawMetadata("creator", iptc.creator) end
            if iptc.copyright then photo:setRawMetadata("copyright", iptc.copyright) end
            photo:addKeyword(projectKeyword)
            for _, keyword in ipairs(peopleKeywords) do photo:addKeyword(keyword) end
            progress:setCaption("Metadata " .. tostring(index) .. "/" .. tostring(#photos))
            progress:setPortionComplete(index, #photos)
            if index % 25 == 0 then LrTasks.yield() end
        end
    end)
end

function M.validateConnection()
    local context = runPhotara("plugin context --format lua")
    local projectCount = #(context.projects or {})
    LrDialogs.message(
        "Photara connection is ready",
        "Configuration, credentials, Storexa, and PostgreSQL are reachable.\n\n" ..
        "Projects: " .. tostring(projectCount) .. "\n" ..
        "People: " .. tostring(tableCount(context.people)) .. "\n" ..
        "Locations: " .. tostring(tableCount(context.locations)) .. "\n" ..
        "Scenes: " .. tostring(tableCount(context.scenes)),
        "info"
    )
end

function M.applyProjectToSelection()
    local catalog = LrApplication.activeCatalog()
    local photos = catalog:getTargetPhotos() or {}
    if #photos == 0 then
        LrDialogs.message("Photara", "Select one or more photos first.", "warning")
        return
    end

    local context = runPhotara("plugin context --format lua")
    local projectSlug = chooseProject(context, #photos)
    if not projectSlug then return end
    local plan = runPhotara("metadata plan " .. shellQuote(projectSlug) .. " --format lua")
    if not confirmPlan(plan, #photos) then return end

    local progress = LrProgressScope({ title = "Apply Photara project" })
    local peopleKeywords = {}
    for _, keyword in ipairs(plan.people_keywords or {}) do
        table.insert(peopleKeywords, ensureKeywordPath(catalog, keyword.path))
    end
    for _, keyword in ipairs(plan.managed_keyword_catalog or {}) do
        ensureKeywordPath(catalog, keyword.path)
    end
    local projectKeyword = ensureKeywordPath(catalog, plan.project_keyword.path)
    applyMetadata(catalog, photos, plan, projectKeyword, peopleKeywords, progress)
    reconcileCollections(catalog, plan, progress)
    progress:done()

    LrDialogs.message(
        "Photara",
        "Applied " .. plan.project.display_name .. " to " .. tostring(#photos) .. " photo(s).\n\n" ..
        "To persist metadata beside proprietary RAW files, enable Automatically Write Changes Into XMP or use Metadata > Save Metadata to File.",
        "info"
    )
end

function M.applyImportedSelections()
    local catalog = LrApplication.activeCatalog()
    local context = runPhotara("plugin context --format lua")
    local projectSlug = chooseProject(context, nil)
    if not projectSlug then return end
    local plan = runPhotara("selections plan " .. shellQuote(projectSlug) .. " --format lua")
    local metadataPlan = runPhotara("metadata plan " .. shellQuote(projectSlug) .. " --format lua")

    local counts = plan.effective_counts or {}
    local message = table.concat({
        "Project: " .. tostring(plan.project.display_name),
        "Client Favorites: " .. tostring(counts["client-favorite"] or 0),
        "Client Shortlist: " .. tostring(counts["client-shortlist"] or 0),
        "Hero: " .. tostring(counts.hero or 0),
        "",
        "Photara will replace only imported selection keywords for this project.",
    }, "\n")
    if LrDialogs.confirm("Apply imported selections?", message, "Apply", "Cancel") ~= "ok" then
        return
    end

    local projectKeyword = ensureKeywordPath(catalog, metadataPlan.project_keyword.path)
    local projectPhotos = projectKeyword:getPhotos() or {}
    local photosByFilename = {}
    local filenameByPhoto = {}
    for _, photo in ipairs(projectPhotos) do
        local filename = photo:getFormattedMetadata("fileName")
        if photosByFilename[filename] then
            error("Project contains duplicate catalog filename: " .. tostring(filename))
        end
        photosByFilename[filename] = photo
        filenameByPhoto[photo] = filename
    end

    local managedKeywords = {}
    local keywordsByKey = {}
    for _, keywordPath in ipairs(plan.managed_keywords or {}) do
        local keyword = ensureKeywordPath(catalog, keywordPath.path)
        table.insert(managedKeywords, keyword)
        keywordsByKey[keywordKey(keywordPath)] = keyword
    end

    local desiredByFilename = {}
    for _, assignment in ipairs(plan.assignments or {}) do
        if not photosByFilename[assignment.original_filename] then
            error("Imported selection is missing from the Lightroom project: " .. assignment.original_filename)
        end
        local desired = {}
        for _, keywordPath in ipairs(assignment.keywords or {}) do
            local keyword = keywordsByKey[keywordKey(keywordPath)]
            if not keyword then error("Selection plan references an unmanaged keyword") end
            table.insert(desired, keyword)
        end
        desiredByFilename[assignment.original_filename] = desired
    end

    local progress = LrProgressScope({ title = "Apply Photara selections" })
    catalog:withWriteAccessDo("Photara: apply imported selections", function()
        for index, photo in ipairs(projectPhotos) do
            for _, keyword in ipairs(managedKeywords) do photo:removeKeyword(keyword) end
            local filename = filenameByPhoto[photo]
            for _, keyword in ipairs(desiredByFilename[filename] or {}) do
                photo:addKeyword(keyword)
            end
            progress:setCaption("Selections " .. tostring(index) .. "/" .. tostring(#projectPhotos))
            progress:setPortionComplete(index, #projectPhotos)
            if index % 25 == 0 then LrTasks.yield() end
        end
    end)
    reconcileCollections(catalog, metadataPlan, progress)
    progress:done()

    LrDialogs.message(
        "Photara",
        "Applied imported selections to " .. tostring(#projectPhotos) .. " project photo(s).\n\n" ..
        "Select the project photos and use Metadata > Save Metadata to File to persist the keywords to XMP.",
        "info"
    )
end

function M.applyVerifiedCloudPresence()
    local catalog = LrApplication.activeCatalog()
    local preparationProgress = LrProgressScope({ title = "Photara — Verify Cloud presence" })
    preparationProgress:setCaption("Loading the latest verified Cloud mappings…")
    preparationProgress:setPortionComplete(0, 2)
    LrTasks.yield()

    local planOk, plan = LrTasks.pcall(function()
        return runPhotara("cloud presence-plan --account personal --format lua")
    end)
    if not planOk then
        preparationProgress:done()
        error(plan, 0)
    end
    preparationProgress:setCaption("Matching verified assets to this Lightroom catalog…")
    preparationProgress:setPortionComplete(1, 2)
    LrTasks.yield()

    local plannedByRelativePath = {}
    for _, original in ipairs(plan.originals or {}) do
        if plannedByRelativePath[original.original_relative_path] then
            error("Cloud plan contains duplicate original path: " .. original.original_relative_path)
        end
        plannedByRelativePath[original.original_relative_path] = original
    end

    local photosByRelativePath = {}
    local duplicatePaths = {}
    local catalogPhotos = catalog:getAllPhotos() or {}
    for index, photo in ipairs(catalogPhotos) do
        if not photo:getRawMetadata("isVirtualCopy") then
            local relativePath = archiveRelativePath(photo:getRawMetadata("path"))
            if relativePath and plannedByRelativePath[relativePath] then
                if photosByRelativePath[relativePath] then
                    duplicatePaths[relativePath] = true
                else
                    photosByRelativePath[relativePath] = photo
                end
            end
        end
        if index % 250 == 0 then
            preparationProgress:setCaption(
                "Matching catalog photos " .. tostring(index) .. "/" .. tostring(#catalogPhotos) .. "…"
            )
            LrTasks.yield()
        end
    end
    if next(duplicatePaths) then
        preparationProgress:done()
        error("The Lightroom catalog contains duplicate originals for a verified archive path")
    end

    local foundCount = tableCount(photosByRelativePath)
    local missingCount = (plan.verified_count or 0) - foundCount
    local selected = {}
    for _, photo in ipairs(catalog:getTargetPhotos() or {}) do selected[photo] = true end
    local selectedVerifiedCount = 0
    for _, photo in pairs(photosByRelativePath) do
        if selected[photo] then selectedVerifiedCount = selectedVerifiedCount + 1 end
    end
    preparationProgress:setPortionComplete(2, 2)
    preparationProgress:done()

    local scope = LrFunctionContext.callWithContext("Photara Cloud scope", function(functionContext)
        local properties = LrBinding.makePropertyTable(functionContext)
        properties.scope = selectedVerifiedCount > 0 and "selected" or "all"
        local items = {}
        if selectedVerifiedCount > 0 then
            table.insert(items, {
                title = "Selected verified originals (" .. tostring(selectedVerifiedCount) .. ")",
                value = "selected",
            })
        end
        table.insert(items, {
            title = "All matched verified originals (" .. tostring(foundCount) .. ")",
            value = "all",
        })
        local factory = LrView.osFactory()
        local result = LrDialogs.presentModalDialog({
            title = "Photara — Verified Cloud Presence",
            actionVerb = "Continue",
            cancelVerb = "Cancel",
            contents = factory:column({
                bind_to_object = properties,
                spacing = factory:control_spacing(),
                factory:static_text({
                    title = "Adobe inventory: " .. tostring(plan.inventory_asset_count) ..
                        " asset(s)\nMapped to Classic originals: " .. tostring(plan.verified_count) ..
                        "\nCloud assets without a known Classic original: " ..
                        tostring(plan.unmapped_inventory_count or 0),
                    font = "<system/bold>",
                    width = 480,
                    height_in_lines = 3,
                }),
                factory:static_text({
                    title = "Matched Classic originals: " .. tostring(foundCount) ..
                        "\nMissing from this catalog: " .. tostring(missingCount),
                    width = 480,
                    height_in_lines = 2,
                }),
                factory:row({
                    factory:static_text({ title = "Apply to:", width = 90 }),
                    factory:popup_menu({
                        value = bind("scope"),
                        items = items,
                        width = 350,
                    }),
                }),
            }),
        })
        if result ~= "ok" then return nil end
        return properties.scope
    end)
    if not scope then return end

    local photos = {}
    for _, photo in pairs(photosByRelativePath) do
        if scope == "all" or selected[photo] then table.insert(photos, photo) end
    end
    if #photos == 0 then
        LrDialogs.message("Photara", "No verified Cloud originals matched the chosen scope.", "warning")
        return
    end
    local message = table.concat({
        "Adobe catalog: " .. tostring(plan.remote_catalog_id),
        "Adobe inventory: " .. tostring(plan.inventory_asset_count),
        "Mapped verified originals: " .. tostring(plan.verified_count),
        "Applying to Classic originals: " .. tostring(#photos),
        "Missing from this catalog: " .. tostring(missingCount),
        "Cloud assets without a known Classic original: " ..
            tostring(plan.unmapped_inventory_count or 0),
        "",
        "Photara will add only workflow | cloud | present.",
    }, "\n")
    if LrDialogs.confirm("Apply verified Cloud presence?", message, "Apply", "Cancel") ~= "ok" then
        return
    end

    local keyword = ensureKeywordPath(catalog, plan.keyword_path)
    local progress = LrProgressScope({ title = "Apply verified Cloud presence" })
    catalog:withWriteAccessDo("Photara: apply verified Cloud presence", function()
        for index, photo in ipairs(photos) do
            photo:addKeyword(keyword)
            progress:setCaption("Cloud presence " .. tostring(index) .. "/" .. tostring(#photos))
            progress:setPortionComplete(index, #photos)
            if index % 25 == 0 then LrTasks.yield() end
        end
    end)
    progress:done()

    LrDialogs.message(
        "Photara",
        "Marked " .. tostring(#photos) .. " original(s) as present in Lightroom Cloud.\n\n" ..
        "Use Metadata > Save Metadata to File to persist the keyword to XMP.",
        "info"
    )
end

function M.updatePhotographerFinal(selected)
    local catalog = LrApplication.activeCatalog()
    local photos = catalog:getTargetPhotos() or {}
    if #photos == 0 then
        LrDialogs.message("Photara", "Select one or more camera originals first.", "warning")
        return
    end
    local context = runPhotara("plugin context --format lua")
    local projectSlug = chooseProject(context, #photos)
    if not projectSlug then return end
    local projectDisplayName = nil
    local projectKeywordPath = nil
    for _, project in ipairs(context.projects or {}) do
        if project.slug == projectSlug then
            projectDisplayName = project.display_name
            projectKeywordPath = { "projects", project.display_name }
        end
    end
    if not projectDisplayName then error("Photara project is missing from plugin context") end
    local projectKeyword = ensureKeywordPath(catalog, projectKeywordPath)

    local paths = {}
    local seen = {}
    for _, photo in ipairs(photos) do
        if photo:getRawMetadata("isVirtualCopy") then
            error("Photographer Final must reference camera originals, not virtual copies")
        end
        if not photoHasKeyword(photo, projectKeyword) then
            error("Every selected photo must belong to " .. projectDisplayName)
        end
        local path = photo:getRawMetadata("path")
        if not path or path == "" then error("A selected photo has no local camera-original path") end
        if seen[path] then error("The selection contains a duplicate camera-original path") end
        seen[path] = true
        table.insert(paths, path)
    end

    local verb = selected and "Add" or "Remove"
    local preposition = selected and "to" or "from"
    local message = table.concat({
        "Project: " .. projectDisplayName,
        "Selected originals: " .. tostring(#paths),
        "",
        verb .. " these originals " .. preposition .. " Photographer Final?",
        "Photara will fingerprint the originals, persist the decision, and update only the Photographer Final keyword.",
    }, "\n")
    if LrDialogs.confirm(verb .. " Photographer Final?", message, verb, "Cancel") ~= "ok" then
        return
    end

    local command = "decisions " .. (selected and "add " or "remove ") .. shellQuote(projectSlug)
    for _, path in ipairs(paths) do
        command = command .. " --original " .. shellQuote(path)
    end
    command = command .. " --format lua"
    local report = runPhotara(command)
    if report.affected_count ~= #photos then
        error("Photara persisted a different number of decisions than Lightroom selected")
    end

    local keyword = ensureKeywordPath(catalog, report.keyword_path)
    local progress = LrProgressScope({ title = verb .. " Photographer Final" })
    catalog:withWriteAccessDo("Photara: " .. string.lower(verb) .. " Photographer Final", function()
        for index, photo in ipairs(photos) do
            if selected then photo:addKeyword(keyword) else photo:removeKeyword(keyword) end
            progress:setCaption("Photographer Final " .. tostring(index) .. "/" .. tostring(#photos))
            progress:setPortionComplete(index, #photos)
            if index % 25 == 0 then LrTasks.yield() end
        end
    end)
    progress:done()

    LrDialogs.message(
        "Photara",
        (selected and "Added " or "Removed ") .. tostring(#photos) ..
        " original(s) " .. preposition .. " Photographer Final.\n" ..
        "Changed: " .. tostring(report.changed_count or report.affected_count) ..
        "; already in that state: " .. tostring(report.unchanged_count or 0) .. ".\n\n" ..
        "Cloud presence, transfer history, and registered representations were preserved.\n\n" ..
        "Use Metadata > Save Metadata to File to persist the decision to XMP.",
        "info"
    )
end

function M.applyVerifiedCloudWithdrawal()
    local catalog = LrApplication.activeCatalog()
    local photos = catalog:getTargetPhotos() or {}
    if #photos == 0 then
        LrDialogs.message("Photara", "Select one or more retained camera originals first.", "warning")
        return
    end
    local context = runPhotara("plugin context --format lua")
    local projectSlug = chooseProject(context, #photos)
    if not projectSlug then return end
    local projectDisplayName = nil
    local projectKeywordPath = nil
    for _, project in ipairs(context.projects or {}) do
        if project.slug == projectSlug then
            projectDisplayName = project.display_name
            projectKeywordPath = { "projects", project.display_name }
        end
    end
    if not projectDisplayName then error("Photara project is missing from plugin context") end
    local projectKeyword = ensureKeywordPath(catalog, projectKeywordPath)

    local paths = {}
    for _, photo in ipairs(photos) do
        if photo:getRawMetadata("isVirtualCopy") then
            error("Cloud withdrawal must reference camera originals, not virtual copies")
        end
        if not photoHasKeyword(photo, projectKeyword) then
            error("Every selected photo must belong to " .. projectDisplayName)
        end
        local path = photo:getRawMetadata("path")
        if not path or path == "" then error("A selected photo has no local camera-original path") end
        table.insert(paths, path)
    end
    local command = "cloud withdrawal-keywords " .. shellQuote(projectSlug)
    for _, path in ipairs(paths) do command = command .. " --original " .. shellQuote(path) end
    command = command .. " --format lua"

    local progress = LrProgressScope({ title = "Verify Cloud withdrawal" })
    progress:setCaption("Checking Photara ledger and Adobe verification...")
    progress:setIndeterminate(true)
    local plan = runPhotara(command)
    progress:setIndeterminate(false)
    if plan.verified_count ~= #photos then
        progress:done()
        error("Photara verified a different number of withdrawals than Lightroom selected")
    end
    local keywords = {}
    for _, path in ipairs(plan.keyword_paths_to_remove or {}) do
        table.insert(keywords, ensureKeywordPath(catalog, path))
    end
    catalog:withWriteAccessDo("Photara: apply verified Cloud withdrawal", function()
        for index, photo in ipairs(photos) do
            for _, keyword in ipairs(keywords) do photo:removeKeyword(keyword) end
            progress:setCaption("Updating retained original " .. tostring(index) .. "/" .. tostring(#photos))
            progress:setPortionComplete(index, #photos)
            if index % 25 == 0 then LrTasks.yield() end
        end
    end)
    progress:done()
    LrDialogs.message(
        "Photara",
        "Applied " .. tostring(#photos) .. " verified Cloud withdrawal(s).\n\n" ..
        "Removed Photographer Final and Cloud Present keywords only. " ..
        "The RAW, XMP, asset record, transfer evidence, and decision history remain intact.\n\n" ..
        "Use Metadata > Save Metadata to File to update the XMP sidecar.",
        "info"
    )
end

function M.planPhotographerFinalTransfer()
    local context = runPhotara("plugin context --format lua")
    local projectSlug = chooseProject(context, nil)
    if not projectSlug then return end
    local plan = runPhotara(
        "cloud transfer-plan " .. shellQuote(projectSlug) .. " --account personal --format lua"
    )
    local preview = {}
    for _, item in ipairs(plan.items or {}) do
        if item.state == "planned" and #preview < 5 then
            table.insert(preview, item.planned_filename)
        end
    end
    local lines = {
        "Project: " .. tostring(plan.project),
        "Photographer Final: " .. tostring(plan.photographer_final_count),
        "DNGs to prepare: " .. tostring(plan.planned_count),
        "Already verified in Cloud: " .. tostring(plan.skipped_already_present_count),
        "",
        "Example planned filenames:",
    }
    for _, filename in ipairs(preview) do table.insert(lines, "• " .. filename) end
    table.insert(lines, "")
    table.insert(lines, "Reserve this immutable transfer batch? No files will be generated or uploaded yet.")
    if LrDialogs.confirm("Reserve Photographer Final transfer?", table.concat(lines, "\n"), "Reserve", "Cancel") ~= "ok" then
        return
    end
    local reservation = runPhotara(
        "cloud reserve-transfer " .. shellQuote(projectSlug) ..
        " --account personal --format lua"
    )
    LrDialogs.message(
        "Photara",
        "Transfer batch reserved.\n\n" ..
        "Batch: " .. tostring(reservation.batch_id) .. "\n" ..
        "DNGs to prepare: " .. tostring(reservation.expected_upload_count) .. "\n" ..
        "Already present: " .. tostring(reservation.skipped_already_present_count) .. "\n" ..
        "Reused existing batch: " .. tostring(reservation.reused_existing_batch) .. "\n\n" ..
        "No DNGs have been generated or uploaded yet.",
        "info"
    )
    if reservation.expected_upload_count == 0 then return end
    if reservation.state ~= "planned" and reservation.state ~= "exporting" then
        LrDialogs.message(
            "Photara",
            "This batch is already in state " .. tostring(reservation.state) ..
            ". No DNGs were rendered again.",
            "info"
        )
        return
    end
    local prepareChoice = LrDialogs.confirm(
        "Prepare reserved DNGs?",
        "Lightroom Classic will now render the " .. tostring(reservation.expected_upload_count) ..
        " reserved camera originals as DNGs.\n\n" ..
        "Photara will place them in an isolated XDG cache directory, validate each file, and " ..
        "record its SHA-256 fingerprint. Nothing will be uploaded or deleted.",
        "Prepare All",
        "Later",
        "Test One"
    )
    if prepareChoice == "ok" then
        M.exportTransferBatch(reservation.batch_id, nil)
    elseif prepareChoice == "other" then
        M.exportTransferBatch(reservation.batch_id, 1)
    end
end

function M.exportTransferBatch(batchId, maximumCount)
    local catalog = LrApplication.activeCatalog()
    local batch = runPhotara(
        "cloud export-batch " .. shellQuote(batchId) .. " --format lua"
    )
    local wantedByRelativePath = {}
    local pending = {}
    for _, item in ipairs(batch.items or {}) do
        if item.state == "planned" then
            local relativePath = tostring(item.source_key):match("^images:(.+)$")
            if not relativePath then error("Transfer item has a non-portable source key") end
            if wantedByRelativePath[relativePath] then
                error("Transfer batch contains a duplicate camera-original path")
            end
            wantedByRelativePath[relativePath] = item
            table.insert(pending, item)
        end
    end

    local photosByRelativePath = {}
    for _, photo in ipairs(catalog:getAllPhotos() or {}) do
        if not photo:getRawMetadata("isVirtualCopy") then
            local relativePath = archiveRelativePath(photo:getRawMetadata("path"))
            if relativePath and wantedByRelativePath[relativePath] then
                if photosByRelativePath[relativePath] then
                    error("The Lightroom catalog contains duplicate originals for " .. relativePath)
                end
                photosByRelativePath[relativePath] = photo
            end
        end
    end
    for relativePath in pairs(wantedByRelativePath) do
        if not photosByRelativePath[relativePath] then
            error("Reserved original is missing from this Lightroom catalog: " .. relativePath)
        end
    end

    local workCount = #pending
    if maximumCount and maximumCount < workCount then workCount = maximumCount end
    local progress = LrProgressScope({ title = "Prepare Photara DNGs" })
    for index = 1, workCount do
        local item = pending[index]
        local relativePath = tostring(item.source_key):match("^images:(.+)$")
        local targetPath = item.staged_path
        progress:setCaption("DNG " .. tostring(index) .. "/" .. tostring(workCount) ..
            ": " .. tostring(item.planned_filename))
        progress:setPortionComplete(index - 1, workCount)

        if not regularFileExists(targetPath) then
            local renderDirectory = LrPathUtils.child(
                batch.staging_directory,
                ".render-" .. tostring(item.asset_id)
            )
            local created, createError = LrFileUtils.createAllDirectories(renderDirectory)
            if created == false then
                error("Could not create DNG render directory: " .. tostring(createError))
            end
            local exportSession = LrExportSession({
                photosToExport = { photosByRelativePath[relativePath] },
                exportSettings = {
                    LR_export_destinationType = "specificFolder",
                    LR_export_destinationPathPrefix = renderDirectory,
                    LR_export_destinationPathSuffix = "",
                    LR_export_useSubfolder = false,
                    LR_collisionHandling = "ask",
                    LR_format = "DNG",
                    LR_DNG_compatibility = 84148224,
                    LR_DNG_compressed = true,
                    LR_DNG_conversionMethod = "preserveRAW",
                    LR_DNG_embedRAW = false,
                    LR_DNG_previewSize = "medium",
                    LR_extensionCase = "uppercase",
                    LR_includeVideoFiles = false,
                    LR_minimizeEmbeddedMetadata = false,
                    LR_outputSharpeningOn = false,
                    LR_reimportExportedPhoto = false,
                    LR_renamingTokensOn = false,
                    LR_size_doConstrain = false,
                    LR_useWatermark = false,
                },
            })
            local renderedPath = nil
            for _, rendition in exportSession:renditions({ stopIfCanceled = true }) do
                local success, pathOrMessage = rendition:waitForRender()
                if not success then error("Lightroom DNG export failed: " .. tostring(pathOrMessage)) end
                renderedPath = pathOrMessage
            end
            if not renderedPath or not regularFileExists(renderedPath) then
                error("Lightroom did not produce the reserved DNG")
            end
            if regularFileExists(targetPath) then
                error("Photara will not overwrite an existing staged DNG: " .. targetPath)
            end
            local moved, moveError = LrFileUtils.move(renderedPath, targetPath)
            if moved == false then error("Could not stage rendered DNG: " .. tostring(moveError)) end
            pcall(LrFileUtils.delete, renderDirectory)
        end

        runPhotara(
            "cloud record-export " .. shellQuote(batchId) ..
            " --asset " .. shellQuote(item.asset_id) ..
            " --file " .. shellQuote(targetPath) .. " --format lua"
        )
        progress:setPortionComplete(index, workCount)
        LrTasks.yield()
    end
    if workCount < #pending then
        progress:done()
        LrDialogs.message(
            "Photara",
            "Canary DNG prepared and recorded.\n\n" ..
            "Validated: " .. tostring(workCount) .. "\n" ..
            "Still pending: " .. tostring(#pending - workCount) .. "\n" ..
            "Staging: " .. tostring(batch.staging_directory) .. "\n\n" ..
            "Inspect the DNG, then run Prepare Photographer Final DNGs again to resume the batch.",
            "info"
        )
        return
    end
    local completion = runPhotara(
        "cloud finish-export " .. shellQuote(batchId) .. " --format lua"
    )
    progress:done()
    LrDialogs.message(
        "Photara",
        "DNG preparation complete.\n\n" ..
        "Batch: " .. tostring(completion.batch_id) .. "\n" ..
        "Validated DNGs: " .. tostring(completion.exported_count) .. "\n" ..
        "Already present in Cloud: " .. tostring(completion.skipped_already_present_count) .. "\n" ..
        "Staging: " .. tostring(completion.staging_directory) .. "\n\n" ..
        "No files were uploaded or deleted.",
        "info"
    )
end

function M.prepareEditComparisonSources()
    local context = runPhotara("plugin context --format lua")
    local projectSlug = chooseProject(context, nil)
    if not projectSlug then return end
    local post = promptForPost("package-a")
    if not post or post == "" then return end
    local platform = "instagram"
    local manifest = runPhotara(
        "posts prepare-edit-comparison-sources " .. shellQuote(projectSlug) .. " " ..
        shellQuote(post) .. " --platform " .. platform .. " --format lua"
    )
    if LrDialogs.confirm(
        "Prepare Edit Comparison sources?",
        "Photara will temporarily reset " .. tostring(#manifest.items) ..
        " catalog original(s), apply Adobe Color, export neutral TIFFs, then restore and verify every develop setting.\n\n" ..
        "XMP sidecars will not be written or changed.",
        "Prepare", "Cancel"
    ) ~= "ok" then return end

    local catalog = LrApplication.activeCatalog()
    local progress = LrProgressScope({ title = "Prepare Edit Comparison sources" })
    local report = {
        schema_version = 1,
        project = manifest.project,
        post = manifest.post,
        platform = manifest.platform,
        source_sha256 = manifest.source_sha256,
        items = {},
    }
    LrApplicationView.switchToModule("develop")
    for index, item in ipairs(manifest.items) do
        progress:setCaption("Neutral Adobe Color source " .. tostring(index) .. "/" .. tostring(#manifest.items))
        progress:setPortionComplete(index - 1, #manifest.items)
        if progress:isCanceled() then error("Edit comparison source preparation was canceled") end
        local photo = catalog:findPhotoByPath(item.camera_raw_path)
        if not photo then error("Camera original is not in this Lightroom catalog: " .. item.camera_raw_path) end
        local saved = photo:getDevelopSettings()
        local restored = false
        local renderedPath = nil
        local ok, message = LrTasks.pcall(function()
            catalog:setSelectedPhotos(photo, {})
            LrTasks.sleep(0.2)
            LrDevelopController.resetAllDevelopAdjustments()
            LrTasks.sleep(0.5)
            catalog:withWriteAccessDo("Photara: temporary Adobe Color profile", function()
                photo:applyDevelopSettings({ CameraProfile = "Adobe Color" }, "Photara temporary Adobe Color", true)
            end, { timeout = 5, asynchronous = false })
            LrTasks.sleep(0.5)
            local reset = photo:getDevelopSettings()
            local profile = reset.CameraProfile or reset.ProfileName or ""
            if profile ~= "Adobe Color" then
                error("Lightroom did not establish Adobe Color; got " .. tostring(profile))
            end
            local outputPath = LrPathUtils.child(manifest.project_root, item.output_relative_path)
            local outputDirectory = LrPathUtils.parent(outputPath)
            local created, createError = LrFileUtils.createAllDirectories(outputDirectory)
            if created == false then error("Could not create neutral source directory: " .. tostring(createError)) end
            local renderDirectory = LrPathUtils.child(outputDirectory, ".render-" .. tostring(item.asset_id))
            LrFileUtils.createAllDirectories(renderDirectory)
            local exportSession = LrExportSession({
                photosToExport = { photo },
                exportSettings = {
                    LR_export_destinationType = "specificFolder",
                    LR_export_destinationPathPrefix = renderDirectory,
                    LR_export_destinationPathSuffix = "",
                    LR_export_useSubfolder = false,
                    LR_collisionHandling = "overwrite",
                    LR_format = "TIFF",
                    LR_tiff_bitDepth = 16,
                    LR_tiff_compressionMethod = "compressionMethod_ZIP",
                    LR_colorSpace = "ProPhotoRGB",
                    LR_extensionCase = "uppercase",
                    LR_includeVideoFiles = false,
                    LR_minimizeEmbeddedMetadata = false,
                    LR_outputSharpeningOn = false,
                    LR_reimportExportedPhoto = false,
                    LR_renamingTokensOn = false,
                    LR_size_doConstrain = false,
                    LR_useWatermark = false,
                },
            })
            for _, rendition in exportSession:renditions({ stopIfCanceled = true }) do
                local success, pathOrMessage = rendition:waitForRender()
                if not success then error("Lightroom neutral TIFF export failed: " .. tostring(pathOrMessage)) end
                renderedPath = pathOrMessage
            end
            if not renderedPath or not regularFileExists(renderedPath) then error("Lightroom produced no neutral TIFF") end
            if regularFileExists(outputPath) then LrFileUtils.delete(outputPath) end
            local moved, moveError = LrFileUtils.move(renderedPath, outputPath)
            if moved == false then error("Could not place neutral TIFF: " .. tostring(moveError)) end
            pcall(LrFileUtils.delete, renderDirectory)
            item._profile = profile
            item._metadata = {
                make = photo:getFormattedMetadata("cameraMake") or "",
                model = photo:getFormattedMetadata("cameraModel") or "",
                lens = photo:getFormattedMetadata("lens") or "",
                iso = photo:getRawMetadata("isoSpeedRating") or 0,
                focal_length_mm = photo:getRawMetadata("focalLength") or 0,
                aperture = photo:getRawMetadata("aperture") or 0,
                exposure_seconds = photo:getRawMetadata("shutterSpeed") or 0,
            }
        end)
        local restoreOk, restoreMessage = LrTasks.pcall(function()
            catalog:withWriteAccessDo("Photara: restore develop settings", function()
                photo:applyDevelopSettings(saved, "Photara restored authored settings", true)
            end, { timeout = 10, asynchronous = false })
            LrTasks.sleep(0.5)
            restored = scalarSettingsEqual(saved, photo:getDevelopSettings())
            if not restored then error("Lightroom develop settings did not restore exactly") end
        end)
        if not restoreOk then error("Critical: could not restore " .. item.original_filename .. ": " .. tostring(restoreMessage)) end
        if not ok then error(tostring(message)) end
        table.insert(report.items, {
            item_id = item.item_id,
            slot = item.slot,
            asset_id = item.asset_id,
            state = "rendered",
            output_relative_path = item.output_relative_path,
            output_sha256 = "pending-verification",
            output_byte_size = 0,
            profile = item._profile,
            restored = restored,
            metadata = item._metadata,
        })
    end
    progress:done()
    local reportPath = LrPathUtils.child(manifest.project_root, "Photara Edit Comparison Source Report.json")
    writeFile(reportPath, jsonEncode(report) .. "\n")
    local verified = runPhotara(
        "posts verify-edit-comparison-sources " .. shellQuote(projectSlug) .. " " ..
        shellQuote(post) .. " --platform " .. platform .. " --format lua"
    )
    LrDialogs.message(
        "Photara",
        "Prepared and verified " .. tostring(verified.verified) .. " neutral Adobe Color source(s).\n\n" ..
        "All catalog develop settings were restored. Photara will independently fingerprint the TIFFs before rendering.",
        "info"
    )
end

return M
