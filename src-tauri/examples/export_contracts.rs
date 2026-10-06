use ai_deck::{
    error::AppError, hugging_face_client::HubModel, model_analysis::*, model_catalog::*,
    model_import_service::ModelImportResult, runpod_client::*, types::*,
};
use ts_rs::TS;

fn main() {
    let mut types =
        String::from("// Generated from Rust types. Run npm run contracts; do not edit.\n\n");
    macro_rules! export { ($($ty:ty),* $(,)?) => { $(types.push_str("export "); types.push_str(&<$ty>::decl()); types.push_str("\n\n");)* }; }
    export!(
        AppError,
        DeploymentStage,
        CliKind,
        SessionStatus,
        Policy,
        AppSettings,
        Project,
        Deployment,
        Session,
        LocalState,
        AppSnapshot,
        ProvisionRequest,
        LaunchRequest,
        CliInstallation,
        TerminalChunk,
        TerminalReplay,
        RuntimeProfile,
        Sampling,
        ModelAccess,
        CompatibilityEvidence,
        ModelProfile,
        CatalogDocument,
        ResolvedProfile,
        GpuType,
        GpuAvailability,
        DataCenter,
        NetworkVolume,
        Hardware,
        HubModel,
        ModelAnalysis,
        GpuQuote,
        ImportedModel,
        ModelImportResult
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let outputs = [
        ("src/shared/contracts.ts", types),
        (
            "src/shared/defaults.json",
            serde_json::to_string_pretty(&AppSettings::default()).unwrap(),
        ),
        (
            "model-catalog/schema.json",
            serde_json::to_string_pretty(&schemars::schema_for!(CatalogDocument)).unwrap(),
        ),
    ];
    let check = std::env::args().any(|arg| arg == "--check");
    for (name, contents) in outputs {
        let path = root.join(name);
        if check {
            assert_eq!(
                std::fs::read_to_string(&path).ok().as_deref(),
                Some(contents.as_str()),
                "Generated file {name} is stale; run npm run contracts."
            );
        } else {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }
    }
}
