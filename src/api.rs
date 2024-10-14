use curl::easy::{Easy, List};

use crate::cargo::CargoDependency;

pub struct CratesIoResponse {
    pub repository: Option<String>,
    pub description: Option<String>,
    pub latest_version: String,
    pub latest_version_date: Option<String>,
    pub current_version_date: Option<String>,
}

fn get_string_from_value(
    value: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<String> {
    value
        .and_then(|d| d.get(key))
        .and_then(|s| s.as_str())
        .map(|s| s.trim().to_string())
}

fn get_field_from_versions(
    versions: Option<&Vec<serde_json::Value>>,
    version: &str,
    key: &str,
) -> Option<String> {
    versions.and_then(|v| {
        v.iter()
            .find(|v| v.get("num").and_then(|v| v.as_str()).unwrap_or("") == version)
            .and_then(|v| v.get(key))
            .and_then(|s| s.as_str())
            .map(|s| s.trim().to_string())
    })
}

impl CratesIoResponse {
    fn from_value(value: serde_json::Value, version: &str) -> Self {
        let data = value.get("crate").and_then(|c| c.as_object());
        let versions = value.get("versions").and_then(|c| c.as_array());

        let latest_version = get_string_from_value(data, "max_stable_version")
            .unwrap_or_else(|| version.to_string());

        Self {
            repository: get_string_from_value(data, "repository"),
            description: get_string_from_value(data, "description"),
            latest_version_date: get_field_from_versions(versions, &latest_version, "updated_at"),
            current_version_date: get_field_from_versions(versions, version, "updated_at"),
            latest_version,
        }
    }
}

pub fn get_latest_version(
    CargoDependency { name, version }: &CargoDependency,
) -> Result<CratesIoResponse, Box<dyn std::error::Error>> {
    let mut headers = List::new();

    let package_name = env!("CARGO_PKG_NAME");
    let package_repository = env!("CARGO_PKG_REPOSITORY");

    // As required by the crates.io API - https://doc.rust-lang.org/cargo/reference/registry-web-api.html
    headers.append(&format!(
        "User-Agent: {package_name} ({package_repository})"
    ))?;

    let mut body = vec![];
    let mut handle = Easy::new();

    handle.get(true)?;
    handle.url(&format!("https://crates.io/api/v1/crates/{name}"))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crates_io_response_from_value() {
        let response = serde_json::json!({
            "crate": {
                "repository": "\thttps://github.com/user/repo ",
                "description": " A description\n ",
                "max_stable_version": "0.2.0",
            },
            "versions": [
                {
                    "num": "0.1.0",
                    "updated_at": " 2023-07-01T00:00:00Z\n"
                },
                {
                    "num": "0.2.0",
                    "updated_at": "2023-07-02T00:00:00Z"
                },
                {}
            ]
        });

        let response = CratesIoResponse::from_value(response, "0.1.0");

        assert_eq!(
            response.repository,
            Some("https://github.com/user/repo".to_string())
        );
        assert_eq!(response.description, Some("A description".to_string()));
        assert_eq!(response.latest_version, "0.2.0");
        assert_eq!(
            response.latest_version_date,
            Some("2023-07-02T00:00:00Z".to_string())
        );
        assert_eq!(
            response.current_version_date,
            Some("2023-07-01T00:00:00Z".to_string())
        );
    }

    #[test]
    fn test_crates_io_empty_response() {
        let response = serde_json::json!({});

        let response = CratesIoResponse::from_value(response, "0.1.0");

        assert_eq!(response.repository, None);
        assert_eq!(response.description, None);
        assert_eq!(response.latest_version, "0.1.0");
        assert_eq!(response.latest_version_date, None);
        assert_eq!(response.current_version_date, None);
    }
}
