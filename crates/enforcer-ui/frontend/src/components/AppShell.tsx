import {
  Activity,
  Boxes,
  Cpu,
  FileCheck2,
  FlaskConical,
  GitBranch,
  LayoutDashboard,
  ListChecks,
  Network,
  Radar,
  ScanLine,
  ShieldCheck,
  SlidersHorizontal,
  Settings2,
  Wrench,
} from "lucide-react";
import type { ReactElement, ReactNode } from "react";

export type WorkspaceKey = "overview" | "setup" | "findings" | "projects" | "engine" | "analysis" | "runs" | "rules" | "doctrine" | "settings" | "assurance" | "hub" | "proofs" | "memory";

type AppShellProps = {
  active: WorkspaceKey;
  onNavigate: (key: WorkspaceKey) => void;
  nativeShell: string;
  bindingMode: string;
  children: ReactNode;
};

const projectNavItems: Array<{ key: WorkspaceKey; label: string; icon: ReactNode }> = [
  { key: "projects", label: "Projects", icon: <Boxes size={18} /> },
  { key: "overview", label: "Overview", icon: <LayoutDashboard size={18} /> },
  { key: "setup", label: "Setup", icon: <Wrench size={18} /> },
  { key: "engine", label: "Engine", icon: <Cpu size={18} /> },
  { key: "findings", label: "Scan", icon: <ScanLine size={18} /> },
  { key: "analysis", label: "Analysis", icon: <FlaskConical size={18} /> },
  { key: "runs", label: "Runs", icon: <Activity size={18} /> },
  { key: "rules", label: "Rules", icon: <ListChecks size={18} /> },
  { key: "doctrine", label: "Policy", icon: <Settings2 size={18} /> },
  { key: "settings", label: "Settings", icon: <SlidersHorizontal size={18} /> },
  { key: "assurance", label: "Assurance", icon: <ShieldCheck size={18} /> },
  { key: "proofs", label: "Proofs", icon: <FileCheck2 size={18} /> },
  { key: "memory", label: "Memory", icon: <Network size={18} /> },
];

const hubNavItems: Array<{ key: WorkspaceKey; label: string; icon: ReactNode }> = [
  { key: "hub", label: "Lane Hub", icon: <GitBranch size={18} /> },
];

export const AppShell = ({
  active,
  onNavigate,
  nativeShell,
  bindingMode,
  children,
}: AppShellProps): ReactElement => {
  const activeMode = active === "hub" ? "hub" : "project";

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark"><Radar size={22} /></span>
          <span>Enforcer</span>
        </div>
        <div className="side-mode-tabs" aria-label="Sidebar mode">
          <button
            className={activeMode === "project" ? "mode-tab active" : "mode-tab"}
            onClick={() => onNavigate("projects")}
            title="Project mode"
            aria-label="Project mode"
          >
            <Boxes className="mode-icon" size={16} />
            <span>Project</span>
          </button>
          <button
            className={activeMode === "hub" ? "mode-tab active" : "mode-tab"}
            onClick={() => onNavigate("hub")}
            title="Hub mode"
            aria-label="Hub mode"
          >
            <GitBranch className="mode-icon" size={16} />
            <span>Hub</span>
          </button>
        </div>
        <nav className="side-nav">
          {activeMode === "project"
            ? projectNavItems.map((item) => (
                <button
                  key={item.key}
                  className={item.key === active ? "nav-item active" : "nav-item"}
                  onClick={() => onNavigate(item.key)}
                  title={item.label}
                >
                  {item.icon}
                  <span>{item.label}</span>
                </button>
              ))
            : hubNavItems.map((item) => (
                <button
                  key={item.key}
                  className={item.key === active ? "nav-item active hub-item" : "nav-item hub-item"}
                  onClick={() => onNavigate(item.key)}
                  title={item.label}
                >
                  {item.icon}
                  <span>{item.label}</span>
                </button>
              ))}
        </nav>
        <div className="sidebar-footer" title={`Desktop shell: ${nativeShell}. Binding mode: ${bindingMode}.`}>
          <span className="dot online" />
          <span>{nativeShell === "tauri" ? "Desktop live" : "Preview mode"}</span>
          <small>{bindingMode === "mixed-live-and-staged" ? "mixed bindings" : bindingMode}</small>
        </div>
      </aside>
      <section className="workspace">{children}</section>
    </div>
  );
};
