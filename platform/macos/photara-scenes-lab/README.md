# Photara Scenes Lab

Independent native authoring host for `../photara-scenes`. It compiles production
shared sources directly, with deterministic in-memory fixtures. No Rust bridge,
database, project, account, NAS or network is required. Project Info also compiles
People/Locations/Scenes field editors because creating a missing record uses
those exact editors in production.

```sh
platform/macos/photara-scenes-lab/build-scenes-lab.sh
open 'platform/macos/photara-scenes-lab/.build/Photara Scenes Lab.app'
```

Choose Populated, Empty, Loading or Error; switch Light/Dark and resize the
native split. Each record has a distinct fixture thumbnail. Search, select,
create, edit and delete operate on fixture values; reset restores deterministic
identities. Choosing a thumbnail uses an optional native image picker. It does
not change the real Library. The action log shows semantic intents.
Project Info exercises references, repeated scene occurrences, search/assign,
create-and-assign, occurrence details and undo. The fixture deliberately includes
a renamed scene snapshot. File exports, cloud sync and live account sign-in are
not simulated as successful operations.

Author the module's shared view/fields and common thumbnail primitives as needed;
there is no second production implementation or copied view to promote. Shell
Lab owns frame geometry, fill/material and header treatment. Build all hosts
with `platform/macos/build-ui.sh`; verify with the shared and production UI
scripts in `photara-ui-tests`. Generated bundles and module caches are ignored.
