import { Database, FolderGit2, SlidersHorizontal } from "lucide-react";
import type { Project } from "../data/enforcerAppData";

export function ProjectRail({
  projects,
  selectedProjectId,
  onSelectProject,
  doctrines,
}: {
  projects: Project[];
  selectedProjectId: string;
  onSelectProject: (id: string) => void;
  doctrines: Array<{ id: string; label: string; selected: string; options: string[] }>;
}) {
  return (
    <aside className="project-rail">
      <div className="section-title">
        <FolderGit2 size={17} />
        Project inventory
      </div>
      <div className="project-list">
        {projects.map((project) => (
          <button
            className={project.id === selectedProjectId ? "project-row selected" : "project-row"}
            key={project.id}
            onClick={() => onSelectProject(project.id)}
          >
            <span>
              <strong>{project.name}</strong>
              <small>{project.root}</small>
            </span>
            <em className={`status ${project.indexed}`}>{project.indexed}</em>
          </button>
        ))}
      </div>
      <div className="section-title compact">
        <SlidersHorizontal size={17} />
        Doctrine controls
      </div>
      <div className="doctrine-stack">
        {doctrines.map((doctrine) => (
          <div className="doctrine-control" key={doctrine.id}>
            <label>{doctrine.label}</label>
            <select defaultValue={doctrine.selected}>
              {doctrine.options.map((option) => (
                <option key={option}>{option}</option>
              ))}
            </select>
          </div>
        ))}
      </div>
      <div className="index-card">
        <Database size={18} />
        <span>
          <strong>Index gap</strong>
          <small>Same project across worktrees needs an identity strategy.</small>
        </span>
      </div>
    </aside>
  );
}
