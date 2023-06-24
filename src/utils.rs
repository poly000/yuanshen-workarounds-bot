use arrayvec::ArrayString;
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;

use crate::expiry;
use moka::future::Cache;
use reqwest::Client;

pub async fn get_mixin_key() -> anyhow::Result<ArrayString<32>> {
    let lock = MIXIN_KEY.get_or_init(|| {
        Mutex::new(
            Cache::builder()
                .max_capacity(1)
                .expire_after(expiry::DayExpiry)
                .build_with_hasher(ahash::RandomState::default()),
        )
    });
    let guard = lock.lock().await;
    if let Some(key) = guard.get(&()) {
        return Ok(key);
    }

    let client = Client::builder().user_agent(crate::USER_AGENT).build()?;
    req_client_init(&client).await?;
    let wbi_key = bili_wbi_sign_rs::get_wbi_keys(&client).await?;
    let _mixin_key = unsafe { bili_wbi_sign_rs::mixin_key(wbi_key.as_bytes()) };
    let mut mixin_key = ArrayString::new();
    mixin_key.push_str(&_mixin_key);
    guard.insert((), mixin_key).await;
    Ok(mixin_key)
}

pub async fn req_client_init(client: &Client) -> anyhow::Result<()> {
    client.get("https://www.bilibili.com").send().await?;
    Ok(())
}

pub static MIXIN_KEY: OnceCell<Mutex<Cache<(), ArrayString<32>, ahash::RandomState>>> =
    OnceCell::new();
