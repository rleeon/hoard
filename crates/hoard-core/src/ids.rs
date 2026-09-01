//! Identity newtypes with the gate in `serde` (ADR 0021, C.3).
//!
//! The poison came in through *persisted data*, not through code building
//! things wrong. One save ended up tracked with its `game_slug` set to the
//! Windows account name, and since the account name is a path component of every
//! executable in the profile, any app at all triggered "you are playing" (the
//! phantom correlation of July 2026). A `GameSlug` that validates in `new()` but
//! derives `Deserialize` plainly would have stopped none of it: deserialisation
//! is a back door that builds the type without meeting the validator.
//!
//! So nothing here derives `Deserialize` plainly. Every newtype carries
//! `#[serde(try_from = "String")]`, which makes `Self::parse` the only way to
//! build one, including from JSON off the disk or the network. Skipping the gate
//! is impossible without editing this file.
//!
//! ## Two gates, two jobs
//!
//! | | [`parse`](GameSlug::parse) | [`repair`](GameSlug::repair) |
//! |---|---|---|
//! | Who uses it | `serde`: wire, IPC, new data | the loader of already-persisted state |
//! | Invalid value | error | re-derived or quarantined |
//!
//! The second exists because the poison is *already on disk*. A strict
//! `try_from` over `state.json` or the server's DB would brick existing installs
//! (the engine would not start). The ADR is explicit about it (C.3, "upgrade
//! hazard"): the strict gate protects what is new, what is old gets cleaned on
//! read by re-deriving, logging and flagging, and is never hard-rejected. The
//! durable cleanup lives in the Slice 5 migration.
//!
//! ## Why newtypes and not `String`
//!
//! Two independent layers:
//!
//! 1. `parse` guarantees the *shape* of the value: an empty slug, or one full of
//!    junk, does not exist as a `GameSlug`.
//! 2. The type system guarantees the *category*: `slug == username`, the real
//!    mistake behind the phantom correlation, is a compile error the moment both
//!    sides are [`GameSlug`] and [`Username`], because there is no `PartialEq`
//!    between them and no implicit conversion.

use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

/// Slug length cap, matching [`slugify`], which minted every slug in the
/// catalogue.
pub const MAX_SLUG_LEN: usize = 96;

/// Username length cap. Generous on purpose: the server never validated
/// usernames (`hoard-admin user create` takes whatever the operator types), so
/// this only stops the absurd.
pub const MAX_USERNAME_LEN: usize = 128;

/// Synthetic slug for time attributed to a day but not to a particular game.
/// It is playtime protocol vocabulary, declared separately by agent and server
/// today, so [`GameSlug::parse`] accepts it as reserved: not a game's name, but
/// a legitimate value of the field.
pub const OTHER_SLUG: &str = "__other__";

/// Shortest an identity token can be and still count in the generic match.
/// Below this (`gta`, `ori`, `ff`) it is short enough to collide with any old
/// folder or process name.
pub const MIN_IDENTITY_TOKEN_LEN: usize = 4;

/// Plumbing tokens: user-profile and install-path components. A degenerate slug
/// equal to one of these turns arbitrary processes into a strong "you are
/// playing" signal, which is the phantom correlation.
///
/// The list is static and pure on purpose, because the kernel cannot read the
/// environment. Components of the real home directory, the account name
/// included, which was the actual case, are added by the shell; see
/// `hoard_agent::agent::is_generic_identity_token`, which extends this list with
/// `directories::UserDirs`.
pub const GENERIC_IDENTITY_TOKENS: &[&str] = &[
    // User profile and system roots.
    "users",
    "user",
    "public",
    "home",
    "appdata",
    "roaming",
    "local",
    "locallow",
    "documents",
    "library",
    "applicationsupport",
    "config",
    "share",
    "state",
    "cache",
    "temp",
    "windows",
    "desktop",
    "downloads",
    // Save containers: they say there are saves inside, not which game's.
    "savedgames",
    "mygames",
    "save",
    "saves",
    "savegame",
    "savegames",
    "savedata",
    "savefiles",
    "profile",
    "profiles",
    "slot",
    "slots",
    // Install and storefront plumbing.
    "games",
    "game",
    "programfiles",
    "programfilesx86",
    "steam",
    "steamapps",
    "steamuser",
    "userdata",
    "common",
    "compatdata",
    "drivec",
    "remote",
    // Service folders a game leaves next to its saves.
    "settings",
    "options",
    "data",
    "content",
    "default",
    "logs",
    "crashes",
    // Handheld and emulator plumbing. `storage` is the one that bit: an
    // emulator front-end keeps its per-emulator trees under
    // `~/Emulation/storage`, which is one of our own deep-scan roots, so the
    // ancestor walk minted a game called "storage". And on an image-based Linux
    // distro every containerised process runs out of
    // `.../containers/storage/overlay/<hash>/merged/...`, whose path components
    // then matched that slug as a STRONG identity signal. Result: a game that is
    // "running" forever and can never be closed. `bios` is deliberately absent,
    // because the catalog has a game by that name.
    "storage",
    "emulation",
    "roms",
    "states",
    "savestates",
    "screenshots",
    "backups",
    "containers",
    "overlay",
    "merged",
];

