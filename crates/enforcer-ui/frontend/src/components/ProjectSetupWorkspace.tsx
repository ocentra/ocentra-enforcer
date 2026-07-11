import {
  ArrowUpRight,
  BookOpenCheck,
  Cable,
  Database,
  FileCheck2,
  Flag,
  FolderCheck,
  GitPullRequest,
  Settings2,
} from "lucide-react";
import { useState, type ComponentType } from "react";
import type { Project } from "../data/enforcerAppData";
import type { WorkspaceKey } from "./AppShell";
import type { SettingsTab } from "./ProjectSettingsWorkspace";

type SetupTone = "ready" | "partial" | "missing";
type SetupSection = "foundation" | "integration" | "evidence" | "delivery";

type SetupCard = {
  title: string;
  detail: string;
  state: string;
  tone: SetupTone;
  icon: ComponentType<{ size?: number }>;
  section: SetupSection;
  wide?: boolean;
  action?: { label: string; workspace: WorkspaceKey; settingsTab?: SettingsTab };
};

type ProjectSettings = {
  sourcePath: string;
  nativeTies: Array<{ tool: string; mode: string; scope: string }>;
  ruleToggles: Array<{ ruleId: string; enabled: boolean }>;
};

type ScanScopeSettings = {
  sourcePath: string;
  exists: boolean;
  profileName: string;
  ignoreDirs: string[];
  ignoreFileGlobs: string[];
};

type ProofSnapshot = {
  claim: { state: "unconfigured" | "invalid-registry" | "no-required-proofs" | "ready" | "blocked" };
  journal: { state: "missing" | "verified" | "invalid"; recordCount: number };
};

