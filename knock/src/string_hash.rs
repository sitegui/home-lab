use anyhow::Context;
use serde::{Deserialize, Serialize, Serializer};
use sha2::digest::typenum::Unsigned;
use sha2::digest::{Output, OutputSizeUser};
use sha2::{Digest, Sha256};
use std::fmt::{Debug, Formatter};

/// Represents the SHA-256 hash of a string that can be used for constant-type comparison without
/// storing the original string
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub struct StringHash(Output<Sha256>);

impl StringHash {
    pub fn new(input: &str) -> Self {
        Self(Sha256::digest(input))
    }

    fn as_hex(&self) -> String {
        hex::encode(self.0)
    }

    fn from_hex(data: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(data).context("failed to parse as hex string")?;
        let hash: [u8; <Sha256 as OutputSizeUser>::OutputSize::USIZE] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("failed to parse as a sha-256 hash"))?;

        Ok(Self(Output::<Sha256>::from(hash)))
    }
}

impl Debug for StringHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_hex())
    }
}

impl Serialize for StringHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_hex().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StringHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let a = StringHash::new("something");
        let b = StringHash::new("something else");
        let c = StringHash::new("something");

        assert_ne!(a, b);
        assert_eq!(a, c);

        let a2: StringHash = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();

        assert_eq!(a, a2);
        assert_ne!(a2, b);
        assert_eq!(a2, c);
    }
}
