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
        let versions = value.get("versions").and_then(|c| c.as_array());

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
                .unwrap_or_else(|| version.to_string()),
            latest_version_date: data
                .and_then(|d| d.get("updated_at"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string()),
            current_version_date: versions.and_then(|v| {
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
    let mut headers = List::new();
    headers.append("User-Agent: cargo-interactive-update (tbd under development)")?;

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
                "repository": "https://github.com/user/repo",
                "description": "A description",
                "max_version": "0.1.0",
                "newest_version": "0.1.0",
                "updated_at": "2023-07-01T00:00:00Z",
            },
            "versions": [
                {
                    "num": "0.1.0",
                    "updated_at": "2023-07-01T00:00:00Z"
                }
            ]
        });

        let response = CratesIoResponse::from_value(response, "0.1.0");

        assert_eq!(
            response.repository,
            Some("https://github.com/user/repo".to_string())
        );
        assert_eq!(response.description, Some("A description".to_string()));
        assert_eq!(response.latest_version, "0.1.0");
        assert_eq!(
            response.latest_version_date,
            Some("2023-07-01T00:00:00Z".to_string())
        );
        assert_eq!(
            response.current_version_date,
            Some("2023-07-01T00:00:00Z".to_string())
        );
    }

    #[test]
    fn test_crates_io_response_from_value_only_newest_version() {
        let response = serde_json::json!({
            "crate": {
                "newest_version": "0.1.0",
            },
        });

        let response = CratesIoResponse::from_value(response, "0.1.0");

        assert_eq!(response.repository, None);
        assert_eq!(response.description, None);
        assert_eq!(response.latest_version, "0.1.0");
        assert_eq!(response.latest_version_date, None);
        assert_eq!(response.current_version_date, None);
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
