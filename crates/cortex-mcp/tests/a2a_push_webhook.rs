//! Push webhook fires on task completion when enabled.

use cortex_a2a::push::TaskPushNotificationConfig;
use cortex_a2a::{A2aHub, SpawnSessionRequest};
use cortex_core::A2aConfig;
use cortex_core::a2a_config::A2aPushConfig;
use httpmock::MockServer;
#[tokio::test]
async fn push_delivers_on_task_complete() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(httpmock::Method::POST).path("/callback");
        then.status(200);
    });

    let mut config = A2aConfig {
        enabled: true,
        consensus_max_rounds: 2,
        ..A2aConfig::default()
    };
    config.push = A2aPushConfig {
        enabled: true,
        default_callback_timeout_secs: 5,
        ..A2aPushConfig::default()
    };

    let hub = A2aHub::new(config);
    let resp = hub
        .spawn_session(SpawnSessionRequest::with_scope(
            "Fix deadlock in src/transport.rs",
            "consensus_review",
            vec!["src/transport.rs".to_string()],
            4000,
        ))
        .expect("spawn");

    hub.push().create_config(TaskPushNotificationConfig {
        id: String::new(),
        task_id: resp.task_id.clone(),
        url: format!("{}/callback", server.base_url()),
        token: None,
    });

    for _ in 0..80 {
        if mock.hits() > 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    assert!(
        mock.hits() >= 1,
        "expected push callback after task completion"
    );
}
