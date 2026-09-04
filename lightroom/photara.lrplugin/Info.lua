return {
    LrSdkVersion = 13.0,
    LrSdkMinimumVersion = 6.0,

    LrToolkitIdentifier = "com.orbyts.photara",
    LrPluginName = "Photara",
    LrMetadataProvider = "MetadataProvider.lua",

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
            title = "Apply Verified Cloud Withdrawal",
            file = "ApplyCloudWithdrawalMain.lua",
        },
        {
            title = "Prepare Photographer Final DNGs",
            file = "PlanTransferMain.lua",
        },
        {
            title = "Import Verified Layered Masters",
            file = "ImportMastersMain.lua",
        },
        {
            title = "Reconcile Layered Master Collections",
            file = "ReconcileMastersMain.lua",
        },
        {
            title = "Prepare Edit Comparison Sources",
            file = "PrepareEditComparisonMain.lua",
        },
    },

    VERSION = {
        major = 0,
        minor = 1,
        revision = 2,
        build = 1,
    },
}
