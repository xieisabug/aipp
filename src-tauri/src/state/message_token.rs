use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub struct MessageTokenManager {
    tokens: Arc<Mutex<HashMap<i64, CancellationToken>>>,
}

impl MessageTokenManager {
    pub fn new() -> Self {
        Self { tokens: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub async fn exist(&self, message_id: i64) -> bool {
        let map = self.tokens.lock().await;
        map.contains_key(&message_id)
    }

    pub async fn store_token(&self, message_id: i64, token: CancellationToken) {
        let mut map = self.tokens.lock().await;
        map.insert(message_id, token);
    }

    pub async fn cancel_request(&self, message_id: i64) {
        let mut map = self.tokens.lock().await;
        if let Some(token) = map.remove(&message_id) {
            token.cancel();
        } else {
            println!("未找到message_id {} 对应的 cancel token", message_id);
        }
    }

    pub async fn remove_token(&self, message_id: i64) {
        let mut map = self.tokens.lock().await;
        map.remove(&message_id);
    }

    pub fn get_tokens(&self) -> Arc<Mutex<HashMap<i64, CancellationToken>>> {
        Arc::clone(&self.tokens)
    }
}
