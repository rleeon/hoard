//! The Hoard Wrapped share card: a local avatar and "take the picture".
//!
//! Everything this piece touches is **this machine's and this machine's only**: the
//! card's picture and name never travel to the server or the cloud, and they are not
//! in the account export. The picture lives as a single `avatar.png` under the
//! app-data dir; the rest of the configuration (name, phrase, rank) the frontend
//! keeps in its local store. Deleting the file is all the "forget me" there is.
//!
//! The avatar's cropping and scaling happen in the webview (canvas) before reaching
//! here, so this module only ever sees already normalised PNG: one format, a bounded
//! size, no guessing at MIME types when reading it back.
//!
//! `wrapple_save_card` writes the rendered card into the system gallery
//! (`Pictures/Hoard/`) and injects PNG `tEXt` metadata: title, software author and
//! `https://hoard.services`. That is the SEO half of the brief: an image shared on
//! its own carries where it came from, both visibly (the watermark) and in its
//! metadata, which is what image searches and viewers read.

use std::path::{Path, PathBuf};

use base64::Engine;
use tauri::ipc::Response;
use tauri::Manager;

/// The extensions accepted when picking a picture. The same list as for custom
/// covers: whatever the webview knows how to decode.
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif"];

/// The ceiling when reading the file the user picks. A phone photo is around 5 MB;
/// 32 leaves plenty of room without letting a 400 MB TIFF land whole in the
/// webview's RAM.
const MAX_SOURCE_BYTES: u64 = 32 * 1024 * 1024;

/// Tope al guardar. El avatar sale de un canvas de 512×512 (≈300 KB) y la
/// tarjeta de uno de 1200×630 (≈1,5 MB); 16 MB es holgura, no permiso.
const MAX_PNG_BYTES: usize = 16 * 1024 * 1024;

fn wrapple_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("wrapple"))
}

fn avatar_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(wrapple_dir(app)?.join("avatar.png"))
}

fn has_image_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
}

fn decode_png(data: &str) -> Result<Vec<u8>, String> {
    // The frontend sends bare base64, but the whole data URL is accepted too in
    // case somebody passes `toDataURL()` through as it is.
    let payload = data.rsplit_once(",").map_or(data, |(_, tail)| tail);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|e| format!("imagen ilegible: {e}"))?;
    if bytes.is_empty() {
        return Err("imagen vacía".into());
    }
    if bytes.len() > MAX_PNG_BYTES {
        return Err("imagen demasiado grande".into());
    }
    if !bytes.starts_with(&PNG_SIGNATURE) {
        return Err("la imagen no es un PNG".into());
    }
    Ok(bytes)
}

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Reads an image file the user picked and hands its bytes back to the webview,
/// which is what crops and scales it. Nothing is copied yet: until the crop is
/// confirmed, no trace is left on disk.
#[tauri::command]
pub async fn wrapple_read_image(source_path: String) -> Result<Response, String> {
    let src = PathBuf::from(&source_path);
    if !has_image_extension(&src) {
        return Err("ese fichero no es una imagen".into());
    }
    let meta = tokio::fs::metadata(&src)
        .await
        .map_err(|e| format!("no se pudo leer la imagen: {e}"))?;
    if !meta.is_file() {
        return Err("la ruta no es un fichero".into());
    }
    if meta.len() > MAX_SOURCE_BYTES {
        return Err("la imagen pesa demasiado (máx. 32 MB)".into());
    }
    tokio::fs::read(&src)
        .await
        .map(Response::new)
        .map_err(|e| format!("no se pudo leer la imagen: {e}"))
}

