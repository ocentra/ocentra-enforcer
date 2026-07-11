import { Braces, Check, FolderGit2, FolderPlus, GitBranch, HardDrive, RefreshCw, Search, X } from "lucide-react";
import { type ReactElement, useState } from "react";
import { summarizeLanguages, type Project } from "../data/enforcerAppData";

type ProjectRegistrationPreview = { requestedRoot: string; project: Project; topology: string; gitWorktreeCount: number };

/** Renders the desktop-local registry and Git worktree inventory. */
export function ProjectsWorkspace({ projects, selectedProjectId, registryError, onOpenProject, onAddProject, onPreviewProjectRegistration, onDiscoverProjectWorktrees }: { projects: Project[]; selectedProjectId: string; registryError: string; onOpenProject: (id: string) => void; onAddProject: (project: Project) => Promise<void>; onPreviewProjectRegistration: (root: string) => Promise<ProjectRegistrationPreview>; onDiscoverProjectWorktrees: () => Promise<void> }): ReactElement {
  const [adding, setAdding] = useState(false);
  const [discovering, setDiscovering] = useState(false);
  const [confirmingDiscovery, setConfirmingDiscovery] = useState(false);
  const [discoveryError, setDiscoveryError] = useState("");
  const [projectQuery, setProjectQuery] = useState("");
  const selectedProject = projects.find((project) => project.id === selectedProjectId);
  const normalizedQuery = projectQuery.trim().toLowerCase();
  const matchesProjectQuery = (project: Project): boolean => !normalizedQuery || [project.name, project.root, project.mainRoot, project.branch]
    .filter((value): value is string => typeof value === "string")
    .some((value) => value.toLowerCase().includes(normalizedQuery));
  const mainProjects = projects.filter((project) => project.kind === "main");
  const worktrees = projects.filter((project) => project.kind === "worktree" && matchesProjectQuery(project));
  const externalProjects = projects.filter((project) => project.kind === "external" && matchesProjectQuery(project));
  const repositoryFamilies = mainProjects.map((root) => ({
    root,
    worktrees: worktrees.filter((worktree) => rootsEqual(worktree.mainRoot, root.root)),
  })).filter((family) => matchesProjectQuery(family.root) || family.worktrees.length > 0);
  const unregisteredRootWorktrees = worktrees.filter((worktree) => !repositoryFamilies.some((family) => family.worktrees.some((child) => child.id === worktree.id)));
  const matchingProjectCount = projects.filter(matchesProjectQuery).length;
  async function discoverWorktrees() {
    setDiscovering(true);
    setDiscoveryError("");
    try {
      await onDiscoverProjectWorktrees();
    } catch (error) {
      setDiscoveryError(String(error));
    } finally {
      setDiscovering(false);
    }
  }
  return (
    <section className="main-surface projects-workspace">
      <div className="scan-panel full project-directory">
        <div className="panel-head"><span><strong>Connected projects</strong><small>Registered roots available to scan and index. Git can discover the selected root's primary checkout and linked worktrees.</small></span><div className="action-row"><button className="secondary-action" onClick={() => setConfirmingDiscovery(true)} disabled={discovering || !selectedProject} title={selectedProject ? `Review discovered worktrees for ${selectedProject.name}` : "Select a project before discovering worktrees"}><RefreshCw size={16} /> {discovering ? "Registering..." : "Register discovered worktrees"}</button><button className="primary-action" onClick={() => setAdding(true)}><FolderPlus size={16} /> Register project</button></div></div>
        <div className="run-status">Git discovery writes only desktop-local registrations. It reads branch, root/worktree identity, and bounded observed language stack from Git and the filesystem.{selectedProject && <strong className="project-discovery-target" title={displayProjectPath(selectedProject.root)}> Target: {selectedProject.name}</strong>}</div>
        {discoveryError && <div className="index-error">{discoveryError}</div>}
        {registryError && <div className="index-error">{registryError}</div>}
        <div className="project-directory-toolbar"><label className="project-search"><Search size={16} /><input value={projectQuery} onChange={(event) => setProjectQuery(event.target.value)} placeholder="Filter connected projects" aria-label="Filter connected projects" /></label>{normalizedQuery && <small>{matchingProjectCount} matching project{matchingProjectCount === 1 ? "" : "s"}</small>}</div>
        <div className="project-directory-content">
          <section className="project-group project-family-list"><div className="project-group-title"><FolderGit2 size={16} /><strong>Repository families</strong><small>Primary roots and their registered linked worktrees.</small></div>{repositoryFamilies.map((family) => <ProjectFamily key={family.root.id} root={family.root} worktrees={family.worktrees} selectedProjectId={selectedProjectId} onOpenProject={onOpenProject} />)}</section>
          <ProjectGroup title="Worktrees with unregistered primary root" description="The linked checkout is registered, but its primary root is not in this desktop inventory." projects={unregisteredRootWorktrees} selectedProjectId={selectedProjectId} onOpenProject={onOpenProject} />
          {externalProjects.length > 0 && <ProjectGroup title="External roots" projects={externalProjects} selectedProjectId={selectedProjectId} onOpenProject={onOpenProject} />}
          {normalizedQuery && matchingProjectCount === 0 && <div className="empty-project-results">No connected project matches this filter.</div>}
        </div>
      </div>
      {adding && <AddProjectDialog onClose={() => setAdding(false)} onPreview={onPreviewProjectRegistration} onAdd={async (project) => { await onAddProject(project); setAdding(false); onOpenProject(project.id); }} />}
      {confirmingDiscovery && selectedProject && <DiscoverWorktreesDialog project={selectedProject} onClose={() => setConfirmingDiscovery(false)} onConfirm={() => { setConfirmingDiscovery(false); void discoverWorktrees(); }} />}
    </section>
  );
}

