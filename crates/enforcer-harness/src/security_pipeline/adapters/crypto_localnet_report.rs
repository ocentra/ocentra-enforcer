//! BOUNDARY parser for the OPTIONAL crypto-localnet stage
//! (solana-test-validator/Anchor/Bankrun recorded reports), consumed
//! only when the optional crypto pack turns the stage on.
//!
//! BOUNDARY-INVARIANT: [`run_stage`] never even reads the raw text while
//! the stage is off ([`CryptoLocalnetActivation::Disabled`] short-circuits
//! first); when on, it accepts raw recorded JSON and either returns a
//! branded [`crate::security_pipeline::crypto_localnet::CryptoLocalnetOutcome`]
//! or rejects the text as malformed/dishonest with a typed decode
//! failure — same honesty rule as every always-on stage.
//!
//! boundaryOwnerNote: h07 `security_pipeline` owns this parsing seam;
//! e-pack-crypto-blockchain consumes it read-only when enabled.

use enforcer_core::error::{DecodeError, Result};

use crate::security_pipeline::crypto_localnet::{
    CryptoLocalnetActivation, CryptoLocalnetConfig, CryptoLocalnetOutcome,
};

/// Raw wire shape of one recorded crypto-localnet report.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CryptoLocalnetRecord {
    tool_present: bool,
    outcome: String,
    ran: u32,
    error_message: Option<String>,
}

/// Run the crypto-localnet stage over one recorded report:
/// [`CryptoLocalnetOutcome::Disabled`] immediately (without touching
/// `raw`) while the pack is off; otherwise parse `raw` through the same
/// honesty rule every other stage uses.
///
/// # Errors
/// Returns a typed decode failure naming the violated invariant — only
/// ever when the stage is enabled.
pub fn run_stage(config: &CryptoLocalnetConfig, raw: &str) -> Result<CryptoLocalnetOutcome> {
    if config.activation == CryptoLocalnetActivation::Disabled {
        return Ok(CryptoLocalnetOutcome::Disabled);
    }

    let record: CryptoLocalnetRecord = serde_json::from_str(raw).map_err(|source| {
        DecodeError::new("securityPipeline.cryptoLocalnet", format!("{source}"))
    })?;

    super::reject_dishonest_shape(
        record.tool_present,
        &record.outcome,
        record.error_message.is_some(),
    )?;

    match record.outcome.as_str() {
        "skipped" => Ok(CryptoLocalnetOutcome::Skipped { ran: record.ran }),
        "errored" => Ok(CryptoLocalnetOutcome::Errored {
            error_message: record
                .error_message
                .unwrap_or_else(|| String::from("the recorded report carried no error message")),
        }),
        "ran" => Ok(CryptoLocalnetOutcome::Ran { ran: record.ran }),
        other => Err(DecodeError::new(
            "securityPipeline.cryptoLocalnet.outcome",
            format!("unrecognized outcome `{other}` — expected skipped/errored/ran"),
        )
        .into()),
    }
}
