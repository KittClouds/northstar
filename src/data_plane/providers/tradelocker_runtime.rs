//! Protected TradeLocker runtime configuration.
//!
//! Only the credential target name is configured in Northstar. User name and
//! password are read from Windows Credential Manager into short-lived process
//! memory and are never serializable or printable.

use std::fmt;
use std::sync::Arc;
use thiserror::Error;
use zeroize::Zeroize;

pub const TRADELOCKER_SERVER_ENV: &str = "NORTHSTAR_TRADELOCKER_SERVER";
pub const TRADELOCKER_CREDENTIAL_TARGET_ENV: &str = "NORTHSTAR_TRADELOCKER_CREDENTIAL_TARGET";
pub const TRADELOCKER_ACCOUNT_ID_ENV: &str = "NORTHSTAR_TRADELOCKER_ACCOUNT_ID";
pub const TRADELOCKER_ACC_NUM_ENV: &str = "NORTHSTAR_TRADELOCKER_ACC_NUM";
pub const TRADELOCKER_LIVE_ROOT: &str = "https://live.tradelocker.com/backend-api";
pub const DEFAULT_CREDENTIAL_TARGET: &str = "Northstar/TradeLocker/live";

pub struct TradeLockerSecret {
    email: String,
    password: String,
}

impl TradeLockerSecret {
    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

impl fmt::Debug for TradeLockerSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TradeLockerSecret(<redacted>)")
    }
}

impl Drop for TradeLockerSecret {
    fn drop(&mut self) {
        self.email.zeroize();
        self.password.zeroize();
    }
}

#[derive(Clone)]
pub struct TradeLockerRuntimeConfig {
    pub rest_root: Arc<str>,
    pub server: Arc<str>,
    pub credential_target: Arc<str>,
    pub account_id: Option<u64>,
    pub acc_num: Option<u64>,
}

impl fmt::Debug for TradeLockerRuntimeConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TradeLockerRuntimeConfig")
            .field("rest_root", &self.rest_root)
            .field("server", &self.server)
            .field("credential_target", &self.credential_target)
            .field("account_id", &self.account_id)
            .field("acc_num", &self.acc_num)
            .finish()
    }
}

impl TradeLockerRuntimeConfig {
    pub fn from_environment() -> Result<Self, TradeLockerRuntimeConfigError> {
        let server = std::env::var(TRADELOCKER_SERVER_ENV)
            .ok()
            .filter(|value| valid_label(value))
            .ok_or(TradeLockerRuntimeConfigError::ServerMissing)?;
        let credential_target = std::env::var(TRADELOCKER_CREDENTIAL_TARGET_ENV)
            .unwrap_or_else(|_| DEFAULT_CREDENTIAL_TARGET.into());
        if !valid_label(&credential_target) {
            return Err(TradeLockerRuntimeConfigError::CredentialTargetInvalid);
        }
        let account_id = optional_u64_env(TRADELOCKER_ACCOUNT_ID_ENV)?;
        let acc_num = optional_u64_env(TRADELOCKER_ACC_NUM_ENV)?;
        if account_id.is_some() && acc_num.is_some() {
            return Err(TradeLockerRuntimeConfigError::AmbiguousAccountSelection);
        }
        Ok(Self {
            rest_root: TRADELOCKER_LIVE_ROOT.into(),
            server: server.into(),
            credential_target: credential_target.into(),
            account_id,
            acc_num,
        })
    }

    pub fn load_secret(&self) -> Result<TradeLockerSecret, TradeLockerRuntimeConfigError> {
        read_generic_credential(&self.credential_target)
    }
}

fn valid_label(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn optional_u64_env(name: &'static str) -> Result<Option<u64>, TradeLockerRuntimeConfigError> {
    let Some(value) = std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(None);
    };
    let parsed = value
        .parse::<u64>()
        .map_err(|_| TradeLockerRuntimeConfigError::AccountSelectionInvalid(name))?;
    if parsed == 0 {
        return Err(TradeLockerRuntimeConfigError::AccountSelectionInvalid(name));
    }
    Ok(Some(parsed))
}

