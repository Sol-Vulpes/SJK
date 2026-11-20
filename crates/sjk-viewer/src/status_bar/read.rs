//! What the bars show, read from the game four times a second (the clock once a second)
//! into fixed storage: the hub's numbers, the own look and toys, the weapons held, the
//! Force known, the match's facts from the server's info and the players' slots, the
//! ping and the frame rate.

use super::times::{self, info_number, info_value};
use super::{
    CLOCK_EVERY, EVERY, Effect, Inputs, Line, MAX_POWERS, MAX_WEAPONS, Score, TOY_COUNT, TopInputs,
    Weapons, feed, map_key, tactical_words, toys,
};
use crate::menu::sjk::color;
use std::time::Instant;

/// `CS_PLAYERS`, the first player's config string.
const CS_PLAYERS: usize = 1_131;
/// `persistant[PERS_SCORE]` and `persistant[PERS_RANK]`, and `RANK_TIED_FLAG`.
const PERS_SCORE: usize = 0;
const PERS_RANK: usize = 2;
const RANK_TIED: u32 = 0x4000;
/// `GT_TEAM`: the first game type played in teams.
const FIRST_TEAM_GAME: i64 = 6;
/// `WP_SABER`, whose icon follows the saber style.
const WP_SABER: u8 = 3;
/// Ammunition this low reads in ember, as the HUD's own count does.
const LOW_AMMO: u32 = 5;

impl crate::GpuState {
    /// Read what both bars show now into the bars' storage.
    pub(super) fn read_status_bars(&mut self, now: Instant) {
        let mut inputs = self.status_bar_inputs();
        inputs.new = self.status_bar.nudge.follow(
            crate::collection_seen::total(),
            map_key(&self.world_load_map),
        );
        inputs.weapons = self.status_bar_weapons();
        let (clock, date) = match self.status_bar.clock_read {
            Some(at) if now.saturating_duration_since(at) < CLOCK_EVERY => {
                (self.status_bar.inputs.clock, self.status_bar.inputs.date)
            }
            _ => {
                self.status_bar.clock_read = Some(now);
                let (clock, date) = times::split(sjk_shell::local_time::LocalTime::now());
                (Some(clock), Some(date))
            }
        };
        inputs.clock = clock;
        inputs.date = date;
        let mut top = self.status_bar_top_inputs(now);
        top.feed = self.status_bar.top_inputs.feed;
        let state = &mut self.status_bar;
        state.inputs = inputs;
        state.top_inputs = top;
        state.read = Some(now);
    }

    /// Follow who joins and leaves, four times a second while a game is on, shown or not
    /// ([`feed`]); out of one, the next is read afresh.
    pub(super) fn observe_join_feed(&mut self, now: Instant) {
        let Some(session) = self.live_session.as_ref() else {
            self.status_bar.feed.forget();
            self.status_bar.feed_read = None;
            return;
        };
        if self
            .status_bar
            .feed_read
            .is_some_and(|at| now.saturating_duration_since(at) < EVERY)
        {
            return;
        }
        self.status_bar.feed_read = Some(now);
        let game = session.game_state();
        // A new game state (a map change, another server) has a new checksum feed.
        let key = (u64::from(game.checksum_feed as u32) << 32) | u64::from(game.client_num as u32);
        self.status_bar.feed.observe(key, now, |slot| {
            game.config_string(CS_PLAYERS + slot)
                .filter(|info| !info.is_empty())?;
            let (name, _) = crate::hud::client_name_and_team(game, slot as u16);
            Some(feed::Name::from(name.as_ref()))
        });
    }

