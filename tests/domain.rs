use mytop_linux::*;
use serde_json::{json, Value};
use std::path::Path;
fn game(title: &str, rating: f64) -> Game {
    serde_json::from_value(
        json!({"title":title,"rating":rating,"completions":0,"status":"PLAYING"}),
    )
    .unwrap()
}
fn store() -> Store {
    Store::open(Path::new(":memory:")).unwrap()
}
#[test]
fn mobile_example_and_roundtrip() {
    let text = include_str!("../examples/games.json");
    let games = decode_import(text).unwrap();
    assert!(!games.is_empty());
    assert!(games.iter().all(|g| g.id == 0));
    let output = export(&games, &Query::default(), &ExportOptions::default(), "ru").unwrap();
    let restored = decode_import(&output).unwrap();
    assert_eq!(Query::default().apply(&games), restored);
}
#[test]
fn combined_unicode_search_filters_and_ties() {
    let mut a = game("  Ведьмак 3", 9.5);
    a.id = 1;
    a.genres = vec!["RPG".into()];
    a.platforms = vec!["Linux".into()];
    a.developers = vec!["CD Projekt".into()];
    let mut b = a.clone();
    b.title = "Другая игра".into();
    b.id = 2;
    b.rating = 5.0;
    let q = Query {
        search: "\tВЕДЬ\u{a0}projekt\nlinux ".into(),
        genre: Some("rpg".into()),
        status: Some(Status::Playing),
        minimum_rating: 9.0,
        ..Default::default()
    };
    assert_eq!(q.apply(&[b, a.clone()]), vec![a]);
    let a = game("Б", 8.0);
    let b = game("А", 8.0);
    assert_eq!(Query::default().apply(&[a, b])[0].title, "А");
}
#[test]
fn all_sort_modes_and_missing_year() {
    let mut a = game("Z", 9.0);
    a.id = 2;
    a.release_year = Some(2020);
    let mut b = game("A", 3.0);
    b.id = 5;
    for (sort, first) in [
        (Sort::RatingDesc, "Z"),
        (Sort::RatingAsc, "A"),
        (Sort::Title, "A"),
        (Sort::Newest, "A"),
        (Sort::ReleaseYear, "Z"),
    ] {
        assert_eq!(
            Query {
                sort,
                ..Default::default()
            }
            .apply(&[a.clone(), b.clone()])[0]
                .title,
            first
        );
    }
}
#[test]
fn import_is_atomic_and_ids_are_local() {
    let mut db = store();
    let mut a = game("Original", 7.0);
    a.opinion = Some("keep".into());
    let saved = db.save(a).unwrap();
    let bad = json!([{"id":999,"title":"Valid","rating":3,"completions":0,"status":"COMPLETED"},{"title":"Broken","rating":11,"completions":0,"status":"PLAYING"}]);
    assert!(db.import(&bad.to_string()).is_err());
    assert_eq!(db.games().unwrap(), vec![saved.clone()]);
    let text=json!([{"id":999,"title":" Imported ","rating":3,"completions":0,"status":"COMPLETED","futureField":true}]).to_string();
    db.import(&text).unwrap();
    db.import(&text).unwrap();
    let games = db.games().unwrap();
    assert_eq!(games.len(), 3);
    assert_eq!(games[1].title, "Imported");
    assert_ne!(games[1].id, 999);
    assert_ne!(games[1].id, games[2].id);
    assert_eq!(games[0], saved);
}
#[test]
fn editing_preserves_all_fields_and_does_not_upsert_missing_game() {
    let mut db = store();
    let mut g = decode_import(include_str!("../examples/games.json"))
        .unwrap()
        .remove(0);
    g.cover_uri = Some("/tmp/cover.png".into());
    g.opinion = Some("my notes".into());
    g.completion_dates = vec!["01.01.2026".into()];
    g.playtime_hours = Some(12.5);
    let mut saved = db.save(g).unwrap();
    saved.title = "Renamed".into();
    db.save(saved.clone()).unwrap();
    assert_eq!(db.games().unwrap(), vec![saved.clone()]);
    db.delete(saved.id).unwrap();
    assert!(db.save(saved).is_err());
    assert!(db.games().unwrap().is_empty());
}
#[test]
fn malformed_and_limited_imports() {
    for text in [
        "{}",
        "null",
        "{\"version\":2,\"games\":[]}",
        "[{\"title\":\"A\"}]",
        "[{\"title\":\"A\",\"rating\":1,\"completions\":-1,\"status\":\"PLAYING\"}]",
        "[{\"title\":\" \u{a0}\",\"rating\":1,\"completions\":0,\"status\":\"PLAYING\"}]",
        "[{\"title\":\"A\",\"rating\":1,\"completions\":0,\"status\":\"UNKNOWN\"}]",
    ] {
        assert!(decode_import(text).is_err(), "{text}");
    }
    assert!(decode_import(&" ".repeat(MAX_IMPORT_BYTES + 1)).is_err());
    let too_many = vec![json!({"title":"a","rating":0,"completions":0,"status":"PLAYING"}); 10_001];
    assert!(decode_import(&serde_json::to_string(&too_many).unwrap()).is_err());
    let mut g = game("A", 0.0);
    g.playtime_hours = Some(-1.0);
    assert!(g.validate().is_err());
}
#[test]
fn export_scope_omissions_and_text_top() {
    let mut a = game("A", 9.5);
    a.opinion = Some("private".into());
    a.genres = vec!["RPG".into()];
    a.playtime_hours = Some(2.0);
    let games = [a, game("B", 2.0)];
    let q = Query {
        minimum_rating: 9.0,
        ..Default::default()
    };
    let o = ExportOptions {
        filtered: true,
        include_metadata: false,
        include_history: false,
        include_opinion: false,
        include_cover: false,
        ..Default::default()
    };
    let text = export(&games, &q, &o, "ru").unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    let r = &v["games"][0];
    for key in ["genres", "series", "playtimeHours", "opinion", "coverUri"] {
        assert!(r.get(key).is_none());
    }
    assert_eq!(v["games"].as_array().unwrap().len(), 1);
    assert_eq!(decode_import(&text).unwrap().len(), 1);
    let text = export(
        &games,
        &q,
        &ExportOptions {
            format: "TEXT".into(),
            filtered: true,
            top_only: true,
            ..Default::default()
        },
        "ru",
    )
    .unwrap();
    assert_eq!(text, "1. A — 9,5 / 10");
    let text = export(
        &[],
        &q,
        &ExportOptions {
            format: "TEXT".into(),
            ..Default::default()
        },
        "en",
    )
    .unwrap();
    assert!(text.contains("— / 10"));
}
#[test]
fn settings_roundtrip_and_sanitization() {
    let mut db = store();
    let a = Appearance {
        text_scale: 4.0,
        accent_hex: " 12ab34 ".into(),
        language_tag: "ru".into(),
        ..Default::default()
    }
    .sanitize();
    assert_eq!(a.text_scale, 1.2);
    assert_eq!(a.accent_hex, "#12AB34");
    db.set_setting("appearance", &a).unwrap();
    assert_eq!(
        db.setting::<Appearance>("appearance").unwrap().language_tag,
        "ru"
    );
    assert_eq!(
        Appearance {
            font: "CUSTOM".into(),
            ..Default::default()
        }
        .sanitize()
        .font,
        "SYSTEM"
    );
}
#[test]
fn sync_matches_mobile_stub() {
    assert_eq!(SyncSettings::default().synchronize(), "sync_disabled");
    for url in [
        "http://example.com",
        "https://user:pass@example.com",
        "https://@example.com",
        "https://example.com?x",
        "https://example.com#x",
        "https://example.com:0",
        "invalid",
    ] {
        assert_eq!(
            SyncSettings {
                enabled: true,
                server_url: url.into()
            }
            .synchronize(),
            "sync_invalid_url",
            "{url}"
        );
    }
    assert_eq!(
        SyncSettings {
            enabled: true,
            server_url: "https://example.com/api".into()
        }
        .synchronize(),
        "sync_unavailable"
    );
}
#[test]
fn statistics_and_all_locales() {
    assert_eq!(average(&[]), None);
    assert_eq!(average(&[game("A", 8.0), game("B", 10.0)]), Some(9.0));
    let strings: Value = serde_json::from_str(STRINGS).unwrap();
    assert_eq!(strings.as_object().unwrap().len(), 13);
    for (_, lang) in strings.as_object().unwrap() {
        for key in strings["en"].as_object().unwrap().keys() {
            assert!(lang.get(key).is_some(), "missing {key}");
        }
    }
}
