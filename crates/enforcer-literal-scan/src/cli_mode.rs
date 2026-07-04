use enforcer_literal_scan::CliOptions;

pub(crate) fn consume_mode(args: &[String], opts: &mut CliOptions) -> usize {
    if args.first().map(String::as_str) == Some("scan") {
        return 1;
    }
    if args.first().map(String::as_str) == Some("languages") {
        opts.command = "languages".to_string();
        return args.len();
    }
    if args.first().map(String::as_str) == Some("explain") {
        opts.command = "explain".to_string();
        opts.explain_category = args.get(1).cloned();
        return args.len();
    }
    0
}