    /// What the bottom bar shows now: the hub's numbers, the own look and the toys'
    /// switches.
    fn status_bar_inputs(&self) -> Inputs {
        let mut inputs = Inputs::default();
        if let Some((credits, shards, crystals, coins)) =
            crate::player_identity::with_snapshot(|snapshot| {
                let me = snapshot.me.as_ref();
                // How many dormant kybers wait, and the best tier among them.
                let crystals = me.map(|me| {
                    me.holocrons
                        .iter()
                        .filter(|holocron| holocron.opened.is_none())
                        .filter_map(|holocron| crate::kyber::from_id(&holocron.tier))
                        .fold((0_u32, 0_usize), |(count, best), tier| {
                            (count + 1, best.max(tier.index))
                        })
                });
                (
                    me.map(|me| me.credits),
                    me.map(|me| me.shards),
                    crystals,
                    snapshot.wheel_of_doom.as_ref().map(|wheel| wheel.coins),
                )
            })
        {
            inputs.credits = credits;
            inputs.shards = shards;
            if let Some((count, best)) = crystals {
                inputs.crystals = Some(count);
                inputs.crystal_tier = best;
            }
            inputs.coins = coins;
        }
        let own = self.looks.own();
        inputs.saber = own.saber_skin;
        if let crate::looks::SecondSkin::Own(second) = own.second
            && second != own.saber_skin
        {
            inputs.second = Some(second);
        }
        // The blade's extras switched on (`blade_extras.rs`): a word of the options names
        // an extra on.
        if let Some(extras) = own.saber_skin.and_then(crate::blade_extras::of) {
            for extra in &extras.list {
                if own.options_off & crate::blade_skin_file::extras::option_bit(&extra.id) != 0 {
                    inputs.push_effect(Effect {
                        words: extra.name.as_str(),
                        colour: color::gold_bright(),
                    });
                }
            }
        }
        if let Some(finisher) = own.finisher.and_then(crate::finishers::get) {
            inputs.push_effect(Effect {
                words: finisher.name,
                colour: finisher.tier.colour(),
            });
        }
        let console = self.console.as_ref();
        let on = |cvar: &str| {
            console
                .and_then(|console| console.bool_cvar(cvar))
                .unwrap_or(false)
        };
        if let Some(words) = tactical_words(crate::tactical::level()) {
            inputs.push_effect(Effect {
                words,
                colour: color::holo(),
            });
        }
        if on(crate::chat::quiet::CVAR) {
            inputs.push_effect(Effect {
                words: "Do not disturb",
                colour: color::ember(),
            });
        }
        if crate::away_emote::own_away() {
            inputs.push_effect(Effect {
                words: "Away",
                colour: color::muted(),
            });
        }
        if on(crate::toon_choice::CVAR) {
            inputs.push_effect(Effect {
                words: "Toon",
                colour: color::holo(),
            });
        }
        let mut switches = [false; TOY_COUNT];
        switches[toys::ILLUMINATE] = self.illuminate.lit();
        switches[toys::CARDBOARD_BOX] = self.cardboard_box.on();
        switches[toys::ORDER66] = self.order66.on();
        switches[toys::HAX] = self.hax.on();
        switches[toys::RAGDOLL] = self.ragdoll.on();
        switches[toys::DRONE] = self.drone.on();
        switches[toys::BOOMBOX] = self.boombox.on();
        switches[toys::MONITOR] = self.monitor.on();
        switches[toys::BOX_HUNT] = self.box_hunt.on();
        switches[toys::QUESTS] = self.quests.on();
        switches[toys::PROPGUN] = self.propgun.on();
        switches[toys::FISHING] = self.fishing.on();
        switches[toys::WHEEL_OF_DOOM] = self.wheel_of_doom.running();
        switches[toys::MINING] = self.mining.on();
        inputs.toys = switches;
        inputs
    }

