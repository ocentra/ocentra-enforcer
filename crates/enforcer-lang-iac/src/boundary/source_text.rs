//! Raw line-oriented parsing primitives for IaC validators.

use std::num::NonZeroU32;

use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::telemetry_types::SourceLine;

/// Closed matcher vocabulary used by built-in IaC rules.
#[derive(Debug, Clone, Copy)]
pub(crate) enum IacPattern {
    OpenIngress,
    AccessKeyId,
    SecretAccessKey,
    PasswordAssignment,
    S3Bucket,
    ServerSideEncryption,
    RequiredProviders,
    Version,
    S3Backend,
    Encrypt,
    CloudFormationS3Bucket,
    PublicAccessBlock,
    WildcardAction,
    PrivilegedContainer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PatternPresence {
    Present,
    Absent,
}

impl IacPattern {
    pub(crate) fn presence_in(self, source: ValidationSource<'_>) -> PatternPresence {
        let pattern = match self {
            Self::OpenIngress => "0.0.0.0/0",
            Self::AccessKeyId => "aws_access_key_id",
            Self::SecretAccessKey => "aws_secret_access_key",
            Self::PasswordAssignment => "password =",
            Self::S3Bucket => "aws_s3_bucket",
            Self::ServerSideEncryption => "server_side_encryption_configuration",
            Self::RequiredProviders => "required_providers",
            Self::Version => "version",
            Self::S3Backend => "backend \"s3\"",
            Self::Encrypt => "encrypt",
            Self::CloudFormationS3Bucket => "AWS::S3::Bucket",
            Self::PublicAccessBlock => "PublicAccessBlockConfiguration",
            Self::WildcardAction => "\"Action\": \"*\"",
            Self::PrivilegedContainer => "privileged: true",
        };
        if source.as_str().contains(pattern) {
            PatternPresence::Present
        } else {
            PatternPresence::Absent
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ScannedLine<'a> {
    pub(crate) number: SourceLine,
    pub(crate) text: ValidationSource<'a>,
}

pub(crate) fn lines(source: ValidationSource<'_>) -> impl Iterator<Item = ScannedLine<'_>> {
    source
        .as_str()
        .lines()
        .scan(Some(NonZeroU32::MIN), |next_line, text| {
            let current = (*next_line)?;
            *next_line = current.checked_add(1);
            Some(ScannedLine {
                number: SourceLine::try_new(current),
                text: ValidationSource::from_text(text),
            })
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommentLine {
    Yes,
    No,
}

pub(crate) fn is_comment_only_line(text: ValidationSource<'_>) -> CommentLine {
    let trimmed = text.as_str().trim_start();
    if trimmed.starts_with('#') || trimmed.starts_with("//") {
        CommentLine::Yes
    } else {
        CommentLine::No
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use enforcer_domain::boundary::validation::ValidationSource;
    use enforcer_domain::telemetry_types::SourceLine;

    use super::{is_comment_only_line, lines, CommentLine, IacPattern, PatternPresence};

    #[test]
    fn pattern_matching_distinguishes_present_and_absent_literals() {
        assert_eq!(
            IacPattern::OpenIngress
                .presence_in(ValidationSource::from_text("cidr_blocks = [\"0.0.0.0/0\"]",)),
            PatternPresence::Present
        );
        assert_eq!(
            IacPattern::OpenIngress.presence_in(ValidationSource::from_text(
                "cidr_blocks = [\"10.0.0.0/16\"]",
            )),
            PatternPresence::Absent
        );
    }

    #[test]
    fn comment_only_line_detection() {
        assert_eq!(
            is_comment_only_line(ValidationSource::from_text("  # encrypt = true")),
            CommentLine::Yes
        );
        assert_eq!(
            is_comment_only_line(ValidationSource::from_text("encrypt = true")),
            CommentLine::No
        );
    }

    #[test]
    fn lines_are_one_based() {
        let collected: Vec<_> = lines(ValidationSource::from_text("a\nb\nc")).collect();
        assert_eq!(collected[0].number, SourceLine::try_new(NonZeroU32::MIN));
        assert_eq!(
            collected[2].number,
            SourceLine::try_new(NonZeroU32::MIN.saturating_add(2))
        );
    }
}
