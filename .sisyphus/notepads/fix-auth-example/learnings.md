## Learnings

- Fixed compilation error in `auth_example.rs` by replacing the removed `AuthManager::signer()` method with `auth_manager.key_manager().get_or_create_signer(address)`.
- The new `AuthManager` uses a `PersistentKeyManager` to handle Ed25519 keys per wallet address.
