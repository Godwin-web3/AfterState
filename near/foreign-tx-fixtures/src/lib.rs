//! Version-pinned NEAR MPC ForeignTx payload fixtures.
//!
//! Keep the MPC dependency pinned to the exact release commit. Updating it is a
//! protocol change: review DTO/Borsh changes and regenerate fixtures deliberately.

pub const MPC_RELEASE: &str = "3.19.0";
pub const MPC_COMMIT: &str = "86e2cc4d124397ec81f436b11f48bf9b6dfd062e";
