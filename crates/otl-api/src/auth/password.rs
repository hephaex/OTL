/// Password hashing and verification using Argon2id
///
/// Implements secure password hashing following OWASP recommendations:
/// - Algorithm: Argon2id (memory-hard, resistant to GPU attacks)
/// - Memory: 64 MB
/// - Iterations: 3
/// - Parallelism: 4 threads
/// - Salt: 16 bytes random
/// - Output: 32 bytes hash
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};
use thiserror::Error;

/// Password hashing and verification errors
#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("Failed to hash password: {0}")]
    HashingFailed(String),

    #[error("Failed to verify password: {0}")]
    VerificationFailed(String),

    #[error("Invalid password hash format")]
    InvalidHashFormat,

    #[error("Password does not match")]
    PasswordMismatch,
}

/// Password hashing configuration
///
/// These parameters are tuned for security while maintaining acceptable performance.
/// Increasing memory or iterations improves security but slows down hashing.
#[derive(Debug, Clone)]
pub struct PasswordConfig {
    /// Memory cost in KB (default: 65536 = 64 MB)
    pub memory_cost: u32,
    /// Time cost (iterations, default: 3)
    pub time_cost: u32,
    /// Parallelism (threads, default: 4)
    pub parallelism: u32,
    /// Output length in bytes (default: 32)
    pub output_len: Option<usize>,
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            memory_cost: 65536, // 64 MB
            time_cost: 3,
            parallelism: 4,
            output_len: Some(32),
        }
    }
}

impl PasswordConfig {
    /// Create Argon2 parameters from this configuration
    fn to_params(&self) -> Result<Params, PasswordError> {
        Params::new(
            self.memory_cost,
            self.time_cost,
            self.parallelism,
            self.output_len,
        )
        .map_err(|e| PasswordError::HashingFailed(e.to_string()))
    }
}

/// Hash a plaintext password using Argon2id
///
/// # Arguments
///
/// * `password` - The plaintext password to hash
///
/// # Returns
///
/// * `Ok(String)` - PHC string format hash (includes algorithm, parameters, salt, and hash)
/// * `Err(PasswordError)` - If hashing fails
///
/// # Security Notes
///
/// - The returned hash is safe to store in the database
/// - The hash includes the salt, so no separate storage is needed
/// - Uses cryptographically secure random salt generation
///
/// # Example
///
/// ```no_run
/// use otl_api::auth::password::hash_password;
///
/// let password = "SecureP@ssw0rd!";
/// let hash = hash_password(password).expect("Failed to hash password");
/// println!("Hash: {}", hash);
/// // Output: $argon2id$v=19$m=65536,t=3,p=4$...
/// ```
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let config = PasswordConfig::default();
    hash_password_with_config(password, &config)
}

/// Hash a password with custom configuration
///
/// # Arguments
///
/// * `password` - The plaintext password to hash
/// * `config` - Custom Argon2 parameters
///
/// # Returns
///
/// * `Ok(String)` - PHC string format hash
/// * `Err(PasswordError)` - If hashing fails
pub fn hash_password_with_config(
    password: &str,
    config: &PasswordConfig,
) -> Result<String, PasswordError> {
    // Generate a random salt
    let salt = SaltString::generate(&mut OsRng);

    // Create Argon2 instance with custom parameters
    let params = config.to_params()?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    // Hash the password
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| PasswordError::HashingFailed(e.to_string()))?;

    Ok(password_hash.to_string())
}

/// Verify a plaintext password against a stored hash
///
/// # Arguments
///
/// * `password` - The plaintext password to verify
/// * `hash` - The stored password hash (PHC string format)
///
/// # Returns
///
/// * `Ok(true)` - Password matches
/// * `Ok(false)` - Password does not match
/// * `Err(PasswordError)` - If verification fails due to invalid hash format
///
/// # Example
///
/// ```no_run
/// use otl_api::auth::password::{hash_password, verify_password};
///
/// let password = "SecureP@ssw0rd!";
/// let hash = hash_password(password).unwrap();
///
/// // Correct password
/// assert!(verify_password(password, &hash).unwrap());
///
/// // Wrong password
/// assert!(!verify_password("WrongPassword", &hash).unwrap());
/// ```
pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    // Parse the PHC string
    let parsed_hash = PasswordHash::new(hash).map_err(|_| PasswordError::InvalidHashFormat)?;

    // Create Argon2 instance for verification
    let argon2 = Argon2::default();

    // Verify the password
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(PasswordError::VerificationFailed(e.to_string())),
    }
}

/// Validate password strength
///
/// Checks if a password meets minimum security requirements:
/// - At least 8 characters
/// - At least 1 uppercase letter
/// - At least 1 lowercase letter
/// - At least 1 digit
/// - At least 1 special character
///
/// # Arguments
///
/// * `password` - The password to validate
///
/// # Returns
///
/// * `Ok(())` - Password meets requirements
/// * `Err(String)` - Description of why password is invalid
///
/// # Example
///
/// ```no_run
/// use otl_api::auth::password::validate_password_strength;
///
/// assert!(validate_password_strength("SecureP@ssw0rd!").is_ok());
/// assert!(validate_password_strength("weak").is_err());
/// ```
pub fn validate_password_strength(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters long".to_string());
    }

    if !password.chars().any(|c| c.is_uppercase()) {
        return Err("Password must contain at least one uppercase letter".to_string());
    }

    if !password.chars().any(|c| c.is_lowercase()) {
        return Err("Password must contain at least one lowercase letter".to_string());
    }

    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err("Password must contain at least one digit".to_string());
    }

    if !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err("Password must contain at least one special character".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let password = "SecureP@ssw0rd!";
        let hash = hash_password(password).expect("Failed to hash password");

        // Verify correct password
        assert!(verify_password(password, &hash).expect("Verification failed"));

        // Verify incorrect password
        assert!(!verify_password("WrongPassword", &hash).expect("Verification failed"));
    }

    #[test]
    fn test_different_passwords_produce_different_hashes() {
        let password1 = "Password1!";
        let password2 = "Password2!";

        let hash1 = hash_password(password1).unwrap();
        let hash2 = hash_password(password2).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_same_password_produces_different_hashes() {
        // Due to random salt, same password should produce different hashes
        let password = "SamePassword123!";

        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_invalid_hash_format() {
        let result = verify_password("password", "invalid-hash-format");
        assert!(matches!(result, Err(PasswordError::InvalidHashFormat)));
    }

    #[test]
    fn test_password_strength_validation() {
        // Valid passwords
        assert!(validate_password_strength("SecureP@ssw0rd!").is_ok());
        assert!(validate_password_strength("Abcdef123!").is_ok());

        // Too short
        assert!(validate_password_strength("Abc123!").is_err());

        // No uppercase
        assert!(validate_password_strength("password123!").is_err());

        // No lowercase
        assert!(validate_password_strength("PASSWORD123!").is_err());

        // No digit
        assert!(validate_password_strength("Password!").is_err());

        // No special character
        assert!(validate_password_strength("Password123").is_err());
    }

    #[test]
    fn test_custom_config() {
        let config = PasswordConfig {
            memory_cost: 32768, // 32 MB (lighter for tests)
            time_cost: 2,
            parallelism: 2,
            output_len: Some(32),
        };

        let password = "TestPassword123!";
        let hash = hash_password_with_config(password, &config).unwrap();

        // Should still verify correctly
        assert!(verify_password(password, &hash).unwrap());

        // Check that hash contains the custom parameters
        assert!(hash.contains("m=32768"));
        assert!(hash.contains("t=2"));
        assert!(hash.contains("p=2"));
    }
}
