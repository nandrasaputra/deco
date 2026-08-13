import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Bot,
  Smartphone,
  Layers,
  MonitorSmartphone,
  Package,
  Settings,
  SlidersHorizontal,
  Plus,
  FolderOpen,
  Search,
  List,
  Grid2X2,
  Play,
  Snowflake,
  Trash2,
  Square,
  Camera,
  FileText,
  Pencil,
  Copy,
  RefreshCw,
  X,
  RotateCcw,
} from "lucide-react";
import "./App.css";

interface DeviceDefinition {
  id: string;
  name: string;
}

interface SystemImage {
  api: string;
  tag: string;
  abi: string;
  path: string;
  label: string;
}

interface SnapshotInfo {
  name: string;
  avd: string;
  created: string;
  size: string;
  is_current: boolean;
}

interface LogLine {
  text: string;
}

interface Emulator {
  name: string;
  display_name: string;
  api: string;
  abi: string;
  resolution: string;
  device: string;
  system_image: string;
  size: string;
  status: "Running" | "Stopped";
  serial: string | null;
}

interface SdkInfo {
  path: string;
  installed: boolean;
}

const navItems = [
  { label: "Devices", icon: Smartphone },
  { label: "Snapshots", icon: Layers },
  { label: "Device Profiles", icon: MonitorSmartphone },
  { label: "AVD Manager", icon: Package },
  { label: "SDK Manager", icon: Settings },
  { label: "Settings", icon: SlidersHorizontal },
];

