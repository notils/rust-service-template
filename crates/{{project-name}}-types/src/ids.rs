//! Typed identifiers.
//!
//! Distinct newtypes per entity mean the compiler rejects passing e.g. a
//! `UserId` where an `OrgId` is expected — a whole class of bug that a bare
//! `Uuid` everywhere cannot catch.
//!
//! Declare your own domain ids with the macro below:
//!
//! ```ignore
//! uuid_newtype!(
//!     /// A user of the system.
//!     UserId
//! );
//! ```

/// Declares a UUID newtype with the same small surface for each id kind.
///
/// Every path inside is fully qualified (`::uuid::Uuid`, `::serde::Serialize`)
/// rather than relying on a `use` at the call site — this macro is
/// `#[macro_export]`ed, so it must be self-contained: it may expand in a
/// crate that never imported `serde` or `uuid` under those names.
#[macro_export]
macro_rules! uuid_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::serde::Serialize,
            ::serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(::uuid::Uuid);

        impl $name {
            /// Generates a fresh time-sortable id.
            pub fn new() -> Self {
                Self(::uuid::Uuid::now_v7())
            }

            pub const fn from_uuid(id: ::uuid::Uuid) -> Self {
                Self(id)
            }

            pub const fn as_uuid(&self) -> ::uuid::Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<::uuid::Uuid> for $name {
            fn from(id: ::uuid::Uuid) -> Self {
                Self(id)
            }
        }

        impl From<$name> for ::uuid::Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = ::uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(::uuid::Uuid::parse_str(s)?))
            }
        }
    };
}

pub use uuid_newtype;

/// Declares a plain-string newtype — for identifiers that are open by design
/// (a new value is data, not a schema/code change), unlike the UUID ids above.
///
/// ```ignore
/// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// #[serde(transparent)]
/// pub struct ProviderId(String);
/// string_newtype!(ProviderId);
/// ```
#[macro_export]
macro_rules! string_newtype {
    ($name:ident) => {
        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

pub use string_newtype;

#[cfg(test)]
mod tests {
    use super::*;

    uuid_newtype!(
        /// Exists only to exercise the macro in tests — delete once you've
        /// declared your own real ids and this file's only job is the macros.
        ExampleId
    );

    #[test]
    fn ids_are_v7_and_time_sortable() {
        let first = ExampleId::new();
        let second = ExampleId::new();

        assert_eq!(first.as_uuid().get_version_num(), 7);
        // v7 ids sort by creation time, which is why they work as primary keys.
        assert!(first < second);
    }

    #[test]
    fn ids_serialize_transparently_as_a_bare_string() {
        let id = ExampleId::new();
        let json = serde_json::to_string(&id).unwrap();

        assert_eq!(json, format!("\"{id}\""));
        assert_eq!(serde_json::from_str::<ExampleId>(&json).unwrap(), id);
    }
}
