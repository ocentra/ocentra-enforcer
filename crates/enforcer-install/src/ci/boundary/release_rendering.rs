//! CI release rendering boundary.
//!
//! BOUNDARY-INVARIANT: release tags and archive file names are raw only in
//! this module. `ReleaseVersionWire::from_raw_ci_tag` rejects malformed input and the
//! matrix maps every rendered record to a typed `ReleaseAsset` before the
//! release gate consumes it. The negative empty-version test proves that an
//! invalid release tag cannot enter the matrix.
//! The explicit `toDomain` mapping is `to_domain_asset`; Rust naming keeps the
//! implementation snake_case while this boundary contract names the crossing.
//! boundaryOwnerNote: enforcer-install owns the producer CI/archive seam.

use enforcer_domain::install_types::ReleaseAsset;
use enforcer_domain::install_types::{
    BinaryVariant, EmptyReleaseVersion, ReleaseVersion, TargetPlatform,
};

/// Raw CI release-tag DTO converted to [`ReleaseVersion`] before rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseVersionWire {
    /// Raw release tag supplied by CI.
    pub value: String,
}

impl ReleaseVersionWire {
    /// Validates raw CI input at this boundary.
    pub fn from_raw_ci_tag(value: String) -> Result<Self, EmptyReleaseVersion> {
        ReleaseVersion::try_from(value.clone())?;
        Ok(Self { value })
    }

    fn to_domain(&self) -> Result<ReleaseVersion, EmptyReleaseVersion> {
        ReleaseVersion::try_from(self.value.clone())
    }
}

/// Rendered archive-provider record paired with its typed gate asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAssetRecord {
    /// Typed release-gate identity for this rendered record.
    pub asset: ReleaseAsset,
    /// Archive file name sent to the release provider.
    pub asset_name: String,
    /// Checksum manifest published alongside the archive.
    pub checksum_name: String,
}

/// Produces the complete provider-facing matrix and its typed gate identities.
#[must_use]
pub fn release_matrix(version: &ReleaseVersionWire) -> Vec<ReleaseAssetRecord> {
    let Ok(version) = version.to_domain() else {
        return Vec::new();
    };
    let mut assets = Vec::with_capacity(TargetPlatform::all().len() * BinaryVariant::all().len());
    for platform in TargetPlatform::all() {
        for variant in BinaryVariant::all() {
            assets.push(render_asset_domain(*platform, *variant, &version));
        }
    }
    assets
}

/// Renders one provider archive record and maps it to the typed gate asset.
#[must_use]
pub fn render_asset(
    platform: TargetPlatform,
    variant: BinaryVariant,
    version: &ReleaseVersionWire,
) -> ReleaseAssetRecord {
    match version.to_domain() {
        Ok(version) => render_asset_domain(platform, variant, &version),
        Err(EmptyReleaseVersion) => ReleaseAssetRecord {
            asset: to_domain_asset(platform, variant),
            asset_name: String::new(),
            checksum_name: String::new(),
        },
    }
}

fn render_asset_domain(
    platform: TargetPlatform,
    variant: BinaryVariant,
    version: &ReleaseVersion,
) -> ReleaseAssetRecord {
    let extension = if matches!(platform, TargetPlatform::WindowsX86_64) {
        "zip"
    } else {
        "tar.gz"
    };
    let asset_name = format!(
        "enforcer-v{}-{}-{}.{}",
        version.as_str(),
        render_variant_label(variant),
        platform.target_triple(),
        extension
    );
    ReleaseAssetRecord {
        asset: to_domain_asset(platform, variant),
        checksum_name: format!("{asset_name}.sha256"),
        asset_name,
    }
}

/// Renders a CI-facing binary-variant label.
#[must_use]
pub fn render_variant_label(variant: BinaryVariant) -> &'static str {
    match variant {
        BinaryVariant::Full => "full",
        BinaryVariant::Lite => "lite",
    }
}

/// Renders the archive name a consumer installer requests.
#[must_use]
pub fn render_asset_name(
    platform: TargetPlatform,
    variant: BinaryVariant,
    version: &str,
) -> String {
    let version = ReleaseVersionWire::from_raw_ci_tag(version.to_owned());
    match version {
        Ok(version) => render_asset(platform, variant, &version).asset_name,
        Err(EmptyReleaseVersion) => String::new(),
    }
}

/// Maps provider coordinates into the domain asset consumed by release gating.
fn to_domain_asset(platform: TargetPlatform, variant: BinaryVariant) -> ReleaseAsset {
    ReleaseAsset { platform, variant }
}

#[cfg(test)]
mod tests {
    use super::release_matrix;
    use super::ReleaseVersionWire;
    use enforcer_domain::install_types::{BinaryVariant, EmptyReleaseVersion, TargetPlatform};

    #[test]
    fn rejects_empty_release_version_at_boundary() {
        assert_eq!(
            ReleaseVersionWire::from_raw_ci_tag(String::new()),
            Err(EmptyReleaseVersion)
        );
    }

    #[test]
    fn matrix_maps_rendered_asset_to_typed_domain_identity() {
        let matrix = match ReleaseVersionWire::from_raw_ci_tag("0.1.0".to_owned()) {
            Ok(version) => release_matrix(&version),
            Err(EmptyReleaseVersion) => Vec::new(),
        };
        assert!(matrix.iter().any(|entry| {
            entry.asset.platform == TargetPlatform::WindowsX86_64
                && entry.asset.variant == BinaryVariant::Lite
                && entry.asset_name == "enforcer-v0.1.0-lite-x86_64-pc-windows-msvc.zip"
        }));
    }
}
