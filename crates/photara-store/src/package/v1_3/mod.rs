//! Frozen PS2 wire building blocks, without filesystem effects or writable admission.
//!
//! Limits are supplied by callers and are resource budgets, not portable format
//! ceilings. Successful decoding does not establish storage qualification, an
//! audited package closure, or an Accepted/Saved acknowledgement.

mod packed;
pub use packed::{Arena, FrameKind, FrameLimits, PackedAllocation, PhysicalRef, encode_frame};

mod tree;
pub use tree::{PhysicalRecords, PhysicalTree, PhysicalTreeAudit, PhysicalTreeKind, TreeLimits};

mod locator;
pub use locator::{BlobAudit, BlobMetadata, Locator, Membership, StructuralBlob};

pub mod publication;

mod selected;
pub use selected::{SelectedEnvelope, SelectedRole, SelectionIdentity, StructuralState};

mod logical_tree;
pub use logical_tree::{
    LogicalKey, LogicalRecords, LogicalTree, LogicalTreeAudit, LogicalTreeKind,
};

mod operations;
pub use operations::{OperationAudit, audit_operations};

mod semantic;
pub use semantic::{SemanticClosure, inspect_legacy_semantics};

mod accounting;
pub use accounting::{AccountingLimits, AccountingMetadata, audit_settled_accounting};

mod resources;
pub use resources::{ResourceLimits, ResourceMetadata, inspect_resources};
mod ownership;
pub use ownership::{
    AllocationInspection, AllocationLayout, AllocationObservation, OwnershipAudit,
    OwnershipContext, audit_ownership,
};

pub mod repeatable;

mod origins;
mod reader;
pub use reader::{
    ControlRecords, DirectoryObservation, ReaderLimits, RetainedObservation, RoleClosure,
    SettledPackage, SettledRegistration, inspect_recovery, inspect_settled, verify_original,
};
