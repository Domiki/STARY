//! Looks on GitHub for a release newer than the running program.

use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};

const RELEASES_URL: &str = "https://api.github.com/repos/Domiki/STARY/releases?per_page=30";
/// Overrides the address, for tests with a server of one's own.
const URL_VARIABLE: &str = "STARY_RELEASES_URL";
const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Deserialize)]
pub struct Release {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    published_at: Option<String>,
}

/// What the screen shows about a newer release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NewerRelease {
    pub version: String,
    pub name: String,
    pub notes: String,
    pub url: String,
    pub published: Option<String>,
    pub prerelease: bool,
}

fn version_of(tag: &str) -> Option<Version> {
    Version::parse(tag.trim().trim_start_matches(['v', 'V'])).ok()
}

/// The newest of the releases that is newer than `current`; drafts and tags that are not versions
/// are left out.
pub fn newest_after(current: &str, releases: Vec<Release>) -> Option<NewerRelease> {
    let current = version_of(current)?;
    releases
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| version_of(&release.tag_name).map(|version| (version, release)))
        .filter(|(version, _)| *version > current)
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(version, release)| NewerRelease {
            name: release
                .name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| release.tag_name.clone()),
            version: version.to_string(),
            notes: release.body.unwrap_or_default(),
            url: release.html_url,
            published: release.published_at,
            prerelease: release.prerelease,
        })
}

fn fetch() -> Result<Vec<Release>, String> {
    let url = std::env::var(URL_VARIABLE).unwrap_or_else(|_| RELEASES_URL.to_string());
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();
    let mut response = agent
        .get(&url)
        .header("User-Agent", concat!("STARY/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| error.to_string())?;
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

/// Asks GitHub for the releases and picks the newest one after `current`.
pub fn check(current: &str) -> Result<Option<NewerRelease>, String> {
    fetch().map(|releases| newest_after(current, releases))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn releases(json: &str) -> Vec<Release> {
        serde_json::from_str(json).unwrap()
    }

    const LIST: &str = r#"[
        {"tag_name": "v0.2.0", "name": "STARY 0.2.0", "body": "- faster", "html_url": "https://github.com/x/y/releases/tag/v0.2.0", "draft": false, "prerelease": false, "published_at": "2026-12-01T00:00:00Z"},
        {"tag_name": "v0.3.0", "name": "", "body": null, "html_url": "https://github.com/x/y/releases/tag/v0.3.0", "draft": true, "prerelease": false},
        {"tag_name": "v0.1.1-pre", "name": "pre", "body": "b", "html_url": "https://github.com/x/y/releases/tag/v0.1.1-pre", "prerelease": true},
        {"tag_name": "v0.1.0", "name": "STARY 0.1.0", "body": "first", "html_url": "https://github.com/x/y/releases/tag/v0.1.0", "prerelease": true},
        {"tag_name": "nightly", "name": "nightly", "body": "", "html_url": "https://github.com/x/y/releases/tag/nightly"}
    ]"#;

    #[test]
    fn the_newest_release_after_the_running_version_is_found() {
        let found = newest_after("0.1.0", releases(LIST)).unwrap();
        assert_eq!(found.version, "0.2.0");
        assert_eq!(found.name, "STARY 0.2.0");
        assert_eq!(found.notes, "- faster");
        assert!(found.url.ends_with("v0.2.0"));
        assert!(!found.prerelease);
    }

    #[test]
    fn drafts_and_tags_that_are_not_versions_are_left_out() {
        let found = newest_after("0.1.0", releases(LIST)).unwrap();
        assert_ne!(found.version, "0.3.0");
    }

    #[test]
    fn nothing_is_offered_when_the_program_is_up_to_date() {
        assert_eq!(newest_after("0.2.0", releases(LIST)), None);
        assert_eq!(newest_after("0.9.0", releases(LIST)), None);
    }

    #[test]
    fn a_pre_release_is_older_than_its_final_version() {
        let list = releases(r#"[{"tag_name": "v0.1.0-pre", "html_url": "u", "prerelease": true}]"#);
        assert_eq!(newest_after("0.1.0", list), None);
        let list = releases(r#"[{"tag_name": "v0.1.0", "html_url": "u"}]"#);
        assert!(newest_after("0.1.0-pre", list).is_some());
    }

    #[test]
    fn a_release_without_a_name_is_called_by_its_tag() {
        let list = releases(r#"[{"tag_name": "v0.2.0", "name": "", "html_url": "u"}]"#);
        assert_eq!(newest_after("0.1.0", list).unwrap().name, "v0.2.0");
    }

    #[test]
    fn a_current_version_that_cannot_be_read_offers_nothing() {
        assert_eq!(newest_after("dev", releases(LIST)), None);
    }
}
