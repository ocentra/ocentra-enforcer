use std::path::{Path, PathBuf};
use std::process::Command;

use enforcer_literal_scan::language_registry;
use enforcer_memory::ids::repo_root;
use enforcer_memory::store::Store;
use serde::{Deserialize, Serialize};

use crate::detect_project_languages;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopProject {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) root: String,
    pub(crate) repo_key: String,
    pub(crate) kind: String,
    pub(crate) main_root: Option<String>,
    pub(crate) branch: String,
    pub(crate) worktree: String,
    pub(crate) indexed: String,
    pub(crate) detected_languages: Vec<String>,
    pub(crate) inspection: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectDiscoveryPayload {
    pub(crate) projects: Vec<DesktopProject>,
    pub(crate) discovered_count: usize,
    pub(crate) main_root: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectRegistrationPreview {
    pub(crate) requested_root: String,
    pub(crate) project: DesktopProject,
    pub(crate) topology: String,
    pub(crate) git_worktree_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitWorktree {
    pub(crate) root: PathBuf,
    pub(crate) branch: String,
}

#[tauri::command]
pub(crate) fn load_desktop_projects() -> Result<Vec<DesktopProject>, String> {
    let path = desktop_project_registry_path()?;
    load_desktop_projects_from(&path)
}

pub(crate) fn load_desktop_projects_from(path: &Path) -> Result<Vec<DesktopProject>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read desktop project registry: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("cannot decode desktop project registry: {error}"))
}

#[tauri::command]
pub(crate) fn register_desktop_project(
    project: DesktopProject,
) -> Result<Vec<DesktopProject>, String> {
    let path = desktop_project_registry_path()?;
    register_desktop_project_at(&path, project)
}

#[tauri::command]
pub(crate) fn preview_desktop_project_registration(
    root: String,
) -> Result<ProjectRegistrationPreview, String> {
    desktop_project_registration_preview(&PathBuf::from(root))
}

pub(crate) fn desktop_project_registration_preview(
    requested_root: &Path,
) -> Result<ProjectRegistrationPreview, String> {
    if !requested_root.is_dir() {
        return Err(format!(
            "project root is not a directory: {}",
            requested_root.display()
        ));
    }

    let requested_root = requested_root
        .canonicalize()
        .map_err(|error| format!("cannot resolve project root: {error}"))?;
    let git_root = git_value(&requested_root, &["rev-parse", "--show-toplevel"]).map(PathBuf::from);

    let (root, repo_key, kind, main_root, branch, worktree, topology, git_worktree_count) =
        if let Some(git_root) = git_root {
            let worktrees = discover_git_worktrees(&git_root)?;
            let main_root = worktrees
                .first()
                .ok_or_else(|| "git did not report a primary worktree".to_owned())?
                .root
                .clone();
            let selected = worktrees
                .iter()
                .enumerate()
                .find(|(_, worktree)| paths_equal(&worktree.root, &git_root))
                .ok_or_else(|| {
                    format!(
                        "Git reported {} as a checkout, but it was absent from worktree discovery",
                        git_root.display()
                    )
                })?;
            let is_primary = selected.0 == 0;
            let root = selected.1.root.clone();
            let kind = if is_primary { "main" } else { "worktree" };
            let worktree = if is_primary { "primary" } else { "linked" };
            let topology = if is_primary {
                "Git primary checkout"
            } else {
                "Git linked worktree"
            };
            (
                root,
                project_repo_key(&main_root.display().to_string()),
                kind.to_owned(),
                (!is_primary).then(|| main_root.display().to_string()),
                selected.1.branch.clone(),
                worktree.to_owned(),
                topology.to_owned(),
                worktrees.len(),
            )
        } else {
            (
                requested_root.clone(),
                project_repo_key(&requested_root.display().to_string()),
                "external".to_owned(),
                None,
                "not a Git checkout".to_owned(),
                "external".to_owned(),
                "External filesystem root".to_owned(),
                0,
            )
        };
    let root_display = root.display().to_string();
    let indexed = memory_index_available(&root)
        .then_some("ready")
        .unwrap_or("missing");
    let project = DesktopProject {
        id: project_id_for_root(&root_display),
        name: project_name_for_root(&root),
        root: root_display,
        repo_key,
        kind,
        main_root,
        branch,
        worktree,
        indexed: indexed.to_owned(),
        detected_languages: detect_project_languages(&root),
        inspection: Some("live".to_owned()),
    };
    Ok(ProjectRegistrationPreview {
        requested_root: requested_root.display().to_string(),
        project,
        topology,
        git_worktree_count,
    })
}

