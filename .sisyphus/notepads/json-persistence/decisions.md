# JSON Persistence Patterns in Rust - Architectural Decisions

## Decision 1: Atomic Write Pattern

**Context**: Need to ensure data integrity when writing JSON files

**Options**:
1. Direct write to file (simple but risky)
2. Write to temp file, then rename (atomic)
3. Write to memory buffer first, then write to file (safer but not atomic)

**Decision**: Use temp file + rename

**Rationale**:
- Prevents partial writes due to crashes
- Guarantees file consistency on disk
- Most reliable pattern for single-file writes
- Standard practice in many applications

**Implementation**:
```rust
let mut temp_file = NamedTempFile::new_in(parent_dir)?;
temp_file.write_all(data.as_bytes())?;
temp_file.flush()?;
temp_file.persist(path)?;
```

**Trade-offs**:
- Slightly slower than direct write
- Requires disk space for temporary file
- Works only on same filesystem

---

## Decision 2: Error Handling Strategy

**Context**: Need to handle various failure scenarios

**Options**:
1. Use std::io::Error directly (simple)
2. Create custom error enum (comprehensive)
3. Use anyhow for simple errors
4. Use thiserror for structured errors

**Decision**: Use thiserror to create custom error enum

**Rationale**:
- Provides structured, typed errors
- Supports automatic conversion via From trait
- Enables pattern matching on error types
- Improves debuggability with context

**Implementation**:
```rust
#[derive(Error, Debug)]
enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
```

**Trade-offs**:
- Requires defining error enum
- Slightly more code than using anyhow
- Better for libraries than simple scripts

---

## Decision 3: Async vs Sync APIs

**Context**: Need to support both sync and async applications

**Options**:
1. Provide only sync API (simple)
2. Provide only async API (modern)
3. Provide both APIs via feature flags
4. Provide both APIs in separate modules

**Decision**: Implement both sync and async APIs in same struct

**Rationale**:
- Supports widest range of use cases
- Clear separation in method names (read vs read_async)
- Reuses common functionality internally
- Easy to maintain

**Implementation**:
```rust
impl Storage {
    pub fn read(&self) -> Result<Data> { /* sync */ }
    pub async fn read_async(&self) -> Result<Data> { /* async */ }
}
```

**Trade-offs**:
- More code to maintain
- Requires careful API design
- May have some code duplication

---

## Decision 4: Schema Migration Approach

**Context**: Need to handle breaking changes to data structures

**Options**:
1. Version in file name (config-v1.json, config-v2.json)
2. Version field in JSON ({"version": 1, "data": ...})
3. External migration scripts
4. Deserialize with default values

**Decision**: Use version field in JSON with migration functions

**Rationale**:
- Keeps data in single file for simplicity
- Supports forward/backward compatibility
- Enables gradual migration
- Self-contained and maintainable

**Implementation**:
```rust
fn migrate_data(data: &str) -> Result<DataV2> {
    match serde_json::from_str(data) {
        Ok(v2) => Ok(v2),
        Err(_) => {
            let v1: DataV1 = serde_json::from_str(data)?;
            Ok(migrate_v1_to_v2(v1))
        }
    }
}
```

**Trade-offs**:
- Requires maintaining migration functions
- More complex deserialization logic
- Risk of migration errors

---

## Decision 5: Caching Strategy

**Context**: Need to optimize frequent reads

**Options**:
1. No caching (simple but slow for frequent reads)
2. In-memory cache with TTL
3. Persistent cache (Redis, SQLite)
4. File system cache

**Decision**: In-memory cache with Moka

**Rationale**:
- High performance for frequent reads
- Simple to implement
- Built-in TTL/TTI support
- Concurrent access support

**Implementation**:
```rust
use moka::sync::Cache;

let cache = Cache::builder()
    .max_capacity(1000)
    .time_to_live(Duration::from_secs(300))
    .build();
```

**Trade-offs**:
- Cache not persistent across restarts
- May use significant memory for large datasets
- Need to handle cache invalidation

---

## Decision 6: Data Validation Strategy

**Context**: Need to ensure data integrity

