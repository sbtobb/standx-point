# JSON Persistence Patterns in Rust - Learnings

## Atomic File Writes

**Pattern**: Write to temporary file, then rename (atomic operation)

**Implementation**:
```rust
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

fn atomic_write_json<T: serde::Serialize>(
    path: impl AsRef<Path>,
    data: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let parent_dir = path.as_ref().parent().ok_or("Invalid path")?;
    
    // Create temporary file in the same directory
    let mut temp_file = NamedTempFile::new_in(parent_dir)?;
    
    // Serialize data to JSON and write to temp file
    let json_str = serde_json::to_string_pretty(data)?;
    temp_file.write_all(json_str.as_bytes())?;
    temp_file.flush()?;
    
    // Atomic rename (guaranteed by OS if same filesystem)
    temp_file.persist(path)?;
    
    Ok(())
}
```

**Key Crates**:
- `tempfile`: For creating temporary files in safe manner
- `serde_json`: For JSON serialization
- `fs` module: For file operations

**Advantages**:
- Prevents partial writes (file remains in consistent state)
- Handles errors before overwriting original file
- Atomic rename is guaranteed by most file systems

---

## Async File I/O with tokio::fs

**Pattern**: Async reading/writing of JSON files with Tokio

**Implementation**:
```rust
use tokio::fs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    host: String,
    port: u16,
    timeout: u64,
}

async fn read_config() -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string("config.json").await?;
    let config: Config = serde_json::from_str(&contents)?;
    Ok(config)
}

async fn write_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let json_str = serde_json::to_string_pretty(config)?;
    fs::write("config.json", json_str).await?;
    Ok(())
}

async fn read_large_file() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = fs::File::open("large_data.json").await?;
    let mut buffer = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut buffer).await?;
    Ok(buffer)
}

async fn write_large_file(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = fs::File::create("large_data.json").await?;
    tokio::io::AsyncWriteExt::write_all(&mut file, data).await?;
    file.flush().await?;
    Ok(())
}
```

**Key Crates**:
- `tokio`: Async runtime and file system operations
- `serde_json`: JSON serialization
- `tokio::io`: Async I/O traits

**Best Practices**:
- Use `read_to_string` and `write` for small files (< 1MB)
- Use `AsyncReadExt` and `AsyncWriteExt` with buffers for large files
- Always handle errors properly with Result types
- Consider buffering for large I/O operations

---

## Schema Migration Patterns

**Pattern**: Versioned data structures with serde

**Implementation**:
```rust
use serde::{Deserialize, Serialize};

// Version 1 of the data structure
#[derive(Deserialize)]
struct UserV1 {
    id: u64,
    username: String,
    email: String,
}

// Version 2 adds optional profile field
#[derive(Serialize, Deserialize, Debug)]
struct UserV2 {
    id: u64,
    username: String,
    email: String,
    profile: Option<Profile>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Profile {
    bio: String,
    avatar_url: String,
}

// Migration function from V1 to V2
fn migrate_user_v1_to_v2(user_v1: UserV1) -> UserV2 {
    UserV2 {
        id: user_v1.id,
        username: user_v1.username,
        email: user_v1.email,
        profile: None, // Default for new field
    }
}

// Generic migration helper
fn read_and_migrate_config() -> Result<UserV2, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string("user.json")?;
    
    // Try to deserialize to latest version first
    match serde_json::from_str(&contents) {
        Ok(user) => Ok(user),
        Err(_) => {
            // If that fails, try to deserialize to V1 and migrate
            let user_v1: UserV1 = serde_json::from_str(&contents)?;
            Ok(migrate_user_v1_to_v2(user_v1))
        }
    }
}

// For forward compatibility (ignore unknown fields)
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
struct Config {
    host: String,
    port: u16,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
            extra: Default::default(),
        }
    }
}
```

**Key Patterns**:
1. **Versioned structs**: Each version has its own struct definition
2. **Migration functions**: Convert between versions
3. **Forward compatibility**: Use `#[serde(default)]` and `#[serde(flatten)]`
4. **Deserialize fallback**: Try latest version first, then older versions

