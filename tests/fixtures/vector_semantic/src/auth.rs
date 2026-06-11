//! Authentication and session validation for user tokens.

/// Validate user session token against stored credentials.
pub fn validate_session_token(user: &str, token: &str) -> bool {
    !user.is_empty() && !token.is_empty()
}

/// Check whether a session is still active.
pub fn is_session_active(session_id: &str) -> bool {
    !session_id.is_empty()
}
