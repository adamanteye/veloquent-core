/// Used in axum server to perform graceful shutdown.
///
/// Adapted from [axum graceful-shutdown](https://github.com/tokio-rs/axum/tree/main/examples/graceful-shutdown) with non-unix part removed.
pub(super) async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::event!(tracing::Level::INFO, "gracefully shutting down");
}

use crate::entity;

use std::sync::{LazyLock, OnceLock};

use regex::Regex;

pub fn good_email(email: &str) -> bool {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
        )
        .unwrap()
    });
    RE.is_match(email)
}

pub fn good_phone(phone: &str) -> bool {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9]{11}").unwrap());
    RE.is_match(phone)
}

pub fn validate_passwd(passwd: &str, salt: &str, hash: &str) -> anyhow::Result<bool> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(passwd);
    h.update(salt);
    let h = h.finalize();
    let mut buf = [0u8; 64];
    let h = base16ct::lower::encode_str(&h, &mut buf)
        .map_err(|e| anyhow::format_err!(e))?
        .to_string();
    if h == hash {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn gen_hash_and_salt(passwd: &str) -> Result<(String, String), anyhow::Error> {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use sha2::{Digest, Sha256};
    let salt: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    let mut hash = Sha256::new();
    hash.update(passwd);
    hash.update(salt.clone());
    let hash = hash.finalize();
    let mut buf = [0u8; 64];
    let hash = base16ct::lower::encode_str(&hash, &mut buf)
        .map_err(|e| anyhow::format_err!(e))?
        .to_string();
    Ok((hash, salt))
}

pub(super) static UPLOAD_DIR: OnceLock<String> = OnceLock::new();

impl From<entity::upload::Model> for std::path::PathBuf {
    fn from(upload: entity::upload::Model) -> Self {
        let mut buf = std::path::PathBuf::new();
        buf.push(UPLOAD_DIR.get().unwrap());
        buf.push(upload.uuid.to_string());
        buf.push(upload.typ);
        buf.to_owned()
    }
}

pub static UUID_NIL: LazyLock<uuid::Uuid> = LazyLock::new(uuid::Uuid::nil);

static UPLOAD_UUID: LazyLock<uuid::Uuid> =
    LazyLock::new(|| uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, b"veloquent"));

pub fn bytes_as_uuid(bytes: &axum::body::Bytes) -> uuid::Uuid {
    uuid::Uuid::new_v5(&UPLOAD_UUID, bytes)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn generate_hash_and_salt() {
        let (hash, salt) = gen_hash_and_salt("123456").unwrap();
        assert_eq!(hash.len(), 64);
        assert_eq!(salt.len(), 30);
        assert!(validate_passwd("123456", &salt, &hash).unwrap());
        assert!(!validate_passwd("1234356", &salt, &hash).unwrap());
    }
}
