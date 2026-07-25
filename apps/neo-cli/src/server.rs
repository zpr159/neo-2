use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::bootstrap::NeoSystem;
use crate::error::{CliError, CliResult};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn json_response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        body.len()
    )
}

fn not_found() -> String {
    json_response("404 Not Found", r#"{"error":"not found"}"#)
}

async fn handle_request(
    request_line: &str,
    _headers: &[String],
    body: &str,
    system: &Arc<NeoSystem>,
) -> String {
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return json_response("400 Bad Request", r#"{"error":"bad request"}"#);
    }
    let method = parts[0];
    let path = parts[1];

    match (method, path) {
        ("GET", "/health") => {
            let uptime = system.start_time.elapsed().as_millis() as u64;
            let resp_body = serde_json::json!({
                "status": "ok",
                "version": VERSION,
                "uptime_ms": uptime,
            });
            json_response("200 OK", &resp_body.to_string())
        }
        ("GET", "/metrics") => {
            let stats = system.runtime.statistics();
            let resp_body = serde_json::json!({
                "state": format!("{:?}", stats.state),
                "uptime_ms": stats.uptime_ms,
                "services_registered": stats.services_registered,
                "services_running": stats.services_running,
                "tasks_scheduled": stats.tasks_scheduled,
                "events_published": stats.events_published,
                "plugins_loaded": stats.plugins_loaded,
            });
            json_response("200 OK", &resp_body.to_string())
        }
        ("GET", "/api/v1/status") => {
            let summary = system.executive.inspect_execution();
            match serde_json::to_value(&summary) {
                Ok(v) => json_response("200 OK", &v.to_string()),
                Err(e) => json_response("500 Internal Server Error",
                    &format!(r#"{{"error":"serialization failed: {e}"}}"#)),
            }
        }
        ("GET", "/api/v1/goals") => {
            let goals = system.executive.goal_manager().all_goals();
            match serde_json::to_value(&goals) {
                Ok(v) => json_response("200 OK", &v.to_string()),
                Err(e) => json_response("500 Internal Server Error",
                    &format!(r#"{{"error":"serialization failed: {e}"}}"#)),
            }
        }
        ("GET", "/api/v1/tasks") => {
            let tasks = system.executive.task_manager().all_tasks();
            match serde_json::to_value(&tasks) {
                Ok(v) => json_response("200 OK", &v.to_string()),
                Err(e) => json_response("500 Internal Server Error",
                    &format!(r#"{{"error":"serialization failed: {e}"}}"#)),
            }
        }
        ("GET", "/api/v1/config") => {
            match serde_json::to_value(&system.config) {
                Ok(v) => json_response("200 OK", &v.to_string()),
                Err(e) => json_response("500 Internal Server Error",
                    &format!(r#"{{"error":"serialization failed: {e}"}}"#)),
            }
        }
        ("POST", "/api/v1/reasoning") => {
            let parsed: serde_json::Value = match serde_json::from_str(body) {
                Ok(v) => v,
                Err(_) => return json_response("400 Bad Request", r#"{"error":"invalid JSON"}"#),
            };
            let query = match parsed.get("query").and_then(|q| q.as_str()) {
                Some(q) => q,
                None => return json_response("400 Bad Request", r#"{"error":"missing query field"}"#),
            };

            match &system.reasoning {
                Some(reasoner) => {
                    let request = neo_reasoning::ReasoningRequest::new(query.to_string());
                    match reasoner.start_session(request.clone()).await {
                        Ok(session_id) => {
                            match reasoner.execute_session(session_id, request).await {
                                Ok(response) => {
                                    let resp_body = serde_json::json!({
                                        "session_id": response.session_id,
                                        "conclusion": response.conclusion,
                                        "confidence": response.confidence,
                                        "strategy_used": response.strategy_used,
                                        "reasoning_depth": response.reasoning_depth,
                                        "latency_ms": response.latency_ms,
                                        "explanation": response.explanation,
                                    });
                                    json_response("200 OK", &resp_body.to_string())
                                }
                                Err(e) => json_response("500 Internal Server Error",
                                    &format!(r#"{{"error":"reasoning failed: {e}"}}"#)),
                            }
                        }
                        Err(e) => json_response("500 Internal Server Error",
                            &format!(r#"{{"error":"session start failed: {e}"}}"#)),
                    }
                }
                None => json_response("503 Service Unavailable",
                    r#"{"error":"reasoning system not available"}"#),
            }
        }
        _ => not_found(),
    }
}

pub async fn run(
    system: &Arc<NeoSystem>,
    bind: &str,
    port: u16,
) -> CliResult<()> {
    let addr = format!("{bind}:{port}");
    let listener = TcpListener::bind(&addr).await.map_err(|e| {
        CliError::server(format!("failed to bind to {addr}: {e}"))
    })?;

    println!("Listening on http://{addr}");
    println!("Endpoints:");
    println!("  GET  /health");
    println!("  GET  /metrics");
    println!("  GET  /api/v1/status");
    println!("  GET  /api/v1/goals");
    println!("  GET  /api/v1/tasks");
    println!("  GET  /api/v1/config");
    println!("  POST /api/v1/reasoning");
    println!();

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received");
        let _ = shutdown_tx.send(true);
    });

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((mut stream, _peer)) => {
                        let mut buf = vec![0u8; 8192];
                        let n = match stream.read(&mut buf).await {
                            Ok(n) if n > 0 => n,
                            _ => continue,
                        };
                        let request_bytes = &buf[..n];

                        let request_str = String::from_utf8_lossy(request_bytes);
                        let mut lines = request_str.split("\r\n");

                        let request_line = match lines.next() {
                            Some(l) => l.to_string(),
                            None => continue,
                        };

                        let mut headers: Vec<String> = Vec::new();
                        let mut body_start = 0;
                        let full_str = request_str.as_bytes();
                        let mut total_read = request_line.len() + 2;
                        for line in lines {
                            if line.is_empty() {
                                body_start = total_read;
                                break;
                            }
                            headers.push(line.to_string());
                            total_read += line.len() + 2;
                        }

                        let body = if body_start < full_str.len() {
                            String::from_utf8_lossy(&full_str[body_start..]).to_string()
                        } else {
                            String::new()
                        };

                        let response = handle_request(&request_line, &headers, &body, system).await;
                        let _ = stream.write_all(response.as_bytes()).await;
                    }
                    Err(e) => {
                        tracing::error!("accept error: {e}");
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                println!("Shutting down server...");
                break;
            }
        }
    }

    Ok(())
}