**Best Practices**:
- Keep migration functions simple and focused
- Test migrations with real data
- Document breaking changes in schema versions
- Consider using semver for schema versioning

---

## Caching Strategies

**Pattern**: In-memory caching for frequently accessed JSON data

**Implementation**:
```rust
use moka::sync::Cache;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Product {
    id: u64,
    name: String,
    price: f64,
}

struct ProductRepository {
    cache: Cache<u64, Product>,
}

impl ProductRepository {
    fn new() -> Self {
        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(60 * 5)) // 5 minutes TTL
            .time_to_idle(Duration::from_secs(60 * 2)) // 2 minutes TTI
            .build();
        
        Self { cache }
    }

    // Get product from cache or load from file
    fn get_product(&self, id: u64) -> Result<Option<Product>, Box<dyn std::error::Error>> {
        if let Some(product) = self.cache.get(&id) {
            return Ok(Some(product));
        }

        let path = format!("products/{}.json", id);
        if std::path::Path::new(&path).exists() {
            let contents = std::fs::read_to_string(&path)?;
            let product: Product = serde_json::from_str(&contents)?;
            self.cache.insert(id, product.clone());
            Ok(Some(product))
        } else {
            Ok(None)
        }
    }

    // Update product and invalidate cache
    fn update_product(&self, product: &Product) -> Result<(), Box<dyn std::error::Error>> {
        let path = format!("products/{}.json", product.id);
        let json_str = serde_json::to_string_pretty(product)?;
        std::fs::write(&path, json_str)?;
        self.cache.insert(product.id, product.clone());
        Ok(())
    }

    // Remove product and invalidate cache
    fn remove_product(&self, id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let path = format!("products/{}.json", id);
        if std::path::Path::new(&path).exists() {
            std::fs::remove_file(&path)?;
        }
        self.cache.remove(&id);
        Ok(())
    }
}

// Async version with moka future cache
use moka::future::Cache as AsyncCache;
use tokio::fs;

struct AsyncProductRepository {
    cache: AsyncCache<u64, Product>,
}

impl AsyncProductRepository {
    fn new() -> Self {
        let cache = AsyncCache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(60 * 5))
            .time_to_idle(Duration::from_secs(60 * 2))
            .build();
        
        Self { cache }
    }

    async fn get_product(&self, id: u64) -> Result<Option<Product>, Box<dyn std::error::Error>> {
        let cache = self.cache.clone();
        let result = self.cache.try_get_with(id, async move {
            let path = format!("products/{}.json", id);
            if tokio::fs::metadata(&path).await.is_ok() {
                let contents = fs::read_to_string(&path).await?;
                serde_json::from_str(&contents)
            } else {
                Err("Product not found".into())
            }
        }).await;

        match result {
            Ok(product) => Ok(Some(product)),
            Err(_) => Ok(None),
        }
    }
}
```

**Key Crates**:
- `moka`: High-performance, concurrent cache library
- `serde_json`: JSON serialization
- `tokio`: Async runtime for async caching

**Caching Strategies**:
- **TTL (Time To Live)**: Evict entries after fixed duration
- **TTI (Time To Idle)**: Evict entries if not accessed for duration
- **Capacity limits**: Evict least recently used (LRU) entries
- **Cache invalidation**: Remove entries when data is updated

**Best Practices**:
- Choose appropriate TTL/TTI values based on data volatility
- Implement cache invalidation for write operations
- Monitor cache hit ratios to tune parameters
- Use async cache for async applications

---

## Error Handling

**Pattern**: Comprehensive error handling for file operations

