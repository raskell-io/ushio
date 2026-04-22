use std::path::PathBuf;

/// Path to test fixtures
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

mod har_parsing {
    use super::*;

    #[test]
    fn parse_simple_har() {
        let content = std::fs::read_to_string(fixture_path("simple.har")).unwrap();
        let har = ushio::har::parse_har(&content).unwrap();
        assert_eq!(har.log.entries.len(), 3);
        assert_eq!(har.log.entries[0].request.method, "GET");
        assert_eq!(har.log.entries[1].request.method, "POST");
        assert_eq!(har.log.entries[2].response.status, 403);
    }

    #[test]
    fn har_to_capture_preserves_requests() {
        let content = std::fs::read_to_string(fixture_path("simple.har")).unwrap();
        let har = ushio::har::parse_har(&content).unwrap();
        let requests = ushio::har::har_to_capture(har);

        assert_eq!(requests.len(), 3);

        // First request
        assert_eq!(requests[0].method, "GET");
        assert!(requests[0].url.contains("/api/users"));
        assert_eq!(requests[0].expected_status, Some(200));
        assert!(requests[0].body.is_none());

        // Second request (POST with body)
        assert_eq!(requests[1].method, "POST");
        assert!(requests[1].body.is_some());
        assert!(requests[1].body.as_ref().unwrap().contains("username"));

        // Third request (expected 403)
        assert_eq!(requests[2].expected_status, Some(403));
    }

    #[test]
    fn har_headers_converted() {
        let content = std::fs::read_to_string(fixture_path("simple.har")).unwrap();
        let har = ushio::har::parse_har(&content).unwrap();
        let requests = ushio::har::har_to_capture(har);

        let has_accept = requests[0]
            .headers
            .iter()
            .any(|(k, v)| k == "Accept" && v == "application/json");
        assert!(has_accept);
    }
}

mod capture_format {
    use super::*;

    #[test]
    fn load_capture_file() {
        let content = std::fs::read_to_string(fixture_path("capture.json")).unwrap();
        let capture: ushio::capture::Capture = serde_json::from_str(&content).unwrap();

        assert_eq!(capture.version, "1.0");
        assert_eq!(capture.source.as_deref(), Some("test-capture"));
        assert_eq!(capture.requests.len(), 2);
        assert_eq!(capture.requests[0].method, "GET");
        assert_eq!(capture.requests[1].method, "POST");
    }

    #[test]
    fn capture_round_trip() {
        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://example.com/test".to_string(),
            headers: vec![("Accept".to_string(), "text/html".to_string())],
            body: None,
            expected_status: Some(200),
        }];

        let capture = ushio::capture::Capture::new(requests).with_source("test".to_string());
        let json = serde_json::to_string_pretty(&capture).unwrap();
        let loaded: ushio::capture::Capture = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.version, "1.0");
        assert_eq!(loaded.source.as_deref(), Some("test"));
        assert_eq!(loaded.requests.len(), 1);
        assert_eq!(loaded.requests[0].method, "GET");
        assert_eq!(loaded.requests[0].expected_status, Some(200));
    }
}

mod replay_engine {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn replay_against_mock_server() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/health"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("{\"status\":\"ok\"}")
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/data"))
            .respond_with(ResponseTemplate::new(201).set_body_string("{\"id\":1}"))
            .mount(&mock_server)
            .await;

        let requests = vec![
            ushio::capture::CapturedRequest {
                method: "GET".to_string(),
                url: "https://example.com/api/health".to_string(),
                headers: vec![],
                body: None,
                expected_status: Some(200),
            },
            ushio::capture::CapturedRequest {
                method: "POST".to_string(),
                url: "https://example.com/api/data".to_string(),
                headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                body: Some("{\"key\":\"value\"}".to_string()),
                expected_status: Some(201),
            },
        ];

        let config = ushio::replay::ReplayConfig::default();
        let session = ushio::replay::replay(&requests, &mock_server.uri(), config)
            .await
            .unwrap();

