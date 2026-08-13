# Deco — Android Emulator Manager

Deco is a native desktop app for managing Android Virtual Devices (AVDs). It
gives you a clean, single window to list your emulators, see their live status,
and run the common lifecycle operations — without opening Android Studio.

## Product summary

- **List emulators** — every AVD installed under `~/.android/avd`, with its
  display name, API level, ABI, resolution, device profile, and system image.
- **Live status** — shows whether each emulator is `Running` or `Stopped`,
  detected from `adb devices` and refreshed automatically.
- **Lifecycle actions** — Start, Cold Start (`-no-snapshot-load`), Wipe Data
  (`-wipe-data`), and Stop (`adb emu kill`).
- **Create & delete** — create a new AVD from any installed device definition
  and system image; delete AVDs you no longer need.
- **Snapshots** — save, restore, and delete emulator snapshots.
- **Logs** — tail `adb logcat` output for a running emulator.
- **SDK integration** — detects the Android SDK path and can launch the
  `sdkmanager` UI.

## Tech stack

| Layer    | Technology                                      |
| -------- | ----------------------------------------------- |
| Shell    | [Tauri 2](https://tauri.app) (Rust)             |
| Backend  | Rust — invokes the Android SDK CLI tools        |
| Frontend | React 19 + TypeScript, styled with plain CSS    |
| Build    | Vite 7                                          |
| Icons    | [lucide-react](https://lucide.dev)              |

## How it works

The Rust backend shells out to the standard Android SDK command-line tools:

- `emulator` — start / cold start / wipe an AVD
- `adb` — device status, stop, snapshots, logcat
- `avdmanager` — list device definitions, create and delete AVDs
- `sdkmanager` — open the SDK manager UI

AVD metadata (display name, API level, ABI, resolution, device, system image)
is read directly from each `~/.android/avd/<name>.avd/config.ini` file. Running
state is resolved by matching `adb devices` serials to their AVD name via
`adb -s <serial> emu avd name`.

## SDK detection

The SDK path is resolved in this order:

1. `$ANDROID_SDK_ROOT`
2. `$ANDROID_HOME`
3. `~/Library/Android/sdk` (macOS default)

## Project structure

```
deco-tauri/
  src/                 # React frontend
    App.tsx            # main UI + all screens
    App.css            # styles
    main.tsx           # entry point
  src-tauri/           # Rust backend
    src/lib.rs         # all Tauri commands
    src/main.rs        # binary entry point
    tauri.conf.json    # window + bundle config
```

## Development

```bash
cd deco-tauri

# Run the native app (hot reload for frontend)
npm run tauri dev

# Production bundle (.app / .dmg)
npm run tauri build
```

## Current status

Core device management is implemented and functional. The sidebar navigation
items **Snapshots**, **Device Profiles**, **AVD Manager**, and **Settings** are
placeholders without dedicated views yet. The action-grid **Edit** and **Clone**
buttons are also placeholders.