**Implementation**:
```rust
use thiserror::Error;
use std::io;
use serde_json;
use tempfile;

#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Temporary file error: {0}")]
    TempFile(#[from] tempfile::PersistError),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Data corruption: {0}")]
    DataCorruption(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Schema version mismatch: expected {expected}, got {actual}")]
    SchemaVersionMismatch { expected: u32, actual: u32 },
}

type Result<T> = std::result::Result<T, PersistenceError>;

fn read_and_validate_json<T: serde::Deserialize>(
    path: impl AsRef<std::path::Path>,
) -> Result<T> {
    let path = path.as_ref();
    
    // Check if file exists
    if !path.exists() {
        return Err(PersistenceError::FileNotFound(
            path.to_string_lossy().into_owned()
        ));
    }

    // Try to read file
    let contents = std::fs::read_to_string(path)?;

    // Try to deserialize
    match serde_json::from_str(&contents) {
        Ok(data) => Ok(data),
        Err(e) => {
            // Handle common JSON errors
            if e.is_data() {
                Err(PersistenceError::DataCorruption(e.to_string()))
            } else if e.is_syntax() {
                Err(PersistenceError::DataCorruption(e.to_string()))
            } else {
                Err(PersistenceError::Json(e))
            }
        }
    }
}

fn safe_read_config() -> Result<()> {
    let path = "config.json";
    
    match read_and_validate_json::<serde_json::Value>(path) {
        Ok(data) => {
            // Validate data schema
            if let Some(version) = data.get("version").and_then(|v| v.as_u64()) {
                if version < 2 {
                    return Err(PersistenceError::SchemaVersionMismatch {
                        expected: 2,
                        actual: version as u32,
                    });
                }
            } else {
                return Err(PersistenceError::DataCorruption(
                    "Missing 'version' field".to_string()
                ));
            }
            
            Ok(())
        }
        Err(PersistenceError::FileNotFound(_)) => {
            // File doesn't exist - create default config
            let default_config = serde_json::json!({
                "version": 2,
                "host": "localhost",
                "port": 8080
            });
            
            let json_str = serde_json::to_string_pretty(&default_config)?;
            std::fs::write(path, json_str)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

// Async version with tokio
use tokio::fs;

async fn async_safe_read_config() -> Result<()> {
    let path = "config.json";
    
    match fs::try_exists(path).await {
        Ok(exists) if exists => {
            let contents = fs::read_to_string(path).await?;
            let data: serde_json::Value = serde_json::from_str(&contents)?;
            
            if let Some(version) = data.get("version").and_then(|v| v.as_u64()) {
                if version < 2 {
                    return Err(PersistenceError::SchemaVersionMismatch {
                        expected: 2,
                        actual: version as u32,
                    });
                }
            }
            
            Ok(())
        }
        _ => {
            let default_config = serde_json::json!({
                "version": 2,
                "host": "localhost",
                "port": 8080
            });
            
            let json_str = serde_json::to_string_pretty(&default_config)?;
            fs::write(path, json_str).await?;
            Ok(())
        }
    }
}
```

**Key Crates**:
- `thiserror`: Custom error types with minimal boilerplate
- `serde_json`: JSON error handling
- `tempfile`: Temporary file error types
- `std::io`: Standard library IO errors

**Error Handling Patterns**:
1. **Custom error enum**: Enumerate all possible error types
2. **From trait**: Automatically convert from other error types
3. **Contextual errors**: Provide detailed error messages
4. **Fallback handling**: Handle missing files with default values
5. **Validation**: Check data validity before processing

**Best Practices**:
- Handle errors at appropriate layers
- Provide meaningful error messages for debugging
- Implement fallback behavior for recoverable errors
- Log errors with context (file path, operation type)

---

## Production-Ready Example: Complete Persistence Module