/// Why a value failed the gate. `kind` names the newtype so the message is
/// diagnosable without wrapping it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdError {
    #[error("{kind}: valor vacío")]
    Empty { kind: &'static str },
    #[error("{kind}: {len} caracteres, el máximo es {max}")]
    TooLong {
        kind: &'static str,
        len: usize,
        max: usize,
    },
    #[error("{kind}: carácter inválido {ch:?} en {raw:?}")]
    BadChar {
        kind: &'static str,
        ch: char,
        raw: String,
    },
    #[error("{kind}: forma inválida ({expected}), recibido {raw:?}")]
    BadShape {
        kind: &'static str,
        expected: &'static str,
        raw: String,
    },
}

/// Why a persisted value could not be repaired and goes to quarantine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuarantineReason {
    /// Nothing left after normalising: empty, whitespace, symbols only.
    /// Re-deriving here would *fabricate* an identifier nobody wrote.
    Empty,
    /// Syntactically valid but degenerate: it matches a plumbing token
    /// ([`GENERIC_IDENTITY_TOKENS`]). This is the phantom-correlation poison,
    /// and repair does not apply because the value already has the right shape.
    /// What is wrong is that it means anything at all.
    Degenerate,
    /// The shape is unrecoverable by construction: you cannot invent a UUID or
    /// a SHA-256 out of junk.
    Unrecoverable,
}

impl fmt::Display for QuarantineReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            QuarantineReason::Empty => "vacío tras normalizar",
            QuarantineReason::Degenerate => "token genérico de fontanería",
            QuarantineReason::Unrecoverable => "forma irrecuperable",
        };
        f.write_str(s)
    }
}

/// The result of putting an already-persisted value through the lenient gate.
/// Never an error, because loading old state cannot fail (ADR 0021 C.3); it can
/// only land in one of these three places.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repair<T> {
    /// Already valid, passed through untouched.
    Clean(T),
    /// A valid value was re-derived from the raw one, which is kept for the log.
    Repaired { value: T, raw: String },
    /// There is no value worth trusting. The caller decides what to do with the
    /// raw one, either leaving it in place and flagging it or dropping the row.
    /// What it may not do is abort the load.
    Quarantined {
        raw: String,
        reason: QuarantineReason,
    },
}

impl<T> Repair<T> {
    /// The repaired value, or `None` if it was quarantined.
    pub fn value(&self) -> Option<&T> {
        match self {
            Repair::Clean(v) | Repair::Repaired { value: v, .. } => Some(v),
            Repair::Quarantined { .. } => None,
        }
    }

    /// Consumes and returns the repaired value, or `None` if quarantined.
    pub fn into_value(self) -> Option<T> {
        match self {
            Repair::Clean(v) | Repair::Repaired { value: v, .. } => Some(v),
            Repair::Quarantined { .. } => None,
        }
    }

    /// Did it pass untouched? Useful for not logging 99.9% of cases.
    pub fn is_clean(&self) -> bool {
        matches!(self, Repair::Clean(_))
    }

    /// Did it end up quarantined?
    pub fn is_quarantined(&self) -> bool {
        matches!(self, Repair::Quarantined { .. })
    }
}

