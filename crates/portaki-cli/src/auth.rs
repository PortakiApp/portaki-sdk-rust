//! Where the CLI keeps its credentials.
//!
//! In the system keychain — Keychain on macOS, Secret Service on Linux, Credential Manager on
//! Windows — and not in a plain file under `~/.portaki/`. Three lines more, and a stray `cat`
//! during a screencast no longer broadcasts a week of access.

use anyhow::{bail, Context, Result};

const SERVICE: &str = "app.portaki.cli";
const ACCESS_ENTRY: &str = "access-token";
const REFRESH_ENTRY: &str = "refresh-token";

/// Reads the access token: environment first, then the keychain.
///
/// The environment wins so CI can inject a token without a keychain — a build agent has none.
pub fn access_token() -> Result<String> {
    if let Ok(token) = std::env::var("PORTAKI_DEV_TOKEN") {
        if !token.trim().is_empty() {
            return Ok(token);
        }
    }
    match read(ACCESS_ENTRY) {
        Ok(Some(token)) => Ok(token),
        Ok(None) => bail!("not signed in — run `portaki login`"),
        Err(failure) => Err(failure),
    }
}

/// Renouvelle le jeton d'accès et range la paire tournée.
///
/// Le jeton d'accès vit quinze minutes, une session `--watch` bien plus. Sans ceci, elle
/// s'arrêterait au milieu sur un 401, et la seule issue serait de relancer `portaki login`.
///
/// La plateforme se souvient désormais du client et des scopes attachés au jeton de
/// rafraîchissement, donc le jeton renouvelé ouvre les mêmes portes que le premier — sans cette
/// mémoire, il repartait avec la seule audience `portaki-api`.
pub async fn refresh() -> Result<String> {
    let refresh_token = match read(REFRESH_ENTRY)? {
        Some(token) => token,
        None => bail!("no refresh token stored — run `portaki login`"),
    };

    let response = reqwest::Client::new()
        .post(format!("{}/api/v1/auth/refresh", api_base_url(None)))
        .json(&serde_json::json!({ "refreshToken": refresh_token }))
        .send()
        .await
        .context("renew the access token")?;
    let body = response.text().await.unwrap_or_default();
    let renewed: RenewedTokens = crate::api::unwrap(&body)?;

    // La rotation invalide l'ancien jeton de rafraîchissement : ne pas ranger le nouveau
    // reviendrait à se déconnecter au renouvellement suivant.
    store(&renewed.access_token, &renewed.refresh_token)?;
    Ok(renewed.access_token)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenewedTokens {
    access_token: String,
    refresh_token: String,
}

/// `--url`, puis `PORTAKI_API_URL`, puis la production.
pub fn api_base_url(explicit: Option<&str>) -> String {
    let raw = explicit
        .map(str::to_owned)
        .or_else(|| std::env::var("PORTAKI_API_URL").ok())
        .unwrap_or_else(|| "https://api.portaki.app".to_string());
    raw.trim_end_matches('/').to_string()
}

pub fn store(access_token: &str, refresh_token: &str) -> Result<()> {
    write(ACCESS_ENTRY, access_token)?;
    write(REFRESH_ENTRY, refresh_token)
}

pub fn forget() -> Result<()> {
    delete(ACCESS_ENTRY)?;
    delete(REFRESH_ENTRY)
}

fn entry(name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, name).context("open the system keychain")
}

fn read(name: &str) -> Result<Option<String>> {
    match entry(name)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(failure) => Err(failure).context("read from the system keychain"),
    }
}

fn write(name: &str, value: &str) -> Result<()> {
    entry(name)?
        .set_password(value)
        .context("write to the system keychain")
}

fn delete(name: &str) -> Result<()> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(failure) => Err(failure).context("clear the system keychain"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un seul test pour les deux cas : ils partagent une variable d'environnement, et les
    /// séparer les ferait courir en parallèle dans le même processus — donc s'écraser l'un
    /// l'autre au hasard de l'ordonnancement.
    #[test]
    fn the_environment_is_the_way_in_when_there_is_no_keychain() {
        // Un agent de CI n'a pas de trousseau : l'injection doit rester une porte d'entrée.
        std::env::set_var("PORTAKI_DEV_TOKEN", "injected");
        assert_eq!(access_token().unwrap(), "injected");

        // Une variable vide n'est pas un jeton — sinon on part avec une chaîne blanche.
        //
        // On n'exige pas d'échec : sur une machine où `portaki login` est passé, le trousseau
        // répond, et c'est le comportement voulu. Ce test affirmait le contraire et devenait
        // rouge dès la première connexion — un test dont le résultat dépend de l'historique de
        // la machine ne garde rien. L'invariant réel est qu'une variable blanche ne devient
        // jamais un jeton.
        std::env::set_var("PORTAKI_DEV_TOKEN", "   ");
        match access_token() {
            Ok(from_keychain) => assert!(
                !from_keychain.trim().is_empty(),
                "une variable blanche ne doit pas devenir un jeton"
            ),
            Err(failure) => {
                let message = failure.to_string();
                assert!(
                    message.contains("portaki login") || message.contains("keychain"),
                    "{message}"
                );
            }
        }

        std::env::remove_var("PORTAKI_DEV_TOKEN");
    }
}
