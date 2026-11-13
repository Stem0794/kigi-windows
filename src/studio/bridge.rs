//! Versioned, read-only Studio protocol. No file paths or system commands cross it.
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub protocol_version: u8,
    pub id: String,
    pub method: String,
    pub params: EmptyParams,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyParams {}

pub fn parse(raw: &str) -> Result<Request, Value> {
    if raw.len() > 64 * 1024 {
        return Err(error(
            None,
            "PAYLOAD_TOO_LARGE",
            "Requête trop volumineuse.",
        ));
    }
    let request: Request = serde_json::from_str(raw)
        .map_err(|_| error(None, "INVALID_REQUEST", "Requête invalide."))?;
    if request.id.is_empty()
        || request.id.len() > 64
        || !request
            .id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
    {
        return Err(error(
            None,
            "INVALID_REQUEST",
            "Identifiant de requête invalide.",
        ));
    }
    if request.protocol_version != 1 {
        return Err(error(
            Some(&request.id),
            "UNSUPPORTED_VERSION",
            "Version du protocole incompatible.",
        ));
    }
    // EmptyParams is deliberately strict. Mutations arrive in a later phase.
    let _ = &request.params;
    if !matches!(
        request.method.as_str(),
        "app.bootstrap"
            | "spaces.list"
            | "permissions.get"
            | "diagnostics.summary"
            | "app.openNativeFallback"
    ) {
        return Err(error(
            Some(&request.id),
            "UNKNOWN_METHOD",
            "Commande indisponible dans cette version.",
        ));
    }
    Ok(request)
}
pub fn permitted_document(main_frame: bool, actual: &str, expected: &str) -> bool {
    main_frame && !expected.is_empty() && actual == expected
}
pub fn success(id: &str, data: Value) -> Value {
    json!({"protocolVersion":1,"id":id,"ok":true,"data":data})
}
pub fn error(id: Option<&str>, code: &str, message: &str) -> Value {
    json!({"protocolVersion":1,"id":id,"ok":false,"error":{"code":code,"message":message,"recoverable":true}})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_rejects_mutations_unknown_fields_versions_and_large_messages() {
        let request = |method: &str, params: &str| {
            format!(r#"{{"protocolVersion":1,"id":"req-1","method":"{method}","params":{params}}}"#)
        };
        assert!(parse(&request("spaces.list", "{}")).is_ok());
        assert!(parse(&request("spaces.delete", "{}")).is_err());
        assert!(parse(&request("spaces.list", r#"{"path":"/tmp"}"#)).is_err());
        assert!(
            parse(&request("spaces.list", "{}").replace("Version\":1", "Version\":2")).is_err()
        );
        assert!(parse(&" ".repeat(65537)).is_err());
        assert!(parse("not json").is_err());
    }
    #[test]
    fn authority_is_exact_local_main_document_only() {
        let entry = "file:///Applications/Kigi.app/Contents/Resources/studio/index.html";
        assert!(permitted_document(true, entry, entry));
        assert!(!permitted_document(false, entry, entry));
        for url in [
            "https://example.org/",
            "file:///tmp/index.html",
            "file:///Applications/Kigi.app/Contents/Resources/studio/index.html?x=1",
        ] {
            assert!(!permitted_document(true, url, entry));
        }
    }
}
