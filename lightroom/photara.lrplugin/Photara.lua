local LrApplication = import "LrApplication"
local LrBinding = import "LrBinding"
local LrDialogs = import "LrDialogs"
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
        error("Photara command failed (exit " .. tostring(status) .. ").\n" .. message)
    end
    local ok, result = pcall(dofile, outputPath)
    local parseError = ok and nil or result
    removeFile(outputPath)
    removeFile(errorPath)
    if not ok then error("Could not read Photara response: " .. tostring(parseError)) end
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

local function applyMetadata(catalog, photos, plan, peopleKeywords, managedPeopleKeywords, progress)
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
            for _, keyword in ipairs(managedPeopleKeywords) do photo:removeKeyword(keyword) end
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
    local managedPeopleKeywords = {}
    collectKeywordTree(keywordChild(catalog, nil, "people"), managedPeopleKeywords)
    applyMetadata(catalog, photos, plan, peopleKeywords, managedPeopleKeywords, progress)
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

    local projectPhotos = {}
    local photosByFilename = {}
    local filenameByPhoto = {}
    for _, photo in ipairs(catalog:getAllPhotos() or {}) do
        if photo:getFormattedMetadata("jobIdentifier") == plan.project.display_name then
            local filename = photo:getFormattedMetadata("fileName")
            if photosByFilename[filename] then
                error("Project contains duplicate catalog filename: " .. tostring(filename))
            end
            photosByFilename[filename] = photo
            filenameByPhoto[photo] = filename
            table.insert(projectPhotos, photo)
        end
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

return M
