use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone)]
pub struct EmulatorInfo {
    pub name: String,
    pub display_name: String,
    pub api: String,
    pub abi: String,
    pub resolution: String,
    pub device: String,
    pub system_image: String,
    pub size: String,
    pub status: String,
    pub serial: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SdkInfo {
    pub path: String,
    pub installed: bool,
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "Could not determine HOME directory".to_string())
}

fn sdk_path() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("ANDROID_SDK_ROOT") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Ok(p);
        }
    }
    if let Ok(p) = std::env::var("ANDROID_HOME") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Ok(p);
        }
    }
    let default = home_dir()?.join("Library/Android/sdk");
    if default.exists() {
        return Ok(default);
    }
    Err("Android SDK not found".to_string())
}

fn emulator_bin(sdk: &Path) -> PathBuf {
    sdk.join("emulator").join("emulator")
}

fn adb_bin(sdk: &Path) -> PathBuf {
    sdk.join("platform-tools").join("adb")
}

fn avd_dir() -> Result<PathBuf, String> {
    Ok(home_dir()?.join(".android").join("avd"))
}

fn run_output(cmd: &mut Command) -> Result<String, String> {
    let out = cmd.output().map_err(|e| format!("Failed to run command: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Parse a single `key=value`-style INI config file.
fn parse_ini(path: &Path) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    map
}

/// Map running serials (`emulator-5554`) to their AVD name via `adb -s <serial> emu avd name`.
fn running_avd_names(sdk: &Path) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let adb = adb_bin(sdk);
    let devices = run_output(Command::new(&adb).arg("devices")).unwrap_or_default();
    for line in devices.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let serial = parts.next().unwrap_or("").to_string();
        let state = parts.next().unwrap_or("");
        if state != "device" {
            continue;
        }
        let raw = run_output(
            Command::new(&adb)
                .arg("-s")
                .arg(&serial)
                .arg("emu")
                .arg("avd")
                .arg("name"),
        )
        .unwrap_or_default();
        // `adb emu avd name` prints "<name>\nOK"; take only the first non-empty line.
        let name = raw
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && *l != "OK")
            .unwrap_or("")
            .to_string();
        result.push((name, serial));
    }
    result
}

fn parse_api_level(target: &str) -> String {
    // e.g. "android-37.1", "android-33", "Google Inc.:Google APIs:33"
    target
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .find(|s| !s.is_empty())
        .unwrap_or("?")
        .to_string()
}

#[tauri::command]
fn get_sdk_info() -> Result<SdkInfo, String> {
    match sdk_path() {
        Ok(p) => Ok(SdkInfo {
            path: p.to_string_lossy().to_string(),
            installed: true,
        }),
        Err(_) => Ok(SdkInfo {
            path: String::new(),
            installed: false,
        }),
    }
}

#[tauri::command]
fn list_emulators() -> Result<Vec<EmulatorInfo>, String> {
    let sdk = sdk_path()?;
    let avd = avd_dir()?;

    let running = running_avd_names(&sdk);

    let mut emulators: Vec<EmulatorInfo> = Vec::new();

    let entries = std::fs::read_dir(&avd)
        .map_err(|e| format!("Could not read AVD directory: {e}"))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "ini").unwrap_or(false));

    for entry in entries {
        let ini_path = entry.path();
        let ini = parse_ini(&ini_path);

        let name = ini
            .get("AvdId")
            .cloned()
            .unwrap_or_else(|| ini_path.file_stem().unwrap_or_default().to_string_lossy().to_string());

        let config_path = avd.join(format!("{name}.avd/config.ini"));
        let cfg = parse_ini(&config_path);

        let display_name = cfg
            .get("avd.ini.displayname")
            .cloned()
            .unwrap_or_else(|| name.clone());

        let api = cfg
            .get("target")
            .map(|t| parse_api_level(t))
            .unwrap_or_else(|| "?".to_string());

        let abi = cfg
            .get("abi.type")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let width = cfg.get("hw.lcd.width").cloned().unwrap_or_default();
        let height = cfg.get("hw.lcd.height").cloned().unwrap_or_default();
        let density = cfg.get("hw.lcd.density").cloned().unwrap_or_default();
        let resolution = if !width.is_empty() && !height.is_empty() {
            if density.is_empty() {
                format!("{width} × {height}")
            } else {
                format!("{width} × {height} ({density}dpi)")
            }
        } else {
            "—".to_string()
        };

        let device = cfg
            .get("hw.device.name")
            .cloned()
            .unwrap_or_else(|| "generic".to_string());

        let system_image = cfg
            .get("tag.id")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let size = cfg
            .get("disk.dataPartition.size")
            .cloned()
            .map(|s| format!("{s}"))
            .unwrap_or_else(|| "—".to_string());

        let (status, serial) = match running.iter().find(|(n, _)| n == &name) {
            Some((_, serial)) => ("Running".to_string(), Some(serial.clone())),
            None => ("Stopped".to_string(), None),
        };

        emulators.push(EmulatorInfo {
            name,
            display_name,
            api,
            abi,
            resolution,
            device,
            system_image,
            size,
            status,
            serial,
        });
    }

    emulators.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(emulators)
}

