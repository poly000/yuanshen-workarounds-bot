use tokio::sync::Mutex;
use once_cell::sync::OnceCell;

use crate::expiry;
use reqwest::Client;
use moka::future::Cache;

pub async fn get_mixin_key() -> anyhow::Result<String> {
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

    Ok(guard
        .get_with((), async {
            let mixin_key = unsafe { bili_wbi_sign_rs::mixin_key(wbi_key.as_bytes()) };
            mixin_key
        })
        .await)
}

pub async fn req_client_init(client: &Client) -> anyhow::Result<()> {
    client.get("https://www.bilibili.com").send().await?;
    Ok(())
}

pub static MIXIN_KEY: OnceCell<Mutex<Cache<(), String, ahash::RandomState>>> = OnceCell::new();