**Options**:
1. No validation (simple but risky)
2. Validate after deserialization
3. Validate during deserialization
4. Use JSON schema validation

**Decision**: Validate after deserialization + serde validations

**Rationale**:
- Catch errors early in processing pipeline
- Leverage serde's validation capabilities
- Simple to implement
- Works well with strongly typed data structures

**Implementation**:
```rust
#[derive(Serialize, Deserialize)]
struct Data {
    #[serde(validate = "validate_positive")]
    id: u64,
    #[serde(validate = "validate_email")]
    email: String,
}
```

**Trade-offs**:
- Adds overhead to deserialization
- Requires writing validation functions
- Not as comprehensive as JSON schema validation

---

## Decision 7: File Organization

**Context**: Need to organize multiple JSON files

**Options**:
1. Single large JSON file
2. Directory per data type
3. File per entity (id.json)
4. Embedded database (SQLite)

**Decision**: File per entity in data directory

**Rationale**:
- Simple to implement
- Easy to debug (each file is separate)
- Supports partial updates without rewriting entire dataset
- Works well with file system tools

**Implementation**:
```rust
let product_dir = "data/products/";
let product_file = format!("{}/{}.json", product_dir, product.id);
```

**Trade-offs**:
- Directory traversal overhead for many entities
- Harder to query across all entities
- File system limits on number of files

---

## Decision 8: Serialization Format

**Context**: Need to choose JSON serialization options

**Options**:
1. Compact JSON (smaller, faster)
2. Pretty-printed JSON (human-readable)
3. Binary JSON (BSON, CBOR)
4. Compressed JSON (gzip, zstd)

**Decision**: Pretty-printed JSON for most use cases

**Rationale**:
- Human-readable for debugging
- Self-documenting
- Compatible with most tools
- Performance trade-off acceptable for many applications

**Implementation**:
```rust
serde_json::to_string_pretty(&data)?;
```

**Trade-offs**:
- Larger file sizes
- Slightly slower serialization
- Not optimal for very large datasets

---

## Decision 9: Dependency Management

**Context**: Need to choose and manage dependencies

**Options**:
1. Minimal dependencies (std only)
2. Use well-maintained crates
3. Roll your own implementations
4. Use feature flags for optional features

**Decision**: Use well-maintained crates with minimal footprint

**Rationale**:
- Reliable and tested implementations
- Active maintenance and security updates
- Saves development time
- Community support

**Key Dependencies**:
- serde_json: JSON serialization
- thiserror: Custom error types
- tempfile: Temporary file operations
- tokio: Async runtime
- moka: Caching

**Trade-offs**:
- External dependencies can have breaking changes
- May increase compile time
- Requires managing versions

---

## Decision 10: Testing Strategy

**Context**: Need to test persistence layer

**Options**:
1. Unit tests only
2. Integration tests with real file system
3. Property-based testing
4. Mock file system

**Decision**: Integration tests with temp directory + property-based testing

**Rationale**:
- Tests real file system interactions
- Catches edge cases
- Property-based testing finds unexpected behavior
- Temp directory ensures isolation

**Implementation**:
```rust
#[test]
fn test_write_and_read() {
    let tmp_dir = tempdir::TempDir::new().unwrap();
    let path = tmp_dir.path().join("test.json");
    
    let data = Data { id: 1, name: "Test" };
    write_data(&path, &data).unwrap();
    
    let read_data = read_data(&path).unwrap();
    assert_eq!(data, read_data);
}
```

**Trade-offs**:
- Integration tests are slower than unit tests
- May have platform-specific behavior
- Requires more setup code

---

## Summary of Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Write pattern | Atomic (temp file + rename) | Data integrity |
| Error handling | Custom enum with thiserror | Structured errors |
| APIs | Sync + Async | Supports all use cases |
| Migration | Version field + functions | Simple and maintainable |
| Caching | Moka with TTL | Performance and simplicity |
| Validation | Post-deserialization | Early error detection |
| File organization | File per entity | Easy debugging |
| Serialization | Pretty-printed JSON | Human-readable |
| Dependencies | Well-maintained crates | Reliability |
| Testing | Integration + property-based | Real-world validation |

