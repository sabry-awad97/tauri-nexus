//! Application context and services

use super::User;
use std::sync::Arc;
use tauri_plugin_rpc::RpcError;
use tokio::sync::RwLock;

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

/// Async-safe database service using tokio::sync::RwLock
///
/// This service uses `RwLock` instead of `Mutex` to allow concurrent reads
/// without blocking, while ensuring exclusive access for writes.
#[derive(Clone)]
pub struct DbService {
    users: Arc<RwLock<Vec<User>>>,
    next_id: Arc<RwLock<u32>>,
}

impl DbService {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(vec![
                User::new(1, "Alice", "alice@example.com"),
                User::new(2, "Bob", "bob@example.com"),
            ])),
            next_id: Arc::new(RwLock::new(3)),
        }
    }

    /// Get a user by ID (read operation - allows concurrent access)
    pub async fn get_user(&self, id: u32) -> Option<User> {
        let users = self.users.read().await;
        users.iter().find(|u| u.id == id).cloned()
    }

    /// List all users (read operation - allows concurrent access)
    pub async fn list_users(&self) -> Vec<User> {
        let users = self.users.read().await;
        users.clone()
    }

    /// Create a new user (write operation - exclusive access)
    pub async fn create_user(&self, name: &str, email: &str) -> Result<User, RpcError> {
        let mut users = self.users.write().await;
        let mut next_id = self.next_id.write().await;

        let user = User::new(*next_id, name, email);
        *next_id += 1;
        users.push(user.clone());
        Ok(user)
    }

    /// Update an existing user (write operation - exclusive access)
    pub async fn update_user(
        &self,
        id: u32,
        name: Option<&str>,
        email: Option<&str>,
    ) -> Option<User> {
        let mut users = self.users.write().await;
        let user = users.iter_mut().find(|u| u.id == id)?;

        if let Some(n) = name {
            user.name = n.to_string();
        }
        if let Some(e) = email {
            user.email = e.to_string();
        }

        Some(user.clone())
    }

    /// Delete a user by ID (write operation - exclusive access)
    pub async fn delete_user(&self, id: u32) -> bool {
        let mut users = self.users.write().await;
        let len = users.len();
        users.retain(|u| u.id != id);
        users.len() < len
    }

    /// Count total users (read operation - allows concurrent access)
    pub async fn count_users(&self) -> u32 {
        let users = self.users.read().await;
        users.len() as u32
    }
}

