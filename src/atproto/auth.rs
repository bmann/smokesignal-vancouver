use p256::SecretKey;

pub trait OAuthSessionProvider {
    fn oauth_access_token(&self) -> String;
    fn oauth_issuer(&self) -> String;
    fn dpop_secret(&self) -> SecretKey;
}

pub struct SimpleOAuthSessionProvider {
    pub access_token: String,
    pub issuer: String,
    pub dpop_secret: SecretKey,
}

impl OAuthSessionProvider for SimpleOAuthSessionProvider {
    fn oauth_access_token(&self) -> String {
        self.access_token.clone()
    }

    fn oauth_issuer(&self) -> String {
        self.issuer.clone()
    }

    fn dpop_secret(&self) -> SecretKey {
        self.dpop_secret.clone()
    }
}
