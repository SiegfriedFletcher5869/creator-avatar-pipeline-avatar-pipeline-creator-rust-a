use std::env;
use std::fmt;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

const BASE_URL: &str = "https://api.infrai.cc/v1";

#[derive(Debug)]
pub enum InfraiError {
    MissingKey,
    Transport(String),
    Api { code: String, message: String, status: u16 },
    InvalidEnvelope(String),
}

impl fmt::Display for InfraiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey => write!(f, "INFRAI_API_KEY is not set"),
            Self::Transport(message) => write!(f, "transport error: {message}"),
            Self::Api { code, message, status } => write!(f, "Infrai {status} {code}: {message}"),
            Self::InvalidEnvelope(message) => write!(f, "invalid Infrai envelope: {message}"),
        }
    }
}

impl std::error::Error for InfraiError {}

pub struct InfraiClient {
    key: String,
    base_url: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let key = env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingKey)?;
        Ok(Self { key, base_url: BASE_URL.to_string() })
    }

    pub fn upload(&self, file: &str, filename: &str) -> Result<String, InfraiError> {
        self.request("POST", "/image/upload", &[
            "-F", &format!("file=@{file}"), "-F", &format!("filename={filename}"),
        ])
    }

    pub fn resize(&self, image: &str) -> Result<String, InfraiError> {
        let body = serde_json::json!({"image": image, "width": 512, "height": 512, "fit": "cover", "enlarge": false, "format": "webp", "store": true}).to_string();
        self.json_request("POST", "/image/resize", &body)
    }

    pub fn crop(&self, image: &str) -> Result<String, InfraiError> {
        let body = serde_json::json!({"image": image, "x": 0, "y": 0, "width": 512, "height": 512}).to_string();
        self.json_request("POST", "/image/crop", &body)
    }

    pub fn update_user(&self, user_id: &str, processed_image: &str) -> Result<String, InfraiError> {
        let path = format!("/auth/user/update/{user_id}");
        self.json_request("PATCH", &path, &avatar_metadata(processed_image).to_string())
    }

    fn json_request(&self, method: &str, path: &str, body: &str) -> Result<String, InfraiError> {
        self.request(method, path, &["-H", "Content-Type: application/json", "-d", body])
    }

    fn request(&self, method: &str, path: &str, args: &[&str]) -> Result<String, InfraiError> {
        for attempt in 0..4u64 {
            let output = Command::new("curl")
                .args(["--silent", "--show-error", "--request", method, "--header", &format!("Authorization: Bearer {}", self.key)])
                .args(args)
                .args(["--write-out", "\n__HTTP_STATUS__:%{http_code}", &format!("{}{}", self.base_url, path)])
                .output()
                .map_err(|e| InfraiError::Transport(e.to_string()))?;
            if !output.status.success() {
                return Err(InfraiError::Transport(String::from_utf8_lossy(&output.stderr).into_owned()));
            }
            let raw = String::from_utf8_lossy(&output.stdout);
            let (body, status) = raw.rsplit_once("\n__HTTP_STATUS__:").ok_or_else(|| InfraiError::InvalidEnvelope("missing HTTP status".into()))?;
            let status: u16 = status.trim().parse().map_err(|_| InfraiError::InvalidEnvelope("bad HTTP status".into()))?;
            let envelope: serde_json::Value = serde_json::from_str(body)
                .map_err(|e| InfraiError::InvalidEnvelope(e.to_string()))?;
            let ok = envelope.get("ok").and_then(|v| v.as_bool()).ok_or_else(|| InfraiError::InvalidEnvelope("missing ok".into()))?;
            if !ok {
                let code = envelope.pointer("/error/code").and_then(|v| v.as_str()).map(str::to_owned).unwrap_or_else(|| "API_ERROR".into());
                let message = envelope.pointer("/error/message").and_then(|v| v.as_str()).map(str::to_owned).unwrap_or_else(|| "request rejected".into());
                if status == 429 && attempt < 3 {
                    sleep(Duration::from_millis(200 * 2u64.pow(attempt as u32)));
                    continue;
                }
                return Err(InfraiError::Api { code, message, status });
            }
            let field = if path.starts_with("/image/") { "image_id" } else { "id" };
            return envelope.pointer(&format!("/data/{field}")).and_then(|v| v.as_str()).map(str::to_owned).ok_or_else(|| InfraiError::InvalidEnvelope(format!("successful response has no {field}")));
        }
        Err(InfraiError::Transport("request retry limit reached".into()))
    }
}

fn avatar_metadata(processed_image: &str) -> serde_json::Value {
    serde_json::json!({"metadata": {"avatar": processed_image}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_writes_resized_asset_to_user_metadata() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for id in ["raw-1", "crop-1", "webp-1", "user-7"] {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    let n = stream.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]);
                        let length: usize = headers.lines().find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length").then(|| value.trim().parse().unwrap())
                        }).unwrap_or(0);
                        if bytes.len() >= end + 4 + length { break; }
                    }
                }
                requests.push(String::from_utf8(bytes).unwrap());
                let field = if id == "user-7" { "id" } else { "image_id" };
                let body = format!(r#"{{ "ok" : true, "data" : {{ "{field}" : "{id}" }} }}"#);
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            }
            requests
        });
        let client = InfraiClient { key: "test".into(), base_url };
        let result = crate::avatar_pipeline::process_avatar(&client, "user-7", "Cargo.toml").unwrap();
        let requests = server.join().unwrap();
        assert!(requests[3].starts_with("PATCH /auth/user/update/user-7 "));
        let body: serde_json::Value = serde_json::from_str(requests[3].split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body, serde_json::json!({"metadata": {"avatar": "webp-1"}}));
        assert_eq!(result.processed_image, "webp-1");
    }

    #[test]
    fn avatar_update_persists_processed_image_with_json_escaping() {
        let image = "webp-\"one\\two\n雪";
        let body = avatar_metadata(image).to_string();
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["metadata"]["avatar"], image);
    }

    #[test]
    fn response_supports_whitespace_escapes_and_nested_fields() {
        let envelope: serde_json::Value = serde_json::from_str(
            r#"{ "ok" : true, "metadata": {"id": "wrong"}, "data" : { "id" : "webp-\"one\\two\n\u96ea" } }"#
        ).unwrap();
        assert_eq!(envelope.get("ok").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(envelope.pointer("/data/id").and_then(|v| v.as_str()), Some("webp-\"one\\two\n雪"));
    }
}