/// Generates the common wrapper of an identity newtype.
///
/// What it does *not* generate is `parse`: each type writes its own, and the
/// macro wires it up as the only entry point, so `TryFrom`, `FromStr` and
/// `Deserialize` all go through it. The inner field is private, and this module
/// is the only place in the workspace where one can be built bypassing `parse`.
macro_rules! newtype_id {
    ($(#[$meta:meta])* $name:ident, $kind:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
        #[serde(try_from = "String")]
        pub struct $name(String);

        impl $name {
            /// The type's name, for error messages.
            pub const KIND: &'static str = $kind;

            /// The value as a `&str`. There is no implicit `From<$name> for
            /// String` besides [`Self::into_inner`]: going to text is explicit.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consumes the newtype and returns the `String` inside.
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;
            fn try_from(s: String) -> Result<Self, Self::Error> {
                Self::parse(&s)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = IdError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::parse(s)
            }
        }

        impl FromStr for $name {
            type Err = IdError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        /// Lets `HashMap<$name, _>::get(&str)` work without building the newtype.
        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }

        /// By hand rather than `#[serde(transparent)]` or `into = "String"`:
        /// serialising must not clone, and this makes it plain that the value
        /// travels as a bare string, identical to the `String` it replaced.
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&self.0)
            }
        }
    };
}

// ---- GameSlug

newtype_id!(
    /// Stable identifier for a game in the catalogue: ASCII lowercase, digits
    /// and hyphens (`stardew-valley`, `2064-read-only-memories`).
    ///
    /// That is the shape [`slugify`] produces, and `slugify` minted the whole
    /// catalogue, so the gate accepts exactly that. It normalises edge
    /// whitespace and case, since two slugs differing only in case are the same
    /// game and treating them as different duplicated rows, and rejects
    /// everything else.
    GameSlug,
    "game_slug"
);

impl GameSlug {
    /// The reserved `__other__` slug (see [`OTHER_SLUG`]).
    pub fn other() -> Self {
        Self(OTHER_SLUG.to_string())
    }

    /// Marker for a persisted slug that is unrecoverable (empty, symbols only)
    /// and still has to go out over the wire because the row exists. It is
    /// deliberately visible and matches no real game, so it can correlate with
    /// nothing. The alternative, a 500, would cost the user their entire listing
    /// over one bad row.
    pub fn unknown() -> Self {
        Self("unknown-game".to_string())
    }

    /// Is this the synthetic `__other__` rather than a real game?
    pub fn is_other(&self) -> bool {
        self.0 == OTHER_SLUG
    }

    /// The gate. `serde`, `TryFrom` and `FromStr` all come through here.
    ///
    /// Normalises (trim plus ASCII lowercase) and then demands the canonical
    /// shape. Normalising rather than rejecting is deliberate: it is idempotent
    /// (`parse(parse(x)) == parse(x)`), it cannot brick anything, and it kills
    /// the "same game in two different cases" class of bug. What gets rejected
    /// is what [`slugify`] could never have emitted.
    pub fn parse(raw: &str) -> Result<Self, IdError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(IdError::Empty { kind: Self::KIND });
        }
        if trimmed == OTHER_SLUG {
            return Ok(Self(OTHER_SLUG.to_string()));
        }
        let s = trimmed.to_ascii_lowercase();
        if s.chars().count() > MAX_SLUG_LEN {
            return Err(IdError::TooLong {
                kind: Self::KIND,
                len: s.chars().count(),
                max: MAX_SLUG_LEN,
            });
        }
        if let Some(ch) = s.chars().find(|c| !c.is_ascii_alphanumeric() && *c != '-') {
            return Err(IdError::BadChar {
                kind: Self::KIND,
                ch,
                raw: raw.to_string(),
            });
        }
        if !s.starts_with(|c: char| c.is_ascii_alphanumeric()) {
            return Err(IdError::BadShape {
                kind: Self::KIND,
                expected: "empieza por letra o dígito",
                raw: raw.to_string(),
            });
        }
        if s.ends_with('-') {
            return Err(IdError::BadShape {
                kind: Self::KIND,
                expected: "no termina en guion",
                raw: raw.to_string(),
            });
        }
        if s.contains("--") {
            return Err(IdError::BadShape {
                kind: Self::KIND,
                expected: "sin guiones consecutivos",
                raw: raw.to_string(),
            });
        }
        Ok(Self(s))
    }

    /// The lenient gate, only for slugs already on disk.
    ///
    /// - Valid becomes [`Repair::Clean`].
    /// - Recoverable is re-derived with [`slugify`], the same algorithm that
    ///   minted it, and becomes [`Repair::Repaired`].
    /// - Degenerate ([`GENERIC_IDENTITY_TOKENS`]) or empty becomes
    ///   [`Repair::Quarantined`].
    ///
    /// A degenerate slug is *not* re-derived: `users` is already well formed,
    /// the problem is that it matches everything. Making up another name for it
    /// would be inventing a game. The right move is to flag it and let the
    /// caller decide, which today means keeping it as-is and excluding it from
    /// correlation, without touching the identity the server already knows.
    pub fn repair(raw: &str) -> Repair<Self> {
        let degenerate = |s: &str| {
            let tok = canon_token(s);
            tok.len() >= MIN_IDENTITY_TOKEN_LEN && GENERIC_IDENTITY_TOKENS.contains(&tok.as_str())
        };
        if let Ok(v) = Self::parse(raw) {
            if !v.is_other() && degenerate(v.as_str()) {
                return Repair::Quarantined {
                    raw: raw.to_string(),
                    reason: QuarantineReason::Degenerate,
                };
            }
            return Repair::Clean(v);
        }
        // Nothing usable: `slugify` would return its `game` filler, which is
        // inventing a game out of thin air.
        if !raw.chars().any(|c| c.is_ascii_alphanumeric()) {
            return Repair::Quarantined {
                raw: raw.to_string(),
                reason: QuarantineReason::Empty,
            };
        }
        match Self::parse(&slugify(raw)) {
            Ok(v) if !degenerate(v.as_str()) => Repair::Repaired {
                value: v,
                raw: raw.to_string(),
            },
            Ok(_) => Repair::Quarantined {
                raw: raw.to_string(),
                reason: QuarantineReason::Degenerate,
            },
            Err(_) => Repair::Quarantined {
                raw: raw.to_string(),
                reason: QuarantineReason::Unrecoverable,
            },
        }
    }
}

