//! `portaki publish` — OCI push via `oci-distribution` (ORAS-compatible layout).
//!
//! Always runs `portaki build --release` first (unless `--skip-build`) so the OCI catalog layer
//! comes from `target/portaki/publish-manifest.json`, not a hand-edited repo file at publish time.
//!
//! Authenticates with `GITHUB_TOKEN` / `GHCR_TOKEN` or Docker `~/.docker/config.json`.
//!
//! Set `PORTAKI_PUBLISH_VERSION` (e.g. from CI git tag `*-vX.Y.Z`) to fail fast if `publish-manifest.json`
//! version does not match.
//!
//! Après la poussée OCI, la publication est **annoncée au registre Portaki**. Sans cette annonce
//! l'artefact existe sur GHCR mais n'entre dans aucun catalogue : c'est ce qui manquait pour que
//! l'orchestrator puisse lire son catalogue depuis le registre plutôt que depuis GHCR.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use crate::commands::build::{self, BuildArgs};
use crate::{auth, oci};

#[derive(Debug, Parser)]
/// Arguments for `portaki publish`.
pub struct PublishArgs {
    /// OCI registry prefix (GitHub Container Registry).
    #[arg(long, default_value = "ghcr.io/portakiapp")]
    pub registry: String,
    /// Validate packaging without pushing.
    #[arg(long)]
    pub dry_run: bool,
    /// Artifact directory (defaults to `target/portaki`).
    #[arg(long)]
    pub artifact_dir: Option<PathBuf>,
    /// Skip the implicit `portaki build --release` (not recommended).
    #[arg(long)]
    pub skip_build: bool,
    /// Canal de diffusion chez le registre Portaki.
    #[arg(long, default_value = "stable")]
    pub channel: String,
    /// Base URL de la plateforme. Défaut : PORTAKI_API_URL, puis la production.
    #[arg(long)]
    pub url: Option<String>,
    /// Pousser sur GHCR sans annoncer au registre — l'artefact n'entrera alors dans aucun catalogue.
    #[arg(long)]
    pub no_announce: bool,
}

/// Runs `portaki publish`.
pub async fn run(args: PublishArgs) -> Result<()> {
    let module_root = std::env::current_dir().context("current_dir")?;
    let artifact_dir = args
        .artifact_dir
        .clone()
        .unwrap_or_else(|| module_root.join("target/portaki"));

    if !args.skip_build {
        build::run(BuildArgs {
            release: true,
            manifest_only: false,
        })
        .await
        .context("portaki build --release before publish")?;
    }

    oci::package_artifact_with_root(&module_root, &artifact_dir).context("package OCI artifact")?;
    assert_publish_version_matches_env(&module_root, &artifact_dir)?;

    if args.dry_run {
        println!(
            "Dry-run: artifact ready at {} (registry: {})",
            artifact_dir.display(),
            args.registry
        );
        return Ok(());
    }

    let pushed = oci::push_artifact(&module_root, &artifact_dir, &args.registry)
        .await
        .context("push OCI artifact — set GITHUB_TOKEN or docker login ghcr.io")?;
    println!("Pushed to {} ({})", args.registry, pushed.manifest_url);

    if args.no_announce {
        println!("Skipped the registry announcement — this version is in no catalogue.");
        return Ok(());
    }

    let coords = oci::pack::read_module_coordinates(&module_root, &artifact_dir)?;
    announce(&args, &coords, &pushed).await
}

