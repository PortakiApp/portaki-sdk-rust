//! OCI artifact packaging and push (ORAS-compatible layout).

mod auth;
pub mod pack;

use std::path::Path;

use anyhow::{Context, Result};
use oci_distribution::client::{Client, Config};
use oci_distribution::Reference;

/// Validates that `portaki build` produced a publish manifest under `artifact_dir`.
pub fn package_artifact_with_root(_module_root: &Path, artifact_dir: &Path) -> Result<()> {
    let publish = pack::publish_manifest_path(artifact_dir);
    if !publish.exists() {
        anyhow::bail!("missing {} — run portaki build first", publish.display());
    }
    Ok(())
}

/// Ce qu'une poussée laisse derrière elle.
///
/// Le digest est relu chez le registre OCI plutôt que déduit de l'URL du manifeste : c'est lui
/// qui identifie une publication chez le registre Portaki (ADR-0005), et le déduire d'un
/// en-tête `Location` serait suspendu au format d'un serveur.
#[derive(Debug, Clone)]
pub struct PushedArtifact {
    /// `ghcr.io/portakiapp/portaki-modules-nuki:1.4.0`
    pub image_ref: String,
    /// URL du manifeste, telle que rendue par le registre OCI.
    pub manifest_url: String,
    /// `sha256:…`
    pub digest: String,
}

impl PushedArtifact {
    /// La forme attendue par `POST /registry/v1/publications` — le tag y est ignoré, seul le
    /// dépôt compte, mais le garder rend la trace lisible.
    pub fn artifact_ref(&self) -> String {
        format!("oci://{}", self.image_ref)
    }
}

/// Pushes the module artifact to `registry` using `oci-distribution`.
///
/// Expects `portaki build` output under `artifact_dir`:
/// - `publish-manifest.json` (frozen catalog for OCI)
/// - `module_root/target/wasm32-unknown-unknown/release/*.wasm`
/// - `module_root/i18n/*.json` (optional)
///
/// Authentication: `GITHUB_TOKEN` / `GHCR_TOKEN` or Docker `config.json` for the registry host.
pub async fn push_artifact(
    module_root: &Path,
    artifact_dir: &Path,
    registry: &str,
) -> Result<PushedArtifact> {
    package_artifact_with_root(module_root, artifact_dir)?;

    let layers = pack::collect_push_layers(module_root, artifact_dir)?;
    let coords = pack::read_module_coordinates(module_root, artifact_dir)?;
    let image_ref = pack::image_reference(registry, &coords)?;
    let reference: Reference = image_ref
        .parse()
        .with_context(|| format!("invalid OCI reference: {image_ref}"))?;

    let image_layers = pack::layers_to_image_layers(&layers)?;
    let config = Config::new(
        br#"{}"#.to_vec(),
        "application/vnd.oci.empty.v1+json".to_string(),
        None,
    );

    let auth = auth::resolve_registry_auth(registry)?;
    let client = Client::default();
    let response = client
        .push(&reference, &image_layers, config, &auth, None)
        .await
        .context("OCI push to registry")?;

    let digest = client
        .fetch_manifest_digest(&reference, &auth)
        .await
        .context("read back the pushed manifest digest")?;

    Ok(PushedArtifact {
        image_ref,
        manifest_url: response.manifest_url,
        digest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_artifact_ref_carries_the_oci_scheme_the_registry_strips() {
        let pushed = PushedArtifact {
            image_ref: "ghcr.io/portakiapp/portaki-modules-nuki:1.4.0".to_string(),
            manifest_url: String::new(),
            digest: "sha256:9f2c".to_string(),
        };

        assert_eq!(
            pushed.artifact_ref(),
            "oci://ghcr.io/portakiapp/portaki-modules-nuki:1.4.0"
        );
    }

    #[test]
    fn package_artifact_requires_publish_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let err = package_artifact_with_root(root, root).unwrap_err();
        assert!(err.to_string().contains("publish-manifest.json"));
    }
}
