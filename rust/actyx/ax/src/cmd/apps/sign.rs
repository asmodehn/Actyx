use crate::certs::{AppManifest, DeveloperCertificate};
use crate::cmd::AxCliCommand;
use crate::private_key::AxPrivateKey;
use crate::util::formats::{ActyxOSCode, ActyxOSResult, ActyxOSResultExt};
use futures::{stream, Stream};
use std::{fs, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// sign an app manifest
pub struct SignOpts {
    /// Path to certificate that shall be used for signing
    pub path_to_certificate: PathBuf,
    /// Path to app manifest that shall be signed
    pub path_to_manifest: PathBuf,
}

pub fn create_signed_app_manifest(opts: SignOpts) -> ActyxOSResult<AppManifest> {
    let dev_cert = fs::read_to_string(&opts.path_to_certificate)
        .ax_err_ctx(ActyxOSCode::ERR_IO, "Failed to read developer certificate")?;
    let dev_cert: DeveloperCertificate = serde_json::from_str(&dev_cert).ax_err_ctx(
        ActyxOSCode::ERR_INVALID_INPUT,
        "Failed to deserialize developer certificate",
    )?;
    let dev_privkey = dev_cert
        .private_key()
        .map(ActyxOSResult::Ok)
        .unwrap_or_else(|| Ok(AxPrivateKey::from_file(AxPrivateKey::default_user_identity_path()?)?.to_private()))?;
    let app_manifest =
        fs::read_to_string(&opts.path_to_manifest).ax_err_ctx(ActyxOSCode::ERR_IO, "Failed to read app manifest")?;
    let app_manifest: AppManifest = serde_json::from_str(&app_manifest)
        .ax_err_ctx(ActyxOSCode::ERR_INVALID_INPUT, "Failed to deserialize app manifest")?;

    let signed_manifest = AppManifest::sign(
        app_manifest.app_id(),
        app_manifest.display_name().to_owned(),
        app_manifest.version().to_owned(),
        dev_privkey,
        dev_cert.manifest_dev_cert(),
    )
    .ax_err_ctx(ActyxOSCode::ERR_INVALID_INPUT, "Failed to create signed manifest")?;
    let serialized = serde_json::to_string(&signed_manifest)
        .ax_err_ctx(ActyxOSCode::ERR_IO, "Failed to serialize signed app manifest")?;
    fs::write(opts.path_to_manifest, serialized).ax_err_ctx(ActyxOSCode::ERR_IO, "Failed to overwrite app manifest")?;

    Ok(signed_manifest)
}

async fn run(opts: SignOpts) -> ActyxOSResult<AppManifest> {
    create_signed_app_manifest(opts)
}

pub struct AppsSign();

impl AxCliCommand for AppsSign {
    type Opt = SignOpts;
    type Output = AppManifest;
    fn run(opts: SignOpts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts));
        Box::new(stream::once(r))
    }

    fn pretty(_result: Self::Output) -> String {
        "Provided manifest was updated and signed".to_string()
    }
}
