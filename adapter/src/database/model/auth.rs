use std::str::FromStr;

use crate::redis::model::{RedisKey, RedisValue};
use kernel::model::{
    auth::{event::CreateToken, AccessToken},
    id::UserId,
};
use shared::error::AppError;

pub struct UserItem {
    pub user_id: UserId,
    pub password_hash: String,
}

pub struct AuthorizationKey(String);
pub struct AuthorizedUserId(UserId);

pub fn from(event: CreateToken) -> (AuthorizationKey, AuthorizedUserId) {
    (
        AuthorizationKey(event.access_token),
        AuthorizedUserId(event.user_id),
    )
}

impl From<AuthorizationKey> for AccessToken {
    fn from(key: AuthorizationKey) -> Self {
        Self(key.0)
    }
}

impl From<AccessToken> for AuthorizationKey {
    fn from(token: AccessToken) -> Self {
        Self(token.0)
    }
}

impl From<&AccessToken> for AuthorizationKey {
    fn from(token: &AccessToken) -> Self {
        Self(token.0.clone())
    }
}

impl RedisKey for AuthorizationKey {
    type Value = AuthorizedUserId;

    fn inner(&self) -> String {
        self.0.clone()
    }
}

impl RedisValue for AuthorizedUserId {
    fn inner(&self) -> String {
        self.0.to_string()
    }
}

impl TryFrom<String> for AuthorizedUserId {
    type Error = AppError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Ok(Self(UserId::from_str(&s).map_err(|e| {
            AppError::ConversionEntityError(e.to_string())
        })?))
    }
}

impl AuthorizedUserId {
    pub fn into_inner(self) -> UserId {
        self.0
    }
}
