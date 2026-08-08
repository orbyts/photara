return {
    LrSdkVersion = 13.0,
    LrSdkMinimumVersion = 6.0,

    LrToolkitIdentifier = "com.orbyts.photara",
    LrPluginName = "Photara",

    LrLibraryMenuItems = {
        {
            title = "Validate Photara Connection",
            file = "ValidateMain.lua",
        },
        {
            title = "Apply Project to Selected Shoot",
            file = "ApplyProjectMain.lua",
        },
        {
            title = "Apply Imported Selections",
            file = "ApplySelectionsMain.lua",
        },
        {
            title = "Apply Verified Cloud Presence",
            file = "ApplyCloudPresenceMain.lua",
        },
        {
            title = "Add Selected to Photographer Final",
            file = "AddPhotographerFinalMain.lua",
        },
        {
            title = "Remove Selected from Photographer Final",
            file = "RemovePhotographerFinalMain.lua",
        },
        {
            title = "Prepare Photographer Final DNGs",
            file = "PlanTransferMain.lua",
        },
    },

    VERSION = {
        major = 0,
        minor = 0,
        revision = 7,
        build = 3,
    },
}