    /// The weapons the player holds (the followed player's while following, as the HUD
    /// shows them), by their selection icons in the wheel's order, the one in hand lit,
    /// and its ammunition; none while spectating freely.
    fn status_bar_weapons(&self) -> Weapons {
        let mut weapons = Weapons::default();
        let Some(session) = self.live_session.as_ref() else {
            return weapons;
        };
        let player = &session.latest_snapshot().player;
        let inventory = sjk_client::LegacyWeaponInventory::from_player_state(player);
        if inventory.spectator {
            return weapons;
        }
        let current = self
            .selected_weapon
            .filter(|_| !inventory.following)
            .unwrap_or_else(|| player.weapon());
        let (order, count) = crate::weapon_select::held_in_order(&inventory);
        let style = player.saber_draw_style();
        let mut slot = 0;
        for &weapon in &order[..count.min(MAX_WEAPONS)] {
            let empty = !crate::weapon_select::has_ammo(&inventory, weapon);
            let style = if weapon == WP_SABER { style } else { 0 };
            if let Some(texture) = self.hud.icons.weapon_select(weapon, empty, style) {
                weapons.held[slot] = Some((texture, weapon == current));
                slot += 1;
            }
        }
        // `weaponData[].ammoIndex` names a pool for the weapons that fire it.
        let takes_ammo = |weapon: u8| matches!(weapon, 4..=16);
        if order[..count].iter().any(|weapon| takes_ammo(*weapon)) {
            let value = takes_ammo(current)
                .then(|| sjk_client::legacy_weapon_data(current))
                .flatten()
                .and_then(|data| inventory.ammo.get(data.ammo_index).copied());
            weapons.ammo = Some((value, value.is_some_and(|value| value <= LOW_AMMO)));
        }
        weapons
    }

    /// What the top bar shows at `now` (the feed apart): the Force known, the player's
    /// standing, the time left, the server's name, map, game type and players, the ping
    /// and the frame rate.
    fn status_bar_top_inputs(&self, now: Instant) -> TopInputs {
        let mut top = TopInputs {
            fps: self.frame_pacer.fps(),
            ..TopInputs::default()
        };
        let Some(session) = self.live_session.as_ref() else {
            return top;
        };
        let game = session.game_state();
        let snapshot = session.latest_snapshot();
        let player = &snapshot.player;
        if let Some(powers) = self.force_powers() {
            let (slots, count) =
                sjk_client::force_wheel::build_with(powers.known, powers.extensions);
            let active = player.force_powers_active();
            let mut index = 0;
            for &slot in &slots[..count] {
                let Some(texture) =
                    crate::hud::force_wheel::icon(&powers.icons, slot, powers.flamethrower)
                else {
                    continue;
                };
                if index < MAX_POWERS {
                    let in_use = slot < 32 && active & (1 << slot) != 0;
                    top.powers[index] = Some((texture, slot == powers.selected, in_use));
                    index += 1;
                }
            }
        }
        let info = game.config_string(0).unwrap_or_default();
        let gametype = info_number(info, b"g_gametype");
        let own = crate::ingame_menu::OwnSide::of(game, player);
        if !own.watching {
            top.score = Some(
                if gametype.is_some_and(|gametype| gametype >= FIRST_TEAM_GAME)
                    && matches!(own.team, 1 | 2)
                {
                    let [red, blue] = session.team_scores();
                    Score::Teams {
                        red,
                        blue,
                        team: own.team,
                    }
                } else {
                    let rank = player.persistent[PERS_RANK];
                    Score::Place {
                        place: (rank & !RANK_TIED).saturating_add(1),
                        tied: rank & RANK_TIED != 0,
                        score: player.persistent[PERS_SCORE] as i32,
                    }
                },
            );
        }
        top.left = times::left(game, snapshot.server_time);
        if let Some(name) = info_value(info, b"sv_hostname") {
            top.server = Line::from(sjk_client::decode_legacy(name).trim());
        }
        if let Some(map) = info_value(info, b"mapname") {
            let map = sjk_client::decode_legacy(map);
            let map = map.rsplit(['/', '\\']).next().unwrap_or(&map);
            top.map = Line::from(map);
        }
        top.mode = gametype
            .map(|gametype| crate::server_browser::gametype_name(i32::try_from(gametype).ok()))
            .filter(|name| *name != "Unknown");
        let count = (0..32)
            .filter(|slot| {
                game.config_string(CS_PLAYERS + slot)
                    .is_some_and(|info| !info.is_empty())
            })
            .count();
        let slots = info_number(info, b"sv_maxclients").unwrap_or(0);
        if slots > 0 {
            top.players = Some((count as u8, u16::try_from(slots).unwrap_or(u16::MAX)));
        }
        // The server's measure, as the scoreboard shows it ([`super::ping`]); none in a
        // local game, which has no network.
        if !session.is_local() {
            top.ping = self.status_bar.ping.shown(now);
        }
        top
    }
}
