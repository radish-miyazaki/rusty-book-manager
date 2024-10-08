use std::env;

use strum::EnumString;

#[derive(Default, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum Environment {
    Development,
    #[default]
    Production,
}

/// 開発環境・本番環境のどちら向けのビルドであるかを示す。
pub fn which() -> Environment {
    // debug_assertions が true の場合はデバッグビルド
    #[cfg(debug_assertions)]
    let default_env = Environment::Development;
    #[cfg(not(debug_assertions))]
    let default_env = Environment::Production;

    match env::var("ENV") {
        Ok(v) => v.parse().unwrap_or(default_env),
        Err(_) => default_env,
    }
}
