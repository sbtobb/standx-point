use serde::{Deserialize, Serialize};
use std::path::Path;
use std::io::Write;
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
}

type Result<T> = std::result::Result<T, StorageError>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Product {
    pub id: u64,
    pub name: String,
    pub price: f64,
    pub description: String,
}

impl Default for Product {
    fn default() -> Self {
        Self {
            id: 1,
            name: "Sample Product".to_string(),
            price: 9.99,
            description: "A sample product for demonstration purposes".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ProductStorage {
    path: String,
}

impl ProductStorage {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    pub fn read(&self) -> Result<Product> {
        let path = Path::new(&self.path);
        
        if !path.exists() {
            return Ok(Product::default());
        }

        let contents = std::fs::read_to_string(path)?;
        
        match serde_json::from_str(&contents) {
            Ok(product) => Ok(product),
            Err(e) => Err(StorageError::DataCorruption(e.to_string())),
        }
    }

    pub fn write(&self, product: &Product) -> Result<()> {
        let path = Path::new(&self.path);
        let parent_dir = path.parent().ok_or_else(|| {
            StorageError::DataCorruption("Invalid parent directory".to_string())
        })?;

        let mut temp_file = NamedTempFile::new_in(parent_dir)?;
        let json_str = serde_json::to_string_pretty(product)?;
        temp_file.write_all(json_str.as_bytes())?;
        temp_file.flush()?;
        temp_file.persist(path)?;
        
        Ok(())
    }

    pub async fn read_async(&self) -> Result<Product> {
        let path = Path::new(&self.path);
        
        if !fs::try_exists(path).await? {
            return Ok(Product::default());
        }

        let contents = fs::read_to_string(path).await?;
        
        match serde_json::from_str(&contents) {
            Ok(product) => Ok(product),
            Err(e) => Err(StorageError::DataCorruption(e.to_string())),
        }
    }

    pub async fn write_async(&self, product: &Product) -> Result<()> {
        let path = Path::new(&self.path);
        let parent_dir = path.parent().ok_or_else(|| {
            StorageError::DataCorruption("Invalid parent directory".to_string())
        })?;

        let mut temp_file = NamedTempFile::new_in(parent_dir)?;
        let json_str = serde_json::to_string_pretty(product)?;
        temp_file.write_all(json_str.as_bytes())?;
        temp_file.flush()?;
        temp_file.persist(path)?;
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create storage
    let storage = ProductStorage::new("product.json");

    println!("=== JSON Persistence Demo (Async) ===");

    // Read existing or default product
    println!("1. Reading product from file (async)...");
    let mut product = storage.read_async().await?;
    println!("   Success: {:?}", product);

    // Update product
    println!("\n2. Updating product (async)...");
    product.price = 19.99;
    product.description = "Updated product with async API".to_string();
    storage.write_async(&product).await?;
    println!("   Success");

    // Verify update
    println!("\n3. Verifying update (async)...");
    let updated_product = storage.read_async().await?;
    assert_eq!(product, updated_product);
    println!("   Success: {:?}", updated_product);

    // Cleanup
    fs::remove_file("product.json").await.ok();

    println!("\n=== Async Demo completed successfully ===");

    Ok(())
}
