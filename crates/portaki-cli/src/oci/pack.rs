//! Collects module files into OCI layers for push.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use oci_distribution::client::ImageLayer;
use serde::Deserialize;

pub const MANIFEST_MEDIA: &str = "application/vnd.portaki.manifest+json";
pub const SDK_MANIFEST_MEDIA: &str = "application/vnd.portaki.sdk.manifest+json";
const WASM_MEDIA: &str = "application/wasm";
const I18N_MEDIA: &str = "application/vnd.portaki.i18n+json";
const MIGRATIONS_BUNDLE_MEDIA: &str = "application/vnd.portaki.migrations+json";
pub const MIGRATIONS_BUNDLE: &str = "migrations.bundle.json";
const OPERATIONS_BUNDLE_MEDIA: &str = "application/vnd.portaki.operations+json";
pub const OPERATIONS_BUNDLE: &str = "operations.bundle.json";

/// OCI host-catalog layer (`portaki.module.json` freeze) — consumed by API / install.
pub const PUBLISH_MANIFEST: &str = "publish-manifest.json";
/// SDK emissions manifest (`target/portaki/manifest.json`) — wasm surfaces, capabilities, i18n keys.
pub const SDK_MANIFEST: &str = "manifest.json";

/// One blob to upload with its OCI media type.
#[derive(Debug, Clone)]
pub struct PushLayer {
    pub path: PathBuf,
    pub media_type: String,
}

/// Module coordinates read from the publish manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleCoordinates {
    pub id: String,
    pub version: String,
}

/// Parsed publish / SDK manifest (`id` + `version` for OCI tag).
#[derive(Debug, Deserialize)]
struct ArtifactManifest {
    id: String,
    version: String,
}

/// Path to the frozen manifest produced by `portaki build`.
pub fn publish_manifest_path(artifact_dir: &Path) -> PathBuf {
    artifact_dir.join(PUBLISH_MANIFEST)
}

/// Assembles `target/portaki/publish-manifest.json` from sources (catalog + optional SDK build output).
pub fn assemble_publish_manifest(module_root: &Path, artifact_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(artifact_dir).context("create artifact dir")?;
    let dest = publish_manifest_path(artifact_dir);
    let catalog_path = module_root.join("portaki.module.json");
    let sdk_path = artifact_dir.join("manifest.json");

    let source = if catalog_path.exists() {
        catalog_path
    } else if sdk_path.exists() {
        sdk_path
    } else {
        anyhow::bail!(
            "missing portaki.module.json or {} — run portaki build first",
            sdk_path.display()
        );
    };

    let raw = fs::read_to_string(&source).with_context(|| format!("read {}", source.display()))?;
    let stamped = stamp_sdk_version(&raw, resolved_sdk_version(module_root)?)?;
    fs::write(&dest, stamped).with_context(|| format!("write {}", dest.display()))?;
    Ok(dest)
}

/// Version de `portaki-sdk` <strong>réellement liée</strong>, lue dans le graphe résolu par cargo.
///
/// Pas celle déclarée : les modules dépendent du SDK par `workspace = true`, dont la contrainte
/// vaut `*`. Ce qui compte pour choisir un bundle de contrats est ce contre quoi le binaire a été
/// compilé, pas ce que quelqu'un a écrit à côté.
///
/// Rend `None` quand cargo ne répond pas ou que le SDK n'est pas dans le graphe — un module qui
/// n'en dépend pas ne se voit pas inventer une version.
fn resolved_sdk_version(module_root: &Path) -> Result<Option<String>> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .current_dir(module_root)
        .output();
    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Ok(None),
    };
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parse cargo metadata")?;
    let found = metadata
        .get("packages")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("portaki-sdk"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Ok(found)
}

