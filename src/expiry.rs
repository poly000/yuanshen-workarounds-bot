use moka::Expiry;

pub struct DayExpiry;

impl<K, V> Expiry<K, V> for DayExpiry {
    fn expire_after_create(
        &self,
        _key: &K,
        _value: &V,
        _current_time: std::time::Instant,
    ) -> Option<std::time::Duration> {
        bili_wbi_sign_rs::expires_after().and_then(|d| d.to_std().ok())
    }
}
