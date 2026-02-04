# JSON Persistence Patterns in Rust - Common Issues

## Data Corruption

### Problem 1: Partial writes when process crashes

**Symptoms**:
- File is truncated or contains partial JSON
- Deserialization fails with "unexpected EOF" errors
- File size is smaller than expected

**Solution**:
Use atomic write pattern (temp file + rename)

```rust
use tempfile::NamedTempFile;

let mut temp_file = NamedTempFile::new_in(parent_dir)?;
// Write complete data to temp file
temp_file.flush()?;
// Atomic rename ensures file is consistent
temp_file.persist(path)?;
```

**Root Cause**:
Direct writes to the target file leave it in inconsistent state if the process crashes during write.

---

### Problem 2: JSON syntax errors due to invalid data

**Symptoms**:
- serde_json::Error with "expected `{` or `[` at line 1 column 1"
- "invalid type: null" or "expected string" errors
- File contains garbled characters

**Solution**:
1. Validate data before serialization
2. Handle deserialization errors with context

```rust
fn safe_read() -> Result<Data> {
    let contents = fs::read_to_string(path)?;
    match serde_json::from_str(&contents) {
        Ok(data) => Ok(data),
        Err(e) => Err(StorageError::DataCorruption(e.to_string())),
    }
}
```

**Root Cause**:
Data corruption can occur due to:
- Hardware failures
- File system errors
- Process crashes
- Invalid data being written

---

### Problem 3: Incompatible schema changes

**Symptoms**:
- "missing field" errors during deserialization
- "unknown field" warnings
- Application fails to start after update

**Solution**:
Versioned schemas with migration functions

```rust
// Try latest version first
match serde_json::from_str(&contents) {
    Ok(data) => Ok(data),
    Err(_) => {
        let v1 = serde_json::from_str::<DataV1>(&contents)?;
        Ok(migrate_v1_to_v2(v1))
    }
}
```

**Root Cause**:
Breaking changes to data structure without migration support.

---

## Performance Issues

### Problem 4: Slow reads with large JSON files

**Symptoms**:
- High latency when reading configuration
- Application startup delay
- High CPU usage during deserialization

**Solution**:
1. Cache frequent reads in memory
2. Use streaming deserialization for large files
3. Compress large JSON files

```rust
use moka::sync::Cache;

let cache = Cache::builder()
    .max_capacity(100)
    .time_to_live(Duration::from_secs(300))
    .build();

fn get_data(id: u64) -> Result<Data> {
    if let Some(data) = cache.get(&id) {
        return Ok(data);
    }
    // Load from file and cache
}
```

**Root Cause**:
Repeated reads from disk for same data without caching.

---

### Problem 5: Blocking I/O affecting async operations

**Symptoms**:
- Async tasks blocked on file operations
- High latency in async applications
- Thread pool exhaustion

**Solution**:
Use async I/O with tokio::fs

```rust
async fn async_read() -> Result<Data> {
    let contents = fs::read_to_string(path).await?;
    serde_json::from_str(&contents)
}
```

**Root Cause**:
Using blocking std::fs operations in async context.

---

### Problem 6: Large JSON payloads causing memory bloat

**Symptoms**:
- High memory usage during serialization
- Out-of-memory errors for large datasets
- GC pressure in long-running applications

**Solution**:
1. Use streaming serialization/deserialization
2. Split large datasets into smaller files
3. Use binary formats for very large data

```rust
// Streaming deserialization
use serde_json::Deserializer;
use std::io::Read;

let file = File::open("large_data.json")?;
let reader = BufReader::new(file);
let mut deserializer = Deserializer::from_reader(reader);
let data: Vec<Item> = Vec::deserialize(&mut deserializer)?;
```

**Root Cause**:
Loading entire file into memory before deserialization.

---

## Concurrency Issues

### Problem 7: Race conditions in multi-threaded access

**Symptoms**:
- Inconsistent data between reads and writes
- File not found errors after write
- Partial reads during write operations

**Solution**:
1. Use atomic writes
2. Implement proper synchronization
3. Use async-safe cache with moka

```rust
// Atomic write prevents race conditions during updates
fn atomic_write(data: &Data) -> Result<()> {
    let mut temp = NamedTempFile::new_in(parent_dir)?;
    temp.write_all(serde_json::to_string(data)?.as_bytes())?;
    temp.persist(path)?;
    Ok(())
}
```

**Root Cause**:
Concurrent reads and writes without proper synchronization.

---