/// Canonical lower-kebab slug. The single source of truth for the algorithm:
/// `hoard_manifest::ludusavi::slugify` delegates here, and
/// `data/convert-ludusavi.py` is its byte-compatible twin. Diverging silently
/// breaks the catalogue-to-detection-to-server join.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true;
    for ch in name.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("game");
    }
    if out.len() > MAX_SLUG_LEN {
        out.truncate(MAX_SLUG_LEN);
        while out.ends_with('-') {
            out.pop();
        }
    }
    if !out.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        out.insert(0, 'g');
    }
    out
}

/// Canonical identity token for a game or process: ASCII alphanumerics in
/// lowercase, no separators, no extension. It collapses the three shapes the
/// same game shows up in (slug `victoria-3`, display name `Victoria 3`,
/// executable `victoria3.exe`) into one comparable key.
pub fn canon_token(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        }
    }
    out
}

/// Shortest a path segment can be and still plausibly be a game's name. Two
/// characters (`cd`, `ps`) name a medium or a console, never a title.
pub const MIN_NAMEABLE_LEN: usize = 3;

/// `true` when a raw path segment cannot honestly name a game.
///
/// The naming counterpart of [`GENERIC_IDENTITY_TOKENS`]: that list keeps a
/// degenerate slug from poisoning correlation *after the fact*, this keeps one
/// from being minted in the first place. Both read the same list on purpose, so
/// a name this returns `false` for is a name the loader will not quarantine.
///
/// A segment fails in one of three ways:
///
/// * it is plumbing every machine has (`AppData`, `steamapps`, `user`);
/// * it is an identifier the machine minted for itself: a Steam appid, a
///   SteamID64, a profile uuid, the hex ids Citra derives from console keys.
///   None of them is a name, and every one of them differs on the next
///   machine, so a save named after one cannot be paired across devices;
/// * there is not enough of it left to be a title (`cd`).
///
/// Only static text is consulted, so this cannot know the user's own home
/// path, and `C:\Users\<account>` names the account, never the game. Callers
/// with an environment extend it; see `hoard_agent::agent::is_generic_identity_token`.
pub fn is_generic_name(raw: &str) -> bool {
    let tok = canon_token(raw);
    if tok.len() < MIN_NAMEABLE_LEN {
        return true;
    }
    if GENERIC_IDENTITY_TOKENS.contains(&tok.as_str()) {
        return true;
    }
    if tok.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    // Hex ids only. A digit has to be in there: without that clause ordinary
    // words made of hex letters ("facade", "decade") would read as ids.
    tok.len() >= 8
        && tok.chars().any(|c| c.is_ascii_digit())
        && tok.chars().all(|c| c.is_ascii_hexdigit())
}

// ---- Username

