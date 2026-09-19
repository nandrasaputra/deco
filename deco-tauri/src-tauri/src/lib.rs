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

#[derive(Serialize, Deserialize, Clone)]
pub struct SimulatorInfo {
    pub udid: String,
    pub name: String,
    pub runtime: String,
    pub os: String,
    pub device_type: String,
    pub status: String,
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "Could not determine HOME directory".to_string())
}

/// Path to the app's persistent settings file.
fn settings_path() -> Result<PathBuf, String> {
    Ok(home_dir()?.join(".deco").join("settings.json"))
}

/// Read the previously-saved custom SDK path override, if any.
fn sdk_path_override() -> Option<PathBuf> {
    let path = settings_path().ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).ok()?)
        .ok()?;
    match parsed.get("sdk_path") {
        Some(serde_json::Value::String(s)) if !s.is_empty() => Some(PathBuf::from(s)),
        _ => None,
    }
}

/// Save (or clear, when empty) the custom SDK path override.
fn save_sdk_path_override(path: &str) -> Result<(), String> {
    let settings_file = settings_path()?;
    if let Some(parent) = settings_file.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create settings directory: {e}"))?;
    }
    let mut value: serde_json::Value = match std::fs::read_to_string(&settings_file) {
        Ok(s) => serde_json::from_str(&s).unwrap_or(serde_json::Value::Object(Default::default())),
        Err(_) => serde_json::Value::Object(Default::default()),
    };
    value["sdk_path"] = serde_json::Value::String(path.to_string());
    std::fs::write(&settings_file, serde_json::to_string_pretty(&value).unwrap())
        .map_err(|e| format!("Could not write settings: {e}"))
}

fn sdk_path() -> Result<PathBuf, String> {
    // A user-specified override takes precedence over environment defaults.
    if let Some(p) = sdk_path_override() {
        if p.exists() {
            return Ok(p);
        }
        return Err(format!("Android SDK not found at custom path: {}", p.display()));
    }
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

#[derive(Serialize, Deserialize, Clone)]
pub struct SdkPathInfo {
    pub current: String,
    pub custom: String,
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

/// Like run_output but checks for a successful exit and includes stderr on failure.
fn run_checked(cmd: &mut Command) -> Result<String, String> {
    let out = cmd.output().map_err(|e| format!("Failed to run command: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "command failed (no output)".to_string()
        };
        return Err(detail);
    }
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

fn parse_simulator_line(line: &str, runtime: &str) -> Option<SimulatorInfo> {
    let line = line.trim();
    let open = line.rfind(" (")?;
    let status_end = line.len().checked_sub(1)?;
    if !line.ends_with(')') || open >= status_end {
        return None;
    }
    let status = line[open + 2..status_end].to_string();
    let before_status = &line[..open];
    let udid_start = before_status.rfind(" (")?;
    let udid_end = before_status.len().checked_sub(1)?;
    let udid = before_status[udid_start + 2..udid_end].to_string();
    let name = before_status[..udid_start].trim().to_string();
    if name.is_empty() || udid.is_empty() {
        return None;
    }
    let os = runtime.strip_prefix("iOS ").unwrap_or(runtime).to_string();
    let device_type = name
        .split_once(" (")
        .map(|(_, value)| value.trim_end_matches(')').to_string())
        .unwrap_or_else(|| {
            if name.starts_with("iPhone") {
                "iPhone".to_string()
            } else if name.starts_with("iPad") {
                "iPad".to_string()
            } else {
                "Simulator".to_string()
            }
        });
    Some(SimulatorInfo {
        udid,
        name,
        runtime: runtime.to_string(),
        os,
        device_type,
        status,
    })
}

fn list_simulators_impl(output: &str) -> Vec<SimulatorInfo> {
    let mut runtime = String::new();
    let mut simulators = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("-- ") && trimmed.ends_with(" --") {
            runtime = trimmed[3..trimmed.len() - 3].to_string();
            continue;
        }
        if let Some(simulator) = parse_simulator_line(trimmed, &runtime) {
            simulators.push(simulator);
        }
    }
    simulators.sort_by(|a, b| a.name.cmp(&b.name));
    simulators
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
fn get_sdk_path_info() -> Result<SdkPathInfo, String> {
    let current = sdk_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let custom = sdk_path_override().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    Ok(SdkPathInfo { current, custom })
}

#[tauri::command]
fn set_sdk_path(path: String) -> Result<(), String> {
    let trimmed = path.trim().to_string();
    if trimmed.is_empty() {
        // Clearing the custom override: write empty string which get_sdk_path will ignore.
        save_sdk_path_override("")
    } else {
        let p = std::path::Path::new(&trimmed);
        if !p.exists() {
            return Err(format!("Path does not exist: {trimmed}"));
        }
        if !p.join("platform-tools").join("adb").exists()
            && !p.join("emulator").join("emulator").exists()
            && !p.join("cmdline-tools").exists()
        {
            return Err(
                "This path doesn't look like an Android SDK directory (no platform-tools, emulator, or cmdline-tools found). Are you sure?".to_string(),
            );
        }
        save_sdk_path_override(&trimmed)
    }
}

#[tauri::command]
async fn list_simulators() -> Result<Vec<SimulatorInfo>, String> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::process::Command::new("xcrun")
            .args(["simctl", "list", "devices", "available"])
            .output(),
    )
    .await
    .map_err(|_| "xcrun simctl timed out".to_string())?
    .map_err(|e| format!("Failed to run xcrun simctl: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(list_simulators_impl(&String::from_utf8_lossy(&output.stdout)))
}

async fn run_simctl(args: Vec<&str>) -> Result<(), String> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new("xcrun")
            .arg("simctl")
            .args(&args)
            .output(),
    )
    .await
    .map_err(|_| format!("simctl {} timed out", args.join(" ")))?
    .map_err(|e| format!("Failed to run xcrun simctl: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if message.is_empty() {
            "simctl command failed".to_string()
        } else {
            message
        })
    }
}