/// Inscrit `requiresModuleSdk` dans le manifeste, ou refuse si l'auteur en annonce un autre.
///
/// Le champ existe au schéma depuis longtemps et <strong>aucun module ne le remplissait</strong> :
/// la plateforme n'avait donc rien pour choisir le bon jeu de contrats. L'inscrire au build le
/// rend exact par construction plutôt que par discipline.
pub fn stamp_sdk_version(raw: &str, resolved: Option<String>) -> Result<String> {
    let Some(resolved) = resolved else {
        return Ok(raw.to_string());
    };
    let mut manifest: serde_json::Value =
        serde_json::from_str(raw).context("parse module manifest")?;
    match manifest.get("requiresModuleSdk").and_then(|v| v.as_str()) {
        Some(declared) if declared != resolved => anyhow::bail!(
            "portaki.module.json declares requiresModuleSdk {declared} but the build linked \
             portaki-sdk {resolved} — drop the field and let the build stamp it"
        ),
        _ => {}
    }
    if let Some(object) = manifest.as_object_mut() {
        object.insert(
            "requiresModuleSdk".to_string(),
            serde_json::Value::String(resolved),
        );
    }
    serde_json::to_string_pretty(&manifest).context("serialise module manifest")
}

/// Reads module id/version from `publish-manifest.json` under `artifact_dir`.
pub fn read_module_coordinates(
    _module_root: &Path,
    artifact_dir: &Path,
) -> Result<ModuleCoordinates> {
    let manifest_path = publish_manifest_path(artifact_dir);
    let raw = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest: ArtifactManifest =
        serde_json::from_str(&raw).context("parse publish-manifest.json")?;
    Ok(ModuleCoordinates {
        id: manifest.id,
        version: manifest.version,
    })
}

/// Lit id/version dans `portaki.module.json`, sans passer par un build.
///
/// C'est ce qui permet d'annoncer au registre une version déjà présente sur GHCR : rien à
/// recompiler, rien à repousser, donc aucun jeton d'écriture nécessaire.
pub fn read_source_coordinates(module_root: &Path) -> Result<ModuleCoordinates> {
    let manifest_path = module_root.join("portaki.module.json");
    let raw = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest: ArtifactManifest =
        serde_json::from_str(&raw).context("parse portaki.module.json")?;
    Ok(ModuleCoordinates {
        id: manifest.id,
        version: manifest.version,
    })
}

/// Builds the OCI image reference `registry/portaki-modules-{module_id}:version`.
pub fn image_reference(registry: &str, coords: &ModuleCoordinates) -> Result<String> {
    let registry = registry.trim_end_matches('/');
    if registry.is_empty() {
        anyhow::bail!("registry must not be empty");
    }
    let owner = registry
        .strip_suffix("/portaki-modules")
        .unwrap_or(registry);
    Ok(format!(
        "{}/portaki-modules-{}:{}",
        owner, coords.id, coords.version
    ))
}

/// Discovers wasm + publish manifest + optional SDK manifest + i18n layers.
pub fn collect_push_layers(module_root: &Path, artifact_dir: &Path) -> Result<Vec<PushLayer>> {
    let coords = read_module_coordinates(module_root, artifact_dir)?;
    let mut layers = Vec::new();

    let catalog_layer_path = publish_manifest_path(artifact_dir);
    if !catalog_layer_path.exists() {
        anyhow::bail!(
            "missing {} — run portaki build before publish",
            catalog_layer_path.display()
        );
    }
    layers.push(PushLayer {
        path: catalog_layer_path.clone(),
        media_type: MANIFEST_MEDIA.to_string(),
    });

    let sdk_layer_path = artifact_dir.join(SDK_MANIFEST);
    if sdk_layer_path.exists() && publish_layer_is_host_catalog_shape(&catalog_layer_path)? {
        layers.push(PushLayer {
            path: sdk_layer_path,
            media_type: SDK_MANIFEST_MEDIA.to_string(),
        });
    }

    let wasm_path = find_wasm_artifact(module_root, &coords.id)?;
    layers.push(PushLayer {
        path: wasm_path,
        media_type: WASM_MEDIA.to_string(),
    });

    let migrations_path = artifact_dir.join(MIGRATIONS_BUNDLE);
    if migrations_path.is_file() {
        layers.push(PushLayer {
            path: migrations_path,
            media_type: MIGRATIONS_BUNDLE_MEDIA.to_string(),
        });
    }

    let operations_path = artifact_dir.join(OPERATIONS_BUNDLE);
    if operations_path.is_file() {
        layers.push(PushLayer {
            path: operations_path,
            media_type: OPERATIONS_BUNDLE_MEDIA.to_string(),
        });
    }

    let i18n_dir = module_root.join("i18n");
    if i18n_dir.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&i18n_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        entries.sort();
        for path in entries {
            layers.push(PushLayer {
                path,
                media_type: I18N_MEDIA.to_string(),
            });
        }
    }

    Ok(layers)
}