impl Default for DbService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    // =============================================================================
    // Property-Based Tests
    // =============================================================================

    proptest! {
        /// **Property 1: Concurrent Database Access Safety**
        /// *For any* sequence of concurrent read and write operations on the DbService,
        /// the final state SHALL be consistent with some sequential ordering of those
        /// operations, and concurrent reads SHALL complete without blocking each other.
        /// **Validates: Requirements 1.2, 1.3**
        /// **Feature: tauri-rpc-plugin-optimization, Property 1: Concurrent Database Access Safety**
        #[test]
        fn prop_concurrent_database_access_safety(
            num_readers in 2usize..10,
            num_writers in 1usize..5,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = DbService::new();

                // Track created user IDs
                let created_ids = Arc::new(RwLock::new(Vec::<u32>::new()));

                // Spawn concurrent readers
                let mut reader_handles = Vec::new();
                for _ in 0..num_readers {
                    let db_clone = db.clone();
                    let handle = tokio::spawn(async move {
                        // Perform multiple read operations
                        for _ in 0..10 {
                            let users = db_clone.list_users().await;
                            // Verify we got a valid list (not corrupted)
                            prop_assert!(users.iter().all(|u| !u.name.is_empty()));

                            // Also try to get individual users
                            for user in &users {
                                let fetched = db_clone.get_user(user.id).await;
                                if let Some(u) = fetched {
                                    prop_assert_eq!(u.id, user.id);
                                }
                            }

                            // Small yield to allow interleaving
                            tokio::task::yield_now().await;
                        }
                        Ok::<_, proptest::test_runner::TestCaseError>(())
                    });
                    reader_handles.push(handle);
                }

                // Spawn concurrent writers
                let mut writer_handles = Vec::new();
                for i in 0..num_writers {
                    let db_clone = db.clone();
                    let created_ids_clone = created_ids.clone();
                    let handle = tokio::spawn(async move {
                        // Create a user
                        let name = format!("User{}", i);
                        let email = format!("user{}@test.com", i);
                        let user = db_clone.create_user(&name, &email).await?;

                        // Track the created ID
                        {
                            let mut ids = created_ids_clone.write().await;
                            ids.push(user.id);
                        }

                        // Verify the user was created
                        let fetched = db_clone.get_user(user.id).await;
                        prop_assert!(fetched.is_some(), "Created user should be fetchable");
                        prop_assert_eq!(fetched.unwrap().name, name);

                        // Small yield to allow interleaving
                        tokio::task::yield_now().await;

                        Ok::<_, proptest::test_runner::TestCaseError>(())
                    });
                    writer_handles.push(handle);
                }

                // Wait for all operations to complete
                for handle in reader_handles {
                    handle.await.unwrap()?;
                }
                for handle in writer_handles {
                    handle.await.unwrap()?;
                }

                // Verify final state consistency
                let final_users = db.list_users().await;
                let created_ids_final = created_ids.read().await;

                // All created users should exist
                for id in created_ids_final.iter() {
                    let user = db.get_user(*id).await;
                    prop_assert!(user.is_some(), "Created user {} should exist", id);
                }

                // No duplicate IDs
                let ids: HashSet<_> = final_users.iter().map(|u| u.id).collect();
                prop_assert_eq!(ids.len(), final_users.len(), "No duplicate user IDs");

                // Count should match
                let count = db.count_users().await;
                prop_assert_eq!(count as usize, final_users.len(), "Count should match list length");

                Ok(())
            })?;
        }

        /// Property: Concurrent reads do not block each other
        /// Multiple readers should be able to access the database simultaneously
        #[test]
        fn prop_concurrent_reads_do_not_block(num_readers in 5usize..20) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = DbService::new();

                // Spawn many concurrent readers
                let mut handles = Vec::new();
                for _ in 0..num_readers {
                    let db_clone = db.clone();
                    let handle = tokio::spawn(async move {
                        // Each reader performs multiple reads
                        for _ in 0..50 {
                            let _users = db_clone.list_users().await;
                            let _user = db_clone.get_user(1).await;
                            let _count = db_clone.count_users().await;
                        }
                    });
                    handles.push(handle);
                }

                // All readers should complete without deadlock
                for handle in handles {
                    handle.await.unwrap();
                }

                Ok::<_, proptest::test_runner::TestCaseError>(())
            })?;
        }

        /// Property: Write operations maintain data integrity
        /// After concurrent writes, all created users should be present
        #[test]
        fn prop_write_operations_maintain_integrity(num_writes in 1usize..10) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = DbService::new();
                let initial_count = db.count_users().await;

                // Spawn concurrent writers
                let mut handles = Vec::new();
                for i in 0..num_writes {
                    let db_clone = db.clone();
                    let handle = tokio::spawn(async move {
                        let name = format!("Writer{}", i);
                        let email = format!("writer{}@test.com", i);
                        db_clone.create_user(&name, &email).await
                    });
                    handles.push(handle);
                }

                // Collect results
                let mut created_users = Vec::new();
                for handle in handles {
                    let result = handle.await.unwrap();
                    prop_assert!(result.is_ok(), "Create should succeed");
                    created_users.push(result.unwrap());
                }

                // Verify all users were created with unique IDs
                let ids: HashSet<_> = created_users.iter().map(|u| u.id).collect();
                prop_assert_eq!(ids.len(), num_writes, "All users should have unique IDs");

                // Verify final count
                let final_count = db.count_users().await;
                prop_assert_eq!(
                    final_count,
                    initial_count + num_writes as u32,
                    "Count should increase by number of writes"
                );

                Ok(())
            })?;
        }
    }

    // =============================================================================
    // Unit Tests
    // =============================================================================

    #[tokio::test]
    async fn test_db_service_basic_operations() {
        let db = DbService::new();

        // Initial state
        let users = db.list_users().await;
        assert_eq!(users.len(), 2);

        // Get user
        let alice = db.get_user(1).await;
        assert!(alice.is_some());
        assert_eq!(alice.unwrap().name, "Alice");

        // Create user
        let new_user = db.create_user("Charlie", "charlie@test.com").await;
        assert!(new_user.is_ok());
        let charlie = new_user.unwrap();
        assert_eq!(charlie.name, "Charlie");

        // Update user
        let updated = db.update_user(charlie.id, Some("Charles"), None).await;
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().name, "Charles");

        // Delete user
        let deleted = db.delete_user(charlie.id).await;
        assert!(deleted);

        // Verify deletion
        let not_found = db.get_user(charlie.id).await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_db_service_count() {
        let db = DbService::new();

        let initial_count = db.count_users().await;
        assert_eq!(initial_count, 2);

        db.create_user("Test", "test@test.com").await.unwrap();

        let new_count = db.count_users().await;
        assert_eq!(new_count, 3);
    }
}
