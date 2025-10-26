use {
    anyhow::{Context, Result},
    base64::{
        Engine,
        engine::general_purpose::{GeneralPurpose, NO_PAD},
    },
    blake3::{Hash, KEY_LEN},
    common::{bincode, key_type},
    hex::FromHex,
    serde::{Deserialize, Serialize, de::DeserializeOwned},
    std::{fmt, sync::OnceLock},
};

const BASE_64: GeneralPurpose = GeneralPurpose::new(&base64::alphabet::URL_SAFE, NO_PAD);

static KEY: OnceLock<[u8; KEY_LEN]> = OnceLock::new();

key_type!(
    /// The key for hashing data in hex.
    pub(crate) HashKey
);

#[inline]
fn key() -> &'static [u8; KEY_LEN] {
    KEY.get().expect("not initialized")
}

/// Initialize the Blake3 key.
#[deny(dead_code)]
pub(crate) fn init(hash_key: HashKey) -> Result<()> {
    KEY.set(<[u8; KEY_LEN]>::from_hex(&hash_key.0).context("failed to parse hash key")?)
        .expect("already initialized");

    Ok(())
}

/// A Blake3 backed hashed data container.
///
/// Serializes to URL safe base64 encoded string.
#[derive(Serialize, Deserialize)]
pub(crate) struct Container<T> {
    data: T,
    signature: [u8; blake3::OUT_LEN],
}

impl<T> Container<T> {
    /// Extracts the inner data from the container.
    pub(crate) fn into_inner(self) -> T {
        self.data
    }
}

impl<T> Container<T> {
    pub(crate) fn new(data: T) -> Result<Self>
    where
        T: Serialize,
    {
        Ok(Self {
            signature: *blake3::keyed_hash(key(), &bincode::serialize(&data)?).as_bytes(),
            data,
        })
    }
}

impl<T> fmt::Display for Container<T>
where
    T: Serialize,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            BASE_64.encode(bincode::serialize(&self).map_err(|_| fmt::Error)?)
        )
    }
}

impl<T> std::str::FromStr for Container<T>
where
    T: Serialize + DeserializeOwned,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let container = bincode::deserialize::<Container<T>>(&BASE_64.decode(s)?)?;

        if Hash::from(container.signature)
            != blake3::keyed_hash(key(), &bincode::serialize(&container.data)?)
        {
            anyhow::bail!("invalid hash signature");
        }

        Ok(container)
    }
}
