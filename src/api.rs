use curl::easy::{Easy, List};

use crate::cargo::CargoDependency;

pub struct CratesIoResponse {
    pub repository: Option<String>,
    pub description: Option<String>,
    pub latest_version: String,
    pub latest_version_date: Option<String>,
    pub current_version_date: Option<String>,
}

impl CratesIoResponse {
    fn from_value(value: serde_json::Value, version: &str) -> Self {
        let data = value.get("crate").and_then(|c| c.as_object());

        Self {
            repository: data
                .and_then(|d| d.get("repository"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string()),
            description: data
                .and_then(|d| d.get("description"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string()),
            latest_version: data
                .and_then(|d| d.get("newest_version"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
                .expect("Unable to get latest version"),
            latest_version_date: data
                .and_then(|d| d.get("updated_at"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string()),
            current_version_date: value["versions"].as_array().and_then(|v| {
                v.into_iter()
                    .find(|v| v["num"].as_str().unwrap_or("") == version)
                    .and_then(|v| v["updated_at"].as_str())
                    .map(|s| s.to_string())
            }),
        }
    }
}

pub fn get_latest_version(
    CargoDependency { name, version }: &CargoDependency,
) -> Result<CratesIoResponse, Box<dyn std::error::Error>> {
    let mut body = vec![];

    let url = format!("https://crates.io/api/v1/crates/{name}");
    let mut handle = Easy::new();

    handle.get(true)?;
    handle.url(&url)?;
    let mut headers = List::new();
    headers.append("User-Agent: cargo-interactive-update (tbd under development)")?;
    handle.http_headers(headers)?;

    {
        let mut transfer = handle.transfer();
        transfer
            .write_function(|data| {
                body.extend_from_slice(data);
                Ok(data.len())
            })
            .unwrap();
        transfer.perform().unwrap();
    }
    let response = if body.is_empty() {
        "{}".parse()?
    } else {
        serde_json::from_slice(&body)?
    };

    Ok(CratesIoResponse::from_value(response, version))
}
