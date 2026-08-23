//! Enterprise Security Fortress: RBAC, Multi-Tenant Key Sandboxing, Cryptographic Audit & Auth.

pub mod rbac;
pub mod audit;
pub mod auth;

pub use rbac::{User, PERM_READ, PERM_WRITE, PERM_ADMIN, PERM_STREAM, PERM_COMPUTE, PERM_AUDIT, PERM_ALL};
pub use audit::{AuditBlock, AuditLedger};
pub use auth::{SecurityManager, AuthError, MAX_FAILED_ATTEMPTS};
