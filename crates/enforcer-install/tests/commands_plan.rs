use enforcer_install::commands::plan::{
    dispatches_via_real_binary, render_claude_command, render_command_body, render_generic_command,
};

#[test]
fn rendered_plan_commands_keep_each_required_dispatch_once_and_in_order() -> Result<(), String> {
    let body = render_command_body();
    let dispatches = [
        "enforcer plan new <name>",
        "enforcer plan new <name> --force",
        "enforcer plan check",
    ];

    let mut cursor = 0;
    for dispatch in dispatches {
        let remaining = body
            .get(cursor..)
            .ok_or_else(|| "plan command cursor exceeded rendered body".to_owned())?;
        let found_at = remaining
            .find(dispatch)
            .ok_or_else(|| format!("plan command body is missing required dispatch: {dispatch}"))?;
        cursor += found_at + dispatch.len();
    }

    assert!(dispatches_via_real_binary(&body));
    assert!(dispatches_via_real_binary(&render_claude_command()));
    assert!(dispatches_via_real_binary(&render_generic_command()));
    Ok(())
}

#[test]
fn incomplete_or_empty_command_text_cannot_satisfy_real_binary_dispatch_check() {
    assert!(!dispatches_via_real_binary(""));
    assert!(!dispatches_via_real_binary(
        "enforcer plan new <name>\nenforcer plan check"
    ));
}
