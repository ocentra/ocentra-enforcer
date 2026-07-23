import { ShieldCheck } from "lucide-react";
import { summarizeLanguages, type Project } from "../data/enforcerAppData";
import type { ReactElement } from "react";

type CommandBarProps = {
  project: Project;
  projects: Project[];
  onProjectChange: (id: string) => void;
  nativeShell: string;
  bindingMode: string;
};

export const CommandBar = ({
  project,
  projects,
  onProjectChange,
  nativeShell,
  bindingMode,
}: CommandBarProps): ReactElement => {
  return (
    <header className="command-bar">
      <div className="project-switcher">
        <span className="eyeless-label">Project</span>
        <select value={project.id} onChange={(event) => onProjectChange(event.target.value)}>
          {projects.map((item) => (
            <option key={item.id} value={item.id}>
              {item.name}
            </option>
          ))}
        </select>
        <small>{project.kind} / {project.branch} / {project.worktree}</small>
        <small>{summarizeLanguages(project.detectedLanguages)} / index {project.indexed}</small>
      </div>
      <div className="lane-pill">
        <ShieldCheck size={16} />
        <span>Desktop shell</span>
        <small>{nativeShell === "tauri" ? bindingMode === "mixed-live-and-staged" ? "Tauri / mixed bindings" : `Tauri / ${bindingMode}` : "preview mode"}</small>
      </div>
    </header>
  );
};
