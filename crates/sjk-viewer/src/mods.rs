//! The client mods built into this client (`docs/mods.md`): their switches
//! (`mod_<id>`), their saved settings, and their commands, which exist only while
//! a mod is on. A mod's commands run on the main thread through [`host::ViewerHost`],
//! after the console has queued them like the other client commands.

use sjk_mod::{ClientMod, Server};
use sjk_shell::{CvarDefinition, CvarFlags, CvarValue, Shell};

#[path = "mods_host.rs"]
mod host;

/// The mods this build carries, in menu order.
// Each mod is pushed under its own feature, which `vec![]` cannot express.
#[allow(clippy::vec_init_then_push)]
fn builtin() -> Vec<Box<dyn ClientMod>> {
    #[cfg_attr(
        not(any(feature = "mod-japlus", feature = "mod-jof")),
        allow(unused_mut)
    )]
    let mut list: Vec<Box<dyn ClientMod>> = Vec::new();
    #[cfg(feature = "mod-japlus")]
    list.push(Box::new(sjk_mod_japlus::JaPlus::new()));
    #[cfg(feature = "mod-jof")]
    list.push(Box::new(sjk_mod_jof::JoF::new()));
    list
}

/// The setting that switches mod `id` on.
pub(crate) fn switch(id: &str) -> String {
    format!("mod_{id}")
}

struct Entry {
    id: &'static str,
    /// Lent out while one of its commands runs.
    module: Option<Box<dyn ClientMod>>,
    /// Whether its commands are registered.
    on: bool,
}

/// The built-in mods and which are on.
pub(crate) struct Mods {
    entries: Vec<Entry>,
    /// The server the client is on, for the server commands mods complete.
    server: Option<Server>,
}

impl Mods {
    pub(crate) fn new() -> Self {
        Self {
            entries: builtin()
                .into_iter()
                .map(|module| Entry {
                    id: module.id(),
                    module: Some(module),
                    on: false,
                })
                .collect(),
            server: None,
        }
    }

    /// Register every mod's switch and settings; settings are kept while a mod
    /// is off so its saved values survive.
    pub(crate) fn register(&self, shell: &mut Shell) -> Result<(), sjk_shell::CvarError> {
        for module in self
            .entries
            .iter()
            .filter_map(|entry| entry.module.as_ref())
        {
            shell.cvars.register(CvarDefinition::new(
                switch(module.id()),
                false,
                CvarFlags::ARCHIVE,
                format!("Turn on {}: {}", module.title(), module.about()),
            ))?;
            for setting in module.settings() {
                shell.cvars.register(CvarDefinition::new(
                    setting.name,
                    setting.default,
                    CvarFlags::ARCHIVE,
                    setting.help,
                ))?;
            }
        }
        Ok(())
    }

    /// Follow the switches: register the commands of mods turned on and remove
    /// those of mods turned off. Returns whether one changed.
    pub(crate) fn sync(&mut self, shell: &mut Shell) -> bool {
        let mut changed = false;
        for entry in &mut self.entries {
            let Some(module) = entry.module.as_ref() else {
                continue;
            };
            let on = shell
                .cvars
                .get(&switch(entry.id))
                .is_some_and(|cvar| matches!(cvar.value, CvarValue::Bool(true)));
            if on == entry.on {
                continue;
            }
            for command in module.commands() {
                if on {
                    let _ = shell.commands.register(command.name, command.help, |_| {
                        Err(sjk_shell::CommandError::Handler(
                            "Viewer command dispatcher required".into(),
                        ))
                    });
                } else {
                    shell.commands.unregister(command.name);
                }
            }
            entry.on = on;
            changed = true;
        }
        changed
    }

    /// The mod a command belongs to, by its `<id>.` prefix.
    fn owner(&self, name: &str) -> Option<usize> {
        let (prefix, _) = name.split_once('.')?;
        self.entries
            .iter()
            .position(|entry| entry.id.eq_ignore_ascii_case(prefix))
    }

    /// For the console: `None` when `name` is no mod's, else whether its mod
    /// takes it now, or why not.
    pub(crate) fn accepts(&self, name: &str) -> Option<Result<(), String>> {
        let entry = &self.entries[self.owner(name)?];
        let known = entry.module.as_ref().is_none_or(|module| {
            module
                .commands()
                .iter()
                .any(|command| command.name.eq_ignore_ascii_case(name))
        });
        Some(if !known {
            Err(format!("Unknown command \"{name}\""))
        } else if !entry.on {
            Err(format!(
                "{name}: this mod is off; turn it on in Settings or with {} 1",
                switch(entry.id)
            ))
        } else {
            Ok(())
        })
    }

    /// Remember the server the client joined (`None` when it left).
    pub(crate) fn set_server(&mut self, server: Option<Server>) {
        self.server = server;
    }