#[cfg(windows)]
fn read_generic_credential(
    target: &str,
) -> Result<TradeLockerSecret, TradeLockerRuntimeConfigError> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Security::Credentials::{
        CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
    };

    let mut target_wide = target.encode_utf16().collect::<Vec<_>>();
    target_wide.push(0);
    let mut pointer: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: target_wide is NUL-terminated and pointer is a valid out-param.
    let ok = unsafe { CredReadW(target_wide.as_ptr(), CRED_TYPE_GENERIC, 0, &mut pointer) };
    if ok == 0 || pointer.is_null() {
        // SAFETY: GetLastError has no preconditions.
        return Err(TradeLockerRuntimeConfigError::CredentialUnavailable(
            unsafe { GetLastError() },
        ));
    }
    // SAFETY: CredReadW returned a valid CREDENTIALW allocation owned until CredFree.
    let credential = unsafe { &*pointer };
    let email = unsafe { wide_string(credential.UserName) }?;
    let blob_len = usize::try_from(credential.CredentialBlobSize)
        .map_err(|_| TradeLockerRuntimeConfigError::CredentialEncoding)?;
    let password_bytes = if blob_len == 0 || credential.CredentialBlob.is_null() {
        Vec::new()
    } else {
        // SAFETY: CredentialBlob points to CredentialBlobSize bytes in the allocation.
        unsafe { std::slice::from_raw_parts(credential.CredentialBlob, blob_len) }.to_vec()
    };
    // SAFETY: pointer is the allocation returned by CredReadW.
    unsafe { CredFree(pointer.cast()) };
    let mut password = decode_credential_blob(&password_bytes)?;
    let mut password_bytes = password_bytes;
    password_bytes.zeroize();
    if !valid_label(&email) || password.len() < 8 || password.chars().any(char::is_control) {
        password.zeroize();
        return Err(TradeLockerRuntimeConfigError::CredentialInvalid);
    }
    Ok(TradeLockerSecret { email, password })
}

fn decode_credential_blob(bytes: &[u8]) -> Result<String, TradeLockerRuntimeConfigError> {
    if bytes.is_empty() {
        return Err(TradeLockerRuntimeConfigError::CredentialInvalid);
    }
    let looks_utf16 = bytes.len().is_multiple_of(2)
        && bytes.chunks_exact(2).filter(|pair| pair[1] == 0).count() * 4 >= bytes.len();
    if looks_utf16 {
        let units = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|unit| *unit != 0)
            .collect::<Vec<_>>();
        String::from_utf16(&units).map_err(|_| TradeLockerRuntimeConfigError::CredentialEncoding)
    } else {
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| TradeLockerRuntimeConfigError::CredentialEncoding)
    }
}

#[cfg(windows)]
unsafe fn wide_string(pointer: *const u16) -> Result<String, TradeLockerRuntimeConfigError> {
    if pointer.is_null() {
        return Err(TradeLockerRuntimeConfigError::CredentialInvalid);
    }
    let mut length = 0usize;
    while length < 4_096 {
        // SAFETY: pointer belongs to the CREDENTIALW allocation and is NUL-terminated.
        if unsafe { *pointer.add(length) } == 0 {
            // SAFETY: the preceding scan established the initialized slice length.
            return String::from_utf16(unsafe { std::slice::from_raw_parts(pointer, length) })
                .map_err(|_| TradeLockerRuntimeConfigError::CredentialEncoding);
        }
        length += 1;
    }
    Err(TradeLockerRuntimeConfigError::CredentialEncoding)
}

#[cfg(not(windows))]
fn read_generic_credential(
    _target: &str,
) -> Result<TradeLockerSecret, TradeLockerRuntimeConfigError> {
    Err(TradeLockerRuntimeConfigError::PlatformUnsupported)
}

#[derive(Debug, Error)]
pub enum TradeLockerRuntimeConfigError {
    #[error("TradeLocker server is missing; set {TRADELOCKER_SERVER_ENV}")]
    ServerMissing,
    #[error("TradeLocker credential target is invalid")]
    CredentialTargetInvalid,
    #[error("TradeLocker account selector {0} is invalid")]
    AccountSelectionInvalid(&'static str),
    #[error("TradeLocker accountId and accNum selectors cannot both be set")]
    AmbiguousAccountSelection,
    #[error("TradeLocker Windows credential is unavailable (system error {0})")]
    CredentialUnavailable(u32),
    #[error("TradeLocker Windows credential is invalid")]
    CredentialInvalid,
    #[error("TradeLocker Windows credential has invalid encoding")]
    CredentialEncoding,
    #[error("TradeLocker protected credentials are unsupported on this platform")]
    PlatformUnsupported,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_debug_never_contains_material() {
        let secret = TradeLockerSecret {
            email: "person@example.test".into(),
            password: "not-a-real-password".into(),
        };
        let debug = format!("{secret:?}");
        assert_eq!(debug, "TradeLockerSecret(<redacted>)");
        assert!(!debug.contains("person"));
        assert!(!debug.contains("password"));
    }

    #[test]
    fn config_debug_has_no_secret_field() {
        let config = TradeLockerRuntimeConfig {
            rest_root: TRADELOCKER_LIVE_ROOT.into(),
            server: "SANITIZED".into(),
            credential_target: DEFAULT_CREDENTIAL_TARGET.into(),
            account_id: None,
            acc_num: None,
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("credential_target"));
        assert!(!debug.contains("password"));
    }

    #[test]
    fn windows_generic_blob_accepts_utf8_and_utf16le() {
        assert_eq!(
            decode_credential_blob(b"sanitized-password").unwrap(),
            "sanitized-password"
        );
        let utf16 = "sanitized-password"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(
            decode_credential_blob(&utf16).unwrap(),
            "sanitized-password"
        );
    }
}