        assert_eq!(session.total_requests, 2);
        assert_eq!(session.successful, 2);
        assert_eq!(session.failed, 0);
        assert_eq!(session.status_mismatches, 0);
        assert_eq!(session.results[0].status, 200);
        assert!(session.results[0].status_match);
        assert_eq!(session.results[1].status, 201);
        assert!(session.results[1].status_match);
    }

    #[tokio::test]
    async fn replay_captures_body() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/page"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("<html><body>Hello</body></html>"),
            )
            .mount(&mock_server)
            .await;

        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://example.com/page".to_string(),
            headers: vec![],
            body: None,
            expected_status: Some(200),
        }];

        let config = ushio::replay::ReplayConfig::default();
        let session = ushio::replay::replay(&requests, &mock_server.uri(), config)
            .await
            .unwrap();

        assert!(session.results[0].body.is_some());
        assert!(session.results[0].body.as_ref().unwrap().contains("Hello"));
    }

    #[tokio::test]
    async fn replay_detects_status_mismatch() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/blocked"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&mock_server)
            .await;

        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://example.com/blocked".to_string(),
            headers: vec![],
            body: None,
            expected_status: Some(200),
        }];

        let config = ushio::replay::ReplayConfig::default();
        let session = ushio::replay::replay(&requests, &mock_server.uri(), config)
            .await
            .unwrap();

        assert_eq!(session.status_mismatches, 1);
        assert!(!session.results[0].status_match);
        assert_eq!(session.results[0].status, 403);
    }

    #[tokio::test]
    async fn replay_session_round_trip() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://example.com/test".to_string(),
            headers: vec![],
            body: None,
            expected_status: Some(200),
        }];

        let config = ushio::replay::ReplayConfig::default();
        let session = ushio::replay::replay(&requests, &mock_server.uri(), config)
            .await
            .unwrap();

        // Save and reload
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap();
        ushio::replay::save_session(&session, path).unwrap();
        let loaded = ushio::replay::load_session(path).unwrap();

        assert_eq!(loaded.total_requests, session.total_requests);
        assert_eq!(loaded.successful, session.successful);
        assert_eq!(loaded.results.len(), session.results.len());
        assert_eq!(loaded.results[0].status, 200);
        assert_eq!(loaded.results[0].body.as_deref(), Some("ok"));
    }

    #[tokio::test]
    async fn replay_no_body_mode() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("body content"))
            .mount(&mock_server)
            .await;

        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://example.com/test".to_string(),
            headers: vec![],
            body: None,
            expected_status: Some(200),
        }];

        let mut config = ushio::replay::ReplayConfig::default();
        config.capture_body = false;
        let session = ushio::replay::replay(&requests, &mock_server.uri(), config)
            .await
            .unwrap();

        assert!(session.results[0].body.is_none());
        assert!(session.results[0].body_size > 0);
    }

    #[tokio::test]
    async fn replay_concurrent_preserves_order() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/a"))
            .respond_with(ResponseTemplate::new(200).set_body_string("a"))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/b"))
            .respond_with(ResponseTemplate::new(201).set_body_string("b"))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/c"))
            .respond_with(ResponseTemplate::new(202).set_body_string("c"))
            .mount(&mock_server)
            .await;

        let requests = vec![
            ushio::capture::CapturedRequest {
                method: "GET".to_string(),
                url: "https://example.com/a".to_string(),
                headers: vec![],
                body: None,
                expected_status: Some(200),
            },
            ushio::capture::CapturedRequest {
                method: "GET".to_string(),
                url: "https://example.com/b".to_string(),
                headers: vec![],
                body: None,
                expected_status: Some(201),
            },
            ushio::capture::CapturedRequest {
                method: "GET".to_string(),
                url: "https://example.com/c".to_string(),
                headers: vec![],
                body: None,
                expected_status: Some(202),
            },
        ];

        let mut config = ushio::replay::ReplayConfig::default();
        config.concurrency = 3;
        let session = ushio::replay::replay(&requests, &mock_server.uri(), config)
            .await
            .unwrap();

        assert_eq!(session.total_requests, 3);
        assert_eq!(session.successful, 3);
        // Results must be in order regardless of concurrency
        assert_eq!(session.results[0].request_index, 0);
        assert_eq!(session.results[0].status, 200);
        assert_eq!(session.results[1].request_index, 1);
        assert_eq!(session.results[1].status, 201);
        assert_eq!(session.results[2].request_index, 2);
        assert_eq!(session.results[2].status, 202);
    }
}