function DiscoverWorktreesDialog({ project, onClose, onConfirm }: { project: Project; onClose: () => void; onConfirm: () => void }) {
  return <div className="policy-dialog-backdrop" role="presentation" onMouseDown={onClose}><section className="project-dialog" role="dialog" aria-modal="true" aria-label="Register discovered worktrees" onMouseDown={(event) => event.stopPropagation()}><div className="panel-head"><span><strong>Register discovered worktrees</strong><small>Review the target before Git discovery writes entries to the desktop-local project registry.</small></span><button className="icon-button" onClick={onClose} title="Close worktree registration"><X size={18} /></button></div><div className="discovery-confirmation"><span>Discovery target</span><strong>{project.name}</strong><code>{displayProjectPath(project.root)}</code><small>Git may add the primary checkout and every linked worktree it discovers. This does not create .enforce, write repository files, change Git, scan code, or create an index.</small></div><div className="dialog-actions"><button className="secondary-action" onClick={onClose}>Cancel</button><button className="primary-action" onClick={onConfirm}><RefreshCw size={16} /> Register discovered worktrees</button></div></section></div>;
}

function ProjectFamily({ root, worktrees, selectedProjectId, onOpenProject }: { root: Project; worktrees: Project[]; selectedProjectId: string; onOpenProject: (id: string) => void }) {
  const members = [root, ...worktrees];
  return <section className="project-family"><div className="project-family-heading"><span><strong>{root.name}</strong><small>{worktrees.length ? `${worktrees.length} registered linked worktree${worktrees.length === 1 ? "" : "s"}` : "No registered linked worktrees"}</small></span><code>{root.root}</code></div><div className="project-family-members"><div className="project-family-member project-family-member-header"><span>Checkout</span><span>Branch</span><span>Root</span><span>Index</span><span>Languages</span></div>{members.map((project) => <ProjectFamilyMemberRow key={project.id} project={project} selectedProjectId={selectedProjectId} onOpenProject={onOpenProject} />)}</div></section>;
}

function ProjectFamilyMemberRow({ project, selectedProjectId, onOpenProject }: { project: Project; selectedProjectId: string; onOpenProject: (id: string) => void }) {
  const primary = project.kind === "main";
  return <button className={project.id === selectedProjectId ? "project-family-member selected-project-card" : "project-family-member"} onClick={() => onOpenProject(project.id)} title={`Open ${project.name}`}><span><strong>{primary ? "Primary" : "Linked"}</strong><small>{project.name}</small></span><span><strong>{project.branch}</strong><small>{project.inspection === "live" ? "Git observed" : "Configured"}</small></span><code title={displayProjectPath(project.root)}>{displayProjectPath(project.root)}</code><em className={`status ${project.indexed}`}>{project.indexed}</em><span className="project-family-languages"><Braces size={14} /> {summarizeLanguages(project.detectedLanguages)}</span></button>;
}

function ProjectGroup({ title, description, projects, selectedProjectId, onOpenProject }: { title: string; description?: string; projects: Project[]; selectedProjectId: string; onOpenProject: (id: string) => void }) {
  if (projects.length === 0) return <></>;
  return <section className="project-group"><div className="project-group-title"><FolderGit2 size={16} /><span><strong>{title}</strong>{description && <small>{description}</small>}</span></div><div className="project-matrix">{projects.map((project) => <ProjectCard key={project.id} project={project} selectedProjectId={selectedProjectId} onOpenProject={onOpenProject} linked={project.kind === "worktree"} />)}</div></section>;
}