#[tauri::command]
fn start_emulator(name: String, cold: bool) -> Result<(), String> {
    let sdk = sdk_path()?;
    let emulator = emulator_bin(&sdk);
    if !emulator.exists() {
        return Err("Emulator binary not found in SDK".to_string());
    }

    let mut cmd = Command::new(&emulator);
    cmd.arg("-avd").arg(&name);
    if cold {
        cmd.arg("-no-snapshot-load");
    }
    cmd.spawn()
        .map_err(|e| format!("Failed to start emulator: {e}"))?;
    Ok(())
}

#[tauri::command]
fn stop_emulator(serial: String) -> Result<(), String> {
    let sdk = sdk_path()?;
    let adb = adb_bin(&sdk);
    run_output(
        Command::new(&adb)
            .arg("-s")
            .arg(&serial)
            .arg("emu")
            .arg("kill"),
    )
    .map_err(|e| format!("Failed to stop emulator: {e}"))?;
    Ok(())
}

#[tauri::command]
fn wipe_emulator(name: String) -> Result<(), String> {
    let sdk = sdk_path()?;
    let emulator = emulator_bin(&sdk);
    if !emulator.exists() {
        return Err("Emulator binary not found in SDK".to_string());
    }

    // If running, stop it first.
    if let Ok(emus) = list_emulators() {
        if let Some(e) = emus.iter().find(|e| e.name == name && e.status == "Running") {
            if let Some(serial) = &e.serial {
                let adb = adb_bin(&sdk);
                let _ = run_output(
                    Command::new(&adb).arg("-s").arg(serial).arg("emu").arg("kill"),
                );
            }
        }
    }

    Command::new(&emulator)
        .arg("-avd")
        .arg(&name)
        .arg("-wipe-data")
        .spawn()
        .map_err(|e| format!("Failed to wipe emulator: {e}"))?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct DeviceDefinition {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SystemImage {
    pub api: String,
    pub tag: String,
    pub abi: String,
    pub path: String,
    pub label: String,
}

#[derive(Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub name: String,
    pub avd: String,
    pub created: String,
    pub size: String,
    pub is_current: bool,
}

#[derive(Serialize, Deserialize)]
pub struct LogLine {
    pub text: String,
}

fn cmdline_tools(sdk: &Path) -> Result<PathBuf, String> {
    let base = sdk.join("cmdline-tools").join("latest").join("bin");
    if base.exists() {
        return Ok(base);
    }
    // fall back to any version directory
    let cmdline = sdk.join("cmdline-tools");
    if let Ok(entries) = std::fs::read_dir(&cmdline) {
        for e in entries.flatten() {
            let bin = e.path().join("bin");
            if bin.join("avdmanager").exists() {
                return Ok(bin);
            }
        }
    }
    Err("Android cmdline-tools not found in SDK".to_string())
}

fn avdmanager(sdk: &Path) -> Result<PathBuf, String> {
    Ok(cmdline_tools(sdk)?.join("avdmanager"))
}

fn sdkmanager(sdk: &Path) -> Result<PathBuf, String> {
    Ok(cmdline_tools(sdk)?.join("sdkmanager"))
}

#[tauri::command]
fn list_devices() -> Result<Vec<DeviceDefinition>, String> {
    let sdk = sdk_path()?;
    let avdmgr = avdmanager(&sdk)?;
    let out = run_output(Command::new(&avdmgr).arg("list").arg("device"))?;

    let mut devices = Vec::new();
    let mut current_id = String::new();
    for line in out.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("id:") {
            let id_part = rest.split("or").next().unwrap_or("").trim();
            current_id = id_part.trim_matches('"').to_string();
        } else if let Some(rest) = trimmed.strip_prefix("Name:") {
            let current_name = rest.trim().to_string();
            if !current_id.is_empty() {
                devices.push(DeviceDefinition {
                    id: current_id.clone(),
                    name: current_name,
                });
                current_id.clear();
            }
        }
    }
    devices.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(devices)
}

