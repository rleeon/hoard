//! `GET /v1/events` on cloud: the save-landed stream self-hosted already
//! serves, fed by the two commit paths in `saves`.
//!
//! Until the move off Supabase, cloud push was Supabase Realtime watching the
//! `saves` table. That only works while the table lives in a database Supabase
//! hosts. One machine holds every connection, so an in-process broadcast is
//! the whole broker. A client connected across a restart loses the frames in
//! between and does what it does after any reconnect: a full pull.

use crate::cloud::{auth::CloudUser, state::CloudState};
use crate::routes::events::SaveEvent;
use axum::extract::{Extension, State};
use axum::response::sse::{Event, Sse};
use futures::Stream;
use std::convert::Infallible;

pub async fn stream(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    crate::routes::events::sse(state.events.subscribe(user.user_id))
}

pub fn publish(state: &CloudState, user_id: uuid::Uuid, save_id: &str, version_num: i64) {
    state.events.publish(
        user_id,
        SaveEvent {
            save_id: save_id.to_string(),
            version_num,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::events::EventBus;
    use axum::response::IntoResponse;
    use futures::StreamExt;
    use std::time::Duration;
    use uuid::Uuid;

    // What the endpoint promises, checked on the bytes: a commit for a user
    // reaches that user's open stream as one `save` frame, and nobody else's.
    #[tokio::test]
    async fn a_commit_reaches_its_owner_and_only_its_owner() {
        let bus = EventBus::default();
        let owner = Uuid::new_v4();
        let stranger = Uuid::new_v4();

        let mut mine = crate::routes::events::sse(bus.subscribe(owner))
            .into_response()
            .into_body()
            .into_data_stream();
        let mut theirs = crate::routes::events::sse(bus.subscribe(stranger))
            .into_response()
            .into_body()
            .into_data_stream();

        bus.publish(
            owner,
            SaveEvent {
                save_id: "factorio-default".into(),
                version_num: 42,
            },
        );

        let frame = tokio::time::timeout(Duration::from_secs(2), mine.next())
            .await
            .expect("the owner's stream yields a frame")
            .expect("stream open")
            .expect("frame bytes");
        let text = String::from_utf8(frame.to_vec()).unwrap();
        assert!(text.contains("event: save"), "{text}");
        assert!(
            text.contains(r#""save_id":"factorio-default""#)
                && text.contains(r#""version_num":42"#),
            "{text}"
        );

        let other = tokio::time::timeout(Duration::from_millis(200), theirs.next()).await;
        assert!(other.is_err(), "another user's stream must stay silent");
    }

    // A shutdown drains responses in flight; a stream that never ends would
    // hold every restart for the full grace period.
    #[tokio::test]
    async fn closing_the_bus_ends_open_streams() {
        let bus = EventBus::default();
        let mut open = crate::routes::events::sse(bus.subscribe(Uuid::new_v4()))
            .into_response()
            .into_body()
            .into_data_stream();
        bus.close_all();
        let end = tokio::time::timeout(Duration::from_secs(2), open.next())
            .await
            .expect("the stream ends instead of waiting for the keep-alive");
        assert!(end.is_none(), "no frame, just the end of the body");
    }
}