#[tauri::command]
pub(crate) fn discover_desktop_project_worktrees(
    root: String,
) -> Result<ProjectDiscoveryPayload, String> {
    let root_path = PathBuf::from(&root);
    let worktrees = discover_git_worktrees(&root_path)?;
    let main_root = worktrees
        .first()
        .ok_or_else(|| "git did not report a primary worktree".to_owned())?
        .root
        .display()
        .to_string();
    let registry_path = desktop_project_registry_path()?;
    let mut projects = load_desktop_projects_from(&registry_path)?;

    for (index, worktree) in worktrees.iter().enumerate() {
        let root = worktree.root.display().to_string();
        let kind = if index == 0 { "main" } else { "worktree" };
        let generated = DesktopProject {
            id: project_id_for_root(&root),
            name: project_name_for_root(&worktree.root),
            root: root.clone(),
            repo_key: project_repo_key(&main_root),
            kind: kind.to_owned(),
            main_root: (index != 0).then(|| main_root.clone()),
            branch: worktree.branch.clone(),
            worktree: if index == 0 {
                "primary".to_owned()
            } else {
                "linked".to_owned()
            },
            indexed: "missing".to_owned(),
            detected_languages: detect_project_languages(&worktree.root),
            inspection: Some("live".to_owned()),
        };
        if let Some(existing) = projects
            .iter_mut()
            .find(|project| project.root.eq_ignore_ascii_case(&root))
        {
            existing.repo_key = generated.repo_key;
            existing.kind = generated.kind;
            existing.main_root = generated.main_root;
            existing.branch = generated.branch;
            existing.worktree = generated.worktree;
            existing.detected_languages = generated.detected_languages;
            existing.inspection = generated.inspection;
        } else {
            projects.push(generated);
        }
    }
    write_desktop_projects(&registry_path, &mut projects)?;
    Ok(ProjectDiscoveryPayload {
        projects,
        discovered_count: worktrees.len(),
        main_root,
    })
}

pub(crate) fn discover_git_worktrees(root: &Path) -> Result<Vec<GitWorktree>, String> {
    if !root.is_dir() {
        return Err(format!(
            "project root is not a directory: {}",
            root.display()
        ));
    }
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot start Git worktree discovery: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(format!(
            "Git worktree discovery failed for {}: {}",
            root.display(),
            if detail.is_empty() {
                "not a Git worktree"
            } else {
                detail.as_str()
            }
        ));
    }
    parse_git_worktree_porcelain(&String::from_utf8_lossy(&output.stdout))
}

pub(crate) fn paths_equal(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    left.as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
}

pub(crate) fn memory_index_available(root: &Path) -> bool {
    let root_display = root.display().to_string();
    let Ok(normalized_root) = repo_root(&root_display) else {
        return false;
    };
    Store::open(&root.join(".enforce").join("memory"), &normalized_root)
        .map(|store| store.sqlite_path().exists())
        .unwrap_or(false)
}

