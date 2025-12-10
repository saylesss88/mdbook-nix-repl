use serde::{Deserialize, Serialize};
use std::io::Read;
use std::process::Command;
use tiny_http::{Response, Server};

#[derive(Deserialize)]
struct EvalRequest {
    code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvalResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn main() {
    let server = Server::http("0.0.0.0:8080").expect("start server");

    for request in server.incoming_requests() {
        if request.method().as_str() != "POST" {
            let resp = Response::from_string("Only POST").with_status_code(405);
            let _ = request.respond(resp);
            continue;
        }

        let mut body = String::new();
        if request.as_reader().read_to_string(&mut body).is_err() {
            let resp = Response::from_string("Bad request").with_status_code(400);
            let _ = request.respond(resp);
            continue;
        }

        let parsed: EvalRequest = match serde_json::from_str(&body) {
            Ok(p) => p,
            Err(_) => {
                let resp = Response::from_string("Invalid JSON").with_status_code(400);
                let _ = request.respond(resp);
                continue;
            }
        };

        let output = Command::new("nix")
            .arg("eval")
            .arg("--raw")
            .arg("--expr")
            .arg(&parsed.code)
            .output();

        let eval_resp = match output {
            Ok(out) => {
                if out.status.success() {
                    EvalResponse {
                        stdout: Some(String::from_utf8_lossy(&out.stdout).into_owned()),
                        error: None,
                    }
                } else {
                    EvalResponse {
                        stdout: None,
                        error: Some(String::from_utf8_lossy(&out.stderr).into_owned()),
                    }
                }
            }
            Err(e) => EvalResponse {
                stdout: None,
                error: Some(format!("Failed to run nix: {e}")),
            },
        };

        let json = serde_json::to_string(&eval_resp)
            .unwrap_or_else(|_| "{\"error\":\"internal serialization error\"}".to_string());

        let mut resp = Response::from_string(json);
        resp.add_header("Content-Type: application/json".parse().expect("header"));
        let _ = request.respond(resp);
    }
}