**Implementation**:
```rust
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tempfile::NamedTempFile;
use tokio::fs;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Temporary file error: {0}")]
    TempFile(#[from] tempfile::PersistError),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Data corruption: {0}")]
    DataCorruption(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

type Result<T> = std::result::Result<T, StorageError>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub version: u32,
    pub api_key: String,
    pub endpoint: String,
    pub timeout_ms: u64,
    pub retry_count: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            api_key: String::new(),
            endpoint: "https://api.example.com".to_string(),
            timeout_ms: 5000,
            retry_count: 3,
        }
    }
}

#[derive(Debug)]
pub struct SettingsStorage {
    path: String,
}

impl SettingsStorage {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    // Sync read
    pub fn read(&self) -> Result<Settings> {
        let path = Path::new(&self.path);
        
        if !path.exists() {
            return Ok(Settings::default());
        }

        let contents = std::fs::read_to_string(path)?;
        
        match serde_json::from_str(&contents) {
            Ok(settings) => Ok(settings),
            Err(e) => Err(StorageError::DataCorruption(e.to_string())),
        }
    }

    // Sync write (atomic)
    pub fn write(&self, settings: &Settings) -> Result<()> {
        let path = Path::new(&self.path);
        let parent_dir = path.parent().ok_or_else(|| {
            StorageError::PermissionDenied("Invalid parent directory".to_string())
        })?;

        let mut temp_file = NamedTempFile::new_in(parent_dir)?;
        let json_str = serde_json::to_string_pretty(settings)?;
        temp_file.write_all(json_str.as_bytes())?;
        temp_file.flush()?;
        temp_file.persist(path)?;
        
        Ok(())
    }

    // Async read
    pub async fn read_async(&self) -> Result<Settings> {
        let path = Path::new(&self.path);
        
        if !fs::try_exists(path).await? {
            return Ok(Settings::default());
        }

        let contents = fs::read_to_string(path).await?;
        
        match serde_json::from_str(&contents) {
            Ok(settings) => Ok(settings),
            Err(e) => Err(StorageError::DataCorruption(e.to_string())),
        }
    }

    // Async write (atomic)
    pub async fn write_async(&self, settings: &Settings) -> Result<()> {
        let path = Path::new(&self.path);
        let parent_dir = path.parent().ok_or_else(|| {
            StorageError::PermissionDenied("Invalid parent directory".to_string())
        })?;

        // Create temporary file in same directory for atomic rename
        let mut temp_file = NamedTempFile::new_in(parent_dir)?;
        let json_str = serde_json::to_string_pretty(settings)?;
        temp_file.write_all(json_str.as_bytes())?;
        temp_file.flush()?;
        temp_file.persist(path)?;
        
        Ok(())
    }

    // Validate and migrate
    pub fn validate_and_migrate(&self, settings: Settings) -> Result<Settings> {
        let mut validated = settings;
        
        // Migrate from version 0 to 1
        if validated.version == 0 {
            validated.version = 1;
            validated.retry_count = 3;
        }
        
        Ok(validated)
    }
}

// Usage example
fn main() {
    let storage = SettingsStorage::new("settings.json");
    
    // Read settings (creates default if file doesn't exist)
    let mut settings = match storage.read() {
        Ok(s) => storage.validate_and_migrate(s).unwrap(),
        Err(e) => {
            eprintln!("Error reading settings: {}", e);
            Settings::default()
        }
    };
    
    // Update settings
    settings.api_key = "new-api-key".to_string();
    settings.timeout_ms = 10000;
    
    // Write settings (atomic)
    if let Err(e) = storage.write(&settings) {
        eprintln!("Error writing settings: {}", e);
    }
    
    println!("Settings saved: {:?}", settings);
}
```

**Key Features**:
1. **Atomic writes**: Prevents partial writes
2. **Async and sync APIs**: Supports both paradigms
3. **Error handling**: Comprehensive error types
4. **Validation and migration**: Handles schema updates
5. **Default values**: Provides fallback when file not found
6. **Type safety**: Uses strongly typed structs

**Production Recommendations**:
- Add logging for operations and errors
- Implement caching for frequent reads
- Add tests for error scenarios
- Consider using a config crate for more complex scenarios
- Monitor file system operations for performance

---

## Summary of Best Practices

1. **Atomic Operations**: Always use temp file + rename pattern
2. **Error Handling**: Create custom error types with context
3. **Versioning**: Implement schema migration for forward/backward compatibility
4. **Caching**: Cache frequently accessed data with appropriate TTL/TTI
5. **Validation**: Validate data before processing
6. **Performance**: Use async APIs for I/O-bound operations
7. **Reliability**: Handle edge cases (file not found, corruption, permissions)
8. **Testing**: Test with various error scenarios and load conditions
