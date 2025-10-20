//! Helpful [declarative macros][1].
//!
//! Note that macros with `#[macro_export]`are exposed at the root of the
//! create.
//!
//! [1]: <https://doc.rust-lang.org/book/ch20-05-macros.html#declarative-macros-with-macro_rules-for-general-metaprogramming>

/// Macro to create a new-type UUID type.
#[macro_export]
macro_rules! uuid_type {
    ($(#[$attr:meta])* $vis:vis $name:ident) => {
        #[derive(
            ::derive_more::Display,
            ::derive_more::From,
            ::derive_more::FromStr,
            ::serde::Deserialize,
            ::serde::Serialize,
            Clone,
            Copy,
            Debug,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
        )]
        $(#[$attr])*
        #[from(forward)]
        #[serde(transparent)]
        $vis struct $name(::uuid::Uuid);

        #[allow(dead_code)]
        impl $name {
            #[doc = concat!("Creates a new, random `", stringify!($name), "`.")]
            $vis fn new() -> Self {
                Self(::uuid::Uuid::new_v4())
            }

            #[doc = concat!("Parses a `", stringify!($name), "` from a string of hexadecimal digits with optional hyphens.")]
            $vis fn try_parse(value: &str) -> ::std::result::Result<Self, ::uuid::Error> {
                Ok(Self(::uuid::Uuid::try_parse(value)?))
            }
        }

        impl ::std::default::Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::std::convert::AsRef<::uuid::Uuid> for $name {
            fn as_ref(&self) -> &::uuid::Uuid {
                &self.0
            }
        }
    };
}