#[tauri::command]
async fn boot_simulator(udid: String) -> Result<(), String> {
    run_simctl(vec!["boot", &udid]).await?;
    Command::new("open")
        .args(["-a", "Simulator"])
        .spawn()
        .map_err(|e| format!("Failed to open Simulator: {e}"))?;
    Ok(())
}

#[tauri::command]
async fn shutdown_simulator(udid: String) -> Result<(), String> {
    run_simctl(vec!["shutdown", &udid]).await
}

#[tauri::command]
async fn erase_simulator(udid: String) -> Result<(), String> {
    run_simctl(vec!["erase", &udid]).await
}

#[tauri::command]
fn open_simulator_app() -> Result<(), String> {
    Command::new("open")
        .args(["-a", "Simulator"])
        .spawn()
        .map_err(|e| format!("Failed to open Simulator: {e}"))?;
    Ok(())
}

#[tauri::command]
fn list_emulators() -> Result<Vec<EmulatorInfo>, String> {
    let avd = avd_dir()?;

    // Resolve running status only when an SDK/adb is available. Without one we can
    // still list locally-readable AVDs; they are simply treated as stopped.
    let running = match sdk_path() {
        Ok(sdk) => running_avd_names(&sdk),
        Err(_) => Vec::new(),
    };

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
    run_checked(
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
    let out = run_checked(Command::new(&avdmgr).arg("list").arg("device"))?;

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

    let out = run_checked(
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

    // avdmanager writes progress to stdout; a failed create is caught by run_checked
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
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            }
        }
    }

    let out = run_checked(
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

    if !sdk_mgr.exists() {
        return Err("sdkmanager binary not found in Android SDK".to_string());
    }

    // Create a temporary shell script so sdkmanager opens in a visible Terminal window.
    let script_content = format!(
        r#"#!/bin/bash
echo "============================================"
echo "  Android SDK Manager"
echo "  SDK: {}
echo "============================================"
echo ""
echo "Usage examples:"
echo "  sdkmanager --list                     List installed & available packages"
echo "  sdkmanager 'platforms;android-35'    Install a specific package"
echo "  sdkmanager --update                   Update all installed packages"
echo ""
echo "Type 'exit' or Cmd+Q to close this window."
echo "============================================"
echo ""
exec "$(dirname "$0")/sdkmanager" "$@"
"#,
        sdk.display()
    );

    let script_dir = std::env::temp_dir().join("deco-sdkmanager");
    std::fs::create_dir_all(&script_dir)
        .map_err(|e| format!("Could not create temp dir: {e}"))?;

    let script_path = script_dir.join("sdkmanager-launcher.sh");
    std::fs::write(&script_path, &script_content)
        .map_err(|e| format!("Could not write launcher script: {e}"))?;

    // Make executable
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("Could not set permissions: {e}"))?;

    // Also symlink sdkmanager next to it so the script can find it
    let sdkmanager_link = script_dir.join("sdkmanager");
    if !sdkmanager_link.exists() {
        std::os::unix::fs::symlink(&sdk_mgr, &sdkmanager_link)
            .map_err(|e| format!("Could not create symlink: {e}"))?;
    }

    Command::new("open")
        .args(["-a", "Terminal", script_path.to_str().unwrap_or_default()])
        .spawn()
        .map_err(|e| format!("Failed to open Terminal: {e}"))?;

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

    run_checked(
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

    run_checked(
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

    run_checked(
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

#[derive(Serialize, Deserialize, Clone)]
pub struct AvdConfig {
    pub display_name: String,
    pub width: String,
    pub height: String,
    pub density: String,
    pub ram: String,
    pub heap: String,
    pub data_partition: String,
    pub sdcard: String,
    pub cpu_cores: String,
}

fn avd_config_path(name: &str) -> Result<PathBuf, String> {
    Ok(avd_dir()?.join(format!("{name}.avd/config.ini")))
}

fn is_running(name: &str) -> bool {
    match list_emulators() {
        Ok(emus) => emus
            .iter()
            .any(|e| e.name == name && e.status == "Running"),
        Err(_) => false,
    }
}

fn valid_avd_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

#[tauri::command]
fn get_avd_config(name: String) -> Result<AvdConfig, String> {
    let cfg = parse_ini(&avd_config_path(&name)?);
    Ok(AvdConfig {
        display_name: cfg.get("avd.ini.displayname").cloned().unwrap_or_default(),
        width: cfg.get("hw.lcd.width").cloned().unwrap_or_default(),
        height: cfg.get("hw.lcd.height").cloned().unwrap_or_default(),
        density: cfg.get("hw.lcd.density").cloned().unwrap_or_default(),
        ram: cfg.get("hw.ramSize").cloned().unwrap_or_default(),
        heap: cfg.get("vm.heapSize").cloned().unwrap_or_default(),
        data_partition: cfg.get("disk.dataPartition.size").cloned().unwrap_or_default(),
        sdcard: cfg.get("sdcard.size").cloned().unwrap_or_default(),
        cpu_cores: cfg.get("hw.cpu.ncore").cloned().unwrap_or_default(),
    })
}

#[tauri::command]
fn update_avd_config(name: String, config: AvdConfig) -> Result<(), String> {
    if is_running(&name) {
        return Err("Stop the emulator before editing.".to_string());
    }
    let display_name = config.display_name.trim().to_string();
    if display_name.is_empty() {
        return Err("Display name cannot be empty.".to_string());
    }
    let width: u32 = config.width.trim().parse().map_err(|_| "Width must be a number.".to_string())?;
    let height: u32 = config.height.trim().parse().map_err(|_| "Height must be a number.".to_string())?;
    let density: u32 = config.density.trim().parse().map_err(|_| "Density must be a number.".to_string())?;
    let ram: u32 = config.ram.trim().parse().map_err(|_| "RAM must be a number (MB).".to_string())?;
    let heap: u32 = config.heap.trim().parse().map_err(|_| "Heap must be a number (MB).".to_string())?;
    let cpu_cores: u32 = config.cpu_cores.trim().parse().map_err(|_| "CPU cores must be a number.".to_string())?;
    if !(320..=4320).contains(&width) || !(320..=4320).contains(&height) {
        return Err("Resolution must be between 320 and 4320.".to_string());
    }
    if !(120..=640).contains(&density) {
        return Err("Density must be between 120 and 640.".to_string());
    }
    if !(512..=16384).contains(&ram) {
        return Err("RAM must be between 512 and 16384 MB.".to_string());
    }
    if !(64..=2048).contains(&heap) {
        return Err("Heap must be between 64 and 2048 MB.".to_string());
    }
    if !(1..=16).contains(&cpu_cores) {
        return Err("CPU cores must be between 1 and 16.".to_string());
    }
    if config.data_partition.trim().is_empty() || config.sdcard.trim().is_empty() {
        return Err("Storage sizes cannot be empty (e.g. 10G, 512M).".to_string());
    }

    let updates = [
        ("avd.ini.displayname", display_name),
        ("hw.lcd.width", width.to_string()),
        ("hw.lcd.height", height.to_string()),
        ("hw.lcd.density", density.to_string()),
        ("hw.ramSize", ram.to_string()),
        ("vm.heapSize", heap.to_string()),
        ("disk.dataPartition.size", config.data_partition.trim().to_string()),
        ("sdcard.size", config.sdcard.trim().to_string()),
        ("hw.cpu.ncore", cpu_cores.to_string()),
    ];

    let path = avd_config_path(&name)?;
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read AVD config: {e}"))?;
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some((k, _)) = trimmed.split_once('=') {
            let key = k.trim();
            if let Some((_, v)) = updates.iter().find(|(uk, _)| *uk == key) {
                out.push(format!("{key}={v}"));
                seen.insert(key.to_string());
                continue;
            }
        }
        out.push(line.to_string());
    }
    for (k, v) in &updates {
        if !seen.contains(*k) {
            out.push(format!("{k}={v}"));
        }
    }
    std::fs::write(&path, out.join("\n") + "\n")
        .map_err(|e| format!("Could not write AVD config: {e}"))?;
    Ok(())
}

fn copy_avd_dir_filtered(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("Could not create AVD folder: {e}"))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("Could not read AVD folder: {e}"))? {
        let entry = entry.map_err(|e| format!("Could not read AVD entry: {e}"))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".lock") || name == "snapshots" || name.ends_with(".qcow2") {
            continue;
        }
        if name.ends_with(".img") && (name.starts_with("userdata") || name.starts_with("cache")) {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        if src_path.is_dir() {
            copy_avd_dir_filtered(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| format!("Could not copy {name}: {e}"))?;
        }
    }
    Ok(())
}