#[tauri::command]
fn list_system_images() -> Result<Vec<SystemImage>, String> {
    let sdk = sdk_path()?;
    let images_dir = sdk.join("system-images");
    let mut images = Vec::new();

    if let Ok(apis) = std::fs::read_dir(&images_dir) {
        for api_entry in apis.flatten() {
            let api = api_entry.file_name().to_string_lossy().to_string();
            if let Ok(tags) = std::fs::read_dir(api_entry.path()) {
                for tag_entry in tags.flatten() {
                    let tag = tag_entry.file_name().to_string_lossy().to_string();
                    if let Ok(abis) = std::fs::read_dir(tag_entry.path()) {
                        for abi_entry in abis.flatten() {
                            let abi = abi_entry.file_name().to_string_lossy().to_string();
                            let path = abi_entry.path().to_string_lossy().to_string();
                            let label = format!("API {api} · {tag} · {abi}");
                            images.push(SystemImage {
                                api: api.clone(),
                                tag: tag.clone(),
                                abi: abi.clone(),
                                path,
                                label,
                            });
                        }
                    }
                }
            }
        }
    }

    images.sort_by(|a, b| {
        let av = a.api.clone();
        let bv = b.api.clone();
        bv.cmp(&av).then_with(|| a.tag.cmp(&b.tag)).then_with(|| a.abi.cmp(&b.abi))
    });
    Ok(images)
}

#[tauri::command]
fn create_emulator(
    name: String,
    device_id: String,
    system_image_path: String,
) -> Result<(), String> {
    let sdk = sdk_path()?;
    let avdmgr = avdmanager(&sdk)?;

    let out = run_output(
        Command::new(&avdmgr)
            .arg("create")
            .arg("avd")
            .arg("-n")
            .arg(&name)
            .arg("-k")
            .arg(&system_image_path)
            .arg("-d")
            .arg(&device_id)
            .arg("--force"),
    )?;

    // avdmanager writes progress to stdout/stderr; treat missing output as success
    let _ = out;
    Ok(())
}

#[tauri::command]
fn delete_emulator(name: String) -> Result<(), String> {
    let sdk = sdk_path()?;
    let avdmgr = avdmanager(&sdk)?;

    // stop if running
    if let Ok(emus) = list_emulators() {
        if let Some(e) = emus.iter().find(|e| e.name == name) {
            if e.status == "Running" {
                if let Some(serial) = &e.serial {
                    let adb = adb_bin(&sdk);
                    let _ = run_output(Command::new(&adb).arg("-s").arg(serial).arg("emu").arg("kill"));
                }
            }
        }
    }

    let out = run_output(
        Command::new(&avdmgr)
            .arg("delete")
            .arg("avd")
            .arg("-n")
            .arg(&name),
    )?;
    let _ = out;
    Ok(())
}

#[tauri::command]
fn open_sdk_manager() -> Result<(), String> {
    let sdk = sdk_path()?;
    let sdk_mgr = sdkmanager(&sdk)?;

    Command::new(&sdk_mgr)
        .spawn()
        .map_err(|e| format!("Failed to open SDK manager: {e}"))?;
    Ok(())
}

