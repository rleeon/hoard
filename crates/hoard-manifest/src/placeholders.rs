//! Resolve `{TOKEN}` placeholders in [`crate::SavePath::location`] strings.
//!
//! Tokens map to Windows Known Folders via `SHGetKnownFolderPath`. On
//! non-Windows hosts we fall back to a `PlaceholderEnv` lookup table so the
//! resolver can be unit-tested without touching the real shell API.

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PlaceholderError {
    #[error("unknown placeholder `{0}` in path")]
    UnknownToken(String),
    #[error("path placeholder `{0}` not available on this system")]
    Unavailable(String),
    #[error("malformed path `{0}` — unmatched `{{` or `}}`")]
    Malformed(String),
    #[cfg(windows)]
    #[error("Windows shell API failed for `{token}`: {message}")]
    Shell { token: String, message: String },
}

/// Lookup environment for placeholder resolution. On Windows, the default
/// implementation calls `SHGetKnownFolderPath`. Tests can supply a mock map.
pub trait PlaceholderEnv {
    fn lookup(&self, token: &str) -> Result<PathBuf, PlaceholderError>;
}

/// Default Windows resolver via Known Folders.
#[cfg(windows)]
pub struct WindowsEnv;

#[cfg(windows)]
impl PlaceholderEnv for WindowsEnv {
    fn lookup(&self, token: &str) -> Result<PathBuf, PlaceholderError> {
        use windows::core::GUID;
        use windows::Win32::UI::Shell::{
            FOLDERID_Documents, FOLDERID_LocalAppData, FOLDERID_LocalAppDataLow, FOLDERID_Profile,
            FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86, FOLDERID_Public,
            FOLDERID_RoamingAppData, FOLDERID_SavedGames, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
        };

        let folderid: GUID = match token {
            "APPDATA" => FOLDERID_RoamingAppData,
            "LOCALAPPDATA" => FOLDERID_LocalAppData,
            // Unity games (Hollow Knight, Outer Wilds, Vampire Survivors, …)
            // dump saves under %USERPROFILE%\AppData\LocalLow.
            "LOCALAPPDATALOW" => FOLDERID_LocalAppDataLow,
            "USERPROFILE" => FOLDERID_Profile,
            "DOCUMENTS" => FOLDERID_Documents,
            "SAVEDGAMES" => FOLDERID_SavedGames,
            "PUBLIC" => FOLDERID_Public,
            "PROGRAMFILES" => FOLDERID_ProgramFiles,
            "PROGRAMFILESX86" => FOLDERID_ProgramFilesX86,
            other => return Err(PlaceholderError::UnknownToken(other.to_string())),
        };

        // SAFETY: SHGetKnownFolderPath is safe to call with a static GUID
        // and `KF_FLAG_DEFAULT`. The returned PWSTR is freed via CoTaskMemFree
        // by the windows-rs PWSTR Drop impl.
        let pwstr =
            unsafe { SHGetKnownFolderPath(&folderid, KF_FLAG_DEFAULT, None) }.map_err(|e| {
                PlaceholderError::Shell {
                    token: token.to_string(),
                    message: e.to_string(),
                }
            })?;

        // PWSTR -> OsString via wide-string round trip. windows-rs gives us
        // a thin wrapper that already has `to_string()` for UTF-16 → UTF-8.
        let s = unsafe { pwstr.to_string() }.map_err(|e| PlaceholderError::Shell {
            token: token.to_string(),
            message: format!("UTF-16 decode: {e}"),
        })?;
        Ok(PathBuf::from(s))
    }
}

/// Hash-map backed resolver for tests and non-Windows hosts. Insert the
/// tokens you want to support; lookup of an absent token returns
/// [`PlaceholderError::Unavailable`].
pub struct MapEnv(pub HashMap<String, PathBuf>);

impl PlaceholderEnv for MapEnv {
    fn lookup(&self, token: &str) -> Result<PathBuf, PlaceholderError> {
        self.0
            .get(token)
            .cloned()
            .ok_or_else(|| PlaceholderError::Unavailable(token.to_string()))
    }
}

/// Resolve `{TOKEN}` placeholders inside `input` using `env`. Tokens are
/// case-sensitive and must be wrapped in single curly braces. Literal
/// braces are not supported (no manifest needs them today; we'll add
/// `{{` escaping if that ever changes).
///
/// ```
/// use std::collections::HashMap;
/// use std::path::PathBuf;
/// use hoard_manifest::placeholders::{resolve_placeholders, MapEnv};
///
/// let env = MapEnv(HashMap::from([
///     ("APPDATA".to_string(), PathBuf::from("C:/Users/x/AppData/Roaming")),
/// ]));
/// let p = resolve_placeholders("{APPDATA}/Game/Saves", &env).unwrap();
/// assert_eq!(p, PathBuf::from("C:/Users/x/AppData/Roaming/Game/Saves"));
/// ```
pub fn resolve_placeholders(
    input: &str,
    env: &dyn PlaceholderEnv,
) -> Result<PathBuf, PlaceholderError> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after_brace = &rest[start + 1..];
        let end = after_brace
            .find('}')
            .ok_or_else(|| PlaceholderError::Malformed(input.to_string()))?;
        let token = &after_brace[..end];
        if token.is_empty() || token.contains('{') {
            return Err(PlaceholderError::Malformed(input.to_string()));
        }
        let resolved = env.lookup(token)?;
        out.push_str(&resolved.to_string_lossy());
        rest = &after_brace[end + 1..];
    }
    out.push_str(rest);
    Ok(PathBuf::from(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> MapEnv {
        MapEnv(HashMap::from([
            (
                "APPDATA".to_string(),
                PathBuf::from("C:/Users/x/AppData/Roaming"),
            ),
            (
                "DOCUMENTS".to_string(),
                PathBuf::from("C:/Users/x/Documents"),
            ),
        ]))
    }

    #[test]
    fn resolves_single_token() {
        let p = resolve_placeholders("{APPDATA}/StardewValley/Saves", &env()).unwrap();
        assert_eq!(
            p,
            PathBuf::from("C:/Users/x/AppData/Roaming/StardewValley/Saves")
        );
    }

    #[test]
    fn resolves_multiple_tokens() {
        let p = resolve_placeholders("{APPDATA}/a/{DOCUMENTS}/b", &env()).unwrap();
        assert_eq!(
            p,
            PathBuf::from("C:/Users/x/AppData/Roaming/a/C:/Users/x/Documents/b")
        );
    }

    #[test]
    fn passes_through_paths_with_no_tokens() {
        let p = resolve_placeholders("/no/tokens/here", &env()).unwrap();
        assert_eq!(p, PathBuf::from("/no/tokens/here"));
    }

    #[test]
    fn unknown_token_errors() {
        let err = resolve_placeholders("{NOPE}/x", &env()).unwrap_err();
        assert!(matches!(err, PlaceholderError::Unavailable(_)));
    }

    #[test]
    fn unmatched_brace_errors() {
        let err = resolve_placeholders("{APPDATA/x", &env()).unwrap_err();
        assert!(matches!(err, PlaceholderError::Malformed(_)));
    }

    #[test]
    fn empty_token_errors() {
        let err = resolve_placeholders("{}/x", &env()).unwrap_err();
        assert!(matches!(err, PlaceholderError::Malformed(_)));
    }

    #[test]
    fn nested_brace_errors() {
        let err = resolve_placeholders("{A{B}}/x", &env()).unwrap_err();
        assert!(matches!(err, PlaceholderError::Malformed(_)));
    }
}
