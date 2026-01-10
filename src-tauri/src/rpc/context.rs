//! Application context and services

use super::User;
use std::sync::{Arc, Mutex};

/// Application context passed to all handlers
#[derive(Clone)]
pub struct AppContext {
    /// Database service
    pub db: DbService,
}

impl AppContext {
    pub fn new() -> Self {
        Self {
            db: DbService::new(),
        }
    }
}

impl Default for AppContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Database service (simulated in-memory store)
#[derive(Clone)]
pub struct DbService {
    users: Arc<Mutex<Vec<User>>>,
    next_id: Arc<Mutex<u32>>,
}

impl DbService {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(vec![
                User::new(1, "Alice", "alice@example.com"),
                User::new(2, "Bob", "bob@example.com"),
            ])),
            next_id: Arc::new(Mutex::new(3)),
        }
    }

    pub fn get_user(&self, id: u32) -> Option<User> {
        self.users.lock().ok()?.iter().find(|u| u.id == id).cloned()
    }

    pub fn list_users(&self) -> Vec<User> {
        self.users
            .lock()
            .ok()
            .map(|u| u.clone())
            .unwrap_or_default()
    }

    pub fn create_user(&self, name: &str, email: &str) -> Option<User> {
        let mut users = self.users.lock().ok()?;
        let mut next_id = self.next_id.lock().ok()?;

        let user = User::new(*next_id, name, email);
        *next_id += 1;
        users.push(user.clone());
        Some(user)
    }

    pub fn update_user(&self, id: u32, name: Option<&str>, email: Option<&str>) -> Option<User> {
        let mut users = self.users.lock().ok()?;
        let user = users.iter_mut().find(|u| u.id == id)?;

        if let Some(n) = name {
            user.name = n.to_string();
        }
        if let Some(e) = email {
            user.email = e.to_string();
        }

        Some(user.clone())
    }

    pub fn delete_user(&self, id: u32) -> bool {
        self.users
            .lock()
            .ok()
            .map(|mut users| {
                let len = users.len();
                users.retain(|u| u.id != id);
                users.len() < len
            })
            .unwrap_or(false)
    }

    pub fn count_users(&self) -> u32 {
        self.users.lock().ok().map(|u| u.len() as u32).unwrap_or(0)
    }
}

impl Default for DbService {
    fn default() -> Self {
        Self::new()
    }
}
