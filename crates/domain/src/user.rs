/// A person who signed in with Yandex ID. Identified by their Yandex login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub login: String,
    pub display_name: String,
    /// Yandex avatar id, `None` if the user has no avatar of their own.
    pub avatar_id: Option<String>,
}
