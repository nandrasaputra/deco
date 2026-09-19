import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Bot,
  Smartphone,
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
  Menu,
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

interface Simulator {
  udid: string;
  name: string;
  runtime: string;
  os: string;
  device_type: string;
  status: "Booted" | "Shutdown" | string;
}

interface SdkInfo {
  path: string;
  installed: boolean;
}

interface SdkPathInfo {
  current: string;
  custom: string;
}

const navItems = [
  { label: "Devices", icon: Smartphone, enabled: true },
  { label: "Settings", icon: SlidersHorizontal, enabled: true },
];

function App() {
  const [platform, setPlatform] = useState<"android" | "ios">("android");
  const [emulators, setEmulators] = useState<Emulator[]>([]);
  const [simulators, setSimulators] = useState<Simulator[]>([]);
  const [sdk, setSdk] = useState<SdkInfo | null>(null);
  const [sdkMissing, setSdkMissing] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [showSnapshots, setShowSnapshots] = useState(false);
  const [showLogs, setShowLogs] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const [snapshots, setSnapshots] = useState<SnapshotInfo[]>([]);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [view, setView] = useState<"list" | "grid">("list");

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      if (platform === "android") {
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
        // Emulators are readable without an SDK; only flag a hard failure if we
        // could not even locate the AVD directory (not just a missing SDK).
        setSdkMissing(!sdkInfo.installed);
      } else {
        const sims = await invoke<Simulator[]>("list_simulators");
        setSimulators(sims);
        setSelectedId((prev) => {
          if (prev && sims.some((sim) => sim.udid === prev)) return prev;
          return sims[0]?.udid ?? null;
        });
      }
    } catch (e) {
      // Clear the list only on a real failure (unreadable AVD dir, etc.); a
      // missing SDK no longer throws — see list_emulators.
      if (platform === "android") setEmulators([]);
      else setSimulators([]);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 5000);
    return () => clearInterval(id);
  }, [platform]);

  const selected = emulators.find((e) => e.name === selectedId) ?? null;
  const selectedSimulator = simulators.find((sim) => sim.udid === selectedId) ?? null;

  const filtered = emulators.filter((e) =>
    e.display_name.toLowerCase().includes(query.toLowerCase()),
  );
  const filteredSimulators = simulators.filter((sim) =>
    sim.name.toLowerCase().includes(query.toLowerCase()),
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

  async function handleBootSimulator(udid: string) {
    try {
      await invoke("boot_simulator", { udid });
      setTimeout(refresh, 1200);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleShutdownSimulator(udid: string) {
    try {
      await invoke("shutdown_simulator", { udid });
      setTimeout(refresh, 800);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleEraseSimulator(udid: string, name: string) {
    if (!confirm(`Erase all data from simulator "${name}"?`)) return;
    try {
      await invoke("erase_simulator", { udid });
      setTimeout(refresh, 800);
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
      

      <div className="app-body">
        <aside className={`sidebar ${sidebarOpen ? "open" : ""}`}>
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
                  className={`nav-item ${item.enabled ? "" : "disabled"} ${item.label === "Devices" ? "active" : ""}`}
                  title={item.enabled ? item.label : `${item.label} (coming soon)`}
                  disabled={!item.enabled}
                  onClick={() => {
                    if (item.label === "Settings") setShowSettings(true);
                  }}
                >
                  <Icon size={16} />
                  <span>{item.label}</span>
                </button>
              );
            })}
          </nav>
        </aside>

        {sidebarOpen && <div className="sidebar-backdrop" onClick={() => setSidebarOpen(false)} />}

         <main className="main">
           <div className="header">
             <button
               className="icon-btn sidebar-toggle"
               onClick={() => setSidebarOpen((v) => !v)}
               title="Menu"
             >
               <Menu size={16} />
             </button>
             <div className="header-title">
               <h1>Devices</h1>
               <p>{platform === "android" ? "Manage your Android Virtual Devices" : "Manage your iOS Simulators"}</p>
             </div>

             <div className="platform-toggle" role="tablist" aria-label="Platform">
               <button
                 className={platform === "android" ? "active" : ""}
                 onClick={() => {
                   setPlatform("android");
                   setQuery("");
                 }}
               >
                 <Bot size={13} />
                 <span>Android</span>
               </button>
               <button
                 className={platform === "ios" ? "active" : ""}
                 onClick={() => {
                   setPlatform("ios");
                   setQuery("");
                 }}
               >
                 <Smartphone size={13} />
                 <span>iOS</span>
               </button>
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
               className="btn-primary"
               onClick={() => {
                 if (platform === "android") setShowCreate(true);
                 else setError("Create iOS simulators in Xcode, then refresh Deco.");
               }}
             >
               <Plus size={16} />
               <span>{platform === "android" ? "Create Emulator" : "Create Simulator"}</span>
             </button>

            <button
              className="icon-btn"
              onClick={refresh}
              title="Refresh"
              disabled={loading}
            >
              <RefreshCw size={16} className={loading ? "spin" : ""} />
            </button>

            <div className="view-toggle">
              <button
                className={view === "list" ? "active" : ""}
                title="List view"
                onClick={() => setView("list")}
              >
                <List size={18} />
              </button>
              <button
                className={view === "grid" ? "active" : ""}
                title="Grid view"
                onClick={() => setView("grid")}
              >
                <Grid2X2 size={16} />
              </button>
            </div>
          </div>

          {error && <div className="error-banner">{error}</div>}

          {platform === "android" && (
          <div className="sdk-status sdk-status-main">
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
            {sdk && (
              <button
                className="btn-secondary sm-auto"
                onClick={() => setShowSettings(true)}
              >
                {sdk.installed ? "Change…" : "Set SDK path…"}
              </button>
            )}
          </div>
          )}

{view === "list" ? (
          <div className="table-wrap">
             <div className={`table-header ${platform === "ios" ? "ios-columns" : ""}`}>
               <span>Name</span>
               <span>Status</span>
               <span>{platform === "android" ? "API" : "OS"}</span>
               <span>{platform === "android" ? "Size" : "Type"}</span>
               <span>Actions</span>
             </div>
             <div className="table-body">
               {loading && (platform === "android" ? emulators.length === 0 : simulators.length === 0) ? (
                 <div className="empty-state">Loading emulators…</div>
               ) : platform === "android" && filtered.length === 0 ? (
                 <div className="empty-state">
                    {sdkMissing
                      ? "No emulators found. Android SDK is not available — emulator actions will be limited."
                      : "No emulators found."}
                  </div>
               ) : platform === "ios" && filteredSimulators.length === 0 ? (
                 <div className="empty-state">No iOS simulators found. Install a runtime in Xcode.</div>
               ) : platform === "android" ? (
                 filtered.map((emu) => (
                  <div
                    key={emu.name}
                    className={`table-row ${emu.name === selectedId ? "selected" : ""}`}
                    onClick={() => { setSelectedId(emu.name); setDetailOpen(true); }}
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
               ) : (
                 filteredSimulators.map((sim) => (
                   <div
                     key={sim.udid}
                     className={`table-row ios-row ${sim.udid === selectedId ? "selected" : ""}`}
                     onClick={() => { setSelectedId(sim.udid); setDetailOpen(true); }}
                   >
                     <span className="cell-name ios-name">
                       <Smartphone size={14} />
                       <span>
                         <strong>{sim.name}</strong>
                         <small>{sim.device_type}</small>
                       </span>
                     </span>
                     <span>
                       <span className={`status-badge ${sim.status === "Booted" ? "running" : "stopped"}`}>
                         <span className="dot" />
                         {sim.status}
                       </span>
                     </span>
                     <span className="cell-muted">{sim.os}</span>
                     <span className="cell-muted">{sim.device_type}</span>
                     <div className="row-actions">
                       {sim.status === "Booted" ? (
                         <button className="row-action" title="Shutdown" onClick={(e) => { e.stopPropagation(); handleShutdownSimulator(sim.udid); }}>
                           <Square size={14} />
                         </button>
                       ) : (
                         <button className="row-action" title="Boot" onClick={(e) => { e.stopPropagation(); handleBootSimulator(sim.udid); }}>
                           <Play size={14} />
                         </button>
                       )}
                       <button className="row-action" title="Erase Data" onClick={(e) => { e.stopPropagation(); handleEraseSimulator(sim.udid, sim.name); }}>
                         <Trash2 size={14} />
                       </button>
                     </div>
                   </div>
                 ))
               )}
            </div>
          </div>
           ) : (
           <div className="grid-wrap">
             <div className="grid-body">
                {loading && (platform === "android" ? emulators.length === 0 : simulators.length === 0) ? (
                  <div className="empty-state">Loading emulators…</div>
                ) : platform === "android" && filtered.length === 0 ? (
                  <div className="empty-state">
                    {sdkMissing
                      ? "No emulators found. Android SDK is not available — emulator actions will be limited."
                      : "No emulators found."}
                  </div>
                ) : platform === "ios" && filteredSimulators.length === 0 ? (
                  <div className="empty-state">No iOS simulators found. Install a runtime in Xcode.</div>
                ) : platform === "android" ? (
                  filtered.map((emu) => (
                    <div
                      key={emu.name}
                      className={`grid-card ${emu.name === selectedId ? "selected" : ""}`}
                      onClick={() => { setSelectedId(emu.name); setDetailOpen(true); }}
                    >
                      <div className="grid-card-top">
                        <Smartphone size={28} className="grid-card-icon" />
                        <span className={`grid-status ${emu.status.toLowerCase()}`} />
                      </div>
                      <strong className="grid-card-name">{emu.display_name}</strong>
                      <span className="grid-card-meta">API {emu.api} · {emu.abi}</span>
                      <span className="grid-card-meta">{emu.device} · {emu.resolution}</span>
                      <div className="grid-card-actions">
                        {emu.status === "Running" ? (
                          <>
                            <button className="row-action" title="Stop" onClick={(e) => { e.stopPropagation(); if (emu.serial) handleStop(emu.serial); }}>
                              <Square size={14} />
                            </button>
                            <button className="row-action" title="Wipe Data" onClick={(e) => { e.stopPropagation(); handleWipe(emu.name); }}>
                              <Trash2 size={14} />
                            </button>
                          </>
                        ) : (
                          <>
                            <button className="row-action" title="Run" onClick={(e) => { e.stopPropagation(); handleStart(emu.name, false); }}>
                              <Play size={14} />
                            </button>
                            <button className="row-action" title="Cold Start" onClick={(e) => { e.stopPropagation(); handleStart(emu.name, true); }}>
                              <Snowflake size={14} />
                            </button>
                            <button className="row-action" title="Wipe Data" onClick={(e) => { e.stopPropagation(); handleWipe(emu.name); }}>
                              <Trash2 size={14} />
                            </button>
                          </>
                        )}
                      </div>
                      <div className={`grid-card-status-label ${emu.status.toLowerCase()}`}>
                        {emu.status}
                      </div>
                    </div>
                  ))
                ) : (
                  filteredSimulators.map((sim) => (
                    <div
                      key={sim.udid}
                      className={`grid-card ${sim.udid === selectedId ? "selected" : ""}`}
                      onClick={() => { setSelectedId(sim.udid); setDetailOpen(true); }}
                    >
                      <div className="grid-card-top">
                        <Smartphone size={28} className="grid-card-icon" />
                        <span className={`grid-status ${sim.status === "Booted" ? "running" : "stopped"}`} />
                      </div>
                      <strong className="grid-card-name">{sim.name}</strong>
                      <span className="grid-card-meta">{sim.runtime}</span>
                      <span className="grid-card-meta">{sim.device_type}</span>
                      <div className="grid-card-actions">
                        {sim.status === "Booted" ? (
                          <button className="row-action" title="Shutdown" onClick={(e) => { e.stopPropagation(); handleShutdownSimulator(sim.udid); }}>
                            <Square size={14} />
                          </button>
                        ) : (
                          <button className="row-action" title="Boot" onClick={(e) => { e.stopPropagation(); handleBootSimulator(sim.udid); }}>
                            <Play size={14} />
                          </button>
                        )}
                        <button className="row-action" title="Erase Data" onClick={(e) => { e.stopPropagation(); handleEraseSimulator(sim.udid, sim.name); }}>
                          <Trash2 size={14} />
                        </button>
                      </div>
                      <div className={`grid-card-status-label ${sim.status === "Booted" ? "running" : "stopped"}`}>
                        {sim.status}
                      </div>
                    </div>
                  ))
                )}
             </div>
           </div>
           )}
        </main>

        <aside className={`detail-panel ${detailOpen ? "open" : ""}`}>
          <div className="detail-panel-toolbar">
            <button className="icon-btn detail-panel-close" onClick={() => setDetailOpen(false)} title="Close">
              <X size={16} />
            </button>
          </div>
           {platform === "android" && selected ? (
            <>
              <div className="detail-header">
                <h2>{selected.display_name}</h2>
                <div
                  className={`status-line ${selected.status.toLowerCase()}`}
                >
                  {selected.status}
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
                  <div className="action-card disabled" title="Edit (coming soon)">
                    <button>
                      <Pencil size={18} />
                    </button>
                    <span>Edit</span>
                  </div>
                  <div className="action-card disabled" title="Clone (coming soon)">
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
           ) : platform === "ios" && selectedSimulator ? (
             <>
               <div className="detail-header">
                 <h2>{selectedSimulator.name}</h2>
                 <div className={`status-line ${selectedSimulator.status === "Booted" ? "running" : "stopped"}`}>
                   {selectedSimulator.status}
                 </div>
               </div>
                 <div className="detail-metadata">
                   <div className="meta-row"><label>Operating System</label><span>{selectedSimulator.runtime}</span></div>
                   <div className="meta-row"><label>Device Type</label><span>{selectedSimulator.device_type}</span></div>
                   <div className="meta-row"><label>OS Version</label><span>iOS {selectedSimulator.os}</span></div>
                   <div className="meta-row"><label>UDID</label><span className="detail-udid">{selectedSimulator.udid}</span></div>
                 </div>
               <div className="actions-grid">
                 <div className="action-row">
                   <div className="action-card" onClick={() => selectedSimulator.status === "Booted" ? handleShutdownSimulator(selectedSimulator.udid) : handleBootSimulator(selectedSimulator.udid)}>
                     <button className="primary">{selectedSimulator.status === "Booted" ? <Square size={18} /> : <Play size={18} />}</button>
                     <span>{selectedSimulator.status === "Booted" ? "Shutdown" : "Boot"}</span>
                   </div>
                   <div className="action-card" onClick={() => handleEraseSimulator(selectedSimulator.udid, selectedSimulator.name)}>
                     <button className="danger"><Trash2 size={18} /></button>
                     <span>Erase Data</span>
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

      {showSettings && (
        <SettingsModal
          onClose={() => setShowSettings(false)}
          onSaved={() => {
            setShowSettings(false);
            refresh();
          }}
        />
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

function SettingsModal({
  onClose,
  onSaved,
}: {
  onClose: () => void;
  onSaved: () => void;
}) {
  const [info, setInfo] = useState<SdkPathInfo | null>(null);
  const [customPath, setCustomPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const i = await invoke<SdkPathInfo>("get_sdk_path_info");
        setInfo(i);
        setCustomPath(i.custom);
      } catch (e) {
        setErr(String(e));
      }
    })();
  }, []);

  async function handleBrowse() {
    try {
      const picked = await open({
        directory: true,
        multiple: false,
        title: "Select Android SDK folder",
      });
      if (typeof picked === "string") setCustomPath(picked);
    } catch (e) {
      setErr(String(e));
    }
  }

  async function handleSave() {
    setBusy(true);
    setErr(null);
    try {
      await invoke("set_sdk_path", { path: customPath });
      onSaved();
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
          <h2>Settings</h2>
          <button className="modal-close" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          <label className="field">
            <span>Android SDK Path</span>
            <div className="path-row">
              <input
                placeholder="e.g. /Users/me/Library/Android/sdk"
                value={customPath}
                onChange={(e) => setCustomPath(e.target.value)}
              />
              <button className="btn-secondary" onClick={handleBrowse} type="button">
                <FolderOpen size={14} />
                <span>Browse…</span>
              </button>
            </div>
            <em style={{ fontSize: 11, color: "var(--text-faint)" }}>
              Leave empty to use auto-detection ({["ANDROID_SDK_ROOT", "ANDROID_HOME", "~/Library/Android/sdk"].join(" → ")})
            </em>
          </label>

          {info && !customPath && (
            <div className="sdk-info-box">
              <strong>Currently detected:</strong>
              <span>{info.current || "Not found"}</span>
            </div>
          )}

          {err && <div className="modal-error">{err}</div>}
        </div>

        <div className="modal-footer">
          <button className="btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button className="btn-primary" onClick={handleSave} disabled={busy}>
            {busy ? "Saving…" : "Save &amp; Refresh"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default App;
