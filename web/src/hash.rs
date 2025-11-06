use {
    anyhow::{Context, Result},
    base64::{
        Engine,
        engine::general_purpose::{GeneralPurpose, NO_PAD},
    },
    blake3::{Hash, KEY_LEN},
    common::{bincode, key_type},
    hex::FromHex,
    serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned},
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
pub(crate) struct GlassVault<T>(Container<T>);

impl<T> fmt::Debug for GlassVault<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("GlassVault({:?})", self.0.data))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Container<T> {
    data: T,
    signature: [u8; blake3::OUT_LEN],
}

impl<T> GlassVault<T> {
    /// Extracts the inner data from the container.
    pub(crate) fn into_inner(self) -> T {
        self.0.data
    }
}

impl<T> GlassVault<T> {
    pub(crate) fn new(data: T) -> Result<Self>
    where
        T: Serialize,
    {
        Ok(Self(Container {
            signature: *blake3::keyed_hash(key(), &bincode::serialize(&data)?).as_bytes(),
            data,
        }))
    }
}

impl<T> fmt::Display for GlassVault<T>
where
    T: Serialize,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            BASE_64.encode(bincode::serialize(&self.0).map_err(|_| fmt::Error)?)
        )
    }
}

impl<T> std::str::FromStr for GlassVault<T>
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

        Ok(Self(container))
    }
}

impl<'de, T> Deserialize<'de> for GlassVault<T>
where
    T: Serialize + DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        std::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl<T> Serialize for GlassVault<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
