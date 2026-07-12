//! OPTIONAL crypto-localnet stage (h07): the disjoint opt-in seam for
//! solana-test-validator/Anchor/Bankrun recorded reports, consumed only
//! by e-pack-crypto-blockchain. OFF by default —
//! [`CryptoLocalnetActivation::Disabled`] is the `Default`, and while
//! disabled the stage never runs at all: its absence narrows what the
//! pipeline covers, it never blocks the other stages.
//!
//! This module owns only the branded activation/outcome types; the
//! recorded-report parsing (including the shared honesty rule) lives in
//! [`crate::security_pipeline::adapters::crypto_localnet_report::run_stage`],
//! which rejects malformed or dishonest raw text with a typed decode
//! failure and never even reads the text while the stage is off.

/// Whether the optional crypto pack turned this stage on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CryptoLocalnetActivation {
    /// e-pack-crypto-blockchain enabled the stage.
    Enabled,
    /// The stage is off (the default for every install that does not
    /// carry the optional crypto pack).
    #[default]
    Disabled,
}

/// Configuration for the crypto-localnet stage. `Default` is disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CryptoLocalnetConfig {
    /// On/off switch mirroring the e-pack-crypto-blockchain setting.
    pub activation: CryptoLocalnetActivation,
}

/// Honest outcome of the crypto-localnet stage. [`Disabled`] is a
/// FOURTH state, distinct from a tool-absent skip: the pack was never
/// turned on, so no attempt to find the tool was even made.
///
/// [`Disabled`]: CryptoLocalnetOutcome::Disabled
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoLocalnetOutcome {
    /// e-pack-crypto-blockchain is off; this stage did not run at all.
    Disabled,
    /// The pack is on but the localnet tool was not found. Never a pass.
    Skipped {
        // BRAND-INVARIANT: items covered by the run; always 0 for a skip
        // (the adapters boundary rejects a skip that claims coverage).
        ran: u32,
    },
    /// The pack is on and the tool was present but failed.
    Errored {
        // BRAND-INVARIANT: the tool's own failure rendering, carried
        // verbatim for diagnostics; display-only.
        error_message: String,
    },
    /// The pack is on and the tool ran to completion.
    Ran {
        // BRAND-INVARIANT: items covered by the run, as recorded.
        ran: u32,
    },
}