newtype_id!(
    /// A username on a self-hosted server.
    ///
    /// The gate is deliberately permissive. The server never validated usernames
    /// (`hoard-admin user create` inserts whatever it is given) and there are
    /// live accounts with spaces and accents in them. A style rule here would
    /// fix no bug and would leave those users unable to call `whoami`, which
    /// means unable to log in.
    ///
    /// What it does reject is what can never be a user: empty, whitespace only,
    /// control characters (they break logs and headers), and absurd lengths. The
    /// type's real value is not the validation but the *category*: a `Username`
    /// cannot land in a `GameSlug` field by accident, which is exactly what
    /// produced the phantom correlation.
    Username,
    "username"
);

impl Username {
    /// Marker for an unrecoverable persisted username (empty). Same reasoning as
    /// [`GameSlug::unknown`]: on self-hosted the username is presentation data,
    /// since authorisation goes by token to `user_id`, so degrading the display
    /// name is infinitely better than a 500 on `whoami`, which leaves the
    /// account unable to open the app.
    pub fn unknown() -> Self {
        Self("unknown".to_string())
    }

    /// The gate. Normalises by trimming edge whitespace; rejects empty, control
    /// characters and anything over [`MAX_USERNAME_LEN`].
    pub fn parse(raw: &str) -> Result<Self, IdError> {
        let s = raw.trim();
        if s.is_empty() {
            return Err(IdError::Empty { kind: Self::KIND });
        }
        if s.chars().count() > MAX_USERNAME_LEN {
            return Err(IdError::TooLong {
                kind: Self::KIND,
                len: s.chars().count(),
                max: MAX_USERNAME_LEN,
            });
        }
        if let Some(ch) = s.chars().find(|c| c.is_control()) {
            return Err(IdError::BadChar {
                kind: Self::KIND,
                ch,
                raw: raw.to_string(),
            });
        }
        Ok(Self(s.to_string()))
    }

    /// Lenient gate for already-persisted usernames: strips control characters
    /// and truncates, and only quarantines when nothing is left.
    pub fn repair(raw: &str) -> Repair<Self> {
        if let Ok(v) = Self::parse(raw) {
            return Repair::Clean(v);
        }
        let cleaned: String = raw
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(MAX_USERNAME_LEN)
            .collect();
        match Self::parse(&cleaned) {
            Ok(value) => Repair::Repaired {
                value,
                raw: raw.to_string(),
            },
            Err(_) => Repair::Quarantined {
                raw: raw.to_string(),
                reason: QuarantineReason::Empty,
            },
        }
    }
}

// ---- SaveId

newtype_id!(
    /// A save's identifier. Always a canonical v4 UUID: 36 characters,
    /// lowercase, hyphenated. The server mints it (`Uuid::new_v4().to_string()`),
    /// or Postgres does on cloud, or the client does when it creates the local
    /// row before the first upload. All three agree.
    ///
    /// The gate demands the exact canonical shape and does *not* normalise: an
    /// id is a lookup key against the server, and "fixing" it (lowering the case,
    /// stripping braces) would produce an id pointing somewhere other than where
    /// it was written.
    SaveId,
    "save_id"
);

impl SaveId {
    /// The gate. Canonical lowercase hyphenated UUID.
    pub fn parse(raw: &str) -> Result<Self, IdError> {
        let s = raw.trim();
        if s.is_empty() {
            return Err(IdError::Empty { kind: Self::KIND });
        }
        if !is_canonical_uuid(s) {
            return Err(IdError::BadShape {
                kind: Self::KIND,
                expected: "UUID canónico en minúsculas (8-4-4-4-12)",
                raw: raw.to_string(),
            });
        }
        Ok(Self(s.to_string()))
    }

    /// Lenient gate: recovers case only, since an uppercase UUID still points at
    /// the same save. Anything else is unrecoverable, because an identifier the
    /// server never minted cannot be invented.
    pub fn repair(raw: &str) -> Repair<Self> {
        if let Ok(v) = Self::parse(raw) {
            return Repair::Clean(v);
        }
        let lowered = raw.trim().to_ascii_lowercase();
        match Self::parse(&lowered) {
            Ok(value) => Repair::Repaired {
                value,
                raw: raw.to_string(),
            },
            Err(_) => Repair::Quarantined {
                raw: raw.to_string(),
                reason: QuarantineReason::Unrecoverable,
            },
        }
    }
}