mod diff_engine {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn diff_detects_status_difference() {
        let server_a = MockServer::start().await;
        let server_b = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path("/api"))
            .respond_with(ResponseTemplate::new(403).set_body_string("blocked"))
            .mount(&server_b)
            .await;

        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://example.com/api".to_string(),
            headers: vec![],
            body: None,
            expected_status: Some(200),
        }];

        let config = ushio::replay::ReplayConfig::default();
        let session_a = ushio::replay::replay(&requests, &server_a.uri(), config.clone())
            .await
            .unwrap();
        let session_b = ushio::replay::replay(&requests, &server_b.uri(), config)
            .await
            .unwrap();

        let summary = ushio::diff::diff_sessions(&session_a, &session_b);
        assert_eq!(summary.total_requests, 1);
        assert_eq!(summary.different, 1);
        assert_eq!(summary.identical, 0);
        assert!(summary.diffs[0].status_diff.is_some());
        assert!(summary.diffs[0].waf_diff.is_some());
    }

    #[tokio::test]
    async fn diff_detects_body_difference() {
        let server_a = MockServer::start().await;
        let server_b = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/page"))
            .respond_with(ResponseTemplate::new(200).set_body_string("version A"))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path("/page"))
            .respond_with(ResponseTemplate::new(200).set_body_string("version B"))
            .mount(&server_b)
            .await;

        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://example.com/page".to_string(),
            headers: vec![],
            body: None,
            expected_status: Some(200),
        }];

        let config = ushio::replay::ReplayConfig::default();
        let session_a = ushio::replay::replay(&requests, &server_a.uri(), config.clone())
            .await
            .unwrap();
        let session_b = ushio::replay::replay(&requests, &server_b.uri(), config)
            .await
            .unwrap();

        let summary = ushio::diff::diff_sessions(&session_a, &session_b);
        assert_eq!(summary.different, 1);
        assert_eq!(summary.body_diffs, 1);
        assert!(summary.diffs[0].body_diff.is_some());
    }

    #[tokio::test]
    async fn diff_identical_is_clean() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("same"))
            .mount(&server)
            .await;

        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://example.com/test".to_string(),
            headers: vec![],
            body: None,
            expected_status: Some(200),
        }];

        let config = ushio::replay::ReplayConfig::default();
        let session_a = ushio::replay::replay(&requests, &server.uri(), config.clone())
            .await
            .unwrap();
        let session_b = ushio::replay::replay(&requests, &server.uri(), config)
            .await
            .unwrap();

        let summary = ushio::diff::diff_sessions(&session_a, &session_b);
        assert_eq!(summary.identical, 1);
        assert_eq!(summary.different, 0);
        assert!(summary.diffs.is_empty());
    }
}