pub(crate) fn parse_git_worktree_porcelain(input: &str) -> Result<Vec<GitWorktree>, String> {
    let mut worktrees = Vec::new();
    for block in input.split("\n\n").filter(|block| !block.trim().is_empty()) {
        let mut root = None;
        let mut branch = None;
        let mut detached = false;
        for line in block.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                root = Some(PathBuf::from(value));
            } else if let Some(value) = line.strip_prefix("branch ") {
                branch = Some(
                    value
                        .strip_prefix("refs/heads/")
                        .unwrap_or(value)
                        .to_owned(),
                );
            } else if line == "detached" {
                detached = true;
            }
        }
        let root =
            root.ok_or_else(|| "Git worktree output was missing a worktree path".to_owned())?;
        worktrees.push(GitWorktree {
            root,
            branch: branch.unwrap_or_else(|| {
                if detached {
                    "detached".to_owned()
                } else {
                    "unborn".to_owned()
                }
            }),
        });
    }
    if worktrees.is_empty() {
        return Err("Git worktree discovery returned no worktrees".to_owned());
    }
    Ok(worktrees)
}

fn project_id_for_root(root: &str) -> String {
    format!("git-{}", project_repo_key(root))
}

fn project_repo_key(root: &str) -> String {
    let slug: String = root
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    slug.split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn project_name_for_root(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Git worktree")
        .to_owned()
}

pub(crate) fn register_desktop_project_at(
    path: &Path,
    project: DesktopProject,
) -> Result<Vec<DesktopProject>, String> {
    validate_desktop_project(&project)?;
    if !Path::new(&project.root).is_dir() {
        return Err(format!(
            "cannot register a project root that is not a directory: {}",
            project.root
        ));
    }
    let mut projects = load_desktop_projects_from(path)?;
    if let Some(index) = projects
        .iter()
        .position(|existing| existing.id == project.id)
    {
        projects[index] = project;
    } else if let Some(existing) = projects
        .iter()
        .find(|existing| existing.root.eq_ignore_ascii_case(&project.root))
    {
        return Err(format!(
            "this root is already registered as {}",
            existing.name
        ));
    } else {
        projects.push(project);
    }
    write_desktop_projects(path, &mut projects)?;
    Ok(projects)
}

fn write_desktop_projects(path: &Path, projects: &mut Vec<DesktopProject>) -> Result<(), String> {
    projects.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.root.cmp(&right.root))
    });
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create desktop project registry: {error}"))?;
    }
    let encoded = serde_json::to_vec_pretty(projects)
        .map_err(|error| format!("cannot encode desktop project registry: {error}"))?;
    std::fs::write(path, encoded)
        .map_err(|error| format!("cannot write desktop project registry: {error}"))
}

fn validate_desktop_project(project: &DesktopProject) -> Result<(), String> {
    for (label, value) in [
        ("id", project.id.as_str()),
        ("name", project.name.as_str()),
        ("root", project.root.as_str()),
        ("repoKey", project.repo_key.as_str()),
        ("branch", project.branch.as_str()),
        ("worktree", project.worktree.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("desktop project registration requires {label}"));
        }
    }
    if !matches!(project.kind.as_str(), "main" | "worktree" | "external") {
        return Err("desktop project kind must be main, worktree, or external".to_owned());
    }
    if !matches!(project.indexed.as_str(), "ready" | "stale" | "missing") {
        return Err("desktop project index state must be ready, stale, or missing".to_owned());
    }
    if project.kind == "worktree"
        && project
            .main_root
            .as_deref()
            .is_none_or(|root| root.trim().is_empty())
    {
        return Err("desktop worktree registration requires mainRoot".to_owned());
    }
    if project.detected_languages.iter().any(|language| {
        language != "iac" && !language_registry().iter().any(|spec| spec.id == language)
    }) {
        return Err("desktop project languages must be recognized scanner registry IDs".to_owned());
    }
    Ok(())
}

fn desktop_project_registry_path() -> Result<PathBuf, String> {
    let app_data = std::env::var_os("APPDATA")
        .ok_or_else(|| "APPDATA is unavailable for desktop project registration".to_owned())?;
    Ok(PathBuf::from(app_data)
        .join("OcentraEnforcer")
        .join("desktop-projects.json"))
}

pub(crate) fn git_value(root: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