#[tauri::command]
fn clone_emulator(source: String, new_name: String) -> Result<(), String> {
    let new_name = new_name.trim().to_string();
    if !valid_avd_name(&new_name) {
        return Err("Name may only contain letters, numbers, _, - and .".to_string());
    }
    if is_running(&source) {
        return Err("Stop the emulator before cloning.".to_string());
    }
    let dir = avd_dir()?;
    let src_ini = dir.join(format!("{source}.ini"));
    let src_avd = dir.join(format!("{source}.avd"));
    if !src_ini.exists() || !src_avd.is_dir() {
        return Err(format!("Source AVD \"{source}\" not found."));
    }
    if dir.join(format!("{new_name}.ini")).exists() || dir.join(format!("{new_name}.avd")).exists() {
        return Err(format!("An AVD named \"{new_name}\" already exists."));
    }

    let dst_avd = dir.join(format!("{new_name}.avd"));
    copy_avd_dir_filtered(&src_avd, &dst_avd)?;

    let config_path = dst_avd.join("config.ini");
    let content = std::fs::read_to_string(&config_path).unwrap_or_default();
    let mut seen_display = false;
    let mut seen_id = false;
    let mut out: Vec<String> = Vec::new();
    for line in content.lines() {
        if let Some((k, _)) = line.trim().split_once('=') {
            match k.trim() {
                "AvdId" => {
                    out.push(format!("AvdId={new_name}"));
                    seen_id = true;
                    continue;
                }
                "avd.ini.displayname" => {
                    out.push(format!("avd.ini.displayname={new_name}"));
                    seen_display = true;
                    continue;
                }
                _ => {}
            }
        }
        out.push(line.to_string());
    }
    if !seen_id {
        out.push(format!("AvdId={new_name}"));
    }
    if !seen_display {
        out.push(format!("avd.ini.displayname={new_name}"));
    }
    std::fs::write(&config_path, out.join("\n") + "\n")
        .map_err(|e| format!("Could not write cloned config: {e}"))?;

    let new_path = dst_avd.to_string_lossy().to_string();
    let ini_content = format!("avd.ini.encoding=UTF-8\npath={new_path}\npath.rel=avd/{new_name}.avd\ntarget=android-33\n");
    if let Ok(src_ini_content) = std::fs::read_to_string(&src_ini) {
        let mut target = "android-33".to_string();
        for line in src_ini_content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == "target" {
                    target = v.trim().to_string();
                }
            }
        }
        std::fs::write(
            dir.join(format!("{new_name}.ini")),
            format!("avd.ini.encoding=UTF-8\npath={new_path}\npath.rel=avd/{new_name}.avd\ntarget={target}\n"),
        )
        .map_err(|e| format!("Could not write cloned AVD file: {e}"))?;
    } else {
        std::fs::write(dir.join(format!("{new_name}.ini")), ini_content)
            .map_err(|e| format!("Could not write cloned AVD file: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
fn get_logs(serial: String) -> Result<Vec<LogLine>, String> {
    let sdk = sdk_path()?;
    let adb = adb_bin(&sdk);
    let out = run_checked(
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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_emulators,
            start_emulator,
            stop_emulator,
            wipe_emulator,
            get_sdk_info,
            list_simulators,
            boot_simulator,
            shutdown_simulator,
            erase_simulator,
            open_simulator_app,
            list_devices,
            list_system_images,
            create_emulator,
            delete_emulator,
            get_avd_config,
            update_avd_config,
            clone_emulator,
            open_sdk_manager,
            list_snapshots,
            snapshot_emulator,
            restore_snapshot,
            delete_snapshot,
            get_logs,
            get_sdk_path_info,
            set_sdk_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::list_simulators_impl;

    #[test]
    fn parses_available_simulators_by_runtime_and_status() {
        let output = "== Devices ==\n-- iOS 17.5 --\n    iPhone 15 Pro (ABC-123) (Booted)\n    iPad Air (DEF-456) (Shutdown)\n";
        let simulators = list_simulators_impl(output);
        assert_eq!(simulators.len(), 2);
        let iphone = simulators.iter().find(|sim| sim.udid == "ABC-123").unwrap();
        let ipad = simulators.iter().find(|sim| sim.udid == "DEF-456").unwrap();
        assert_eq!(iphone.os, "17.5");
        assert_eq!(iphone.status, "Booted");
        assert_eq!(ipad.udid, "DEF-456");
    }
}
