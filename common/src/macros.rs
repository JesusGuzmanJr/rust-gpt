//! Helpful [declarative macros][1].
//!
//! Note that macros with `#[macro_export]`are exposed at the root of the
//! create.
//!
//! [1]: <https://doc.rust-lang.org/book/ch20-05-macros.html#declarative-macros-with-macro_rules-for-general-metaprogramming>

/// Macro to create a new-type string type.
#[macro_export]
macro_rules! string_type {
    ($(#[$attr:meta])* $vis:vis $name:ident $(#[$inner_attr:meta])*)  => {
        #[derive(
            ::derive_more::Display,
            ::derive_more::From,
            ::derive_more::FromStr,
            ::serde::Deserialize,
            ::serde::Serialize,
            Clone,
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
        $vis struct $name($(#[$inner_attr])* ::std::string::String);

        #[allow(dead_code)]
        impl $name {
            #[doc = concat!("Creates a new `", stringify!($name), "`.")]
            pub fn new(value: impl ToString) -> Self {
                Self(value.to_string())
            }

            /// Returns the length in bytes.
            pub fn len(&self) -> usize {
                self.0.len()
            }

            /// Returns `true` if the length is zero bytes.
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }

            #[doc = concat!("Trims whitespace from the `", stringify!($name), "`.")]
            pub fn trim(&mut self)  {
                self.0 = self.0.trim().into();
            }

            #[doc = concat!("Reference to the `", stringify!($name), "` as a string slice.")]
            pub fn as_str(&self) -> &::std::primitive::str {
                &self.0
            }
        }

        impl ::std::convert::AsRef<str> for $name {
            fn as_ref(&self) -> &::std::primitive::str {
                &self.0
            }
        }

        impl ::std::ops::Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
    ($(#[$attr:meta])* $vis:vis $name:ident($(#[$inner_attr:meta])+)) => {
        string_type! {
            $(#[$attr])*
            $vis $name $(#[$inner_attr])+
        }
    };
}

/// Macro to generate a `Key` type that is used to store a secret key.
/// The type implements an obfuscated `Debug` to avoid leaking the key in
/// logs.
#[macro_export]
macro_rules! key_type {
    ($(#[$attr:meta])* $vis:vis $name:ident) => {
        #[derive(
            ::derive_more::From,
            ::serde::Deserialize,
            ::serde::Serialize,
            ::zeroize::ZeroizeOnDrop,
            Clone,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
        )]
        $(#[$attr])*
        #[from(forward)]
        #[serde(transparent)]
        $vis struct $name(::std::string::String);

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                formatter.write_fmt(format_args!("{}(*****)", stringify!($name)))
            }
        }
    };
    ($(#[$attr:meta])* $vis:vis $name:ident ($(#[$inner_attr:meta])+)) => {
        #[derive(
            ::derive_more::From,
            ::serde::Deserialize,
            ::serde::Serialize,
            ::zeroize::ZeroizeOnDrop,
            Clone,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
        )]
        $(#[$attr])*
        #[from(forward)]
        #[serde(transparent)]
        $vis struct $name ($(#[$inner_attr])* ::std::string::String);

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                formatter.write_fmt(format_args!("{}(*****)", stringify!($name)))
            }
        }
    };
}

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