/// Stores the card's avatar (PNG already cropped by the webview). Local and nothing
/// else: it is never uploaded.
#[tauri::command]
pub async fn wrapple_set_avatar(app: tauri::AppHandle, png_base64: String) -> Result<(), String> {
    let bytes = decode_png(&png_base64)?;
    let dir = wrapple_dir(&app)?;
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    // An atomic write: a half-failure would leave a truncated PNG the webview
    // cannot paint and which persists across starts.
    let dest = dir.join("avatar.png");
    let tmp = dir.join("avatar.png.tmp");
    tokio::fs::write(&tmp, &bytes)
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::rename(&tmp, &dest)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// The stored avatar's bytes. `Err` when there is none, and the frontend simply
/// falls back to initials.
#[tauri::command]
pub async fn wrapple_avatar_bytes(app: tauri::AppHandle) -> Result<Response, String> {
    let path = avatar_path(&app)?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| "sin avatar".to_string())?;
    if bytes.is_empty() {
        return Err("sin avatar".into());
    }
    Ok(Response::new(bytes))
}

/// Olvida la foto local.
#[tauri::command]
pub async fn wrapple_clear_avatar(app: tauri::AppHandle) -> Result<(), String> {
    let path = avatar_path(&app)?;
    let _ = tokio::fs::remove_file(&path).await;
    Ok(())
}

/// Writes the rendered card into the system gallery and returns the final path so
/// it can be shown to the user.
///
/// The destination is `<Pictures>/Hoard/hoard-wrapped-<date>.png`. When the system
/// declares no pictures folder (service accounts, odd environments) it falls back to
/// Downloads and then to the home directory, because failing a save for want of a
/// canonical folder would be absurd.
#[tauri::command]
pub async fn wrapple_save_card(
    app: tauri::AppHandle,
    png_base64: String,
    label: Option<String>,
) -> Result<String, String> {
    let bytes = decode_png(&png_base64)?;
    let bytes = with_seo_metadata(bytes, label.as_deref());

    let paths = app.path();
    let gallery = paths
        .picture_dir()
        .or_else(|_| paths.download_dir())
        .or_else(|_| paths.home_dir())
        .map_err(|e| format!("no se encontró carpeta de imágenes: {e}"))?
        .join("Hoard");
    tokio::fs::create_dir_all(&gallery)
        .await
        .map_err(|e| format!("no se pudo crear {}: {e}", gallery.display()))?;

    let dest = gallery.join(format!("hoard-wrapped-{}.png", timestamp_slug()));
    tokio::fs::write(&dest, &bytes)
        .await
        .map_err(|e| format!("no se pudo guardar la imagen: {e}"))?;
    Ok(dest.to_string_lossy().into_owned())
}

/// `20260801-2137`, hora local. Sin dos puntos ni barras: el nombre tiene que
/// sobrevivir a NTFS igual que a ext4.
fn timestamp_slug() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    format!(
        "{:04}{:02}{:02}-{:02}{:02}",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute()
    )
}

/// Injects `tEXt` chunks with the provenance right after the IHDR.
///
/// A PNG is a signature plus a chain of `[len:u32][type:4][data][crc:u32]` chunks,
/// and `tEXt` chunks are legal anywhere between IHDR and IEND. Ours go right against
/// the IHDR so any reader sees them without walking the whole image. When the buffer
/// does not look the way it should, the PNG comes back untouched: the metadata is an
/// extra, never a reason not to save.
fn with_seo_metadata(png: Vec<u8>, label: Option<&str>) -> Vec<u8> {
    const IHDR_END: usize = 8 + 4 + 4 + 13 + 4; // firma + len + "IHDR" + datos + crc
    if png.len() < IHDR_END || &png[12..16] != b"IHDR" {
        return png;
    }

    let title = match label.map(str::trim).filter(|s| !s.is_empty()) {
        Some(l) => format!("Hoard Wrapped — {l}"),
        None => "Hoard Wrapped".to_string(),
    };
    let fields: [(&str, &str); 5] = [
        ("Title", title.as_str()),
        ("Software", "Hoard — hoard.services"),
        ("Source", "https://hoard.services"),
        ("Copyright", "Hoard · hoard.services"),
        (
            "Description",
            "Resumen de partidas generado con Hoard, copias de seguridad automáticas \
             para tus partidas guardadas — https://hoard.services",
        ),
    ];

    let mut out = Vec::with_capacity(png.len() + 512);
    out.extend_from_slice(&png[..IHDR_END]);
    for (key, value) in fields {
        out.extend_from_slice(&text_chunk(key, value));
    }
    out.extend_from_slice(&png[IHDR_END..]);
    out
}