/// Host catalog is identified by localized `name` map without `manifestVersion`.
fn publish_layer_is_host_catalog_shape(path: &Path) -> Result<bool> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let root: serde_json::Value = serde_json::from_str(&raw).context("parse manifest json")?;
    if root.get("manifestVersion").is_some() {
        return Ok(false);
    }
    Ok(root
        .get("name")
        .and_then(|n| n.as_object())
        .is_some_and(|m| !m.is_empty()))
}

/// Converts push layers to `oci-distribution` image layers (reads bytes from disk).
pub fn layers_to_image_layers(layers: &[PushLayer]) -> Result<Vec<ImageLayer>> {
    let mut image_layers = Vec::with_capacity(layers.len());
    for layer in layers {
        let data = std::fs::read(&layer.path)
            .with_context(|| format!("read layer {}", layer.path.display()))?;
        image_layers.push(ImageLayer::new(data, layer.media_type.clone(), None));
    }
    Ok(image_layers)
}

fn find_wasm_artifact(module_root: &Path, module_id: &str) -> Result<PathBuf> {
    let release_dir = module_root.join("target/wasm32-unknown-unknown/release");
    let candidates = [release_dir.join(format!("{module_id}.wasm"))];
    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    if release_dir.is_dir() {
        let mut wasm_files: Vec<PathBuf> = std::fs::read_dir(&release_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("wasm"))
            .collect();
        wasm_files.sort();
        if let Some(path) = wasm_files.into_iter().next() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "no wasm artifact under {} — run portaki build --release first",
        release_dir.display()
    );
}