    /// The server commands the mods that are on complete on the current server.
    pub(crate) fn server_help(&self) -> Vec<(String, String)> {
        let Some(server) = &self.server else {
            return Vec::new();
        };
        self.entries
            .iter()
            .filter(|entry| entry.on)
            .filter_map(|entry| entry.module.as_ref())
            .flat_map(|module| module.server_commands(server))
            .map(|command| (command.name.to_owned(), command.help.to_owned()))
            .collect()
    }

    /// Each mod's id, title and description, with whether it is on.
    pub(crate) fn listing(&self) -> Vec<(&'static str, &'static str, &'static str, bool)> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let module = entry.module.as_ref()?;
                Some((entry.id, module.title(), module.about(), entry.on))
            })
            .collect()
    }

    fn lend(&mut self, index: usize) -> Option<Box<dyn ClientMod>> {
        self.entries.get_mut(index)?.module.take()
    }

    fn give_back(&mut self, index: usize, module: Box<dyn ClientMod>) {
        self.entries[index].module = Some(module);
    }
}

/// `sjk_client`'s server profile as the mods see it.
pub(crate) fn server_kind(profile: &sjk_client::CompatProfile) -> sjk_mod::ServerKind {
    match profile {
        sjk_client::CompatProfile::BaseJka => sjk_mod::ServerKind::BaseJka,
        sjk_client::CompatProfile::JaPlus { .. } => sjk_mod::ServerKind::JaPlus,
        sjk_client::CompatProfile::TaystJk => sjk_mod::ServerKind::JaPro,
        sjk_client::CompatProfile::Unknown(_) => sjk_mod::ServerKind::Other,
    }
}

impl crate::GpuState {
    /// Run a queued mod command; `None` when `tokens` is no mod's.
    pub(crate) fn run_mod_command(
        &mut self,
        tokens: &[String],
    ) -> Option<Result<Vec<String>, String>> {
        let name = tokens.first()?.to_ascii_lowercase();
        let console = self.console.as_mut()?;
        let index = console.mods.owner(&name)?;
        if let Some(Err(error)) = console.mods.accepts(&name) {
            return Some(Err(error));
        }
        let mut module = console.mods.lend(index)?;
        let result = module.run(&name, &tokens[1..], &mut host::ViewerHost::new(self));
        if let Some(console) = self.console.as_mut() {
            console.mods.give_back(index, module);
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> (Shell, Mods) {
        let mut shell = Shell::new(sjk_shell::CvarRegistry::new(), Default::default());
        let mods = Mods::new();
        mods.register(&mut shell).unwrap();
        (shell, mods)
    }

    #[test]
    fn commands_follow_the_switch() {
        let (mut shell, mut mods) = shell();
        assert!(!mods.sync(&mut shell));
        assert!(!shell.commands.contains("japlus.guntele"));
        assert!(matches!(mods.accepts("japlus.guntele"), Some(Err(_))));
        assert_eq!(mods.accepts("connect"), None);
        shell.cvars.set_text("mod_japlus", "1").unwrap();
        assert!(mods.sync(&mut shell));
        assert!(shell.commands.contains("japlus.guntele"));
        assert_eq!(mods.accepts("JAPLUS.GunTele"), Some(Ok(())));
        assert!(matches!(mods.accepts("japlus.nothing"), Some(Err(_))));
        // The other mod stays off.
        assert!(!shell.commands.contains("jof.commands"));
        shell.cvars.set_text("mod_japlus", "0").unwrap();
        assert!(mods.sync(&mut shell));
        assert!(!shell.commands.contains("japlus.guntele"));
    }

    #[test]
    fn settings_exist_while_the_mod_is_off() {
        let (shell, _) = shell();
        assert!(shell.cvars.get("japlus.loginServer1").is_some());
        assert!(shell.cvars.get("mod_jof").is_some());
    }

    #[test]
    fn server_commands_complete_only_while_on() {
        let (mut shell, mut mods) = shell();
        mods.set_server(Some(Server {
            kind: sjk_mod::ServerKind::JaPlus,
            address: "127.0.0.1:29070".into(),
            info: r"\V\2.5B0".into(),
        }));
        assert!(mods.server_help().is_empty());
        shell.cvars.set_text("mod_japlus", "1").unwrap();
        mods.sync(&mut shell);
        let help = mods.server_help();
        assert!(help.iter().any(|(name, _)| name == "amtele"));
        assert!(!help.iter().any(|(name, _)| name == "gunkick"));
        shell.cvars.set_text("mod_jof", "1").unwrap();
        mods.sync(&mut shell);
        assert!(mods.server_help().iter().any(|(name, _)| name == "gunkick"));
    }
}
