mod app_domain;
mod app_license;
mod developer_certificate;
mod signature;
mod signed_app_manifest;
mod trial_app_manifest;

pub use app_domain::AppDomain;
pub use app_license::{AppLicense, AppLicenseType, Expiring, RequesterInfo, SignedAppLicense};
pub use developer_certificate::{DeveloperCertificate, DeveloperCertificateInput, ManifestDeveloperCertificate};
pub use signed_app_manifest::SignedAppManifest;
pub use trial_app_manifest::TrialAppManifest;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AppManifest {
    // NB! Signed needs to come before Trial, due to how serde deserialize untagged enums
    Signed(SignedAppManifest),
    Trial(TrialAppManifest),
}