#[tauri::command]
fn list_snapshots(avd: String) -> Result<Vec<SnapshotInfo>, String> {
    let snapshots_dir = home_dir()?
        .join(".android")
        .join("avd")
        .join(format!("{avd}.avd"))
        .join("snapshots");

    let mut snapshots = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&snapshots_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name == "default_boot" {
                continue;
            }
            let created = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| format!("{t:?}"))
                .unwrap_or_default();
            let size = dir_size(&path);
            snapshots.push(SnapshotInfo {
                name,
                avd: avd.clone(),
                created,
                size,
                is_current: false,
            });
        }
    }
    snapshots.sort_by(|a, b| b.created.cmp(&a.created));
    Ok(snapshots)
}

fn dir_size(path: &Path) -> String {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                } else if meta.is_dir() {
                    total += dir_size_bytes(&entry.path());
                }
            }
        }
    }
    if total > 1024 * 1024 * 1024 {
        format!("{:.1} GB", total as f64 / (1024.0 * 1024.0 * 1024.0))
    } else {
        format!("{:.1} MB", total as f64 / (1024.0 * 1024.0))
    }
}

fn dir_size_bytes(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                } else if meta.is_dir() {
                    total += dir_size_bytes(&entry.path());
                }
            }
        }
    }
    total
}

#[tauri::command]
fn snapshot_emulator(avd: String, snapshot_name: String) -> Result<(), String> {
    let sdk = sdk_path()?;
    let adb = adb_bin(&sdk);

    // find running serial for this AVD
    let running = running_avd_names(&sdk);
    let serial = running
        .iter()
        .find(|(n, _)| n == &avd)
        .map(|(_, s)| s.clone())
        .ok_or_else(|| "Emulator is not running; snapshots require a running instance".to_string())?;

    run_output(
        Command::new(&adb)
            .arg("-s")
            .arg(&serial)
            .arg("emu")
            .arg("avd")
            .arg("snapshot")
            .arg("save")
            .arg(&snapshot_name),
    )?;
    Ok(())
}

#[tauri::command]
fn restore_snapshot(avd: String, snapshot_name: String) -> Result<(), String> {
    let sdk = sdk_path()?;
    let adb = adb_bin(&sdk);
    let running = running_avd_names(&sdk);
    let serial = running
        .iter()
        .find(|(n, _)| n == &avd)
        .map(|(_, s)| s.clone())
        .ok_or_else(|| "Emulator is not running; snapshots require a running instance".to_string())?;

    run_output(
        Command::new(&adb)
            .arg("-s")
            .arg(&serial)
            .arg("emu")
            .arg("avd")
            .arg("snapshot")
            .arg("load")
            .arg(&snapshot_name),
    )?;
    Ok(())
}

#[tauri::command]
fn delete_snapshot(avd: String, snapshot_name: String) -> Result<(), String> {
    let sdk = sdk_path()?;
    let adb = adb_bin(&sdk);
    let running = running_avd_names(&sdk);
    let serial = running
        .iter()
        .find(|(n, _)| n == &avd)
        .map(|(_, s)| s.clone())
        .ok_or_else(|| "Emulator is not running; snapshots require a running instance".to_string())?;

    run_output(
        Command::new(&adb)
            .arg("-s")
            .arg(&serial)
            .arg("emu")
            .arg("avd")
            .arg("snapshot")
            .arg("delete")
            .arg(&snapshot_name),
    )?;
    Ok(())
}

#[tauri::command]
fn get_logs(serial: String) -> Result<Vec<LogLine>, String> {
    let sdk = sdk_path()?;
    let adb = adb_bin(&sdk);
    let out = run_output(
        Command::new(&adb)
            .arg("-s")
            .arg(&serial)
            .arg("logcat")
            .arg("-d")
            .arg("-t")
            .arg("200"),
    )?;

    let logs = out
        .lines()
        .map(|l| LogLine { text: l.to_string() })
        .collect();
    Ok(logs)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_emulators,
            start_emulator,
            stop_emulator,
            wipe_emulator,
            get_sdk_info,
            list_devices,
            list_system_images,
            create_emulator,
            delete_emulator,
            open_sdk_manager,
            list_snapshots,
            snapshot_emulator,
            restore_snapshot,
            delete_snapshot,
            get_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
