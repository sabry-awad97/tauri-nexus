//! App context and state

use super::User;
use std::sync::Mutex;

/// Application context passed to all handlers
#[derive(Clone)]
pub struct AppContext {
    pub db: DbContext,
}

impl AppContext {
    pub fn new() -> Self {
        Self {
            db: DbContext::new(),
        }
    }
}

impl Default for AppContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Database context (simulated)
#[derive(Clone)]
pub struct DbContext {
    users: std::sync::Arc<Mutex<Vec<User>>>,
    next_id: std::sync::Arc<Mutex<u32>>,
}

impl DbContext {
    pub fn new() -> Self {
        Self {
            users: std::sync::Arc::new(Mutex::new(vec![
                User {
                    id: 1,
                    name: "Alice".into(),
                    email: "alice@example.com".into(),
                    created_at: "2024-01-01T00:00:00Z".into(),
                },
                User {
                    id: 2,
                    name: "Bob".into(),
                    email: "bob@example.com".into(),
                    created_at: "2024-01-02T00:00:00Z".into(),
                },
            ])),
            next_id: std::sync::Arc::new(Mutex::new(3)),
        }
    }

    pub fn get_user(&self, id: u32) -> Option<User> {
        let users = self.users.lock().ok()?;
        users.iter().find(|u| u.id == id).cloned()
    }

    pub fn list_users(&self) -> Vec<User> {
        self.users.lock().map(|u| u.clone()).unwrap_or_default()
    }

    pub fn create_user(&self, name: String, email: String) -> Option<User> {
        let mut users = self.users.lock().ok()?;
        let mut next_id = self.next_id.lock().ok()?;

        let user = User {
            id: *next_id,
            name,
            email,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        *next_id += 1;
        users.push(user.clone());
        Some(user)
    }

    pub fn update_user(&self, id: u32, name: Option<String>, email: Option<String>) -> Option<User> {
        let mut users = self.users.lock().ok()?;
        let user = users.iter_mut().find(|u| u.id == id)?;

        if let Some(n) = name {
            user.name = n;
        }
        if let Some(e) = email {
            user.email = e;
        }

        Some(user.clone())
    }

    pub fn delete_user(&self, id: u32) -> bool {
        let mut users = self.users.lock().ok().unwrap();
        let len_before = users.len();
        users.retain(|u| u.id != id);
        users.len() < len_before
    }
}

impl Default for DbContext {
    fn default() -> Self {
        Self::new()
    }
}