/// Annonce la publication au registre, en renouvelant le jeton une fois sur un 401.
///
/// L'échec ici n'annule pas la poussée OCI — l'artefact est sur GHCR quoi qu'il arrive. Le
/// message dit donc quoi rejouer, plutôt que de laisser croire que rien n'a eu lieu.
async fn announce(
    args: &PublishArgs,
    coords: &oci::pack::ModuleCoordinates,
    pushed: &oci::PushedArtifact,
) -> Result<()> {
    let base = auth::api_base_url(args.url.as_deref());
    let body = serde_json::json!({
        "moduleId": coords.id,
        "version": coords.version,
        "artifactRef": pushed.artifact_ref(),
        "digest": pushed.digest,
        "channel": args.channel,
    });

    let mut token = auth::access_token().context(
        "portaki login required to announce a publication — or pass --no-announce to push to GHCR only",
    )?;
    let mut outcome = post_publication(&base, &body, &token).await?;
    if outcome == Outcome::Unauthorized {
        token = auth::refresh().await?;
        outcome = post_publication(&base, &body, &token).await?;
    }

    match outcome {
        Outcome::Published => {
            println!(
                "Announced to the registry ({} @ {})",
                args.channel, pushed.digest
            );
            Ok(())
        }
        Outcome::AlreadyPublished => {
            // Rejouer une publication n'est pas une erreur d'opérateur : c'est le cas normal
            // d'une CI relancée. Le catalogue porte déjà cette version, il n'y a rien à faire.
            println!("Already in the registry ({} {})", coords.id, coords.version);
            Ok(())
        }
        Outcome::Unauthorized => anyhow::bail!(
            "the registry refused the token even after renewal — run portaki login. \
             The artifact is on GHCR: replay with portaki publish --skip-build"
        ),
        Outcome::Refused {
            status,
            code,
            message,
        } => anyhow::bail!(
            "the registry refused the publication ({status} {code}): {message}. \
             The artifact is on GHCR: fix and replay with portaki publish --skip-build"
        ),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Published,
    /// Cette version est déjà au catalogue — les publications sont immuables (ADR-0005).
    AlreadyPublished,
    Unauthorized,
    Refused {
        status: u16,
        code: String,
        message: String,
    },
}

async fn post_publication(base: &str, body: &serde_json::Value, token: &str) -> Result<Outcome> {
    let response = reqwest::Client::new()
        .post(format!(
            "{}/registry/v1/publications",
            base.trim_end_matches('/')
        ))
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .context("announce the publication to the registry")?;

    Ok(classify(
        response.status().as_u16(),
        &response.text().await.unwrap_or_default(),
    ))
}

/// Le corps de refus du registre porte un `code` stable — c'est lui qui distingue « déjà publié »
/// d'un vrai échec, et il vaut mieux que deviner à partir du seul statut : un 409 recouvre aussi
/// bien une version rejouée qu'un digest déjà ingéré sous un autre nom.
fn classify(status: u16, body: &str) -> Outcome {
    if (200..300).contains(&status) {
        return Outcome::Published;
    }
    if status == 401 {
        return Outcome::Unauthorized;
    }
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
    let code = parsed
        .get("code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();
    if code == "version_already_published" || code == "digest_already_published" {
        return Outcome::AlreadyPublished;
    }
    Outcome::Refused {
        status,
        message: parsed
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(body)
            .to_string(),
        code: if code.is_empty() {
            "unknown".to_string()
        } else {
            code
        },
    }
}

fn assert_publish_version_matches_env(module_root: &Path, artifact_dir: &Path) -> Result<()> {
    let expected = match std::env::var("PORTAKI_PUBLISH_VERSION") {
        Ok(value) => value,
        Err(_) => return Ok(()),
    };
    let expected = expected.trim();
    if expected.is_empty() {
        return Ok(());
    }
    let coords = oci::pack::read_module_coordinates(module_root, artifact_dir)?;
    if coords.version == expected {
        return Ok(());
    }
    anyhow::bail!(
        "publish-manifest version {} does not match PORTAKI_PUBLISH_VERSION={} — \
         align Cargo.toml with the git tag and rebuild (portaki build --release)",
        coords.version,
        expected
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::tempdir;

    #[test]
    fn a_replayed_publication_is_not_an_error() {
        let outcome = classify(
            409,
            r#"{"code":"version_already_published","message":"1.4.0"}"#,
        );

        assert_eq!(outcome, Outcome::AlreadyPublished);
    }

    /// Un 409 ne suffit pas : le même statut couvre un refus dont il n'y a rien à conclure.
    #[test]
    fn another_conflict_is_still_a_refusal() {
        let outcome = classify(409, r#"{"code":"something_else","message":"nope"}"#);

        assert!(matches!(outcome, Outcome::Refused { .. }));
    }

    #[test]
    fn a_refusal_keeps_the_registry_code_so_a_ci_knows_why() {
        let outcome = classify(403, r#"{"code":"module_name_not_owned","message":"nuki"}"#);

        match outcome {
            Outcome::Refused { status, code, .. } => {
                assert_eq!(status, 403);
                assert_eq!(code, "module_name_not_owned");
            }
            other => panic!("attendu un refus, obtenu {other:?}"),
        }
    }

    /// Un corps vide ou illisible ne doit pas faire passer un échec pour un succès.
    #[test]
    fn an_unreadable_refusal_is_still_a_refusal() {
        let outcome = classify(500, "<html>oops</html>");

        match outcome {
            Outcome::Refused { code, .. } => assert_eq!(code, "unknown"),
            other => panic!("attendu un refus, obtenu {other:?}"),
        }
    }

    #[test]
    fn assert_publish_version_matches_env_accepts_matching_version() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(oci::pack::PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.2.1"}"#,
        )
        .unwrap();
        unsafe {
            std::env::set_var("PORTAKI_PUBLISH_VERSION", "0.2.1");
        }
        assert_publish_version_matches_env(root.path(), &artifact).unwrap();
        unsafe {
            std::env::remove_var("PORTAKI_PUBLISH_VERSION");
        }
    }

    #[test]
    fn assert_publish_version_matches_env_rejects_mismatch() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(oci::pack::PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.1.0"}"#,
        )
        .unwrap();
        unsafe {
            std::env::set_var("PORTAKI_PUBLISH_VERSION", "0.2.1");
        }
        let err = assert_publish_version_matches_env(root.path(), &artifact).unwrap_err();
        assert!(err.to_string().contains("0.1.0"));
        assert!(err.to_string().contains("0.2.1"));
        unsafe {
            std::env::remove_var("PORTAKI_PUBLISH_VERSION");
        }
    }
}
