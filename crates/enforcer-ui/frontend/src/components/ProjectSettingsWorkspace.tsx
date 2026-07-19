import { Database, FolderCog, Link2, Plus, RefreshCw, Save, ShieldCheck, Trash2 } from "lucide-react";
import { type ReactElement, useEffect, useState } from "react";
import type { Project } from "../data/enforcerAppData";

export type SettingsTab = "scope" | "index" | "connections";
type ProjectSettingsPayload = {
  sourcePath: string;
  nativeTies: Array<{ tool: string; mode: string; scope: string }>;
  ruleToggles: Array<{ ruleId: string; enabled: boolean; severity?: string; waiverOwner?: string; waiverReason?: string }>;
};

type ScanScopeSettingsPayload = { sourcePath: string; exists: boolean; profileName: string; ignoreDirs: string[]; ignoreFileGlobs: string[] };
export function ProjectSettingsWorkspace({ project, initialTab, onCreateMemoryIndex, settings, settingsLoading, settingsError, scanScopeSettings, scanScopeSettingsLoading, scanScopeSettingsError, onWriteScanScopeSettings }: { project: Project; initialTab: SettingsTab; onCreateMemoryIndex: () => Promise<void>; settings?: ProjectSettingsPayload | undefined; settingsLoading: boolean; settingsError: string; scanScopeSettings?: ScanScopeSettingsPayload | undefined; scanScopeSettingsLoading: boolean; scanScopeSettingsError: string; onWriteScanScopeSettings: (request: Pick<ScanScopeSettingsPayload, "profileName" | "ignoreDirs" | "ignoreFileGlobs">) => Promise<void> }): ReactElement {
  const [tab, setTab] = useState<SettingsTab>(initialTab);
  const [ignoredPaths, setIgnoredPaths] = useState<string[]>([]);
  const [newPath, setNewPath] = useState("");
  const [profileName, setProfileName] = useState("default");
  const [scopeSaving, setScopeSaving] = useState(false);
  const [scopeError, setScopeError] = useState("");
  const [indexState, setIndexState] = useState(project.indexed);
  const [indexError, setIndexError] = useState("");
  const [indexing, setIndexing] = useState(false);

  useEffect(() => {
    setTab(initialTab);
  }, [initialTab, project.id]);

  useEffect(() => {
    if (!scanScopeSettings) return;
    setProfileName(scanScopeSettings.profileName);
    setIgnoredPaths([...scanScopeSettings.ignoreDirs, ...scanScopeSettings.ignoreFileGlobs]);
  }, [scanScopeSettings]);

  function addIgnoredPath() {
    const path = newPath.trim();
    if (!path || ignoredPaths.includes(path)) return;
    setIgnoredPaths((current) => [...current, path]);
    setNewPath("");
  }

  async function createIndex() {
    setIndexError("");
    setIndexing(true);
    try {
      await onCreateMemoryIndex();
      setIndexState("ready");
    } catch (error) {
      setIndexError(String(error));
    } finally {
      setIndexing(false);
    }
  }

  async function saveScanScope() {
    setScopeSaving(true);
    setScopeError("");
    try {
      const isGlob = (path: string) => /[*?[]/.test(path);
      await onWriteScanScopeSettings({ profileName, ignoreDirs: ignoredPaths.filter((path) => !isGlob(path)), ignoreFileGlobs: ignoredPaths.filter(isGlob) });
    } catch (error) {
      setScopeError(String(error));
    } finally {
      setScopeSaving(false);
    }
  }

  return (
    <section className="main-surface project-settings-workspace">
      <aside className="settings-rail">
        <div className="panel-head"><span><strong>Project settings</strong><small>Applies only to {project.name}.</small></span></div>
        <div className="settings-tabs" role="tablist" aria-label="Project settings sections">
          <button className={tab === "scope" ? "settings-tab active" : "settings-tab"} onClick={() => setTab("scope")} role="tab" aria-selected={tab === "scope"}><FolderCog size={16} /> Scan scope</button>
          <button className={tab === "index" ? "settings-tab active" : "settings-tab"} onClick={() => setTab("index")} role="tab" aria-selected={tab === "index"}><Database size={16} /> Index</button>
          <button className={tab === "connections" ? "settings-tab active" : "settings-tab"} onClick={() => setTab("connections")} role="tab" aria-selected={tab === "connections"}><Link2 size={16} /> Connections</button>
        </div>
      </aside>
      <div className="settings-content">
        {tab === "scope" && (
          <>
            <div className="panel-head"><span><strong>Scan scope</strong><small>Canonical scanner policy from `ocentra-enforcer.config.json`, separate from rule-toggle configuration.</small></span><button className="primary-action" onClick={saveScanScope} disabled={scopeSaving || scanScopeSettingsLoading}><Save size={16} /> {scopeSaving ? "Saving scope" : scanScopeSettings?.exists ? "Save scan scope" : "Initialize scan policy"}</button></div>
            {scanScopeSettingsLoading && <div className="run-status">Loading typed scanner policy.</div>}
            {scanScopeSettingsError && <div className="index-error">{scanScopeSettingsError}</div>}
            {scanScopeSettings && <div className="settings-root-row"><span>Scanner policy</span><code>{scanScopeSettings.sourcePath} / {scanScopeSettings.profileName}</code></div>}
            <div className="settings-root-row"><span>Project root</span><code>{project.root}</code></div>
            <label className="policy-field"><span>Scanner profile</span><select value={profileName} onChange={(event) => setProfileName(event.target.value)} disabled={scopeSaving}><option value="default">default</option><option value="strict">strict</option><option value="ocentra-enforcer">ocentra-enforcer</option><option value="ocentra-parent">ocentra-parent</option></select></label>
            <div className="settings-list-section">
              <div className="detail-heading"><ShieldCheck size={17} /><strong>Ignored directories and file globs</strong></div>
              {ignoredPaths.map((path) => <div className="settings-list-row" key={path}><code>{path}</code><button title={`Remove ${path}`} onClick={() => setIgnoredPaths((current) => current.filter((item) => item !== path))} disabled={scopeSaving}><Trash2 size={15} /></button></div>)}
              <div className="settings-add-row"><input value={newPath} onChange={(event) => setNewPath(event.target.value)} onKeyDown={(event) => event.key === "Enter" && addIgnoredPath()} placeholder="directory or glob, for example coverage/**" disabled={scopeSaving} /><button className="primary-action" onClick={addIgnoredPath} disabled={scopeSaving}><Plus size={16} /> Add path</button></div>
            </div>
            {scopeError && <div className="index-error">{scopeError}</div>}
          </>
        )}
        {tab === "index" && (
          <>
            <div className="panel-head"><span><strong>Memory index</strong><small>Create the Store-backed X06 code projection explicitly. The interactive index skips per-file Git history; Memory Explorer never indexes on open.</small></span>{indexState === "ready" ? <strong className="index-status ready">Index ready</strong> : <button className="primary-action" onClick={createIndex} disabled={indexing}><RefreshCw size={16} /> {indexing ? "Creating index" : "Create code index"}</button>}</div>
            <div className="settings-status-grid">
              <div><small>State</small><strong className={`index-status ${indexState}`}>{indexState}</strong></div>
              <div><small>Observed stack</small><strong>{project.detectedLanguages.length ? project.detectedLanguages.slice(0, 3).join(" / ") + (project.detectedLanguages.length > 3 ? ` +${project.detectedLanguages.length - 3}` : "") : "not scanned"}</strong></div>
              <div><small>Repository</small><strong>{project.repoKey}</strong></div>
            </div>
            <div className="settings-toggle-row"><span><strong>Worktree boundary</strong><small>Each registered worktree remains a distinct index root. Cross-worktree traversal is not configured from this screen.</small></span></div>
            {project.mainRoot && <div className="settings-root-row"><span>Main root</span><code>{project.mainRoot}</code></div>}
            {indexError && <div className="index-error">{indexError}</div>}
          </>
        )}
        {tab === "connections" && (
          <>
            <div className="panel-head"><span><strong>Effective configuration</strong><small>Typed project configuration is separate from the cross-project Hub ledger.</small></span></div>
            {settingsLoading && <div className="run-status">Loading typed project configuration.</div>}
            {settingsError && <div className="index-error">{settingsError}</div>}
            {settings && <><div className="settings-root-row"><span>Config source</span><code>{settings.sourcePath}</code></div><div className="settings-list-section"><div className="detail-heading"><ShieldCheck size={17} /><strong>Native tool ties</strong></div>{settings.nativeTies.map((tie) => <div className="settings-list-row" key={tie.tool}><code>{tie.tool}</code><span>{tie.mode} / {tie.scope}</span></div>)}</div><div className="settings-list-section"><div className="detail-heading"><ShieldCheck size={17} /><strong>Explicit rule changes</strong></div>{settings.ruleToggles.length ? settings.ruleToggles.map((toggle) => <div className="settings-list-row" key={toggle.ruleId}><code>{toggle.ruleId}</code><span>{toggle.enabled ? "enabled" : `waived by ${toggle.waiverOwner ?? "unknown"}`} {toggle.severity ? `/ ${toggle.severity}` : ""}</span></div>) : <div className="proof-empty">No explicit rule toggles are recorded in this project configuration.</div>}</div></>}
            <div className="connection-row"><span><Database size={18} /><strong>X06 memory Store</strong></span><em className={indexState === "ready" ? "connected" : "attention"}>{indexState === "ready" ? "Indexed" : "Not indexed"}</em></div>
            <div className="connection-row"><span><ShieldCheck size={18} /><strong>Enforcer policy registry</strong></span><em className={settings ? "connected" : "attention"}>{settings ? "Loaded" : "Unavailable"}</em></div>
            <div className="connection-row"><span><Link2 size={18} /><strong>Proof ledger</strong></span><em>Read separately in Proofs</em></div>
          </>
        )}
      </div>
    </section>
  );
}
