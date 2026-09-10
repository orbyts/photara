# Shared Library presentation primitives

Native immutable record DTOs, semantic actions, browser/editor chrome, thumbnails
and Library & Sync presentation used by People, Locations, Scenes and Project Info.
Feature-specific fields remain in their owning feature directories. Shared views
never open SQLite or hold credentials. The host supplies thumbnail selection and
semantic save/delete/assignment handlers.

Record thumbnails use a 52-point slot with an identity-specific fallback. Modules
have their own scrolling content. Shell owns module icons, headers, rounded surface
boundaries and the global radius control. These sources are compiled directly into
production and each feature lab; they are not a separate binary framework.