### Problem 8: Cache consistency issues

**Symptoms**:
- Stale data in cache after updates
- Inconsistent data between cache and disk
- Multiple cache entries for same data

**Solution**:
1. Implement cache invalidation on writes
2. Use TTL/TTI for cache entries
3. Validate cache data against disk

```rust
fn update_data(data: &Data) -> Result<()> {
    atomic_write(data)?;
    cache.insert(data.id, data.clone()); // Invalidate cache
    Ok(())
}
```

**Root Cause**:
Cache not being invalidated when data is updated.

---

## Permission and Environment Issues

### Problem 9: Permission denied errors

**Symptoms**:
- "Permission denied" when trying to read/write files
- Application fails to start as non-root user
- Files created with incorrect permissions

**Solution**:
1. Set appropriate file permissions
2. Check permissions before operations
3. Use user-specific directories

```rust
use std::os::unix::fs::PermissionsExt;

fn create_config_dir() -> Result<()> {
    let dir = dirs::config_dir().ok_or("No config dir")?;
    let app_dir = dir.join("my_app");
    fs::create_dir_all(&app_dir)?;
    fs::set_permissions(&app_dir, Permissions::from_mode(0o755))?;
    Ok(())
}
```

**Root Cause**:
Files/directories created with incorrect permissions or owned by wrong user.

---

### Problem 10: File not found errors

**Symptoms**:
- "No such file or directory" errors
- Application fails to start
- Works in development but not in production

**Solution**:
1. Check file existence before read
2. Provide default values when file not found
3. Handle errors gracefully

```rust
fn read_config() -> Result<Config> {
    let path = "config.json";
    
    if !Path::new(path).exists() {
        return Ok(Config::default());
    }
    
    fs::read_to_string(path).and_then(|s| serde_json::from_str(&s))
}
```

**Root Cause**:
File paths not properly resolved, or files not created when needed.

---

## Debugging Tips

### Tip 1: Log error context

**Pattern**:
```rust
use tracing::{error, info};

fn read_file() -> Result<()> {
    match fs::read_to_string("data.json") {
        Ok(content) => info!("Read {} bytes", content.len()),
        Err(e) => error!("Failed to read data.json: {}, path: {:?}", e, Path::new("data.json")),
    }
}
```

### Tip 2: Validate JSON schema

**Pattern**:
```rust
use schemars::JsonSchema;
use serde_json::SchemaValidator;

#[derive(Serialize, Deserialize, JsonSchema)]
struct Data {
    id: u64,
    name: String,
}

fn validate_json(data: &str) -> Result<()> {
    let schema = schemars::schema_for!(Data);
    let validator = SchemaValidator::new(schemars::schema::Draft7);
    validator.validate(&serde_json::from_str(data)?, &schema)?;
    Ok(())
}
```

### Tip 3: Check file integrity

**Pattern**:
```rust
use sha2::{Sha256, Digest};

fn check_integrity(path: &Path) -> Result<bool> {
    let contents = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    let hash = hasher.finalize();
    
    // Compare with stored hash
    Ok(stored_hash == format!("{:x}", hash))
}
```

### Tip 4: Use structured error types

**Pattern**:
```rust
#[derive(Error, Debug)]
enum StorageError {
    #[error("File {0} not found")]
    FileNotFound(String),
    #[error("Corrupted data in {0}: {1}")]
    CorruptedData(String, String),
}
```

---

## Root Cause Analysis Framework

When debugging JSON persistence issues:

1. **Identify symptom**: What error or behavior is observed?
2. **Reproduce the issue**: Can you make it happen consistently?
3. **Collect data**: Check logs, file system, permissions
4. **Analyze root cause**: Determine why it's happening
5. **Implement fix**: Apply appropriate pattern from this document
6. **Test**: Verify fix and ensure no regression
7. **Prevent recurrence**: Update tests and documentation

---

## Common Fix Summary

| Problem | Solution | Pattern |
|---------|----------|---------|
| Partial writes | Atomic write | temp file + rename |
| Data corruption | Validation + error handling | Try-deserialize + context |
| Schema changes | Migration + versioning | Deserialize fallback |
| Slow reads | Caching | moka with TTL |
| Blocking I/O | Async operations | tokio::fs |
| Large payloads | Streaming | serde_json Deserializer |
| Race conditions | Atomic operations | temp file + rename |
| Stale cache | Invalidation | Cache on update |
| Permissions | Permission checking | fs::Permissions |
| File not found | Default values | Fallback to default |
