//! API keys in the OS keychain: Keychain on macOS, Credential Manager on
//! Windows, the Secret Service (GNOME Keyring, KWallet) on Linux. Keys are
//! never written to the database, logs or the UI.

const SERVICE: &str = "dev.markpitt.claudeoverview";

pub const ADMIN_KEY: &str = "anthropic-admin-key";

fn entry(name: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, name).map_err(|e| format!("The OS keychain isn't available: {e}"))
}

/// The stored key, or None if there isn't one.
pub fn get(name: &str) -> Result<Option<String>, String> {
    match entry(name)?.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Reading the key from the OS keychain: {e}")),
    }
}

/// Stores the key, or removes it with None.
pub fn set(name: &str, value: Option<&str>) -> Result<(), String> {
    let e = entry(name)?;
    match value {
        Some(v) => e.set_password(v).map_err(|e| format!("Saving the key to the OS keychain: {e}")),
        None => match e.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Removing the key from the OS keychain: {e}")),
        },
    }
}

/// The last four characters, to show which key is saved.
pub fn hint(key: &str) -> String {
    let tail: String = key.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    format!("…{tail}")
}

/// A quick shape check so an ordinary API key isn't saved by mistake.
pub fn check_admin_key(key: &str) -> Result<(), String> {
    if key.starts_with("sk-ant-admin") {
        Ok(())
    } else if key.starts_with("sk-ant-") {
        Err("That is an ordinary API key. Cost data needs an Admin key (sk-ant-admin…) from Console settings.".into())
    } else {
        Err("That doesn't look like an Admin key; they start with sk-ant-admin.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_and_checks() {
        assert_eq!(hint("sk-ant-admin01-abcdWXYZ"), "…WXYZ");
        assert_eq!(hint("ab"), "…ab");
        assert!(check_admin_key("sk-ant-admin01-x").is_ok());
        assert!(check_admin_key("sk-ant-api03-x").unwrap_err().contains("ordinary"));
        assert!(check_admin_key("hello").is_err());
    }
}
