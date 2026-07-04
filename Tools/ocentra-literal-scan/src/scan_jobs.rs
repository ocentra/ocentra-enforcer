use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;

use crate::discovery::discover_files;
use crate::discovery_ignore::{is_probably_binary, IgnoreState};
use crate::normalize_path;
use crate::{
    classify_file_role, detect_language, CliOptions, FileJob, IgnoredSummary, LanguageFamily,
};

pub(crate) fn build_scan_jobs(
    root: &Path,
    opts: &CliOptions,
    ignore_state: &IgnoreState,
    ignored: &mut IgnoredSummary,
) -> io::Result<(usize, Vec<FileJob>)> {
    let language_filter = collect_language_filter(opts);
    let files = discover_files(root, opts, ignore_state, ignored)?;
    let files_discovered = files.len();
    let mut jobs = Vec::new();

    for path in files {
        if should_skip_large_or_binary_file(&path, opts, ignored)? {
            continue;
        }
        let Some(language) = detect_language(&path, opts.include_unknown_code) else {
            ignored.unknown_language += 1;
            continue;
        };
        if should_skip_language(language.id, language.family, &language_filter) {
            continue;
        }
        let rel = normalize_path(path.strip_prefix(root).unwrap_or(&path));
        jobs.push(FileJob {
            role: classify_file_role(&rel, language),
            path,
            rel,
            language,
        });
    }

    Ok((files_discovered, jobs))
}

fn collect_language_filter(opts: &CliOptions) -> HashSet<String> {
    opts.languages.iter().map(|value| value.to_lowercase()).collect()
}

fn should_skip_large_or_binary_file(
    path: &Path,
    opts: &CliOptions,
    ignored: &mut IgnoredSummary,
) -> io::Result<bool> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(true),
    };
    if metadata.len() > opts.max_file_bytes {
        ignored.too_large += 1;
        return Ok(true);
    }
    if is_probably_binary(path)? {
        ignored.binary += 1;
        return Ok(true);
    }
    Ok(false)
}

fn should_skip_language(
    language_id: &str,
    family: LanguageFamily,
    language_filter: &HashSet<String>,
) -> bool {
    let filtered_out = !language_filter.is_empty() && !language_filter.contains(language_id);
    filtered_out || matches!(family, LanguageFamily::CommonText | LanguageFamily::Sql)
}
