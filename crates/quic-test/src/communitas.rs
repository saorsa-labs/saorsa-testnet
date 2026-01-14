use ant_quic::PeerId;
use anyhow::{Context, Result};
use communitas_core::app::CommunitasApp;
use four_word_networking::FourWordEncoder;
use sha2::{Digest, Sha256};
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Deterministic demo identity derived from the QUIC peer ID.
#[derive(Clone, Debug)]
pub struct DemoIdentity {
    pub four_words: String,
    pub display_name: String,
    pub storage_dir: PathBuf,
}

impl DemoIdentity {
    /// Create a demo identity rooted under the provided base directory.
    pub fn derive(peer_id: &PeerId, base_dir: &Path) -> Result<Self> {
        let four_words = generate_four_words(peer_id)?;
        let display_name = format!("Demo {}", title_case(&four_words));
        let storage_dir = base_dir.join("communitas").join(&four_words);
        std::fs::create_dir_all(&storage_dir).with_context(|| {
            format!(
                "Failed to create Communitas storage directory {}",
                storage_dir.display()
            )
        })?;

        Ok(Self {
            four_words,
            display_name,
            storage_dir,
        })
    }
}

/// Embedded Communitas runtime handle.
pub struct CommunitasRuntime {
    identity: DemoIdentity,
    app: Arc<CommunitasApp>,
}

impl CommunitasRuntime {
    /// Launch a CommunitasApp instance tied to the node's peer ID.
    pub async fn launch(peer_id: &PeerId, base_dir: &Path, device_name: &str) -> Result<Self> {
        let identity = DemoIdentity::derive(peer_id, base_dir)?;
        let app = CommunitasApp::new(
            identity.four_words.clone(),
            identity.display_name.clone(),
            device_name.to_string(),
            identity.storage_dir.to_string_lossy().to_string(),
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize CommunitasApp")?;

        Ok(Self {
            identity,
            app: Arc::new(app),
        })
    }

    pub fn identity(&self) -> &DemoIdentity {
        &self.identity
    }

    #[allow(dead_code)]
    pub fn app(&self) -> Arc<CommunitasApp> {
        Arc::clone(&self.app)
    }
}

fn generate_four_words(peer_id: &PeerId) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(peer_id.0);
    let digest = hasher.finalize();
    let mut ip_bytes = [0u8; 4];
    ip_bytes.copy_from_slice(&digest[..4]);
    let ipv4 = Ipv4Addr::from(ip_bytes);

    let port_seed = u16::from_be_bytes([digest[4], digest[5]]);
    let port = (port_seed % 55000) + 1000; // keep within dynamic port range but non-zero

    let encoder = FourWordEncoder::new();
    let words = encoder
        .encode_ipv4(ipv4, port)
        .context("Failed to encode peer identity into four words")?;
    Ok(words.to_dotted_string().replace('.', "-"))
}

fn title_case(words: &str) -> String {
    words
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!(
                    "{}{}",
                    first.to_ascii_uppercase(),
                    chars.as_str().to_ascii_lowercase()
                ),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn peer(bytes: u8) -> PeerId {
        PeerId([bytes; 32])
    }

    #[test]
    fn demo_identity_is_deterministic_and_unique() {
        let tmp = tempdir().unwrap();
        let a = DemoIdentity::derive(&peer(1), tmp.path()).unwrap();
        let b = DemoIdentity::derive(&peer(1), tmp.path()).unwrap();
        let c = DemoIdentity::derive(&peer(2), tmp.path()).unwrap();

        assert_eq!(a.four_words, b.four_words);
        assert_ne!(a.four_words, c.four_words);
        assert!(a.four_words.matches('-').count() == 3);
        assert!(
            a.display_name.starts_with("Demo "),
            "display name should include Demo prefix"
        );
        assert!(a.storage_dir.ends_with(&a.four_words));
    }

    #[tokio::test]
    async fn runtime_launches_communitas_app() {
        let tmp = tempdir().unwrap();
        let runtime = CommunitasRuntime::launch(&peer(9), tmp.path(), "test-device")
            .await
            .expect("launch communitas");
        assert!(runtime.identity().four_words.contains('-'));
    }
}
