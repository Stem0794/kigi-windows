//! Versioned Studio protocol; typed mutations, no generic file or code execution.
use crate::config::SavedWindow;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawRequest {
    protocol_version: u8,
    id: String,
    method: String,
    params: Value,
}
pub struct Request {
    pub id: String,
    pub method: String,
    pub command: Command,
}
#[derive(Debug)]
pub enum Command {
    Bootstrap,
    List,
    Permissions,
    Diagnostics,
    Fallback,
    SelectApp,
    Create(EditParams),
    Update(EditParams),
    Duplicate(IdParams),
    Delete(DeleteParams),
    SetDirty(DirtyParams),
    Close,
    Quit,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyParams {}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditParams {
    pub expected_revision: String,
    pub profile_id: Option<u32>,
    pub name: String,
    pub windows: [SavedWindow; 2],
    pub left_percent: Option<u8>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdParams {
    pub expected_revision: String,
    pub profile_id: u32,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteParams {
    pub expected_revision: String,
    pub profile_id: u32,
    pub confirmed: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirtyParams {
    pub dirty: bool,
}
#[derive(Debug)]
pub struct Failure {
    pub code: &'static str,
    pub message: String,
}
impl Failure {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("INVALID_PARAMS", message)
    }
}
fn parameters<T: DeserializeOwned>(value: Value, id: &str) -> Result<T, Value> {
    serde_json::from_value(value)
        .map_err(|_| error(Some(id), "INVALID_PARAMS", "Paramètres invalides."))
}
pub fn parse(raw: &str) -> Result<Request, Value> {
    if raw.len() > 64 * 1024 {
        return Err(error(
            None,
            "PAYLOAD_TOO_LARGE",
            "Requête trop volumineuse.",
        ));
    }
    let raw: RawRequest = serde_json::from_str(raw)
        .map_err(|_| error(None, "INVALID_REQUEST", "Requête invalide."))?;
    let id = &raw.id;
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
    {
        return Err(error(None, "INVALID_REQUEST", "Identifiant invalide."));
    }
    if raw.protocol_version != 1 {
        return Err(error(
            Some(id),
            "UNSUPPORTED_VERSION",
            "Version du protocole incompatible.",
        ));
    }
    let p = raw.params;
    let command = match raw.method.as_str() {
        "app.bootstrap"
        | "spaces.list"
        | "permissions.get"
        | "diagnostics.summary"
        | "app.openNativeFallback"
        | "spaces.selectApp"
        | "app.closeStudio"
        | "app.quit" => {
            let _: EmptyParams = parameters(p, id)?;
            match raw.method.as_str() {
                "app.bootstrap" => Command::Bootstrap,
                "spaces.list" => Command::List,
                "permissions.get" => Command::Permissions,
                "diagnostics.summary" => Command::Diagnostics,
                "app.openNativeFallback" => Command::Fallback,
                "spaces.selectApp" => Command::SelectApp,
                "app.closeStudio" => Command::Close,
                _ => Command::Quit,
            }
        }
        "spaces.create" | "spaces.update" => {
            let input: EditParams = parameters(p, id)?;
            if (raw.method == "spaces.create" && input.profile_id.is_some())
                || (raw.method == "spaces.update"
                    && input.profile_id.filter(|id| *id > 0).is_none())
            {
                return Err(error(
                    Some(id),
                    "INVALID_PARAMS",
                    "Identifiant d’espace invalide.",
                ));
            }
            if raw.method == "spaces.create" {
                Command::Create(input)
            } else {
                Command::Update(input)
            }
        }
        "spaces.duplicate" => Command::Duplicate(parameters(p, id)?),
        "spaces.delete" => {
            let input: DeleteParams = parameters(p, id)?;
            if !input.confirmed {
                return Err(error(
                    Some(id),
                    "CONFIRMATION_REQUIRED",
                    "Confirmez la suppression de cet espace.",
                ));
            }
            Command::Delete(input)
        }
        "app.setDraftDirty" => Command::SetDirty(parameters(p, id)?),
        _ => {
            return Err(error(
                Some(id),
                "UNKNOWN_METHOD",
                "Commande indisponible dans cette version.",
            ))
        }
    };
    Ok(Request {
        id: raw.id,
        method: raw.method,
        command,
    })
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
    fn protocol_rejects_unsafe_params_unknown_fields_versions_and_large_messages() {
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
