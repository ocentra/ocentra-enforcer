import { type ReactElement, useState } from "react";
import { Braces, FileCog, ListChecks, ShieldCheck } from "lucide-react";
import { summarizeLanguages, type Project } from "../data/enforcerAppData";
import { projectRuleLanguages, type CatalogRule, type RuleOverride, unsupportedProjectRuleLanguages } from "../data/ruleCatalog";

type ProjectSettingsPayload = {
  sourcePath: string;
  nativeTies: Array<{ tool: string; mode: string; scope: string }>;
};
type ScanScopeSettingsPayload = {
  sourcePath: string;
  exists: boolean;
  profileName: string;
  ignoreDirs: string[];
  ignoreFileGlobs: string[];
};

type PolicySection = "binding" | "coverage" | "tools" | "scope" | "overrides";

export function DoctrineWorkspace({ project, catalog, overrides, settings, settingsLoading, settingsError, scanScopeSettings, onOpenRules, onOpenSettings }: { project: Project; catalog: CatalogRule[]; overrides: RuleOverride[]; settings?: ProjectSettingsPayload | undefined; settingsLoading: boolean; settingsError: string; scanScopeSettings?: ScanScopeSettingsPayload | undefined; onOpenRules: () => void; onOpenSettings: () => void }): ReactElement {
  const exclusions = [...(scanScopeSettings?.ignoreDirs ?? []), ...(scanScopeSettings?.ignoreFileGlobs ?? [])];
  const policyLanguages = projectRuleLanguages(project, catalog);
  const observedWithoutPolicy = unsupportedProjectRuleLanguages(project, catalog);
  const [activeSection, setActiveSection] = useState<PolicySection>("binding");
  const sections: Array<{ id: PolicySection; label: string; detail: string; count?: string }> = [
    { id: "binding", label: "Policy binding", detail: "Source and observed stack" },
    { id: "coverage", label: "Rule coverage", detail: "Catalog-backed families", count: String(policyLanguages.length) },
    { id: "tools", label: "Native tools", detail: "Typed tool ties", count: settings ? String(settings.nativeTies.length) : "--" },
    { id: "scope", label: "Scan scope", detail: "Ignored paths and globs", count: scanScopeSettings ? String(exclusions.length) : "--" },
    ...(overrides.length > 0 ? [{ id: "overrides" as const, label: "Overrides", detail: "Explicit project changes", count: String(overrides.length) }] : []),
  ];
  return (
    <section className="main-surface policy-layout">
      <aside className="policy-navigation">
        <div className="panel-head"><span><strong>Project policy</strong><small>Effective configuration for {project.name}.</small></span><FileCog size={18} /></div>
        <div className="policy-section-tabs" role="tablist" aria-label="Project policy sections">
          {sections.map((section) => <button key={section.id} role="tab" aria-selected={activeSection === section.id} className={activeSection === section.id ? "active" : ""} onClick={() => setActiveSection(section.id)}><span><strong>{section.label}</strong><small>{section.detail}</small></span>{section.count && <em>{section.count}</em>}</button>)}
        </div>
        <div className="policy-navigation-note"><ShieldCheck size={16} /><span>Policy records project decisions. Individual rule changes remain in Rules.</span></div>
      </aside>
      <div className="scan-panel policy-content-panel">
        {activeSection === "binding" && <>
          <div className="panel-head"><span><strong>Policy binding</strong><small>Read the selected project's effective policy source and observed stack.</small></span><FileCog size={18} /></div>
          <div className="policy-source"><FileCog size={17} /><span><strong>Project policy binding</strong><small>{settings ? `Reading effective policy from ${settings.sourcePath}.` : settingsLoading ? "Loading effective project policy." : "Project policy is unavailable."}</small></span><span className={`status ${settings ? "ready" : "stale"}`}>{settings ? "read-only live" : "unavailable"}</span></div>
          {settingsError && <div className="index-error">{settingsError}</div>}
          <div className="policy-section"><div className="policy-section-head"><Braces size={17} /><span><strong>Observed project stack</strong><small>{project.detectedLanguages.length} language identifiers found by bounded project inspection.</small></span></div><div className="policy-language-list"><span>{summarizeLanguages(project.detectedLanguages, 8)}</span></div></div>
        </>}
        {activeSection === "coverage" && <>
          <div className="panel-head"><span><strong>Rule policy coverage</strong><small>Only catalog-backed language families appear by default in Rules.</small></span><button className="ghost-button" onClick={onOpenRules}>Review rules</button></div>
          <div className="policy-language-list">{policyLanguages.map((language) => <span key={language}>{language}</span>)}</div>
          {observedWithoutPolicy.length > 0 && <div className="policy-callout"><ShieldCheck size={16} /><span>{summarizeLanguages(observedWithoutPolicy, 6)} is observed, but has no named rule family in the current policy registry.</span></div>}
        </>}
        {activeSection === "tools" && <>
          <div className="panel-head"><span><strong>Native tool ties</strong><small>Resolved typed configuration for native tools.</small></span><ShieldCheck size={18} /></div>
          <div className="policy-table">{settings?.nativeTies.map((tie) => <div key={tie.tool}><strong>{tie.tool}</strong><span>{tie.mode}</span><em>{tie.scope}</em></div>)}{!settings && <div><strong>Unavailable</strong><span>Config read model not loaded</span><em>--</em></div>}</div>
        </>}
        {activeSection === "scope" && <>
          <div className="panel-head"><span><strong>Scan exclusions</strong><small>{scanScopeSettings ? scanScopeSettings.exists ? `Configured by ${scanScopeSettings.sourcePath}.` : "No project scan-policy file exists; defaults apply." : "Scanner policy is still loading."}</small></span><button className="ghost-button" onClick={onOpenSettings}>Edit scope</button></div>
          {scanScopeSettings && <div className="policy-language-list">{exclusions.length ? exclusions.map((path) => <span key={path}>{path}</span>) : <span>No configured ignore paths or globs</span>}</div>}
        </>}
        {activeSection === "overrides" && <>
          <div className="panel-head"><span><strong>Explicit rule changes</strong><small>{overrides.length} project-specific overrides</small></span><ListChecks size={18} /></div>
          <div className="override-summary-list">{overrides.map((override) => <div key={override.ruleId}><strong>{override.ruleId}</strong><span>{override.enabled ? override.severity ?? "registry severity" : "disabled with waiver"}</span>{override.waiver && <small>{override.waiver.owner}: {override.waiver.reason}</small>}</div>)}</div>
          <div className="policy-callout"><ShieldCheck size={17} /><span>Disabling requires a waiver owner and reason. Saved changes use the typed Rust project configuration path and are reloaded from the selected project.</span></div>
          <button className="primary-action full-width" onClick={onOpenRules}><ListChecks size={16} /> Review rule catalog</button>
        </>}
      </div>
    </section>
  );
}
