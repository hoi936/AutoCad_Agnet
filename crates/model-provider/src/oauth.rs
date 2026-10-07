//! Public local-client OAuth. Pending values never cross IPC or enter the database.
use crate::{ProviderError, SecretValue};
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use oauth2::{CsrfToken, PkceCodeChallenge, PkceCodeVerifier};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;
use url::Url;
pub const ISSUER: &str = "https://auth.openai.com";
const TOKEN: &str = "https://auth.openai.com/api/accounts/oauth/token";
const RESOURCE: &str = "https://api.openai.com/v1";
fn err(code: &'static str) -> ProviderError {
    ProviderError::new(
        code,
        "Không thể xác minh phiên ChatGPT. Hãy đăng nhập lại; không tự đổi provider.",
    )
}
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
pub fn http() -> Result<reqwest::Client, ProviderError> {
    reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| err("auth_transport"))
}
pub async fn bounded_json(response: reqwest::Response) -> Result<Value, ProviderError> {
    let status = response.status().as_u16();
    if status != 200 {
        return Err(err(match status {
            401 => "auth_expired",
            403 => "permission_denied",
            429 => "rate_limit",
            _ => "auth_http",
        }));
    }
    let mut response = response;
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    while let Some(chunk) = response.chunk().await.map_err(|_| err("auth_transport"))? {
        if bytes.len() + chunk.len() > 1024 * 1024 {
            return Err(err("auth_size"));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| err("auth_json"))
}
#[derive(Serialize, Deserialize)]
pub struct TokenBundle {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub scope: String,
    pub expires_at: i64,
}
impl Drop for TokenBundle {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.access_token.zeroize();
        self.refresh_token.zeroize();
        self.id_token.zeroize();
    }
}
impl TokenBundle {
    pub fn permitted(&self) -> bool {
        self.scope
            .split_whitespace()
            .any(|s| s == "chatgpt.tokens.use.direct")
    }
    pub fn bearer(&self) -> Result<SecretValue, ProviderError> {
        if !self.permitted() {
            return Err(err("missing_inference_permission"));
        }
        if self.expires_at <= now() {
            return Err(err("auth_expired"));
        }
        SecretValue::new(self.access_token.clone()).map_err(|_| err("auth_token"))
    }
}
pub struct Login {
    listener: TcpListener,
    state: CsrfToken,
    nonce: String,
    verifier: PkceCodeVerifier,
    redirect: String,
    client_id: Option<String>,
    subject: Option<String>,
}
#[derive(Deserialize)]
struct Claims {
    sub: String,
    nonce: Option<String>,
    iat: i64,
    exp: i64,
    azp: Option<String>,
}
pub struct Identity {
    pub subject: String,
    pub client_id: String,
    pub bundle: TokenBundle,
}
impl Login {
    pub async fn begin(
        host: &str,
        returning: Option<(String, String)>,
    ) -> Result<(Self, String), ProviderError> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(|_| err("callback_bind"))?;
        let redirect = format!(
            "http://127.0.0.1:{}/auth/callback",
            listener
                .local_addr()
                .map_err(|_| err("callback_bind"))?
                .port()
        );
        let state = CsrfToken::new_random();
        let nonce = CsrfToken::new_random().secret().clone();
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (client_id, subject) = match returning {
            Some((id, sub)) => (Some(id), Some(sub)),
            None => (None, None),
        };
        let mut url = Url::parse("https://auth.openai.com/api/accounts/authorize").unwrap();
        {
            let mut q = url.query_pairs_mut();
            q.append_pair(
                "client_id",
                client_id.as_deref().unwrap_or("dynamic_agent_client"),
            )
            .append_pair("ext_agent_host_id", host)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", &redirect)
            .append_pair(
                "scope",
                "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct",
            )
            .append_pair("resource", RESOURCE)
            .append_pair("state", state.secret())
            .append_pair("nonce", &nonce)
            .append_pair("code_challenge_method", "S256")
            .append_pair("code_challenge", challenge.as_str());
            if client_id.is_none() {
                q.append_pair("agent_name_hint", "CAD Agent Studio");
            }
        }
        Ok((
            Self {
                listener,
                state,
                nonce,
                verifier,
                redirect,
                client_id,
                subject,
            },
            url.into(),
        ))
    }
    pub async fn finish(self, cancel: CancellationToken) -> Result<Identity, ProviderError> {
        tokio::select! {biased;_=cancel.cancelled()=>Err(err("login_cancelled")),r=tokio::time::timeout(Duration::from_secs(180),self.complete())=>r.unwrap_or_else(|_|Err(err("login_timeout")))}
    }
    async fn complete(self) -> Result<Identity, ProviderError> {
        let (mut stream, peer) = self
            .listener
            .accept()
            .await
            .map_err(|_| err("callback_io"))?;
        if !peer.ip().is_loopback() {
            return Err(err("callback_peer"));
        }
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        loop {
            let mut part = [0u8; 1024];
            let n = stream
                .read(&mut part)
                .await
                .map_err(|_| err("callback_io"))?;
            if n == 0 {
                return Err(err("callback_incomplete"));
            }
            bytes.extend_from_slice(&part[..n]);
            if bytes.len() > 16384 {
                return Err(err("callback_size"));
            }
            if bytes.windows(4).any(|s| s == b"\r\n\r\n") {
                break;
            }
        }
        let request = std::str::from_utf8(&bytes).map_err(|_| err("callback_encoding"))?;
        let line = request
            .lines()
            .next()
            .ok_or_else(|| err("callback_request"))?;
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 3 || parts[0] != "GET" || !parts[1].starts_with("/auth/callback?") {
            return Err(err("callback_path"));
        }
        let url = Url::parse(&format!(
            "{}{}",
            self.redirect.trim_end_matches("/auth/callback"),
            parts[1]
        ))
        .map_err(|_| err("callback_url"))?;
        let parsed = parse_callback(&url, self.state.secret(), self.client_id.as_deref());
        let body = if parsed.is_ok() {
            "Callback received. Return to CAD Agent Studio for verification."
        } else {
            "Sign-in rejected. Return to CAD Agent Studio."
        };
        let response=format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",body.len(),body);
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;
        drop(self.listener); // One callback; no reuse/replay.
        let (code, issued) = parsed?;
        let client = http()?;
        let value = bounded_json(
            client
                .post(TOKEN)
                .form(&[
                    ("grant_type", "authorization_code"),
                    ("client_id", issued.as_str()),
                    ("code", code.as_str()),
                    ("code_verifier", self.verifier.secret()),
                    ("redirect_uri", self.redirect.as_str()),
                    ("resource", RESOURCE),
                ])
                .send()
                .await
                .map_err(|_| err("token_exchange"))?,
        )
        .await?;
        let bundle = parse_tokens(value, None)?;
        let jwks: JwkSet = serde_json::from_value(
            bounded_json(
                client
                    .get("https://auth.openai.com/.well-known/jwks.json")
                    .send()
                    .await
                    .map_err(|_| err("jwks_fetch"))?,
            )
            .await?,
        )
        .map_err(|_| err("jwks_invalid"))?;
        let subject = verify_identity(
            &bundle.id_token,
            &issued,
            &self.nonce,
            self.subject.as_deref(),
            &jwks,
        )?;
        Ok(Identity {
            subject,
            client_id: issued,
            bundle,
        })
    }
}
fn parse_callback(
    url: &Url,
    state: &str,
    client: Option<&str>,
) -> Result<(String, String), ProviderError> {
    if url.scheme() != "http"
        || url.host_str() != Some("127.0.0.1")
        || url.path() != "/auth/callback"
        || url.fragment().is_some()
    {
        return Err(err("callback_url"));
    }
    let mut params = BTreeMap::new();
    for (key, value) in url.query_pairs() {
        if params
            .insert(key.into_owned(), value.into_owned())
            .is_some()
        {
            return Err(err("callback_duplicate"));
        }
    }
    if params.get("state").map(String::as_str) != Some(state) {
        return Err(err("callback_state"));
    }
    if params.contains_key("error") {
        return Err(err("login_declined"));
    }
    let issued = params
        .get("client_id")
        .map(String::as_str)
        .or(client)
        .ok_or_else(|| err("missing_registration"))?;
    if issued == "dynamic_agent_client"
        || !issued.starts_with("oaiapp_")
        || issued.len() > 256
        || client.is_some_and(|c| c != issued)
    {
        return Err(err("registration_mismatch"));
    }
    let code = params
        .get("code")
        .filter(|v| !v.is_empty() && v.len() < 8192)
        .ok_or_else(|| err("callback_code"))?;
    Ok((code.clone(), issued.into()))
}
fn verify_identity(
    token: &str,
    client: &str,
    nonce: &str,
    expected: Option<&str>,
    jwks: &JwkSet,
) -> Result<String, ProviderError> {
    let header = decode_header(token).map_err(|_| err("id_token_header"))?;
    if !matches!(header.alg, Algorithm::RS256 | Algorithm::ES256) {
        return Err(err("id_token_algorithm"));
    }
    let key = jwks
        .find(header.kid.as_deref().ok_or_else(|| err("id_token_kid"))?)
        .ok_or_else(|| err("id_token_key"))?;
    let key = DecodingKey::from_jwk(key).map_err(|_| err("id_token_key"))?;
    let mut validation = Validation::new(header.alg);
    validation.set_audience(&[client]);
    validation.set_issuer(&[ISSUER]);
    validation.leeway = 5;
    validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub"]);
    let claims = decode::<Claims>(token, &key, &validation)
        .map_err(|_| err("id_token_invalid"))?
        .claims;
    validate_claims(&claims, client, nonce, expected)?;
    Ok(claims.sub)
}
fn validate_claims(
    c: &Claims,
    client: &str,
    nonce: &str,
    expected: Option<&str>,
) -> Result<(), ProviderError> {
    if c.nonce.as_deref() != Some(nonce) {
        return Err(err("id_token_nonce"));
    }
    if c.sub.is_empty()
        || c.sub.len() > 512
        || expected.is_some_and(|s| s != c.sub)
        || c.azp.as_ref().is_some_and(|s| s != client)
    {
        return Err(err("identity_mismatch"));
    }
    if c.iat > now() + 5 || c.exp <= now() {
        return Err(err("id_token_expired"));
    }
    Ok(())
}
fn parse_tokens(v: Value, previous: Option<&TokenBundle>) -> Result<TokenBundle, ProviderError> {
    let access = v["access_token"]
        .as_str()
        .ok_or_else(|| err("token_missing"))?;
    if previous.is_some() && v["refresh_token"].as_str().is_none_or(|s| s.is_empty()) {
        return Err(err("refresh_rotation_missing"));
    }
    let refresh = v["refresh_token"]
        .as_str()
        .or_else(|| previous.map(|p| p.refresh_token.as_str()))
        .unwrap_or("");
    let id = v["id_token"]
        .as_str()
        .or_else(|| previous.map(|p| p.id_token.as_str()))
        .unwrap_or("");
    if !v["token_type"]
        .as_str()
        .is_some_and(|s| s.eq_ignore_ascii_case("bearer"))
        || access.len() > 32768
        || refresh.len() > 32768
        || id.len() > 32768
    {
        return Err(err("token_format"));
    }
    let seconds = v["expires_in"]
        .as_i64()
        .filter(|n| *n > 0 && *n <= 86400)
        .ok_or_else(|| err("token_expiry"))?;
    Ok(TokenBundle {
        access_token: access.into(),
        refresh_token: refresh.into(),
        id_token: id.into(),
        scope: v["scope"]
            .as_str()
            .or_else(|| previous.map(|p| p.scope.as_str()))
            .unwrap_or("")
            .into(),
        expires_at: now() + seconds,
    })
}
pub async fn refresh(client_id: &str, old: &TokenBundle) -> Result<TokenBundle, ProviderError> {
    if old.refresh_token.is_empty() {
        return Err(err("refresh_unavailable"));
    }
    let value = bounded_json(
        http()?
            .post(TOKEN)
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", client_id),
                ("refresh_token", old.refresh_token.as_str()),
                ("resource", RESOURCE),
            ])
            .send()
            .await
            .map_err(|_| err("refresh_failed"))?,
    )
    .await?;
    parse_tokens(value, Some(old))
}
pub async fn revoke(client_id: &str, bundle: &TokenBundle) -> Result<(), ProviderError> {
    let client = http()?;
    let discovery = bounded_json(
        client
            .get("https://auth.openai.com/.well-known/openid-configuration")
            .send()
            .await
            .map_err(|_| err("discovery_failed"))?,
    )
    .await?;
    let endpoint = Url::parse(
        discovery["revocation_endpoint"]
            .as_str()
            .ok_or_else(|| err("revoke_endpoint"))?,
    )
    .map_err(|_| err("revoke_endpoint"))?;
    if endpoint.scheme() != "https"
        || endpoint.host_str() != Some("auth.openai.com")
        || endpoint.port_or_known_default() != Some(443)
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
    {
        return Err(err("revoke_origin"));
    }
    let response = client
        .post(endpoint)
        .form(&[
            ("token", bundle.refresh_token.as_str()),
            ("token_type_hint", "refresh_token"),
            ("client_id", client_id),
        ])
        .send()
        .await
        .map_err(|_| err("revoke_failed"))?;
    if response.status().as_u16() != 200 {
        return Err(err("revoke_failed"));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_state_registration_and_duplicates() {
        for query in [
            "state=wrong&code=x&client_id=oaiapp_a",
            "state=s&code=x&client_id=dynamic_agent_client",
            "state=s&state=s&code=x&client_id=oaiapp_a",
        ] {
            assert!(parse_callback(
                &Url::parse(&format!("http://127.0.0.1:1/auth/callback?{query}")).unwrap(),
                "s",
                None
            )
            .is_err());
        }
        let url = Url::parse("http://127.0.0.1:1/auth/callback?state=s&code=x&client_id=oaiapp_a")
            .unwrap();
        assert!(parse_callback(&url, "s", Some("oaiapp_b")).is_err());
        assert!(parse_callback(&url, "s", None).is_ok());
    }
    #[test]
    fn identity_nonce_expiry_and_permission() {
        let mut c = Claims {
            sub: "a".into(),
            nonce: Some("n".into()),
            iat: now(),
            exp: now() + 60,
            azp: None,
        };
        assert!(validate_claims(&c, "oaiapp_a", "wrong", None).is_err());
        assert!(validate_claims(&c, "oaiapp_a", "n", Some("b")).is_err());
        c.exp = now() - 1;
        assert!(validate_claims(&c, "oaiapp_a", "n", None).is_err());
        let t = TokenBundle {
            access_token: "sentinel".into(),
            refresh_token: "".into(),
            id_token: "".into(),
            scope: "openid".into(),
            expires_at: now() + 60,
        };
        assert!(t.bearer().is_err());
    }
    #[tokio::test]
    async fn cancel_closes_loopback_and_state_failure_is_one_shot() {
        let (login, _) = Login::begin("urn:uuid:fixture", None).await.unwrap();
        let addr = login.listener.local_addr().unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(login.finish(cancel).await.is_err());
        assert!(tokio::net::TcpStream::connect(addr).await.is_err());
        let (login, _) = Login::begin("urn:uuid:fixture", None).await.unwrap();
        let addr = login.listener.local_addr().unwrap();
        let task = tokio::spawn(login.finish(CancellationToken::new()));
        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        client.write_all(b"GET /auth/callback?state=wrong&code=x&client_id=oaiapp_a HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").await.unwrap();
        assert!(task.await.unwrap().is_err());
        assert!(tokio::net::TcpStream::connect(addr).await.is_err());
    }
}

#[cfg(test)]
mod signed_identity_tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;
    fn signed(mut claims: Value) -> String {
        if claims.get("iat").is_none() {
            claims["iat"] = json!(now());
        }
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("fixture-only".into());
        encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(include_bytes!("../fixtures/oidc-test-only.pem")).unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn signature_issuer_audience_nonce_expiry_and_selected_identity() {
        let keys: JwkSet =
            serde_json::from_str(include_str!("../fixtures/oidc-test-only-jwks.json")).unwrap();
        let claims =
            json!({"iss":ISSUER,"sub":"person-a","aud":"oaiapp_a","nonce":"n","exp":now()+60});
        assert_eq!(
            verify_identity(
                &signed(claims.clone()),
                "oaiapp_a",
                "n",
                Some("person-a"),
                &keys
            )
            .unwrap(),
            "person-a"
        );
        for (key, value) in [
            ("iss", json!("https://attacker.invalid")),
            ("aud", json!("oaiapp_b")),
            ("nonce", json!("wrong")),
            ("exp", json!(now() - 300)),
            ("sub", json!("person-b")),
        ] {
            let mut bad = claims.clone();
            bad[key] = value;
            assert!(
                verify_identity(&signed(bad), "oaiapp_a", "n", Some("person-a"), &keys).is_err()
            );
        }
        let token = signed(claims);
        let mut bytes = token.into_bytes();
        let pos = bytes.len() - 20;
        bytes[pos] = if bytes[pos] == b'A' { b'B' } else { b'A' };
        assert!(verify_identity(
            std::str::from_utf8(&bytes).unwrap(),
            "oaiapp_a",
            "n",
            None,
            &keys
        )
        .is_err());
    }
}

#[cfg(test)]
mod refresh_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rotation_expiry_and_missing_refresh_fail_closed() {
        let old = TokenBundle {
            access_token: "old".into(),
            refresh_token: "refresh-old".into(),
            id_token: "id-old".into(),
            scope: "chatgpt.tokens.use.direct".into(),
            expires_at: now() - 1,
        };
        assert!(old.bearer().is_err());
        assert!(parse_tokens(
            json!({"access_token":"next","token_type":"Bearer","expires_in":60}),
            Some(&old)
        )
        .is_err());
        let next=parse_tokens(json!({"access_token":"next","refresh_token":"refresh-new","token_type":"Bearer","expires_in":60}),Some(&old)).unwrap();
        assert_eq!(next.refresh_token, "refresh-new");
        assert!(next.bearer().is_ok());
        assert!(parse_tokens(json!({"error":"invalid_grant"}), Some(&old)).is_err());
    }
}
