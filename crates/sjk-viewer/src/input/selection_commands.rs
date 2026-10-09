//! Completion/help metadata for locally dispatched selection commands.

/// Register metadata; execution is intercepted by GameplayInput before shell fallback.
pub(super) fn register(shell: &mut sjk_shell::Shell) -> Result<(), sjk_shell::CommandError> {
    for (name, help) in [
        (
            "forcenext",
            "Select next usable Force power (Use held: inventory)",
        ),
        (
            "forceprev",
            "Select previous usable Force power (Use held: inventory)",
        ),
        (
            "forceselect",
            "Select Force wheel entry N (forcePowers_t number; 18-20 JoF's, 21 Illuminate)",
        ),
        ("invnext", "Select next inventory item"),
        ("invprev", "Select previous inventory item"),
        (
            "force_illuminate",
            "Turn Illuminate's holocron light on or off (cg_illuminate)",
        ),
    ] {
        shell.commands.register(name, help, |_| {
            Err(sjk_shell::CommandError::Handler(
                "Viewer input dispatcher required".into(),
            ))
        })?;
    }
    Ok(())
}