function App() {
  const [emulators, setEmulators] = useState<Emulator[]>([]);
  const [sdk, setSdk] = useState<SdkInfo | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [showSnapshots, setShowSnapshots] = useState(false);
  const [showLogs, setShowLogs] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const [snapshots, setSnapshots] = useState<SnapshotInfo[]>([]);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      const [emus, sdkInfo] = await Promise.all([
        invoke<Emulator[]>("list_emulators"),
        invoke<SdkInfo>("get_sdk_info"),
      ]);
      setEmulators(emus);
      setSdk(sdkInfo);
      setSelectedId((prev) => {
        if (prev && emus.some((e) => e.name === prev)) return prev;
        return emus[0]?.name ?? null;
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 5000);
    return () => clearInterval(id);
  }, []);

  const selected = emulators.find((e) => e.name === selectedId) ?? null;

  const filtered = emulators.filter((e) =>
    e.display_name.toLowerCase().includes(query.toLowerCase()),
  );

  async function handleStart(name: string, cold: boolean) {
    try {
      await invoke("start_emulator", { name, cold });
      setTimeout(refresh, 1500);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleStop(serial: string) {
    try {
      await invoke("stop_emulator", { serial });
      setTimeout(refresh, 1000);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleWipe(name: string) {
    try {
      await invoke("wipe_emulator", { name });
      setTimeout(refresh, 1500);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleDelete(name: string) {
    if (!confirm(`Delete emulator "${name}"? This cannot be undone.`)) return;
    try {
      await invoke("delete_emulator", { name });
      setSelectedId(null);
      setTimeout(refresh, 800);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleOpenSdkManager() {
    try {
      await invoke("open_sdk_manager");
    } catch (e) {
      setError(String(e));
    }
  }

  async function openSnapshots() {
    if (!selected) return;
    setShowSnapshots(true);
    try {
      const snaps = await invoke<SnapshotInfo[]>("list_snapshots", {
        avd: selected.name,
      });
      setSnapshots(snaps);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleSnapshotSave() {
    if (!selected) return;
    const snapName = prompt("Snapshot name:");
    if (!snapName) return;
    try {
      await invoke("snapshot_emulator", { avd: selected.name, snapshotName: snapName });
      const snaps = await invoke<SnapshotInfo[]>("list_snapshots", {
        avd: selected.name,
      });
      setSnapshots(snaps);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleSnapshotRestore(snapName: string) {
    if (!selected) return;
    try {
      await invoke("restore_snapshot", { avd: selected.name, snapshotName: snapName });
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleSnapshotDelete(snapName: string) {
    if (!selected) return;
    if (!confirm(`Delete snapshot "${snapName}"?`)) return;
    try {
      await invoke("delete_snapshot", { avd: selected.name, snapshotName: snapName });
      const snaps = await invoke<SnapshotInfo[]>("list_snapshots", {
        avd: selected.name,
      });
      setSnapshots(snaps);
    } catch (e) {
      setError(String(e));
    }
  }

  async function openLogs() {
    if (!selected?.serial) {
      setError("Emulator is not running.");
      return;
    }
    setShowLogs(true);
    setLogs([]);
    try {
      const lines = await invoke<LogLine[]>("get_logs", { serial: selected.serial });
      setLogs(lines.map((l) => l.text));
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="app">
      <header className="titlebar">
        <div className="titlebar-dots">
          <span className="titlebar-dot close" />
          <span className="titlebar-dot minimize" />
          <span className="titlebar-dot maximize" />
        </div>
        <span className="titlebar-title">Deco — Android Emulator Manager</span>
      </header>

      <div className="app-body">
        <aside className="sidebar">
          <div className="brand">
            <div className="brand-icon">
              <Bot size={22} />
            </div>
            <div className="brand-title">
              <strong>Android Emulator</strong>
              <strong>Manager</strong>
            </div>
          </div>

          <nav className="nav">
            {navItems.map((item) => {
              const Icon = item.icon;
              return (
                <button
                  key={item.label}
                  className={`nav-item ${item.label === "Devices" ? "active" : ""}`}
                >
                  <Icon size={16} />
                  <span>{item.label}</span>
                </button>
              );
            })}
          </nav>

          <div className="sidebar-footer">
            <div className="quick-actions">
              <span className="quick-actions-label">Quick Actions</span>
              <button className="quick-action-btn" onClick={() => setShowCreate(true)}>
                <Plus size={16} />
                <span>Create Emulator</span>
              </button>
              <button className="quick-action-btn" onClick={handleOpenSdkManager}>
                <FolderOpen size={16} />
                <span>Open SDK Manager</span>
              </button>
            </div>

            <div className="sdk-status">
              <span
                className={`sdk-status-dot ${sdk?.installed ? "ok" : "err"}`}
              />
              <div className="sdk-status-text">
                <span>
                  {sdk
                    ? sdk.installed
                      ? "Android SDK: Installed"
                      : "Android SDK: Not Found"
                    : "Detecting SDK…"}
                </span>
                {sdk?.path && <em>SDK Path: {sdk.path}</em>}
              </div>
            </div>
          </div>
        </aside>

        <main className="main">
          <div className="header">
            <div className="header-title">
              <h1>Emulators</h1>
              <p>Manage your Android Virtual Devices</p>
            </div>

            <div className="search">
              <Search size={14} />
              <input
                placeholder="Search emulators..."
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
            </div>

            <button
              className="icon-btn"
              onClick={refresh}
              title="Refresh"
              disabled={loading}
            >
              <RefreshCw size={16} className={loading ? "spin" : ""} />
            </button>

            <div className="view-toggle">
              <button className="active" title="List view">
                <List size={18} />
              </button>
              <button title="Grid view">
                <Grid2X2 size={16} />
              </button>
            </div>

            <button className="btn-primary" onClick={() => setShowCreate(true)}>
              <Plus size={16} />
              <span>Create Emulator</span>
            </button>
          </div>

          {error && <div className="error-banner">{error}</div>}

          <div className="table-wrap">
            <div className="table-header">
              <span>Name</span>
              <span>Status</span>
              <span>API</span>
              <span>Size</span>
              <span>Actions</span>
            </div>
            <div className="table-body">
              {loading && emulators.length === 0 ? (
                <div className="empty-state">Loading emulators…</div>
              ) : filtered.length === 0 ? (
                <div className="empty-state">No emulators found.</div>
              ) : (
                filtered.map((emu) => (
                  <div
                    key={emu.name}
                    className={`table-row ${emu.name === selectedId ? "selected" : ""}`}
                    onClick={() => setSelectedId(emu.name)}
                  >
                    <span className="cell-name">{emu.display_name}</span>
                    <span>
                      <span
                        className={`status-badge ${emu.status.toLowerCase()}`}
                      >
                        <span className="dot" />
                        {emu.status}
                      </span>
                    </span>
                    <span className="cell-muted">{emu.api}</span>
                    <span className="cell-muted">{emu.size}</span>
                    <div className="row-actions">
                      {emu.status === "Running" ? (
                        <>
                          <button
                            className="row-action"
                            title="Stop"
                            onClick={(e) => {
                              e.stopPropagation();
                              if (emu.serial) handleStop(emu.serial);
                            }}
                          >
                            <Square size={14} />
                          </button>
                          <button
                            className="row-action"
                            title="Wipe Data"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleWipe(emu.name);
                            }}
                          >
                            <Trash2 size={14} />
                          </button>
                        </>
                      ) : (
                        <>
                          <button
                            className="row-action"
                            title="Run"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleStart(emu.name, false);
                            }}
                          >
                            <Play size={14} />
                          </button>
                          <button
                            className="row-action"
                            title="Cold Start"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleStart(emu.name, true);
                            }}
                          >
                            <Snowflake size={14} />
                          </button>
                          <button
                            className="row-action"
                            title="Wipe Data"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleWipe(emu.name);
                            }}
                          >
                            <Trash2 size={14} />
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </main>

        <aside className="detail-panel">
          {selected ? (
            <>
              <div className="detail-header">
                <h2>{selected.display_name}</h2>
                <div
                  className={`status-line ${selected.status.toLowerCase()}`}
                >
                  {selected.status}
                </div>
              </div>

              <div className="preview-row">
                <div className="phone-preview">
                  <div className="phone-screen">
                    <div className="phone-time">9:30</div>
                    <div className="phone-wallpaper" />
                    <div className="phone-dock">● ● ● ●</div>
                  </div>
                </div>

                <div className="detail-metadata">
                  <div className="meta-row">
                    <label>Android Version</label>
                    <span>API {selected.api}</span>
                  </div>
                  <div className="meta-row">
                    <label>API Level</label>
                    <span>{selected.api}</span>
                  </div>
                  <div className="meta-row">
                    <label>Resolution</label>
                    <span>{selected.resolution}</span>
                  </div>
                  <div className="meta-row">
                    <label>CPU / ABI</label>
                    <span>{selected.abi}</span>
                  </div>
                  <div className="meta-row">
                    <label>Device</label>
                    <span>{selected.device}</span>
                  </div>
                  <div className="meta-row">
                    <label>System Image</label>
                    <span>{selected.system_image}</span>
                  </div>
                </div>
              </div>

              <div className="actions-grid">
                <div className="action-row">
                  <div
                    className="action-card"
                    onClick={() => {
                      if (selected.status === "Running" && selected.serial)
                        handleStop(selected.serial);
                      else handleStart(selected.name, false);
                    }}
                  >
                    <button className="primary">
                      {selected.status === "Running" ? (
                        <Square size={18} />
                      ) : (
                        <Play size={18} />
                      )}
                    </button>
                    <span>
                      {selected.status === "Running" ? "Stop" : "Start"}
                    </span>
                  </div>
                  <div
                    className="action-card"
                    onClick={() => handleStart(selected.name, true)}
                  >
                    <button>
                      <Snowflake size={18} />
                    </button>
                    <span>Cold Start</span>
                  </div>
                  <div
                    className="action-card"
                    onClick={() => handleWipe(selected.name)}
                  >
                    <button className="danger">
                      <Trash2 size={18} />
                    </button>
                    <span>Wipe Data</span>
                  </div>
                  <div className="action-card" onClick={openSnapshots}>
                    <button>
                      <Camera size={18} />
                    </button>
                    <span>Snapshot</span>
                  </div>
                </div>
                <div className="action-row">
                  <div className="action-card" onClick={openLogs}>
                    <button>
                      <FileText size={18} />
                    </button>
                    <span>View Logs</span>
                  </div>
                  <div className="action-card">
                    <button>
                      <Pencil size={18} />
                    </button>
                    <span>Edit</span>
                  </div>
                  <div className="action-card">
                    <button>
                      <Copy size={18} />
                    </button>
                    <span>Clone</span>
                  </div>
                  <div
                    className="action-card"
                    onClick={() => handleDelete(selected.name)}
                  >
                    <button className="danger">
                      <Trash2 size={18} />
                    </button>
                    <span>Delete</span>
                  </div>
                </div>
              </div>
            </>
          ) : (
            <div className="empty-state">No emulator selected.</div>
          )}
        </aside>
      </div>

      {showCreate && (
        <CreateEmulatorModal
          onClose={() => setShowCreate(false)}
          onCreated={() => {
            setShowCreate(false);
            refresh();
          }}
        />
      )}

      {showSnapshots && selected && (
        <SnapshotsModal
          avd={selected.name}
          snapshots={snapshots}
          onClose={() => setShowSnapshots(false)}
          onSave={handleSnapshotSave}
          onRestore={handleSnapshotRestore}
          onDelete={handleSnapshotDelete}
        />
      )}

      {showLogs && (
        <LogsModal logs={logs} onClose={() => setShowLogs(false)} />
      )}
    </div>
  );
}

function CreateEmulatorModal({
  onClose,
  onCreated,
}: {
  onClose: () => void;
  onCreated: () => void;
}) {
  const [devices, setDevices] = useState<DeviceDefinition[]>([]);
  const [images, setImages] = useState<SystemImage[]>([]);
  const [name, setName] = useState("");
  const [deviceId, setDeviceId] = useState("");
  const [imagePath, setImagePath] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const [ds, ims] = await Promise.all([
          invoke<DeviceDefinition[]>("list_devices"),
          invoke<SystemImage[]>("list_system_images"),
        ]);
        setDevices(ds);
        setImages(ims);
      } catch (e) {
        setErr(String(e));
      }
    })();
  }, []);

  async function create() {
    if (!name || !deviceId || !imagePath) {
      setErr("Please fill in all fields.");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      await invoke("create_emulator", {
        name,
        deviceId,
        systemImagePath: imagePath,
      });
      onCreated();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Create Emulator</h2>
          <button className="modal-close" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          <label className="field">
            <span>Name</span>
            <input
              placeholder="e.g. Pixel 8 API 35"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </label>

          <label className="field">
            <span>Device</span>
            <select
              value={deviceId}
              onChange={(e) => setDeviceId(e.target.value)}
            >
              <option value="">Select a device…</option>
              {devices.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))}
            </select>
          </label>

          <label className="field">
            <span>System Image</span>
            <select
              value={imagePath}
              onChange={(e) => setImagePath(e.target.value)}
            >
              <option value="">Select a system image…</option>
              {images.map((img) => (
                <option key={img.path} value={img.path}>
                  {img.label}
                </option>
              ))}
            </select>
          </label>

          {err && <div className="modal-error">{err}</div>}
        </div>

        <div className="modal-footer">
          <button className="btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button className="btn-primary" onClick={create} disabled={busy}>
            {busy ? "Creating…" : "Create"}
          </button>
        </div>
      </div>
    </div>
  );
}

function SnapshotsModal({
  avd,
  snapshots,
  onClose,
  onSave,
  onRestore,
  onDelete,
}: {
  avd: string;
  snapshots: SnapshotInfo[];
  onClose: () => void;
  onSave: () => void;
  onRestore: (name: string) => void;
  onDelete: (name: string) => void;
}) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Snapshots — {avd}</h2>
          <button className="modal-close" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          {snapshots.length === 0 ? (
            <div className="empty-state">No snapshots. Save one while the emulator is running.</div>
          ) : (
            <div className="snapshot-list">
              {snapshots.map((s) => (
                <div key={s.name} className="snapshot-row">
                  <div className="snapshot-info">
                    <strong>{s.name}</strong>
                    <span>{s.size}</span>
                  </div>
                  <div className="snapshot-actions">
                    <button
                      className="btn-secondary sm"
                      onClick={() => onRestore(s.name)}
                      title="Restore"
                    >
                      <RotateCcw size={14} />
                    </button>
                    <button
                      className="btn-secondary sm danger"
                      onClick={() => onDelete(s.name)}
                      title="Delete"
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn-secondary" onClick={onSave}>
            <Camera size={14} />
            <span>Save Snapshot</span>
          </button>
          <button className="btn-primary" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

function LogsModal({
  logs,
  onClose,
}: {
  logs: string[];
  onClose: () => void;
}) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-lg" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Logcat</h2>
          <button className="modal-close" onClick={onClose}>
            <X size={18} />
          </button>
        </div>
        <div className="modal-body">
          {logs.length === 0 ? (
            <div className="empty-state">Loading logs…</div>
          ) : (
            <pre className="logcat">{logs.join("\n")}</pre>
          )}
        </div>
        <div className="modal-footer">
          <button className="btn-primary" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

export default App;
