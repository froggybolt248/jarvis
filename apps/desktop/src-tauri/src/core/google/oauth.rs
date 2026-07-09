// WP-Google owns this file.
//
// OAuth 2.0 Authorization Code + PKCE flow for Google, using a loopback
// redirect URI (http://127.0.0.1:{port}) as recommended for installed/desktop
// apps (https://developers.google.com/identity/protocols/oauth2/native-app).
//
// NOTE: The `oauth2` crate's built-in reqwest integration pulls in reqwest
// 0.12.x (see its Cargo.toml: `reqwest = "0.12"`), which is a *different*
// crate instance than this workspace's own `reqwest = "0.13.4"` dependency
// (used by `calendar_client.rs`). Both copies coexist in the dependency
// graph. We must use `oauth2::reqwest::Client` (the re-exported 0.12 copy)
// here so its type implements `oauth2::AsyncHttpClient`.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::Command;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    RefreshToken, Scope, TokenResponse, TokenUrl,
};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar";

/// Tokens obtained from a completed OAuth exchange (initial or refresh).
#[derive(Debug, Clone)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub scope: Option<String>,
}

/// Parses the request line of the loopback HTTP redirect, e.g.
/// `GET /?code=abc&state=xyz HTTP/1.1`, returning `(code, state)` with
/// URL-decoded values. Returns `None` if the line isn't a well-formed GET
/// request or is missing `code`/`state` query params.
///
/// This is pure and does not require a browser or network access, so it is
/// unit-testable in isolation.
fn parse_redirect_query(request_line: &str) -> Option<(String, String)> {
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?;
    if method != "GET" {
        return None;
    }
    let path = parts.next()?;
    let query = path.split_once('?').map(|(_, q)| q)?;

    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=')?;
        let decoded = url_decode(value);
        match key {
            "code" => code = Some(decoded),
            "state" => state = Some(decoded),
            _ => {}
        }
    }

    Some((code?, state?))
}

/// Minimal `application/x-www-form-urlencoded` percent-decoder (also maps
/// `+` to space, matching standard query-string decoding).
fn url_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(
                    std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                    16,
                ) {
                    out.push(byte);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A `BasicClient` with auth URL, token URL, and redirect URL configured
/// (client secret is also always set for our Desktop-app OAuth flow).
type GoogleOAuthClient = oauth2::Client<
    oauth2::basic::BasicErrorResponse,
    oauth2::basic::BasicTokenResponse,
    oauth2::basic::BasicTokenIntrospectionResponse,
    oauth2::StandardRevocableToken,
    oauth2::basic::BasicRevocationErrorResponse,
    oauth2::EndpointSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointSet,
>;

fn build_client(
    client_id: &str,
    client_secret: &str,
    redirect_url: &str,
) -> Result<GoogleOAuthClient> {
    let client = BasicClient::new(ClientId::new(client_id.to_string()))
        .set_client_secret(ClientSecret::new(client_secret.to_string()))
        .set_auth_uri(AuthUrl::new(AUTH_URL.to_string())?)
        .set_token_uri(TokenUrl::new(TOKEN_URL.to_string())?)
        .set_redirect_uri(RedirectUrl::new(redirect_url.to_string())?);
    Ok(client)
}

fn http_client() -> oauth2::reqwest::Client {
    oauth2::reqwest::ClientBuilder::new()
        // Following redirects on the token endpoint opens up SSRF risk; we
        // never need to follow one here.
        .redirect(oauth2::reqwest::redirect::Policy::none())
        .build()
        .expect("failed to build oauth2 http client")
}

/// Runs the full interactive desktop OAuth flow:
/// 1. Binds a loopback TCP listener on an OS-assigned free port.
/// 2. Builds the Google authorization URL (PKCE + CSRF state, requesting
///    offline access + forced consent so a refresh token is issued).
/// 3. Opens the URL in the user's default browser.
/// 4. Waits for exactly one inbound redirect, extracts `code`/`state`,
///    validates the CSRF state, and replies with a simple HTML page.
/// 5. Exchanges the authorization code (+ PKCE verifier) for tokens.
pub async fn run_loopback_flow(client_id: &str, client_secret: &str) -> Result<OAuthTokens> {
    let listener = TcpListener::bind("127.0.0.1:0").context("failed to bind loopback listener")?;
    let port = listener.local_addr()?.port();
    let redirect_url = format!("http://127.0.0.1:{port}");

    let client = build_client(client_id, client_secret, &redirect_url)?;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(CALENDAR_SCOPE.to_string()))
        .set_pkce_challenge(pkce_challenge)
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .url();

    open_in_browser(auth_url.as_str())?;

    let (code, state) = accept_one_redirect(&listener)?;
    if state != *csrf_token.secret() {
        return Err(anyhow!("OAuth CSRF state mismatch"));
    }

    let http = http_client();
    let token_result = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http)
        .await
        .map_err(|e| anyhow!("token exchange failed: {e}"))?;

    Ok(to_oauth_tokens(&token_result))
}