function ProjectCard({ project, selectedProjectId, onOpenProject, linked = false }: { project: Project; selectedProjectId: string; onOpenProject: (id: string) => void; linked?: boolean }) {
  return <button className={project.id === selectedProjectId ? "project-card selected-project-card" : "project-card"} onClick={() => onOpenProject(project.id)}><span className={`status ${project.indexed}`}>{project.indexed}</span><strong>{project.name}</strong><small>{project.root}</small>{linked && <small className="project-card-relationship">Linked worktree {project.mainRoot ? `of ${project.mainRoot}` : "with no registered primary root"}</small>}<span className="project-stack"><Braces size={14} /> {summarizeLanguages(project.detectedLanguages)}</span><div className="card-metrics"><span><GitBranch size={15} /> {project.inspection === "live" ? "Git" : "configured"}: {project.branch}</span><span><HardDrive size={15} /> {project.inspection === "live" ? "Git" : "configured"}: {project.worktree}</span></div></button>;
}

function rootsEqual(left: string | undefined, right: string): boolean {
  return left?.replaceAll("\\", "/").replace(/\/+$/, "").toLowerCase() === right.replaceAll("\\", "/").replace(/\/+$/, "").toLowerCase();
}

function displayProjectPath(path: string): string {
  const normalized = path.replaceAll("\\", "/");
  return normalized.startsWith("//?/") ? normalized.slice(4) : normalized;
}

function AddProjectDialog({ onClose, onPreview, onAdd }: { onClose: () => void; onPreview: (root: string) => Promise<ProjectRegistrationPreview>; onAdd: (project: Project) => Promise<void> }) {
  const [name, setName] = useState("");
  const [root, setRoot] = useState("");
  const [preview, setPreview] = useState<ProjectRegistrationPreview>();
  const [inspecting, setInspecting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");
  const canAdd = name.trim().length > 0 && Boolean(preview);
  async function inspect() {
    setInspecting(true);
    setSaveError("");
    try {
      const next = await onPreview(root.trim());
      setPreview(next);
      setName((current) => current.trim() || next.project.name);
    } catch (error) {
      setPreview(undefined);
      setSaveError(String(error));
    } finally {
      setInspecting(false);
    }
  }
  async function save() {
    if (!preview) return;
    setSaving(true);
    setSaveError("");
    try {
      await onAdd({ ...preview.project, name: name.trim() });
    } catch (error) {
      setSaveError(String(error));
      setSaving(false);
    }
  }
  return (
    <div className="policy-dialog-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="project-dialog" role="dialog" aria-modal="true" aria-label="Register project" onMouseDown={(event) => event.stopPropagation()}>
        <div className="panel-head"><span><strong>Register project</strong><small>Inspect a filesystem root first. Git topology, branch, index state, and detected languages come from the machine, then registration writes that derived record locally.</small></span><button className="icon-button" onClick={onClose} title="Close project registration"><X size={18} /></button></div>
        <label className="policy-field"><span>Root path</span><div className="project-root-input"><input value={root} onChange={(event) => { setRoot(event.target.value); setPreview(undefined); }} placeholder="E:/workspace/example" autoFocus /><button className="secondary-action" onClick={inspect} disabled={!root.trim() || inspecting} title="Inspect project root"><Search size={16} /> {inspecting ? "Inspecting" : "Inspect root"}</button></div><small>For Git paths, inspection resolves a nested folder to its owning project or linked worktree before registration.</small></label>
        {preview && <section className="project-inspection-summary" aria-label="Detected project registration"><div><span>Topology</span><strong>{preview.topology}</strong></div><div><span>Branch</span><strong>{preview.project.branch}</strong></div><div><span>Index</span><strong>{preview.project.indexed}</strong></div><div><span>Languages</span><strong>{summarizeLanguages(preview.project.detectedLanguages, 6)}</strong></div>{!rootsEqual(preview.requestedRoot, preview.project.root) && <div><span>Entered path</span><strong>{displayProjectPath(preview.requestedRoot)}</strong></div>}<div><span>Resolved registration root</span><strong>{displayProjectPath(preview.project.root)}</strong></div>{preview.project.mainRoot && <div><span>Primary root</span><strong>{displayProjectPath(preview.project.mainRoot)}</strong></div>}<div><span>Git worktrees discovered</span><strong>{preview.gitWorktreeCount || "not a Git checkout"}</strong></div></section>}
        {preview && <label className="policy-field"><span>Display name</span><input value={name} onChange={(event) => setName(event.target.value)} placeholder="Project name" /></label>}
        {saveError && <div className="index-error">{saveError}</div>}
        <div className="dialog-actions"><button className="secondary-action" onClick={onClose} disabled={saving}>Cancel</button><button className="primary-action" disabled={!canAdd || saving} onClick={save}>{saving ? "Registering..." : <><Check size={16} /> Register project</>}</button></div>
      </section>
    </div>
  );
}
