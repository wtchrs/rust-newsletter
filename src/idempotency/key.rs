#[derive(serde::Deserialize)]
pub struct IdempotencyKey(String);

impl TryFrom<String> for IdempotencyKey {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() {
            anyhow::bail!("Idempotency key cannot be empty.");
        }
        let max_length = 50;
        if s.len() > max_length {
            anyhow::bail!("Idempotency key must be shorter than {max_length} characters.");
        }
        Ok(Self(s))
    }
}

impl From<IdempotencyKey> for String {
    fn from(idempotency_key: IdempotencyKey) -> Self {
        idempotency_key.0
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