/// Exchanges a stored refresh token for a new access token. Google does not
/// always return a new refresh token on refresh; callers should keep the old
/// one if `OAuthTokens::refresh_token` is `None` here.
pub async fn refresh_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<OAuthTokens> {
    // Redirect URI is unused for the refresh grant, but the client type
    // requires one to be set consistently; reuse a fixed loopback value.
    let client = build_client(client_id, client_secret, "http://127.0.0.1:0")?;
    let http = http_client();

    let token_result = client
        .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
        .request_async(&http)
        .await
        .map_err(|e| anyhow!("token refresh failed: {e}"))?;

    Ok(to_oauth_tokens(&token_result))
}

fn to_oauth_tokens(token_result: &oauth2::basic::BasicTokenResponse) -> OAuthTokens {
    let expires_at = Utc::now()
        + token_result
            .expires_in()
            .unwrap_or(Duration::from_secs(3600));
    OAuthTokens {
        access_token: token_result.access_token().secret().clone(),
        refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
        expires_at,
        scope: token_result
            .scopes()
            .map(|scopes| scopes.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" ")),
    }
}

/// Opens `url` in the user's default browser on Windows via `cmd /C start`,
/// without flashing a console window.
fn open_in_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .context("failed to launch browser")?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Best-effort fallback for non-Windows dev environments.
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("failed to launch browser")?;
    }
    Ok(())
}

/// Blocks accepting exactly one connection on `listener`, parses the
/// redirect request line, writes a minimal HTML response, and returns the
/// `(code, state)` pair.
fn accept_one_redirect(listener: &TcpListener) -> Result<(String, String)> {
    let (mut stream, _) = listener
        .accept()
        .context("failed to accept OAuth redirect connection")?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .context("failed to read OAuth redirect request")?;

    let (code, state) = parse_redirect_query(&request_line)
        .ok_or_else(|| anyhow!("malformed OAuth redirect request: {request_line}"))?;

    let body = "<html><body><h3>You can close this tab and return to Jarvis.</h3></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .context("failed to write OAuth redirect response")?;

    Ok((code, state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_redirect_line() {
        let line = "GET /?code=abc&state=xyz HTTP/1.1\r\n";
        assert_eq!(
            parse_redirect_query(line),
            Some(("abc".to_string(), "xyz".to_string()))
        );
    }

    #[test]
    fn missing_code_returns_none() {
        let line = "GET /?state=xyz HTTP/1.1\r\n";
        assert_eq!(parse_redirect_query(line), None);
    }

    #[test]
    fn missing_state_returns_none() {
        let line = "GET /?code=abc HTTP/1.1\r\n";
        assert_eq!(parse_redirect_query(line), None);
    }

    #[test]
    fn non_get_request_returns_none() {
        let line = "POST /?code=abc&state=xyz HTTP/1.1\r\n";
        assert_eq!(parse_redirect_query(line), None);
    }

    #[test]
    fn no_query_string_returns_none() {
        let line = "GET / HTTP/1.1\r\n";
        assert_eq!(parse_redirect_query(line), None);
    }

    #[test]
    fn decodes_url_encoded_values() {
        // '/' encoded as %2F, literal '+' encoded as %2B, ':' as %3A
        let line = "GET /?code=a%2Fb%2Bc&state=x%3Ay HTTP/1.1\r\n";
        assert_eq!(
            parse_redirect_query(line),
            Some(("a/b+c".to_string(), "x:y".to_string()))
        );
    }

    #[test]
    fn decodes_literal_plus_as_space() {
        let line = "GET /?code=abc&state=x+y HTTP/1.1\r\n";
        assert_eq!(
            parse_redirect_query(line),
            Some(("abc".to_string(), "x y".to_string()))
        );
    }

    #[test]
    fn ignores_extra_params_and_preserves_order_independence() {
        let line = "GET /?state=xyz&scope=calendar&code=abc HTTP/1.1\r\n";
        assert_eq!(
            parse_redirect_query(line),
            Some(("abc".to_string(), "xyz".to_string()))
        );
    }

    // The full interactive flow requires a real browser, a real Google OAuth
    // client, and user interaction to grant consent — it cannot be exercised
    // in an automated unit test. To manually verify:
    //   1. Create a Desktop-app OAuth client in a GCP project's OAuth
    //      consent screen, obtaining a client_id/client_secret.
    //   2. Call `run_loopback_flow(client_id, client_secret)` from a small
    //      `#[tokio::main]` test binary.
    //   3. Approve the consent screen in the browser that opens.
    //   4. Confirm the returned `OAuthTokens` has a non-empty access_token
    //      and (on first consent) a refresh_token.
    #[test]
    #[ignore]
    fn manual_full_loopback_flow() {
        // Intentionally left as documentation; see doc-comment above.
    }
}
