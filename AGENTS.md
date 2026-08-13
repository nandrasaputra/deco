# AGENTS.md

Guidance for AI agents working in this repository.

## Project

Deco is a Tauri 2 desktop app for managing Android emulators (AVDs). The
application lives in `deco-tauri/`. It is **not** a web app — the UI renders in a
native WebView and communicates with a Rust backend over Tauri's IPC bridge.

## Layout

```
deco-tauri/
  src/                 # React + TypeScript frontend (Vite)
  src-tauri/src/       # Rust backend — all `#[tauri::command]` definitions
  src-tauri/tauri.conf.json
```

## Commands

- Run the native app: `npm run tauri dev` (from `deco-tauri/`)
- Frontend typecheck: `npx tsc --noEmit`
- Frontend build: `npm run build`
- Backend check: `cargo check` (from `deco-tauri/src-tauri/`)
- Production bundle: `npm run tauri build`

Always run `tsc --noEmit` and `cargo check` after changes and confirm they pass
before claiming completion.

## Architecture rules

- **Frontend never shells out directly.** All Android SDK operations (emulator,
  adb, avdmanager, sdkmanager) go through Tauri commands defined in
  `src-tauri/src/lib.rs`. The frontend calls them via
  `invoke()` from `@tauri-apps/api/core`.
- **New backend functionality** = new `#[tauri::command]` function in
  `lib.rs` + register it in the `generate_handler![...]` list + a corresponding
  `invoke()` call in `App.tsx`.
- **Serde structs** are used for all data crossing the IPC boundary. Return
  `Result<T, String>` from commands; map errors to human-readable strings.
- **SDK path resolution** lives in `sdk_path()` in `lib.rs` and checks
  `ANDROID_SDK_ROOT` → `ANDROID_HOME` → `~/Library/Android/sdk`. Reuse it; do
  not hardcode paths.
- **AVD metadata** comes from `~/.android/avd/<name>.avd/config.ini`.
  **Running status** comes from `adb devices` matched to AVD names via
  `adb -s <serial> emu avd name`.

## Gotchas

- `adb emu avd name` prints two lines: `<name>` then `OK`. Parse only the first
  non-empty, non-`OK` line — trimming the whole string will break status
  matching (this bit us once).
- The emulator binary may not be on `$PATH`; use the resolved SDK path
  (`<sdk>/emulator/emulator`).
- `avdmanager` and `sdkmanager` live under `<sdk>/cmdline-tools/latest/bin/`.
- A single AVD cannot run two instances; starting a second throws
  `Running multiple emulators with the same AVD ...` from the emulator CLI.

## Conventions

- Frontend: React function components + hooks. Styles are plain CSS in
  `src/App.css` using CSS variables defined in `:root`.
- Icons come from `lucide-react`.
- Keep UI text and errors human-readable.
- No comments in code unless they explain a non-obvious workaround.

## Design reference

The visual design is documented in the Pencil file used during this project.
The UI follows a light macOS aesthetic: white surfaces, a pale `#f5f6f8`
sidebar, blue (`#2563eb`) accents, and a three-panel layout
(sidebar / emulator table / detail panel).