mod new_features {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn replay_computes_body_hash() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/hash"))
            .respond_with(ResponseTemplate::new(200).set_body_string("hello world"))
            .mount(&mock_server)
            .await;

        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://example.com/hash".to_string(),
            headers: vec![],
            body: None,
            expected_status: Some(200),
        }];

        let config = ushio::replay::ReplayConfig::default();
        let session = ushio::replay::replay(&requests, &mock_server.uri(), config)
            .await
            .unwrap();

        assert!(session.results[0].body_hash.is_some());
        // SHA256 of "hello world"
        let hash = session.results[0].body_hash.as_ref().unwrap();
        assert_eq!(hash.len(), 64); // hex-encoded SHA256
    }

    #[tokio::test]
    async fn replay_hash_differs_when_body_differs() {
        let server_a = MockServer::start().await;
        let server_b = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/data"))
            .respond_with(ResponseTemplate::new(200).set_body_string("version-a"))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path("/data"))
            .respond_with(ResponseTemplate::new(200).set_body_string("version-b"))
            .mount(&server_b)
            .await;

        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://example.com/data".to_string(),
            headers: vec![],
            body: None,
            expected_status: Some(200),
        }];

        let config = ushio::replay::ReplayConfig::default();
        let sa = ushio::replay::replay(&requests, &server_a.uri(), config.clone())
            .await
            .unwrap();
        let sb = ushio::replay::replay(&requests, &server_b.uri(), config)
            .await
            .unwrap();

        assert_ne!(sa.results[0].body_hash, sb.results[0].body_hash);
    }

    #[tokio::test]
    async fn error_kind_is_populated_on_failure() {
        // Connect to a port that nothing is listening on
        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://127.0.0.1:1/fail".to_string(),
            headers: vec![],
            body: None,
            expected_status: Some(200),
        }];

        let mut config = ushio::replay::ReplayConfig::default();
        config.timeout = std::time::Duration::from_secs(2);
        let session = ushio::replay::replay(&requests, "https://127.0.0.1:1", config)
            .await
            .unwrap();

        assert!(session.results[0].error.is_some());
        assert!(session.results[0].error_kind.is_some());
    }

    #[tokio::test]
    async fn session_metadata_is_populated() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/meta"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let requests = vec![ushio::capture::CapturedRequest {
            method: "GET".to_string(),
            url: "https://example.com/meta".to_string(),
            headers: vec![],
            body: None,
            expected_status: Some(200),
        }];

        let mut config = ushio::replay::ReplayConfig::default();
        config.capture_source = Some("test.har".to_string());
        let session = ushio::replay::replay(&requests, &mock_server.uri(), config)
            .await
            .unwrap();

        assert_eq!(session.meta.ushio_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(session.meta.capture_source.as_deref(), Some("test.har"));
    }

    #[tokio::test]
    async fn junit_output_is_valid_xml() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/ok"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/fail"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let requests = vec![
            ushio::capture::CapturedRequest {
                method: "GET".to_string(),
                url: "https://example.com/ok".to_string(),
                headers: vec![],
                body: None,
                expected_status: Some(200),
            },
            ushio::capture::CapturedRequest {
                method: "GET".to_string(),
                url: "https://example.com/fail".to_string(),
                headers: vec![],
                body: None,
                expected_status: Some(200),
            },
        ];

        let config = ushio::replay::ReplayConfig::default();
        let session = ushio::replay::replay(&requests, &mock_server.uri(), config)
            .await
            .unwrap();

        let junit = ushio::output::print_replay_junit(&session);
        assert!(junit.starts_with("<?xml"));
        assert!(junit.contains("<testsuite"));
        assert!(junit.contains("<testcase"));
        assert!(junit.contains("<failure"));
        assert!(junit.contains("</testsuite>"));
    }

    #[tokio::test]
    async fn fetch_remote_capture_from_mock() {
        let mock_server = MockServer::start().await;

        let capture_json = serde_json::json!({
            "version": "1.0",
            "source": "remote-test",
            "requests": [
                {
                    "method": "GET",
                    "url": "https://example.com/test",
                    "headers": [],
                    "body": null,
                    "expected_status": 200
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/api/logs"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(capture_json.to_string())
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        let url = format!("{}/api/logs", mock_server.uri());
        let requests = ushio::proxy::fetch_remote_capture(&url, false)
            .await
            .unwrap();

        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "GET");
    }
}