#[cfg(test)]
mod tests {
    /// Reprendre un catalogue déjà publié suppose de lire id/version sans build : le
    /// publish-manifest n'existe pas tant que rien n'a été compilé.
    #[test]
    fn the_linked_sdk_version_is_stamped_into_the_manifest() {
        let stamped = stamp_sdk_version(
            r#"{"id":"weather","version":"0.3.24"}"#,
            Some("2.1.1".into()),
        )
        .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&stamped).unwrap();
        assert_eq!(parsed["requiresModuleSdk"], "2.1.1");
        assert_eq!(parsed["id"], "weather");
    }

    /// Un manifeste qui annonce une autre version ment sur ce qui a été compilé.
    #[test]
    fn a_declared_version_that_disagrees_is_refused() {
        let err = stamp_sdk_version(
            r#"{"id":"weather","version":"0.3.24","requiresModuleSdk":"1.0.0"}"#,
            Some("2.1.1".into()),
        )
        .unwrap_err();

        assert!(err.to_string().contains("1.0.0"));
        assert!(err.to_string().contains("2.1.1"));
    }

    /// Déclarée et liée d'accord : rien à signaler.
    #[test]
    fn a_declared_version_that_agrees_passes() {
        stamp_sdk_version(
            r#"{"id":"weather","requiresModuleSdk":"2.1.1"}"#,
            Some("2.1.1".into()),
        )
        .unwrap();
    }

    /// Un module qui ne dépend pas du SDK ne se voit pas inventer une version.
    #[test]
    fn without_a_resolved_sdk_the_manifest_is_untouched() {
        let raw = r#"{"id":"weather","version":"0.3.24"}"#;

        assert_eq!(stamp_sdk_version(raw, None).unwrap(), raw);
    }

    #[test]
    fn source_coordinates_are_read_without_a_build() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("portaki.module.json"),
            r#"{"id":"weather","version":"0.3.24"}"#,
        )
        .unwrap();

        let coords = read_source_coordinates(dir.path()).unwrap();

        assert_eq!(coords.id, "weather");
        assert_eq!(coords.version, "0.3.24");
    }

    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn assemble_publish_manifest_copies_catalog_source() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("portaki.module.json"),
            r#"{"id":"weather","version":"1.3.2"}"#,
        )
        .unwrap();
        let artifact = root.path().join("target/portaki");
        let path = assemble_publish_manifest(root.path(), &artifact).unwrap();
        assert_eq!(path, artifact.join(PUBLISH_MANIFEST));
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"version\":\"1.3.2\""));
    }

    #[test]
    fn assemble_publish_manifest_copies_sdk_manifest_when_no_catalog() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join("manifest.json"),
            r#"{"id":"weather","version":"0.2.0"}"#,
        )
        .unwrap();
        assemble_publish_manifest(root.path(), &artifact).unwrap();
        let raw = fs::read_to_string(artifact.join(PUBLISH_MANIFEST)).unwrap();
        assert!(raw.contains("\"version\":\"0.2.0\""));
    }

    #[test]
    fn read_module_coordinates_reads_publish_manifest_only() {
        let dir = tempdir().unwrap();
        let artifact = dir.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            dir.path().join("portaki.module.json"),
            r#"{"id":"stale","version":"0.0.1"}"#,
        )
        .unwrap();
        fs::write(
            artifact.join(PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.2.0"}"#,
        )
        .unwrap();
        let coords = read_module_coordinates(dir.path(), &artifact).unwrap();
        assert_eq!(
            coords,
            ModuleCoordinates {
                id: "weather".to_string(),
                version: "0.2.0".to_string(),
            }
        );
    }

    #[test]
    fn image_reference_formats_registry_tag() {
        let coords = ModuleCoordinates {
            id: "weather".into(),
            version: "0.2.0".into(),
        };
        let reference = image_reference("ghcr.io/portakiapp/portaki-modules", &coords).unwrap();
        assert_eq!(
            reference,
            "ghcr.io/portakiapp/portaki-modules-weather:0.2.0"
        );
        let reference = image_reference("ghcr.io/portakiapp", &coords).unwrap();
        assert_eq!(
            reference,
            "ghcr.io/portakiapp/portaki-modules-weather:0.2.0"
        );
    }

    #[test]
    fn collect_push_layers_uses_publish_manifest_not_repo_catalog() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("portaki.module.json"),
            r#"{"id":"weather","version":"9.9.9"}"#,
        )
        .unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.1.0"}"#,
        )
        .unwrap();

        let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join("weather.wasm"), b"\0asm").unwrap();

        let layers = collect_push_layers(root.path(), &artifact).unwrap();
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].path, artifact.join(PUBLISH_MANIFEST));
        assert_eq!(layers[0].media_type, MANIFEST_MEDIA);
    }

    #[test]
    fn collect_push_layers_includes_sdk_when_host_catalog_present() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("portaki.module.json"),
            r#"{"id":"weather","version":"1.3.2","name":{"fr":"Météo","en":"Weather"},"description":{"fr":"d","en":"d"}}"#,
        )
        .unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"1.3.2","name":{"fr":"Météo","en":"Weather"},"description":{"fr":"d","en":"d"}}"#,
        )
        .unwrap();
        fs::write(
            artifact.join(SDK_MANIFEST),
            r#"{"manifestVersion":"1","id":"weather","version":"0.2.1","displayName":"module.name"}"#,
        )
        .unwrap();
        let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join("weather.wasm"), b"\0asm").unwrap();

        let layers = collect_push_layers(root.path(), &artifact).unwrap();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].media_type, MANIFEST_MEDIA);
        assert_eq!(layers[1].media_type, SDK_MANIFEST_MEDIA);
        assert_eq!(layers[1].path, artifact.join(SDK_MANIFEST));
    }

    #[test]
    fn collect_push_layers_sdk_only_single_manifest_layer() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(PUBLISH_MANIFEST),
            r#"{"manifestVersion":"1","id":"weather","version":"0.2.1"}"#,
        )
        .unwrap();
        let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join("weather.wasm"), b"\0asm").unwrap();

        let layers = collect_push_layers(root.path(), &artifact).unwrap();
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].media_type, MANIFEST_MEDIA);
        assert!(layers
            .iter()
            .all(|layer| layer.media_type != SDK_MANIFEST_MEDIA));
    }
}