/// `8-4-4-4-12` in lowercase hex. Checked by hand rather than with
/// `uuid::Uuid::parse_str`, because that also accepts the simple, braced and URN
/// forms, and accepting those would let two different strings in for the same
/// save, and therefore two different keys in the state maps.
fn is_canonical_uuid(s: &str) -> bool {
    const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];
    let mut parts = s.split('-');
    for len in GROUPS {
        match parts.next() {
            Some(p) if p.len() == len && p.bytes().all(is_lower_hex) => {}
            _ => return false,
        }
    }
    parts.next().is_none()
}

fn is_lower_hex(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'f')
}

// ---- Sha256

newtype_id!(
    /// A SHA-256 digest in hex: 64 characters, lowercase. That is what
    /// `hex::encode` emits on both ends, so the canonical form is the only one
    /// that exists in practice.
    ///
    /// Note that an *empty* `sha256` field on the wire is not a malformed hash
    /// but "not applicable": content-addressed versions have no whole-archive
    /// digest. That is modelled with `Option<Sha256>` and a deserialiser that
    /// treats `""` as `None`, not by relaxing this gate.
    Sha256,
    "sha256"
);

impl Sha256 {
    /// Hex length of a SHA-256 digest.
    pub const HEX_LEN: usize = 64;

    /// The gate. 64 hex characters; normalises case, because an uppercase digest
    /// is the same digest. Unlike an id, here the value *is* the content rather
    /// than somebody else's key.
    pub fn parse(raw: &str) -> Result<Self, IdError> {
        parse_hex(raw, Self::HEX_LEN, Self::KIND).map(Self)
    }

    /// Lenient gate: normalise the case, or quarantine.
    pub fn repair(raw: &str) -> Repair<Self> {
        repair_hex(raw, Self::parse)
    }
}

// ---- MachineId

newtype_id!(
    /// A machine's stable fingerprint: hex SHA-256 of `/etc/machine-id` (or the
    /// per-OS equivalent) plus the hostname. Same shape as [`Sha256`] but a
    /// deliberately different type: a machine fingerprint and a file's digest are
    /// not interchangeable, and the compiler should say so.
    MachineId,
    "machine_id"
);

impl MachineId {
    /// Hex length of a machine fingerprint.
    pub const HEX_LEN: usize = 64;

    /// The gate. 64 hex characters, case normalised.
    pub fn parse(raw: &str) -> Result<Self, IdError> {
        parse_hex(raw, Self::HEX_LEN, Self::KIND).map(Self)
    }

    /// Lenient gate: normalise the case, or quarantine.
    pub fn repair(raw: &str) -> Repair<Self> {
        repair_hex(raw, Self::parse)
    }
}

fn parse_hex(raw: &str, len: usize, kind: &'static str) -> Result<String, IdError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(IdError::Empty { kind });
    }
    let s = s.to_ascii_lowercase();
    if s.chars().count() != len {
        return Err(IdError::BadShape {
            kind,
            expected: "64 caracteres hex",
            raw: raw.to_string(),
        });
    }
    if let Some(ch) = s.chars().find(|c| !c.is_ascii_hexdigit()) {
        return Err(IdError::BadChar {
            kind,
            ch,
            raw: raw.to_string(),
        });
    }
    Ok(s)
}

