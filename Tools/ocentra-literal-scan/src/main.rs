use std::env;
use std::path::PathBuf;
use std::process;

use ocentra_literal_scan::{run_scan, CliOptions, OutputFormat};

fn main() {
    let opts = match parse_args(env::args().skip(1).collect()) {
        Ok(opts) => opts,
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            process::exit(2);
        }
    };

    if opts.help {
        print_usage();
        return;
    }

    let report = match run_scan(&opts) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("ocentra-literal-scan failed: {error}");
            process::exit(1);
        }
    };

    match opts.output_format {
        OutputFormat::Json => println!("{}", report.to_json_pretty()),
        OutputFormat::JsonLines => {
            for line in report.to_json_lines() {
                println!("{line}");
            }
        }
        OutputFormat::Human => println!("{}", report.to_human()),
    }

    if !report.ok {
        process::exit(1);
    }
}

fn parse_args(args: Vec<String>) -> Result<CliOptions, String> {
    let mut opts = CliOptions::default();
    let mut index = 0;

    if args.get(0).map(String::as_str) == Some("scan") {
        index = 1;
    } else if args.get(0).map(String::as_str) == Some("languages") {
        opts.command = "languages".to_string();
        return Ok(opts);
    } else if args.get(0).map(String::as_str) == Some("explain") {
        opts.command = "explain".to_string();
        opts.explain_category = args.get(1).cloned();
        return Ok(opts);
    }

    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                index += 1;
                let value = args.get(index).ok_or("--root requires a path")?;
                opts.root = PathBuf::from(value);
            }
            "--files" => {
                index += 1;
                while index < args.len() && !args[index].starts_with('-') {
                    opts.files.push(PathBuf::from(&args[index]));
                    index += 1;
                }
                index = index.saturating_sub(1);
            }
            "--json" => opts.output_format = OutputFormat::Json,
            "--jsonl" => opts.output_format = OutputFormat::JsonLines,
            "--human" => opts.output_format = OutputFormat::Human,
            "--include-low" => opts.include_low = true,
            "--include-ignored" => opts.include_ignored = true,
            "--include-unknown-code" => opts.include_unknown_code = true,
            "--no-respect-gitignore" => opts.respect_gitignore = false,
            "--min-score" => {
                index += 1;
                opts.min_score = parse_u8(args.get(index), "--min-score")?;
            }
            "--fail-above" => {
                index += 1;
                opts.fail_above = Some(parse_u8(args.get(index), "--fail-above")?);
            }
            "--max-file-bytes" => {
                index += 1;
                let raw = args.get(index).ok_or("--max-file-bytes requires a number")?;
                opts.max_file_bytes = raw
                    .parse::<u64>()
                    .map_err(|_| "--max-file-bytes must be a positive integer".to_string())?;
            }
            "--languages" => {
                index += 1;
                let raw = args.get(index).ok_or("--languages requires a comma list")?;
                opts.languages = raw
                    .split(',')
                    .map(str::trim)
                    .filter(|entry| !entry.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            "--help" | "-h" => opts.help = true,
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
        index += 1;
    }
    Ok(opts)
}

fn parse_u8(value: Option<&String>, flag: &str) -> Result<u8, String> {
    let raw = value.ok_or_else(|| format!("{flag} requires a number"))?;
    raw.parse::<u8>()
        .map_err(|_| format!("{flag} must be a number from 0 to 100"))
        .and_then(|value| {
            if value <= 100 {
                Ok(value)
            } else {
                Err(format!("{flag} must be between 0 and 100"))
            }
        })
}

fn print_usage() {
    println!(
        r#"ocentra-literal-scan

Usage:
  ocentra-literal-scan scan --root <repo> [options]
  ocentra-literal-scan scan --root <repo> --files <path...> [options]

Options:
  --json                  Print pretty JSON report.
  --jsonl                 Print one JSON object per finding.
  --human                 Print human-readable report.
  --min-score <0-100>     Minimum soft-risk score to include. Default: 40.
  --include-low           Include low-risk findings below min-score.
  --fail-above <0-100>    Turn risks at or above score into blocking hard findings.
  --languages <list>      Comma list of language IDs to scan.
  --include-ignored       Include files normally ignored by defaults/gitignore.
  --include-unknown-code  Use fallback lexer for unknown textual files.
  --no-respect-gitignore  Ignore .gitignore/.ignore rules.
  --max-file-bytes <n>    Skip files larger than this size. Default: 2097152.
"#
    );
}
