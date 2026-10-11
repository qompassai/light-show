# Aseprite sheet importer landed (2026-10-11)

- scripts/import_sheet.lua + scripts/import_sheet.md restored from the
  pre-cleanup art/aseprite/ copies that had been living only in
  ~/aseprite (the art/ tree was removed 2026-10-08).
- The scripts documented invocation was wrong: Aseprite --script-param
  pairs must come BEFORE --script; params after --script are silently
  dropped and bare args after -- are treated as files to open. Header
  and doc now show the working form, verified on primo against
  Aseprite 1.3.18.6 (seraphine sheet -> 24 frames, 6 mood tags in
  range, 0.18s durations).
- Doc refreshed to current paths: sheets/icons live in
  game/assets/sprites/ (Ondine = ondine_sheet_fullbody*.png with
  in-repo .aseprite sources; 16 component icons); Diver actions link
  fixed to lua/games/aseprite/actions.lua.
