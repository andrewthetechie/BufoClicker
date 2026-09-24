//! Browser adapter: fetch the three runtime catalogs against `document.baseURI`
//! and hand the bytes to the game crate for parse + validation.

use game::catalog::{CatalogError, Catalogs};
use gloo_net::http::Request;

#[derive(Debug)]
pub enum LoadError {
    Fetch(String),
    Validation(Vec<CatalogError>),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Fetch(m) => write!(f, "Failed to load game content: {m}"),
            LoadError::Validation(errs) => {
                write!(f, "Game content is invalid: ")?;
                for (i, e) in errs.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{e}")?;
                }
                Ok(())
            }
        }
    }
}

/// The document base URL (set by `<base data-trunk-public-url/>`). Resolving
/// data paths against this is what makes the site work at a path prefix.
fn base_url() -> String {
    let window = web_sys::window().expect("no window");
    let doc = window.document().expect("no document");
    if let Ok(Some(base)) = doc.base_uri() {
        return base;
    }
    let location = window.location();
    if let Ok(href) = location.href() {
        return href;
    }
    "/".to_string()
}

fn resolve(path: &str) -> String {
    let base = base_url();
    if base.ends_with('/') {
        format!("{base}{path}")
    } else {
        format!("{base}/{path}")
    }
}

async fn fetch_text(path: &str) -> Result<String, LoadError> {
    let url = resolve(path);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| LoadError::Fetch(format!("{path}: {e}")))?;
    if !resp.ok() {
        return Err(LoadError::Fetch(format!("{path}: HTTP {}", resp.status())));
    }
    resp.text()
        .await
        .map_err(|e| LoadError::Fetch(format!("{path}: {e}")))
}

/// Fetch and validate all three catalogs. Errors here must block startup and
/// show a retry screen — never an empty shop.
pub async fn load_catalogs() -> Result<Catalogs, LoadError> {
    let generators = fetch_text("assets/data/generators.json").await?;
    let upgrades = fetch_text("assets/data/upgrades.json").await?;
    let achievements = fetch_text("assets/data/achievements.json").await?;
    Catalogs::from_json(&generators, &upgrades, &achievements).map_err(LoadError::Validation)
}