/// Un chunk `tEXt`: clave Latin-1 (1..=79 bytes), NUL, valor. Los caracteres
/// fuera de Latin-1 se caen del valor en vez de romper el chunk.
fn text_chunk(key: &str, value: &str) -> Vec<u8> {
    let latin1 = |s: &str| -> Vec<u8> {
        s.chars()
            .filter(|c| (*c as u32) < 256 && *c != '\0')
            .map(|c| c as u8)
            .collect()
    };
    let mut data = latin1(key);
    data.truncate(79);
    data.push(0);
    data.extend_from_slice(&latin1(value));

    let mut chunk = Vec::with_capacity(data.len() + 12);
    chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut typed = b"tEXt".to_vec();
    typed.extend_from_slice(&data);
    chunk.extend_from_slice(&typed);
    chunk.extend_from_slice(&crc32(&typed).to_be_bytes());
    chunk
}

/// CRC-32 (IEEE, the PNG one). No dependency: it is twelve lines and gets called
/// five times per saved image.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xffff_ffff;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vector conocido: el CRC-32 de "123456789" es 0xCBF43926.
    #[test]
    fn crc32_matches_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    /// El chunk lleva longitud, tipo y un CRC que cubre tipo+datos.
    #[test]
    fn text_chunk_is_well_formed() {
        let chunk = text_chunk("Source", "https://hoard.services");
        let len = u32::from_be_bytes(chunk[0..4].try_into().unwrap()) as usize;
        assert_eq!(&chunk[4..8], b"tEXt");
        assert_eq!(chunk.len(), len + 12);
        let crc = u32::from_be_bytes(chunk[chunk.len() - 4..].try_into().unwrap());
        assert_eq!(crc, crc32(&chunk[4..chunk.len() - 4]));
        // clave NUL valor
        assert_eq!(&chunk[8..14], b"Source");
        assert_eq!(chunk[14], 0);
    }

    /// The metadata goes in after the IHDR and leaves the rest of the file alone.
    #[test]
    fn metadata_goes_after_ihdr() {
        let mut png = Vec::new();
        png.extend_from_slice(&PNG_SIGNATURE);
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&[0u8; 13]);
        png.extend_from_slice(&[1, 2, 3, 4]); // crc de mentira, no lo tocamos
        png.extend_from_slice(b"IDATTAIL");

        let out = with_seo_metadata(png.clone(), Some("Rust"));
        assert!(out.len() > png.len());
        assert_eq!(&out[..33], &png[..33]); // firma + IHDR intactos
        assert_eq!(&out[37..41], b"tEXt"); // the first injected chunk goes right behind it
        assert!(out.ends_with(b"IDATTAIL"));
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("hoard.services"));
        assert!(text.contains("Rust"));
    }

    /// Un buffer que no es PNG sale tal cual, sin panic.
    #[test]
    fn non_png_passes_through() {
        let junk = b"no soy un png".to_vec();
        assert_eq!(with_seo_metadata(junk.clone(), None), junk);
    }

    /// PNG only: valid base64 in another format is rejected.
    #[test]
    fn decode_png_rejects_other_formats() {
        let jpeg = base64::engine::general_purpose::STANDARD.encode([0xff, 0xd8, 0xff, 0xe0]);
        assert!(decode_png(&jpeg).is_err());
        let data_url = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(PNG_SIGNATURE)
        );
        assert!(decode_png(&data_url).is_ok());
    }
}