/// The hex types' `parse` normalises case, so "it was clean" means "the raw
/// value already matched the normalised one".
fn repair_hex<T: AsRef<str>>(raw: &str, parse: fn(&str) -> Result<T, IdError>) -> Repair<T> {
    match parse(raw) {
        Ok(value) if value.as_ref() == raw => Repair::Clean(value),
        Ok(value) => Repair::Repaired {
            value,
            raw: raw.to_string(),
        },
        Err(_) => Repair::Quarantined {
            raw: raw.to_string(),
            reason: QuarantineReason::Unrecoverable,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- the serde gate

    /// The whole point of the slice: a newtype cannot be built by
    /// deserialising. If somebody swaps `try_from` for a plain derive, this
    /// fails.
    #[test]
    fn deserialize_goes_through_the_gate() {
        assert!(serde_json::from_str::<GameSlug>(r#""stardew-valley""#).is_ok());
        for poison in [
            r#""GSE Saves""#,
            r#""""#,
            r#""   ""#,
            r#""stardew--valley""#,
            r#""-leading""#,
            r#""trailing-""#,
            r#""ünïcode""#,
        ] {
            assert!(
                serde_json::from_str::<GameSlug>(poison).is_err(),
                "{poison} debería rebotar en la puerta"
            );
        }
        assert!(serde_json::from_str::<Username>(r#""""#).is_err());
        assert!(serde_json::from_str::<SaveId>(r#""not-a-uuid""#).is_err());
        assert!(serde_json::from_str::<Sha256>(r#""deadbeef""#).is_err());
        assert!(serde_json::from_str::<MachineId>(r#""zz""#).is_err());
    }

    /// A newtype's JSON is the same bare string the `String` it replaced
    /// emitted: changing the type moved not one byte of the wire.
    #[test]
    fn serializes_as_a_bare_string() {
        let slug = GameSlug::parse("stardew-valley").unwrap();
        assert_eq!(serde_json::to_string(&slug).unwrap(), r#""stardew-valley""#);
        let id = SaveId::parse("3f2504e0-4f89-41d3-9a0c-0305e82c3301").unwrap();
        assert_eq!(
            serde_json::to_string(&id).unwrap(),
            r#""3f2504e0-4f89-41d3-9a0c-0305e82c3301""#
        );
    }

    /// Normalisation is idempotent: `parse(parse(x)) == parse(x)`. Without that,
    /// a round trip through disk could move the value.
    #[test]
    fn parse_is_idempotent() {
        for raw in ["  Stardew-Valley ", "STARDEW-VALLEY", "stardew-valley"] {
            let once = GameSlug::parse(raw).unwrap();
            let twice = GameSlug::parse(once.as_str()).unwrap();
            assert_eq!(once, twice);
            assert_eq!(once.as_str(), "stardew-valley");
        }
        let sha = "A".repeat(64);
        let once = Sha256::parse(&sha).unwrap();
        assert_eq!(once, Sha256::parse(once.as_str()).unwrap());
        assert_eq!(once.as_str(), "a".repeat(64));
    }

    // ---- slugs

    #[test]
    fn slug_accepts_the_shapes_slugify_emits() {
        for ok in [
            "stardew-valley",
            "2064-read-only-memories",
            "doom",
            "a",
            OTHER_SLUG,
        ] {
            assert!(GameSlug::parse(ok).is_ok(), "{ok} debería pasar");
        }
        assert!(GameSlug::parse(&"a".repeat(MAX_SLUG_LEN)).is_ok());
        assert!(GameSlug::parse(&"a".repeat(MAX_SLUG_LEN + 1)).is_err());
    }

    /// Everything `slugify` emits passes the gate. That contract is what lets
    /// `repair` re-derive without looping.
    #[test]
    fn slugify_output_always_parses() {
        for raw in [
            "Stardew Valley",
            "GSE Saves",
            "  ...  ",
            "2064: Read Only Memories",
            "ünïcode gäme",
            "!!!",
            "a",
            &"muy largo ".repeat(30),
        ] {
            let s = slugify(raw);
            assert!(
                GameSlug::parse(&s).is_ok(),
                "slugify({raw:?}) = {s:?} no pasa la puerta"
            );
        }
    }

    // ---- repair and quarantine

    #[test]
    fn repair_rederives_a_recoverable_slug() {
        match GameSlug::repair("GSE Saves") {
            Repair::Repaired { value, raw } => {
                assert_eq!(value.as_str(), "gse-saves");
                assert_eq!(raw, "GSE Saves");
            }
            other => panic!("esperaba Repaired, salió {other:?}"),
        }
    }

    /// The July 2026 poison: a slug that is a plumbing token. The real case was
    /// a Windows account name, which the shell adds to the list. It is not
    /// re-derived, since it is already well formed; it is flagged.
    #[test]
    fn repair_quarantines_a_degenerate_slug() {
        for poison in [
            "users",
            "appdata",
            "steamapps",
            "savedgames",
            // Handheld flavour: an emulator front-end's `storage` tree, plus
            // the container store every process runs from on an image-based
            // distro. Same failure, different plumbing.
            "storage",
            "containers",
            "overlay",
        ] {
            match GameSlug::repair(poison) {
                Repair::Quarantined { reason, raw } => {
                    assert_eq!(reason, QuarantineReason::Degenerate);
                    assert_eq!(raw, poison);
                }
                other => panic!("{poison} debería ir a cuarentena, salió {other:?}"),
            }
        }
    }

    /// With nothing alphanumeric there is no slug to derive: `slugify` would
    /// return its `game` filler and we would be inventing a game.
    #[test]
    fn repair_quarantines_instead_of_fabricating() {
        for empty in ["", "   ", "---", "!!!"] {
            assert!(
                GameSlug::repair(empty).is_quarantined(),
                "{empty:?} debería ir a cuarentena"
            );
        }
    }

    #[test]
    fn repair_leaves_clean_values_alone() {
        assert!(GameSlug::repair("stardew-valley").is_clean());
        assert!(GameSlug::repair(OTHER_SLUG).is_clean());
        assert!(Username::repair("jacka").is_clean());
        assert!(SaveId::repair("3f2504e0-4f89-41d3-9a0c-0305e82c3301").is_clean());
        assert!(Sha256::repair(&"ab".repeat(32)).is_clean());
    }

    #[test]
    fn repair_recovers_uuid_case_but_not_garbage() {
        match SaveId::repair("3F2504E0-4F89-41D3-9A0C-0305E82C3301") {
            Repair::Repaired { value, .. } => {
                assert_eq!(value.as_str(), "3f2504e0-4f89-41d3-9a0c-0305e82c3301")
            }
            other => panic!("esperaba Repaired, salió {other:?}"),
        }
        assert!(SaveId::repair("save-a").is_quarantined());
    }

    // ---- category

    /// The other half of the slice's value: `slug == username` does not compile.
    /// This test documents the intent; the compiler enforces it, since comparing
    /// the two types directly is a type error.
    #[test]
    fn slug_and_username_are_different_categories() {
        let slug = GameSlug::parse("jacka").unwrap();
        let user = Username::parse("jacka").unwrap();
        // Comparing means explicitly dropping to `str`, and that descent is
        // exactly where a human asks themselves why they are comparing a user
        // with a game.
        assert_eq!(slug.as_str(), user.as_str());
    }

    // ---- identity shapes

    #[test]
    fn uuid_gate_is_strict_about_shape() {
        assert!(SaveId::parse("3f2504e0-4f89-41d3-9a0c-0305e82c3301").is_ok());
        for bad in [
            "3f2504e04f8941d39a0c0305e82c3301",              // simple
            "{3f2504e0-4f89-41d3-9a0c-0305e82c3301}",        // con llaves
            "3f2504e0-4f89-41d3-9a0c-0305e82c330",           // corto
            "3f2504e0-4f89-41d3-9a0c-0305e82c3301-",         // sobra
            "3f2504e0-4f89-41d3-9a0c-0305e82c330g",          // no hex
            "urn:uuid:3f2504e0-4f89-41d3-9a0c-0305e82c3301", // URN
        ] {
            assert!(SaveId::parse(bad).is_err(), "{bad} debería rebotar");
        }
    }

    #[test]
    fn hex_gates_reject_wrong_length_and_charset() {
        assert!(Sha256::parse(&"a".repeat(63)).is_err());
        assert!(Sha256::parse(&"a".repeat(65)).is_err());
        assert!(Sha256::parse(&"g".repeat(64)).is_err());
        assert!(MachineId::parse(&"0".repeat(64)).is_ok());
    }

    #[test]
    fn username_is_permissive_but_rejects_the_impossible() {
        for ok in ["jacka", "John Doe", "señor-ñ", "a"] {
            assert!(Username::parse(ok).is_ok(), "{ok} debería pasar");
        }
        assert!(Username::parse("").is_err());
        assert!(Username::parse("   ").is_err());
        assert!(Username::parse("na\u{0}me").is_err());
        assert!(Username::parse(&"a".repeat(MAX_USERNAME_LEN + 1)).is_err());
    }

    #[test]
    fn borrow_lets_maps_be_queried_by_str() {
        use std::collections::HashMap;
        let mut m: HashMap<GameSlug, u32> = HashMap::new();
        m.insert(GameSlug::parse("doom").unwrap(), 1);
        assert_eq!(m.get("doom"), Some(&1));
    }

    // ---- slugify (ported from hoard-manifest, which now delegates here)

    #[test]
    fn slugify_examples() {
        assert_eq!(slugify("Stardew Valley"), "stardew-valley");
        assert_eq!(slugify("DOOM (2016)"), "doom-2016");
        assert_eq!(slugify("  spaces  "), "spaces");
        assert_eq!(slugify(""), "game");
        assert_eq!(slugify("!!!"), "game");
        assert_eq!(
            slugify("2064: Read Only Memories"),
            "2064-read-only-memories"
        );
    }

    #[test]
    fn canon_token_strips_everything_but_alphanumerics() {
        assert_eq!(canon_token("Victoria 3"), "victoria3");
        assert_eq!(canon_token("victoria-3"), "victoria3");
        assert_eq!(canon_token("victoria3.exe"), "victoria3exe");
    }
}
