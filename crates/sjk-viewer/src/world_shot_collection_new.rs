//! World shots of New on every kind and the empty states (15/11/2026,
//! `crate::collection_seen`, [`crate::console::collection_panel::empty_state`]): the hall
//! with its "N new" pills (two kybers, a card, a tag, a finisher), the Cards tab with a New
//! card and Mark all seen, the Tags tab and the Finishers page with theirs, then the Cards
//! tab with no card held (holocrons waiting, then none) and an Inventory search nothing
//! matches. On the made-up profile of [`super::preview`], the packs `SJK_TEST_PACKS` names.

use super::*;

/// The New things the shots show, besides the kybers' own marks.
fn news(tag: &str) -> crate::collection_seen::News {
    use crate::collection_seen::Kind;
    crate::collection_seen::News::shown(&[
        (Kind::Card, "sjk_alpha_002", 1),
        (Kind::Tag, tag, 2),
        (Kind::Finisher, "finisher_disintegrate", 0),
    ])
}

/// The cards held in the shots with cards: a few, one of them New.
fn stacks() -> Vec<sjk_identity::CardStack> {
    let stack = |card: &str, rarity: &str, finish: &str| sjk_identity::CardStack {
        card: Some(card.to_owned()),
        rarity: rarity.to_owned(),
        finish: finish.to_owned(),
        edition: "alpha".to_owned(),
        count: 1,
        first: 1_791_641_100,
        last: 1_791_641_100,
    };
    vec![
        stack("sjk_alpha_000", "mythical", "normal"),
        stack("sjk_alpha_001", "legendary", "normal"),
        stack("sjk_alpha_002", "legendary", "holo"),
        stack("sjk_alpha_010", "rare", "normal"),
    ]
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_collection_new() {
    on_big_stack(|| {
        super::super::saber_skins::mount_test_packs();
        let tag = crate::tags::CATALOGUE
            .iter()
            .find(|tag| !tag.free)
            .expect("a dropped tag");
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        gpu.looks.shot_owns_unlocks = true;
        let _ = frame(&mut gpu, 10);
        let unlocks: Vec<sjk_identity::Unlock> = [
            "saber_sun",
            "saber_void",
            "saber_storm",
            "finisher_disintegrate",
            tag.id,
        ]
        .iter()
        .map(|id| sjk_identity::Unlock {
            id: (*id).to_owned(),
            solar: u32::from(*id == tag.id),
            count: 1,
            ..sun()
        })
        .collect();
        let profile = |waiting: u32| {
            let mut preview = preview();
            if let Some(me) = preview.snapshot.me.as_mut() {
                me.credits = 12_340;
                me.shards = 865;
                me.holocron_counts = sjk_identity::HolocronCounts {
                    uncommon: waiting,
                    rare: 0,
                    legendary: 0,
                    mythical: 0,
                };
            }
            preview
        };
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(profile(4));
            console.preview_collection(Some(unlocks), 0.8);
            console.preview_cards(stacks(), 12_340, 865);
            console.collection_marks_for_shot(Some(crate::shader_marks::Marks::shown(
                &[],
                &[],
                &["saber_void", "saber_storm"],
                Some("saber_sun"),
            )));
            console.collection_news_for_shot(Some(news(tag.id)));
            console.open_profile_hub_page(Tab::Hall, ReturnTarget::MainMenu);
        }
        for _ in 0..240 {
            let _ = frame(&mut gpu, 1);
            if crate::medals::art::decoded().is_some() {
                break;
            }
        }
        gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
        println!(
            "{}",
            shoot(&mut gpu, 150, "duel6-collection-new-hall").display()
        );
        assert!(!overflowed(&gpu), "the hall");
        let pages: [(Tab, bool, &str); 3] = [
            (Tab::Cards, false, "duel6-collection-new-cards"),
            (Tab::Tags, false, "duel6-collection-new-tags"),
            (Tab::Shaders, true, "duel6-collection-new-finishers"),
        ];
        for (tab, finishers, name) in pages {
            if let Some(console) = gpu.console.as_mut() {
                console.open_profile_hub_page(tab, ReturnTarget::MainMenu);
                if tab == Tab::Cards {
                    console.collection_card_filter_for_shot(crate::cards::binder::Filter {
                        show: crate::cards::binder::Show::Owned,
                        ..Default::default()
                    });
                    console.collection_card_for_shot(1);
                }
                if tab == Tab::Tags {
                    console.collection_tag_for_shot(tag.id, 2);
                }
                if finishers {
                    console.preview_finishers();
                }
            }
            println!("{}", shoot(&mut gpu, 60, name).display());
            assert!(!overflowed(&gpu), "{name}");
        }
        // No card held: Open holocrons while some wait, else Find a server.
        for (waiting, name) in [
            (4, "duel6-collection-empty-cards"),
            (0, "duel6-collection-empty-cards-none-waiting"),
        ] {
            if let Some(console) = gpu.console.as_mut() {
                console.preview_profile(profile(waiting));
                console.preview_cards(Vec::new(), 12_340, 865);
                console.collection_card_filter_for_shot(Default::default());
                console.open_profile_hub_page(Tab::Cards, ReturnTarget::MainMenu);
            }
            println!("{}", shoot(&mut gpu, 60, name).display());
            assert!(!overflowed(&gpu), "{name}");
        }
        // A search nothing owned matches.
        if let Some(console) = gpu.console.as_mut() {
            console.open_profile_hub_page(Tab::Inventory, ReturnTarget::MainMenu);
            console.inventory_for_shot("cranes", 0);
        }
        println!(
            "{}",
            shoot(&mut gpu, 60, "duel6-collection-empty-inventory-search").display()
        );
        assert!(!overflowed(&gpu), "the Inventory");
    });
}
