use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use config::Secret;
use http::HeaderMap;
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;
use thiserror::Error;

pub const SESSION_COOKIE: &str = "jet_black_session";
pub const CSRF_HEADER: &str = "x-csrf-token";
const TOKEN_BYTES: usize = 32;
const SESSION_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionExchange {
    pub csrf_token: String,
    pub expires_at_unix_ms: i64,
}

#[derive(Debug)]
struct Session {
    csrf_token: String,
    expires_at_unix_ms: i64,
}

#[derive(Debug)]
pub struct SessionManager {
    launch_token: Mutex<Option<String>>,
    sessions: Mutex<HashMap<String, Session>>,
}

impl SessionManager {
    pub fn new() -> Result<(Self, Secret), AuthError> {
        let launch_token = random_token()?;
        Ok((
            Self {
                launch_token: Mutex::new(Some(launch_token.clone())),
                sessions: Mutex::new(HashMap::new()),
            },
            Secret::new(launch_token),
        ))
    }

    pub fn exchange(&self, presented: &str) -> Result<(String, SessionExchange), AuthError> {
        let session_token = random_token()?;
        let csrf_token = random_token()?;
        let expires_at_unix_ms = now_unix_ms() + SESSION_TTL.as_millis() as i64;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AuthError::StateUnavailable)?;
        let mut launch_token = self
            .launch_token
            .lock()
            .map_err(|_| AuthError::StateUnavailable)?;
        let expected = launch_token.as_deref().ok_or(AuthError::ExchangeConsumed)?;
        if !constant_time_equal(expected, presented) {
            return Err(AuthError::InvalidExchangeToken);
        }
        *launch_token = None;
        sessions.insert(
            session_token.clone(),
            Session {
                csrf_token: csrf_token.clone(),
                expires_at_unix_ms,
            },
        );
        Ok((
            session_token,
            SessionExchange {
                csrf_token,
                expires_at_unix_ms,
            },
        ))
    }

    pub fn authenticate(&self, headers: &HeaderMap, require_csrf: bool) -> Result<(), AuthError> {
        let session_token =
            cookie_value(headers, SESSION_COOKIE).ok_or(AuthError::MissingSession)?;
        let now = now_unix_ms();
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AuthError::StateUnavailable)?;
        sessions.retain(|_, session| session.expires_at_unix_ms > now);
        let session = sessions
            .get(session_token)
            .ok_or(AuthError::InvalidSession)?;
        if require_csrf {
            let csrf = headers
                .get(CSRF_HEADER)
                .and_then(|value| value.to_str().ok())
                .ok_or(AuthError::MissingCsrf)?;
            if !constant_time_equal(&session.csrf_token, csrf) {
                return Err(AuthError::InvalidCsrf);
            }
        }
        Ok(())
    }
}

pub fn session_cookie(token: &str) -> String {
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}",
        SESSION_TTL.as_secs()
    )
}

fn cookie_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(http::header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|cookie| cookie.strip_prefix(&format!("{name}=")))
}

fn constant_time_equal(expected: &str, presented: &str) -> bool {
    expected.as_bytes().ct_eq(presented.as_bytes()).into()
}

fn random_token() -> Result<String, AuthError> {
    let mut bytes = [0_u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|_| AuthError::RandomnessUnavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("local authentication state is unavailable")]
    StateUnavailable,
    #[error("secure randomness is unavailable")]
    RandomnessUnavailable,
    #[error("the launch exchange token is invalid")]
    InvalidExchangeToken,
    #[error("the launch exchange token has already been consumed")]
    ExchangeConsumed,
    #[error("the local session cookie is missing")]
    MissingSession,
    #[error("the local session is invalid or expired")]
    InvalidSession,
    #[error("the CSRF token is missing")]
    MissingCsrf,
    #[error("the CSRF token is invalid")]
    InvalidCsrf,
}
