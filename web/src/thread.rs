use {
    crate::user::UserId,
    chrono::{DateTime, Utc},
    common::{string_type, uuid_type},
    garde::Validate,
    native_db::{Key, Models, ToKey, native_db},
    native_model::{Model, native_model},
    serde::{Deserialize, Serialize},
};

uuid_type!(
    /// A unique identifier for a chat thread.
    pub(crate) ThreadId
);

string_type!(
    /// The name of a chat thread.
    #[derive(Validate)]
    pub(crate) ThreadName(#[garde(length(min = 1, max = 64))])
);

pub(crate) mod v1 {
    use super::*;

    /// A chat thread.
    #[derive(Debug, Serialize, Deserialize)]
    #[native_model(id = 1, version = 1, with = native_model::bincode_2::Bincode)]
    #[native_db]
    pub(crate) struct Thread {
        #[primary_key]
        pub(crate) id: ThreadId,
        #[secondary_key]
        pub(crate) user_id: UserId,
        pub(crate) thread_name: ThreadName,
        pub(crate) created_at: DateTime<Utc>,
        pub(crate) updated_at: DateTime<Utc>,
    }
}
