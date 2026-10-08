//! The real HTTP client against a running hub. Ignored by default: start
//! `sjk-hub serve` on this machine and run
//!
//! ```text
//! SJK_HUB_TEST_URL=http://127.0.0.1:8787 cargo test -p sjk-identity --test hub_e2e -- --ignored
//! ```
//!
//! Every run makes fresh keys and a fresh name, so it can repeat against one
//! database. The hub allows 5 registrations an hour per address and a whole run
//! makes more: run a few tests at a time (name them after `--ignored`), restarting
//! the hub in between (its limits live in memory).

use sjk_identity::{HttpHub, Hub, HubError, Identity};

fn hub() -> HttpHub {
    let url = std::env::var("SJK_HUB_TEST_URL").expect("SJK_HUB_TEST_URL");
    HttpHub::new(&url, "sjk-identity-test").unwrap()
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn a_player_registers_with_the_name_they_wear_claims_and_leaves() {
    let mut hub = hub();
    let me = Identity::generate().unwrap();
    let other = Identity::generate().unwrap();
    let suffix = &me.key_id()[..8];
    let name = format!("^1Test{suffix}");

    let profile = hub.register(&me, Some(&name)).unwrap();
    assert_eq!(profile.key_id, me.key_id());
    assert!(!profile.verified);
    assert_eq!(profile.name, name, "the worn name is the display name");
    assert_eq!(profile.names[0].name, name);
    let again = hub.register(&me, None).unwrap();
    assert_eq!(
        (again.created, again.name.as_str()),
        (profile.created, name.as_str()),
        "registering again keeps the profile"
    );

    let saved = hub.set_bio(&me, "line one\nline two").unwrap();
    assert_eq!(saved.name, name, "a bio change keeps the name");
    assert_eq!(hub.profile(&me.key_id()).unwrap().bio, "line one\nline two");

    // Two keys may wear the same name: the name proves nothing, the key does.
    hub.register(&other, Some(&name)).unwrap();

    let server = format!("10.99.{}.{}:29070", &suffix[..2].len(), 7);
    hub.claim(&me, &server, 3, "^1Test").unwrap();
    let players = hub.presence(&server).unwrap();
    assert_eq!(players.len(), 1);
    assert_eq!(
        (players[0].slot, players[0].key_id.as_str()),
        (3, me.key_id().as_str())
    );
    assert_eq!(players[0].claimed_name, "^1Test");

    let taken = hub.claim(&other, &server, 3, "^1Test").unwrap_err();
    assert!(
        matches!(taken, HubError::Rejected { ref code, .. } if code == "slot_taken"),
        "{taken:?}"
    );

    hub.release(&me, &server).unwrap();
    assert!(hub.presence(&server).unwrap().is_empty());
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn an_ipv6_server_address_round_trips() {
    let mut hub = hub();
    let me = Identity::generate().unwrap();
    hub.register(&me, None).unwrap();
    hub.claim(&me, "[2001:db8::7]:29070", 1, "v6").unwrap();
    assert_eq!(hub.presence("[2001:db8::7]:29070").unwrap().len(), 1);
    hub.release(&me, "[2001:db8::7]:29070").unwrap();
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn a_missing_player_is_a_rejection_not_a_crash() {
    let mut hub = hub();
    let error = hub.profile("0000000000000000").unwrap_err();
    assert!(
        matches!(error, HubError::Rejected { status: 404, .. }),
        "{error:?}"
    );
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn the_service_registers_claims_tags_and_releases_on_shutdown() {
    use sjk_identity::{Location, Service, Settings, Status};
    use std::time::{Duration, Instant};

    let url = std::env::var("SJK_HUB_TEST_URL").expect("SJK_HUB_TEST_URL");
    let make: sjk_identity::HubFactory = Box::new(|url| {
        HttpHub::new(url, "sjk-identity-test").map(|hub| Box::new(hub) as Box<dyn Hub>)
    });
    let me = Identity::generate().unwrap();
    let key_id = me.key_id();
    let service = Service::start(me, make);
    service.configure(Settings {
        enabled: true,
        hub_url: url.clone(),
    });
    let server: std::net::SocketAddr = "10.98.0.7:29070".parse().unwrap();
    service.enter(Location {
        server,
        slot: 5,
        name: "^2Svc".to_owned(),
    });

    let wait_for = |what: &str, done: &dyn Fn(&sjk_identity::Snapshot) -> bool| {
        let until = Instant::now() + Duration::from_secs(10);
        while Instant::now() < until {
            if service.with_snapshot(|snapshot| done(snapshot)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("timed out waiting for {what}: {:?}", service.snapshot());
    };
    wait_for("online", &|s| s.status == Status::Online);
    wait_for("itself in the roster", &|s| {
        s.badge(5, "svc")
            .is_some_and(|player| player.key_id == key_id)
    });
    assert!(service.snapshot().badge(5, "Someone else").is_none());
    assert!(service.snapshot().badge(6, "Svc").is_none());

    // Another client sees the claim at the hub until the service shuts down.
    let mut watcher = HttpHub::new(&url, "sjk-identity-test").unwrap();
    assert_eq!(watcher.presence(&server.to_string()).unwrap().len(), 1);
    service.shutdown(Duration::from_secs(5));
    assert!(watcher.presence(&server.to_string()).unwrap().is_empty());
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn a_world_note_and_its_picture_reach_the_hub() {
    let mut hub = hub();
    let me = Identity::generate().unwrap();
    hub.register(&me, Some("NoteTester")).unwrap();
    let note = sjk_identity::WorldNote {
        text: format!("too shiny {}", &me.key_id()[..6]),
        map: "maps/mp/ffa1.bsp".to_owned(),
        build: "test".to_owned(),
        view: Some([2807.0, 726.0, 872.0, 283.0]),
        hit: Some([2900.0, 700.0, 860.5]),
        normal: Some([0.0, 0.0, 1.0]),
        shader: "textures/vjun/newfloor_vjun".to_owned(),
        surface: Some(943),
        lighting: "lightmapped".to_owned(),
        distance: Some(252.0),
        name: "^1NoteTester".to_owned(),
        ..sjk_identity::WorldNote::default()
    };
    let id = hub.note(&me, &note).unwrap();
    assert!(id > 0);
    // The markers of a minimal 2 x 2 JPEG: the hub checks the frame, not the pixels.
    let jpeg = [
        0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x02, 0x00, 0x02, 0x01, 0x01, 0x11, 0x00,
        0xFF, 0xDA, 0x00, 0x02, 0xFF, 0xD9,
    ];
    hub.note_image(&me, id, &jpeg).unwrap();
    let again = hub.note_image(&me, id, &jpeg).unwrap_err();
    assert!(
        matches!(again, HubError::Rejected { ref code, .. } if code == "image_taken"),
        "{again:?}"
    );
}

/// The operator key the hub under test accepts (`--admin-keys`), if the run gives one.
fn operator_key() -> Option<String> {
    std::env::var("SJK_HUB_TEST_ADMIN_KEY").ok()
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL; SJK_HUB_TEST_ADMIN_KEY for the verified half)"]
fn only_a_verified_player_on_the_server_reports_another() {
    use sjk_identity::{Category, PlayerReport};
    let url = std::env::var("SJK_HUB_TEST_URL").expect("SJK_HUB_TEST_URL");
    let mut hub = hub();
    let me = Identity::generate().unwrap();
    let troll = Identity::generate().unwrap();
    let suffix = &troll.key_id()[..6];
    let server = "10.98.0.5:29070";
    hub.register(&me, Some("^2Reporter")).unwrap();
    hub.register(&troll, None).unwrap();
    hub.claim(&me, server, 2, "^2Reporter").unwrap();
    let shown = format!("^1Troll{suffix}");
    hub.claim(&troll, server, 5, &shown).unwrap();
    let report = PlayerReport {
        category: Category::Cheating,
        text: "Speed hacking all round the map".to_owned(),
        server: server.to_owned(),
        server_name: "^4Test ^7server".to_owned(),
        slot: 5,
        target_name: shown.clone(),
        target_key_id: troll.key_id(),
        map: "maps/mp/ffa3.bsp".to_owned(),
        build: "e2e".to_owned(),
        level_time: Some(754),
        name: "^2Reporter".to_owned(),
    };
    let refused = hub.player_report(&me, &report).unwrap_err();
    assert!(
        matches!(refused, HubError::Rejected { ref code, .. } if code == "not_verified"),
        "{refused:?}"
    );
    let Some(admin) = operator_key() else {
        eprintln!("no SJK_HUB_TEST_ADMIN_KEY: the verified half is skipped");
        return;
    };
    let bearer = format!("Bearer {admin}");
    let verified = ureq::patch(&format!("{url}/admin/v1/identities/{}", me.key_id()))
        .header("Authorization", &bearer)
        .header("Content-Type", "application/json")
        .send(r#"{"verified":true}"#)
        .unwrap();
    assert_eq!(verified.status().as_u16(), 200);
    let id = hub.player_report(&me, &report).unwrap();
    assert!(id > 0);
    let again = hub.player_report(&me, &report).unwrap_err();
    assert!(
        matches!(again, HubError::Rejected { ref code, .. } if code == "player_report_duplicate"),
        "{again:?}"
    );
    let mut listed = ureq::get(&format!(
        "{url}/admin/v1/player-reports?target={}",
        troll.key_id()
    ))
    .header("Authorization", &bearer)
    .call()
    .unwrap();
    let list: serde_json::Value =
        serde_json::from_str(&listed.body_mut().read_to_string().unwrap()).unwrap();
    let row = &list["player_reports"][0];
    assert_eq!(row["id"], id);
    assert_eq!(row["category"], "cheating");
    assert_eq!(row["target_name"], shown.as_str());
    assert_eq!(row["target_key_id"], troll.key_id().as_str());
    assert_eq!(row["target_source"], "claim");
    assert_eq!(row["server_name"], "^4Test ^7server");
    assert_eq!(row["level_time"], 754);
    assert_eq!(row["worn"], "^2Reporter");
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn a_bio_keeps_to_the_rules_and_achievements_rise_within_their_allowance() {
    let mut hub = hub();
    let me = Identity::generate().unwrap();
    hub.register(&me, Some("^2Achiever")).unwrap();

    // The hub tidies a bio as the client does, and refuses one breaking a rule.
    let saved = hub
        .set_bio(&me, "  two   spaces \r\n\r\n\r\nnext ")
        .unwrap();
    assert_eq!(saved.bio, "two spaces\n\nnext");
    for (bio, code) in [
        ("emoji \u{1F600}", "bio_characters"),
        ("right\u{202E}left", "bio_characters"),
        ("1\n2\n3\n4\n5\n6\n7", "bio_lines"),
        ("aaaaaaaaaaaa", "bio_noise"),
    ] {
        match hub.set_bio(&me, bio).unwrap_err() {
            HubError::Rejected { code: got, .. } => assert_eq!(got, code, "{bio:?}"),
            other => panic!("{bio:?}: {other:?}"),
        }
    }
    // A non-empty bio is the hub's own Storyteller.
    let profile = hub.profile(&me.key_id()).unwrap();
    let storyteller = profile
        .achievements
        .iter()
        .find(|achievement| achievement.id == "storyteller")
        .expect("storyteller");
    assert!(storyteller.unlocked > 0);

    // Counts rise up to their goal and hourly allowance; the hub's own and unknown
    // ids are ignored.
    let progress: std::collections::BTreeMap<String, u64> = [
        ("first_blood", 5),
        ("kills_100", 400),
        ("streak_5", 3),
        ("decorated", 1),
        ("from_the_future", 9),
    ]
    .into_iter()
    .map(|(id, n)| (id.to_owned(), n))
    .collect();
    let held = hub.set_achievements(&me, &progress).unwrap();
    let of = |id: &str| held.iter().find(|a| a.id == id).cloned();
    let first = of("first_blood").expect("first_blood");
    assert_eq!((first.progress, first.goal), (1, 1));
    assert!(first.unlocked > 0);
    let kills = of("kills_100").expect("kills_100");
    assert_eq!(
        kills.progress, 100,
        "capped at the goal (the allowance is 150)"
    );
    assert_eq!(of("streak_5").map(|a| a.progress), Some(3));
    assert!(of("decorated").is_none(), "the hub counts its own");
    assert!(of("from_the_future").is_none());
    let thousand = of("kills_1000");
    assert!(thousand.is_none(), "not sent, not counted");
    // A thousand at once is held to the hourly allowance.
    let more = [("kills_1000".to_owned(), 1_000_u64)].into_iter().collect();
    let held = hub.set_achievements(&me, &more).unwrap();
    let thousand = held.iter().find(|a| a.id == "kills_1000").unwrap();
    assert_eq!(thousand.progress, 150);
    assert_eq!(thousand.unlocked, 0);
    // The profile lists them too.
    let profile = hub.profile(&me.key_id()).unwrap();
    assert!(
        profile
            .achievements
            .iter()
            .any(|a| a.id == "kills_1000" && a.progress == 150)
    );
}

/// A staff key: also needs `SJK_HUB_TEST_STAFF_SEED`, the 64 hex digits of a key's
/// seed that the test hub's operator made staff (`SJK_HUB_TEST_PLAIN_SEED` likewise for
/// a registered key that is not staff, to see it refused).
#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL) and its staff key (SJK_HUB_TEST_STAFF_SEED)"]
fn a_staff_key_finds_players_gives_and_takes_back_medals_and_clears_achievements() {
    use sjk_identity::StaffRequest;
    let mut hub = hub();
    let seed = |name: &str| {
        let hex = std::env::var(name).expect(name);
        let mut seed = [0_u8; 32];
        for (index, byte) in seed.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap();
        }
        Identity::from_seed(seed)
    };
    let staff = seed("SJK_HUB_TEST_STAFF_SEED");
    let plain = seed("SJK_HUB_TEST_PLAIN_SEED");
    let player = Identity::generate().unwrap();
    let suffix = &player.key_id()[..6];
    hub.register(&staff, Some("^1Staffer")).unwrap();
    hub.register(&plain, Some("Plain")).unwrap();
    hub.register(&player, Some(&format!("^4Padawan{suffix}")))
        .unwrap();
    assert!(hub.profile(&staff.key_id()).unwrap().staff);

    // A key that is not staff is refused.
    match hub
        .staff(&plain, &StaffRequest::Search(String::new()))
        .unwrap_err()
    {
        HubError::Rejected { code, .. } => assert_eq!(code, "not_staff"),
        other => panic!("{other:?}"),
    }

    let found = hub
        .staff(&staff, &StaffRequest::Search(format!("padawan{suffix}")))
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].key_id, player.key_id());
    let by_key = hub
        .staff(&staff, &StaffRequest::Search(player.key_id()))
        .unwrap();
    assert_eq!(by_key[0].key_id, player.key_id());

    let given = hub
        .staff(
            &staff,
            &StaffRequest::Award {
                key_id: player.key_id(),
                medal: "bug_hunter".into(),
                note: "found the fog bug".into(),
            },
        )
        .unwrap();
    assert_eq!(given[0].medals[0].id, "bug_hunter");
    assert_eq!(given[0].medals[0].note, "found the fog bug");
    assert!(
        given[0]
            .achievements
            .iter()
            .any(|a| a.id == "decorated" && a.unlocked > 0)
    );
    let taken = hub
        .staff(
            &staff,
            &StaffRequest::Unaward {
                key_id: player.key_id(),
                medal: "bug_hunter".into(),
            },
        )
        .unwrap();
    assert!(taken[0].medals.is_empty());

    let counts = [
        ("first_blood".to_owned(), 1_u64),
        ("streak_5".to_owned(), 2),
    ]
    .into_iter()
    .collect();
    hub.set_achievements(&player, &counts).unwrap();
    let cleared = hub
        .staff(
            &staff,
            &StaffRequest::ClearAchievements {
                key_id: player.key_id(),
                id: "first_blood".into(),
            },
        )
        .unwrap();
    assert!(
        !cleared[0]
            .achievements
            .iter()
            .any(|a| a.id == "first_blood")
    );
    assert!(cleared[0].achievements.iter().any(|a| a.id == "streak_5"));
    let all = hub
        .staff(
            &staff,
            &StaffRequest::ClearAchievements {
                key_id: player.key_id(),
                id: String::new(),
            },
        )
        .unwrap();
    assert!(all[0].achievements.is_empty());
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn chat_and_emotes_reach_other_players_through_the_long_poll() {
    use std::time::{Duration, Instant};

    let url = std::env::var("SJK_HUB_TEST_URL").expect("SJK_HUB_TEST_URL");
    let mut hub = hub();
    let mut reader =
        HttpHub::with_timeout(&url, "sjk-identity-test", sjk_identity::feed::TIMEOUT).unwrap();
    let me = Identity::generate().unwrap();
    let other = Identity::generate().unwrap();
    hub.register(&me, Some("^2Chatty")).unwrap();
    hub.register(&other, None).unwrap();
    let start = reader.feed(&other, 0, None, 0).unwrap();

    // The rules are the hub's too.
    let refused = hub.chat(&me, "hi \u{1F600}", "").unwrap_err();
    assert!(
        matches!(refused, HubError::Rejected { ref code, .. } if code == "chat_characters"),
        "{refused:?}"
    );

    // A reader waiting at the hub hears a message as soon as it is said.
    let text = format!("hello from {}", &me.key_id()[..8]);
    let said = std::thread::scope(|scope| {
        let waiting = scope.spawn(|| {
            let asked = Instant::now();
            let feed = reader.feed(&other, start.next, None, 25).unwrap();
            (feed, asked.elapsed())
        });
        std::thread::sleep(Duration::from_secs(2));
        hub.chat(&me, &text, "^2Chatty").unwrap();
        waiting.join().unwrap()
    });
    let (feed, waited) = said;
    assert!(
        waited >= Duration::from_millis(1_500) && waited < Duration::from_secs(10),
        "{waited:?}"
    );
    let message = feed
        .chat
        .iter()
        .find(|m| m.text == text)
        .expect("the message");
    assert_eq!(
        (message.key_id.as_str(), message.name.as_str()),
        (me.key_id().as_str(), "^2Chatty")
    );

    // An emote needs a claim and reaches the readers of that server.
    let server = format!(
        "10.97.0.{}:29070",
        u8::from_str_radix(&me.key_id()[..2], 16).unwrap()
    );
    let nowhere = hub.emote(&me, &server, "wave").unwrap_err();
    assert!(
        matches!(nowhere, HubError::Rejected { ref code, .. } if code == "not_on_server"),
        "{nowhere:?}"
    );
    hub.claim(&me, &server, 4, "^2Chatty").unwrap();
    hub.emote(&me, &server, "wave").unwrap();
    let here = reader.feed(&other, feed.next, Some(&server), 0).unwrap();
    assert_eq!(here.emotes.len(), 1);
    assert_eq!(
        (here.emotes[0].slot, here.emotes[0].emote.as_str()),
        (4, "wave")
    );
    assert_eq!(here.emotes[0].claimed_name, "^2Chatty");
    hub.release(&me, &server).unwrap();
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn the_service_reads_the_chat_on_its_own_thread() {
    use sjk_identity::{Service, Settings};
    use std::time::{Duration, Instant};

    let url = std::env::var("SJK_HUB_TEST_URL").expect("SJK_HUB_TEST_URL");
    let make: sjk_identity::HubFactory = Box::new(|url| {
        HttpHub::new(url, "sjk-identity-test").map(|hub| Box::new(hub) as Box<dyn Hub>)
    });
    let make_feed: sjk_identity::HubFactory = Box::new(|url| {
        HttpHub::with_timeout(url, "sjk-identity-test", sjk_identity::feed::TIMEOUT)
            .map(|hub| Box::new(hub) as Box<dyn Hub>)
    });
    let me = Identity::generate().unwrap();
    let service = Service::start_with_feed(me, make, Some(make_feed));
    service.set_name("^3Svc".to_owned());
    service.set_chat(true);
    service.configure(Settings {
        enabled: true,
        hub_url: url,
    });
    let wait_for = |what: &str, done: &dyn Fn(&sjk_identity::ChatState) -> bool| {
        let until = Instant::now() + Duration::from_secs(15);
        while Instant::now() < until {
            if service.with_chat(|chat| done(chat)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!(
            "timed out waiting for {what}: {:?}",
            service.with_chat(Clone::clone)
        );
    };
    wait_for("the feed to reach the hub", &|chat| {
        chat.live && chat.online >= 1
    });
    let text = format!("service says hi {}", std::process::id());
    service.chat(text.clone());
    wait_for("its own message", &|chat| {
        chat.messages
            .iter()
            .any(|message| message.text == text && message.name == "^3Svc")
    });
    assert!(service.with_chat(|chat| chat.outcome.as_ref().is_some_and(|o| o.sent)));
    // Turning the chat off stops the reading.
    service.set_chat(false);
    wait_for("the feed to stop", &|chat| !chat.live);
    service.shutdown(Duration::from_secs(5));
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL; SJK_HUB_TEST_ADMIN_KEY for the unlocked half)"]
fn looks_travel_with_the_claim_and_the_feed() {
    use sjk_identity::Look;
    use std::time::Duration;

    let url = std::env::var("SJK_HUB_TEST_URL").expect("SJK_HUB_TEST_URL");
    let mut hub = hub();
    let me = Identity::generate().unwrap();
    let other = Identity::generate().unwrap();
    hub.register(&me, Some("^2Looky")).unwrap();
    hub.register(&other, None).unwrap();
    let server = format!(
        "10.96.0.{}:29070",
        u8::from_str_radix(&me.key_id()[..2], 16).unwrap()
    );
    let lit = Look {
        saber: String::new(),
        illuminate: true,
    };
    let nowhere = hub.look(&me, &server, &lit).unwrap_err();
    assert!(
        matches!(nowhere, HubError::Rejected { ref code, .. } if code == "not_on_server"),
        "{nowhere:?}"
    );
    hub.claim(&me, &server, 3, "^2Looky").unwrap();
    // A fresh claim has no look.
    let bare = hub.presence(&server).unwrap();
    assert_eq!(bare[0].look, None);
    // Something in the feed's sequence, so the reader's next `after` is not 0 (which
    // gives no looks) on a fresh hub.
    hub.chat(&other, "looks are coming", "").unwrap();
    let start = hub.feed(&other, 0, Some(&server), 0).unwrap();
    assert!(start.looks.is_empty(), "after 0 gives no looks");
    let id = hub.look(&me, &server, &lit).unwrap();
    assert!(id > 0);
    let players = hub.presence(&server).unwrap();
    assert_eq!(players[0].look, Some(lit.clone()));
    let feed = hub.feed(&other, start.next, Some(&server), 0).unwrap();
    let event = &feed.looks[0];
    assert_eq!(event.id, id);
    assert_eq!((event.slot, event.claimed_name.as_str()), (3, "^2Looky"));
    assert_eq!(event.look(), lit);
    // A blade skin the key does not hold is refused, and does not count.
    let sun = Look {
        saber: "saber_sun".to_owned(),
        illuminate: true,
    };
    let refused = hub.look(&me, &server, &sun).unwrap_err();
    assert!(
        matches!(refused, HubError::Rejected { ref code, .. } if code == "not_unlocked"),
        "{refused:?}"
    );
    // Renewing the claim keeps the look.
    hub.claim(&me, &server, 3, "^2Looky").unwrap();
    assert_eq!(hub.presence(&server).unwrap()[0].look, Some(lit.clone()));
    let Some(admin) = operator_key() else {
        eprintln!("no SJK_HUB_TEST_ADMIN_KEY: the unlocked half is skipped");
        hub.release(&me, &server).unwrap();
        return;
    };
    let granted = ureq::post(&format!(
        "{url}/admin/v1/identities/{}/unlocks",
        me.key_id()
    ))
    .header("Authorization", &format!("Bearer {admin}"))
    .header("Content-Type", "application/json")
    .send(r#"{"unlock":"saber_sun","note":"e2e"}"#)
    .unwrap();
    assert_eq!(granted.status().as_u16(), 200);
    let profile = hub.profile(&me.key_id()).unwrap();
    assert_eq!(profile.unlocks[0].id, "saber_sun");
    assert_eq!(profile.unlocks[0].note, "e2e");
    std::thread::sleep(Duration::from_millis(1_100));
    hub.look(&me, &server, &sun).unwrap();
    assert_eq!(hub.presence(&server).unwrap()[0].look, Some(sun));
    // Another slot is another claim: it starts with no look.
    hub.claim(&me, &server, 5, "^2Looky").unwrap();
    assert_eq!(hub.presence(&server).unwrap()[0].look, None);
    hub.release(&me, &server).unwrap();
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn the_service_wears_its_look_on_its_claim_and_reads_looks_with_the_chat_off() {
    use sjk_identity::{Location, Look, Service, Settings};
    use std::time::{Duration, Instant};

    let url = std::env::var("SJK_HUB_TEST_URL").expect("SJK_HUB_TEST_URL");
    let make: sjk_identity::HubFactory = Box::new(|url| {
        HttpHub::new(url, "sjk-identity-test").map(|hub| Box::new(hub) as Box<dyn Hub>)
    });
    let make_feed: sjk_identity::HubFactory = Box::new(|url| {
        HttpHub::with_timeout(url, "sjk-identity-test", sjk_identity::feed::TIMEOUT)
            .map(|hub| Box::new(hub) as Box<dyn Hub>)
    });
    let me = Identity::generate().unwrap();
    let server = format!(
        "10.95.0.{}:29070",
        u8::from_str_radix(&me.key_id()[..2], 16).unwrap()
    );
    let service = Service::start_with_feed(me, make, Some(make_feed));
    service.set_name("^5Lamp".to_owned());
    service.set_chat(false);
    // The skin is not the key's: Illuminate still goes, without it.
    service.set_look(Look {
        saber: "saber_sun".to_owned(),
        illuminate: true,
    });
    service.configure(Settings {
        enabled: true,
        hub_url: url,
    });
    service.enter(Location {
        server: server.parse().unwrap(),
        slot: 6,
        name: "^5Lamp".to_owned(),
    });
    let mut hub = hub();
    let lit = Look {
        saber: String::new(),
        illuminate: true,
    };
    let until = Instant::now() + Duration::from_secs(15);
    let worn = loop {
        let worn = hub
            .presence(&server)
            .unwrap()
            .first()
            .and_then(|p| p.look.clone());
        if worn.as_ref() == Some(&lit) || Instant::now() > until {
            break worn;
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    assert_eq!(worn, Some(lit));
    // The feed reads on the server with the chat off: the service's own look event
    // may have come before the reader started, so a change is made and awaited.
    service.set_look(Look::default());
    let until = Instant::now() + Duration::from_secs(15);
    let put_out = |look: &sjk_identity::LookEvent| look.slot == 6 && !look.illuminate;
    let mut looks = Vec::new();
    while !looks.iter().any(put_out) && Instant::now() < until {
        looks.extend(service.take_looks(server.parse().ok()).events);
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(looks.iter().any(put_out), "{looks:?}");
    assert!(service.with_chat(|chat| chat.messages.is_empty() && !chat.live));
    service.shutdown(Duration::from_secs(5));
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL) and its staff key (SJK_HUB_TEST_STAFF_SEED)"]
fn a_staff_key_unlocks_and_relocks() {
    use sjk_identity::StaffRequest;
    let mut hub = hub();
    let hex = std::env::var("SJK_HUB_TEST_STAFF_SEED").expect("SJK_HUB_TEST_STAFF_SEED");
    let mut seed = [0_u8; 32];
    for (index, byte) in seed.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap();
    }
    let staff = Identity::from_seed(seed);
    let player = Identity::generate().unwrap();
    hub.register(&staff, Some("^1Staffer")).unwrap();
    hub.register(&player, None).unwrap();
    assert!(
        hub.profile(&staff.key_id()).unwrap().staff,
        "make {} staff",
        staff.key_id()
    );
    let unlock = StaffRequest::Unlock {
        key_id: player.key_id(),
        unlock: "saber_sun".to_owned(),
        note: "For testing".to_owned(),
    };
    let answered = hub.staff(&staff, &unlock).unwrap();
    assert_eq!(answered[0].key_id, player.key_id());
    assert_eq!(answered[0].unlocks[0].id, "saber_sun");
    assert_eq!(answered[0].unlocks[0].note, "For testing");
    let again = hub.staff(&staff, &unlock).unwrap_err();
    assert!(
        matches!(again, HubError::Rejected { ref code, .. } if code == "already_unlocked"),
        "{again:?}"
    );
    let relock = StaffRequest::Relock {
        key_id: player.key_id(),
        unlock: "saber_sun".to_owned(),
    };
    assert!(hub.staff(&staff, &relock).unwrap()[0].unlocks.is_empty());
    let gone = hub.staff(&staff, &relock).unwrap_err();
    assert!(
        matches!(gone, HubError::Rejected { ref code, .. } if code == "not_unlocked"),
        "{gone:?}"
    );
}
