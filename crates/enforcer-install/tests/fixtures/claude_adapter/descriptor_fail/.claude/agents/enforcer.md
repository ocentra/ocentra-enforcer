This is a hand-corrupted descriptor: no frontmatter fence at all, so
`ClaudeAdapter::validate_agent_descriptor` must reject it and `verify`
must report `passed: false` for the `agent-descriptor-present` check —
never a silent skip.
