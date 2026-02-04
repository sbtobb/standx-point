use serde::{Deserialize, Serialize};
use std::path::Path;
use std::io::Write;
use thiserror::Error;
use tempfile::{NamedTempFile, TempDir};

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_write() {
        let tmp_dir = TempDir::new().unwrap();
        let path = tmp_dir.path().join("product.json");
        let storage = ProductStorage::new(path.to_string_lossy().into_owned());

        // Write test data
        let product = Product {
            id: 123,
            name: "Test Product".to_string(),
            price: 19.99,
            description: "Test description".to_string(),
        };
        storage.write(&product).unwrap();

        // Read back and verify
        let read_product = storage.read().unwrap();
        assert_eq!(product, read_product);
    }

    #[test]
    fn test_read_default() {
        let tmp_dir = TempDir::new().unwrap();
        let path = tmp_dir.path().join("non_existent.json");
        let storage = ProductStorage::new(path.to_string_lossy().into_owned());

        let product = storage.read().unwrap();
        assert_eq!(product, Product::default());
    }

    #[test]
    fn test_data_corruption() {
        let tmp_dir = TempDir::new().unwrap();
        let path = tmp_dir.path().join("corrupted.json");
        std::fs::write(&path, "invalid json").unwrap();
        let storage = ProductStorage::new(path.to_string_lossy().into_owned());

        let result = storage.read();
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(matches!(e, StorageError::DataCorruption(_)));
        }
    }

    #[test]
    fn test_atomic_write() {
        let tmp_dir = TempDir::new().unwrap();
        let path = tmp_dir.path().join("atomic_test.json");
        let storage = ProductStorage::new(path.to_string_lossy().into_owned());

        let product1 = Product {
            id: 1,
            name: "Product 1".to_string(),
            price: 10.0,
            description: "First product".to_string(),
        };
        storage.write(&product1).unwrap();

        let product2 = Product {
            id: 2,
            name: "Product 2".to_string(),
            price: 20.0,
            description: "Second product".to_string(),
        };
        storage.write(&product2).unwrap();

        let read_product = storage.read().unwrap();
        assert_eq!(product2, read_product);
    }

    #[test]
    fn test_product_equality() {
        let product1 = Product::default();
        let product2 = Product::default();
        assert_eq!(product1, product2);

        let product3 = Product {
            price: 19.99,
            ..Product::default()
        };
        assert_ne!(product1, product3);
    }
}