export function ProjectSetupWorkspace({
  project,
  settings,
  settingsLoading,
  settingsError,
  scanScopeSettings,
  scanScopeSettingsLoading,
  scanScopeSettingsError,
  proofSnapshot,
  proofLoading,
  proofError,
  onNavigate,
}: {
  project: Project;
  settings?: ProjectSettings;
  settingsLoading: boolean;
  settingsError: string;
  scanScopeSettings?: ScanScopeSettings;
  scanScopeSettingsLoading: boolean;
  scanScopeSettingsError: string;
  proofSnapshot?: ProofSnapshot;
  proofLoading: boolean;
  proofError: string;
  onNavigate: (workspace: WorkspaceKey, settingsTab?: SettingsTab) => void;
}) {
  const [activeSection, setActiveSection] = useState<SetupSection>("foundation");
  const setupCards: SetupCard[] = [
    {
      title: "Project registration",
      detail: "This desktop record identifies the root, Git topology, branch, and observed language stack. It is not Enforcer onboarding.",
      state: project.inspection === "live" ? `${project.kind} observed` : "desktop record",
      tone: project.inspection === "live" ? "ready" : "partial",
      icon: FolderCheck,
      section: "foundation",
      action: { label: "Open projects", workspace: "projects" },
    },
    {
      title: "Scan scope",
      detail: scanScopeSettings?.exists ? `Project scan scope uses profile ${scanScopeSettings.profileName}.` : "No project scan-scope file exists; the scanner uses its current default scope.",
      state: scanScopeSettingsLoading ? "loading" : scanScopeSettingsError ? "unavailable" : scanScopeSettings?.exists ? "configured" : "defaults active",
      tone: scanScopeSettingsError ? "missing" : scanScopeSettings?.exists ? "ready" : "partial",
      icon: Settings2,
      section: "foundation",
      action: { label: "Open settings", workspace: "settings", settingsTab: "scope" },
    },
    {
      title: "Rule policy",
      detail: settings ? `${settings.ruleToggles.length} project rule settings and ${settings.nativeTies.length} displayed native-tool ties are readable from the current policy source. This does not prove a resolved f03 enforcement tie.` : "The project policy reader has not returned a source for this root.",
      state: settingsLoading ? "loading" : settingsError ? "unavailable" : settings ? "readable" : "not loaded",
      tone: settingsError ? "missing" : settings ? "ready" : "partial",
      icon: BookOpenCheck,
      section: "foundation",
      action: { label: "Open policy", workspace: "doctrine" },
    },
    {
      title: "Code index",
      detail: project.indexed === "ready" ? "The desktop reports a code index for this root. Refresh remains unavailable until a Rust refresh contract exists." : "No ready code index is reported for this root; creation is available from Settings when supported.",
      state: project.indexed === "ready" ? "index ready" : project.indexed === "stale" ? "index stale" : "index missing",
      tone: project.indexed === "ready" ? "ready" : "partial",
      icon: Database,
      section: "foundation",
      action: { label: "Open index", workspace: "settings", settingsTab: "index" },
    },
    {
      title: "Hub adapters",
      detail: "Harness discovery, adapter capability evidence, installation, repair, and verification are global Hub concerns. They do not belong to this selected project record.",
      state: "Hub-owned capability",
      tone: "partial",
      icon: Cable,
      section: "integration",
      action: { label: "Open Hub", workspace: "hub" },
    },
    {
      title: "Baseline onboarding",
      detail: "The Rust f02 onboarding command, typed project registration, and baseline artifact do not exist yet. Registration above cannot substitute for them.",
      state: "not implemented in Rust",
      tone: "missing",
      icon: Flag,
      section: "evidence",
    },
    {
      title: "Proof readiness",
      detail: proofSnapshot ? `${proofSnapshot.journal.recordCount} journal records are available in the Rust proof read model. A ready claim is evidence state, not CI wiring.` : "The project proof read model is not available for this root.",
      state: proofLoading ? "loading" : proofError ? "unavailable" : proofSnapshot ? proofSnapshot.claim.state : "not loaded",
      tone: proofError ? "missing" : proofSnapshot?.claim.state === "ready" ? "ready" : "partial",
      icon: FileCheck2,
      section: "evidence",
      action: { label: "Open proofs", workspace: "proofs" },
    },
    {
      title: "CI posture",
      detail: "Test Doctrine can inspect CI files and category wiring through the legacy analysis bridge. Its result is observed posture, not CI execution or a verified gate.",
      state: "legacy observation",
      tone: "partial",
      icon: FileCheck2,
      section: "delivery",
      action: { label: "Open analysis", workspace: "analysis" },
    },
    {
      title: "CI lifecycle",
      detail: "The C11 install, inspect, configure, onboard, CI-wire, failing-case, and clean-baseline verification flow has no Rust desktop command or persisted lifecycle state yet.",
      state: "not implemented in Rust",
      tone: "missing",
      icon: GitPullRequest,
      section: "delivery",
      wide: true,
    },
  ];
  const sections: Array<{ id: SetupSection; label: string; detail: string }> = [
    { id: "foundation", label: "Foundation", detail: "Registration, policy, scope, and index" },
    { id: "integration", label: "Integrations", detail: "Global Hub adapters boundary" },
    { id: "evidence", label: "Evidence", detail: "Baseline and proof readiness" },
    { id: "delivery", label: "Delivery", detail: "CI posture and lifecycle" },
  ];
  const visibleCards = setupCards.filter((card) => card.section === activeSection);
  const activeSectionMeta = sections.find((section) => section.id === activeSection)!;

  return (
    <section className="main-surface setup-workspace">
      <header className="setup-heading">
        <span>
          <small>Selected project lifecycle</small>
          <strong>Project setup</strong>
          <em>{project.name} · {project.root}</em>
        </span>
        <span className="setup-boundary">readiness map</span>
      </header>
      <div className="setup-callout">Each card reports a live read model, a real desktop action, or an explicit missing Rust boundary. This is not a completion score.</div>
      <section className="setup-body" aria-label="Project setup lifecycle">
        <aside className="setup-section-rail">
          <div className="panel-head"><span><strong>Lifecycle phases</strong><small>Choose one phase to inspect its backed and missing capability boundaries.</small></span></div>
          <div className="setup-section-tabs" role="tablist" aria-label="Project setup phases">
            {sections.map((section) => { const count = setupCards.filter((card) => card.section === section.id).length; return <button key={section.id} role="tab" aria-selected={activeSection === section.id} className={activeSection === section.id ? "active" : ""} onClick={() => setActiveSection(section.id)}><span><strong>{section.label}</strong><small>{section.detail}</small></span><em>{count}</em></button>; })}
          </div>
        </aside>
        <section className="setup-content setup-content-panel">
          <div className="panel-head"><span><strong>{activeSectionMeta.label}</strong><small>{activeSectionMeta.detail}</small></span><span className="setup-phase-count">{visibleCards.length === 1 ? "1 capability" : `${visibleCards.length} capabilities`}</span></div>
          <div className="setup-grid">
            {visibleCards.map((card) => <SetupLifecycleCard key={card.title} card={card} onNavigate={onNavigate} />)}
          </div>
        </section>
      </section>
    </section>
  );
}

function SetupLifecycleCard({ card, onNavigate }: { card: SetupCard; onNavigate: (workspace: WorkspaceKey, settingsTab?: SettingsTab) => void }) {
  const Icon = card.icon;
  return <article className={card.wide ? "setup-card wide" : "setup-card"}><div className="setup-card-head"><Icon size={18} /><span><strong>{card.title}</strong><em className={card.tone}>{card.state}</em></span></div><p>{card.detail}</p>{card.action && <button className="setup-card-action" onClick={() => onNavigate(card.action!.workspace, card.action!.settingsTab)}>{card.action.label}<ArrowUpRight size={15} /></button>}</article>;
}
