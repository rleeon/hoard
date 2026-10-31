//! Notices and their switches over a real socket: one notice reaches one screen, and
//! a switch moved from one client reaches the others.
//!
//! In a binary of its own because moving a switch writes `prefs.json`, so the whole
//! process runs on a throwaway profile, set before anything reads a path.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use hoard_agent::agent::AgentEvent;
use hoard_core::ipc::{NoticeLevel, NotificationSwitch, Payload, Request, CAP_NOTICES};
use hoardd::client::{Client, Push};
use hoardd::endpoint::Endpoint;
use hoardd::engine::Engine;
use hoardd::journal::EventLog;
use hoardd::notify::text::Note;
use hoardd::notify::{Notifier, Sink};
use hoardd::server::{accept_loop, Daemon};
use hoardd::transport::Listener;

struct Recorder(Mutex<Vec<Note>>);

impl Sink for Recorder {
    fn deliver(&self, note: &Note) -> anyhow::Result<()> {
        self.0.lock().unwrap().push(note.clone());
        Ok(())
    }
}

fn game(stopped: bool) -> AgentEvent {
    let (save_id, game_slug) = ("s1".to_string(), "factorio".to_string());
    if stopped {
        AgentEvent::GameStopped { save_id, game_slug }
    } else {
        AgentEvent::GameStarted { save_id, game_slug }
    }
}

async fn painter(endpoint: &Endpoint) -> Client {
    let mut client = Client::ensure_running_with(endpoint, "painter", &[CAP_NOTICES])
        .await
        .expect("connect");
    client
        .request(Request::Subscribe { since: None })
        .await
        .expect("subscribe");
    client
}

/// The next notice or switch this client hears, skipping journal rows.
async fn next_note(client: &mut Client) -> Option<Push> {
    let wait = async {
        loop {
            match client.next_push().await {
                Ok(Some(Push::Event(_))) => continue,
                Ok(push) => return push,
                Err(_) => return None,
            }
        }
    };
    tokio::time::timeout(Duration::from_millis(400), wait)
        .await
        .ok()
        .flatten()
}

#[tokio::test(flavor = "multi_thread")]
async fn one_notice_reaches_one_screen_and_a_switch_reaches_every_client() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var(
        "HOARD_PROFILE",
        format!("test-notices-{}", std::process::id()),
    );
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    std::env::set_var("XDG_DATA_HOME", dir.path());
    std::env::set_var("XDG_STATE_HOME", dir.path());

    let recorder = Arc::new(Recorder(Mutex::new(Vec::new())));
    let notifier = Arc::new(Notifier::with_sink(recorder.clone()));
    let endpoint = Endpoint::scoped(dir.path(), &format!("notices-{}", std::process::id()));
    let listener = Listener::bind(&endpoint).expect("bind");
    let daemon = Arc::new(
        Daemon::new(Arc::new(EventLog::new()), Engine::new()).with_notifier(notifier.clone()),
    );
    let accept = tokio::spawn(accept_loop(
        Arc::new(tokio::sync::Mutex::new(listener)),
        daemon,
    ));

    let mut app = painter(&endpoint).await;
    let mut plugin = painter(&endpoint).await;
    assert!(app.has_cap(CAP_NOTICES));

    // Moved from one client, heard by the other, and it decides right away.
    let answer = app
        .request(Request::SetNotificationSwitch {
            switch: NotificationSwitch::GameActivity,
            enabled: true,
        })
        .await
        .unwrap();
    let Payload::NotificationSwitches(moved) = answer else {
        panic!("unexpected answer: {answer:?}");
    };
    assert!(moved.game_activity);
    match next_note(&mut plugin).await {
        Some(Push::NotificationSwitches(heard)) => assert_eq!(heard, moved),
        other => panic!("the other client wasn't told: {other:?}"),
    }
    let _ = next_note(&mut app).await;

    // Nobody claiming: the OS, and no client.
    notifier.consider(&game(false)).await;
    assert_eq!(recorder.0.lock().unwrap().len(), 1);
    assert!(next_note(&mut plugin).await.is_none());

    // Game Mode: the claimant, and neither the OS nor the app.
    plugin
        .request(Request::ClaimNotices { active: true })
        .await
        .unwrap();
    notifier.consider(&game(true)).await;
    match next_note(&mut plugin).await {
        Some(Push::Notice(note)) => {
            assert_eq!(note.level, NoticeLevel::Info);
            assert_eq!(note.body, "Factorio");
        }
        other => panic!("the claimant heard nothing: {other:?}"),
    }
    assert!(next_note(&mut app).await.is_none());
    assert_eq!(recorder.0.lock().unwrap().len(), 1);

    // A claimant that goes away gives them back.
    drop(plugin);
    tokio::time::sleep(Duration::from_millis(200)).await;
    notifier.consider(&game(false)).await;
    assert_eq!(recorder.0.lock().unwrap().len(), 2);

    // Nothing to carry the notices, so nothing to claim them with.
    let mut bare = Client::ensure_running_with(&endpoint, "bare", &[CAP_NOTICES])
        .await
        .unwrap();
    assert!(bare
        .request(Request::ClaimNotices { active: true })
        .await
        .is_err());
    notifier.consider(&game(true)).await;
    assert_eq!(recorder.0.lock().unwrap().len(), 3);

    accept.abort();
    if let Ok(path) = hoard_agent::prefs::Prefs::default_path() {
        if let Some(profile) = path.parent() {
            let _ = std::fs::remove_dir_all(profile);
        }
    }
}
