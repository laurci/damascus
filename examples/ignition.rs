use std::collections::BTreeMap;

use damascus::{
    JsonSchema,
    aat::AAT,
    generate::typescript::TypeScriptGenerator,
    path,
    spec::{Spec, Upgrade},
    type_of, type_of_tuple,
};
use damascus_meta::header_value;

#[derive(JsonSchema)]
struct MachineV1 {
    name: String,
    namespace: Option<String>,
    tags: Option<Vec<String>>,
    image: Option<String>,
    build: Option<MachineBuild>,
    resources: MachineResources,
    #[serde(rename = "restart-policy")]
    restart_policy: Option<MachineRestartPolicy>,
    mode: Option<MachineMode>,
    volumes: Option<Vec<MachineVolumeBinding>>,
    command: Option<Vec<String>>,
    /// !sdk:skip-formatting
    environment: Option<BTreeMap<String, String>>,
    #[serde(rename = "depends-on")]
    depends_on: Option<Vec<MachineDependency>>,
}

#[derive(JsonSchema)]
enum MachineBuild {
    #[serde(rename = "auto")]
    NixpacksAuto,
    #[serde(rename = "options")]
    Nixpacks(MachineBuildOptions),
    #[serde(rename = "docker")]
    Docker(MachineDockerOptions),
}

#[derive(JsonSchema)]
struct MachineBuildOptions {
    name: Option<String>,
    tag: Option<String>,
    image: Option<String>,
    dir: Option<String>,
}

#[derive(JsonSchema)]
struct MachineDockerOptions {
    name: Option<String>,
    tag: Option<String>,
    image: Option<String>,
    context: Option<String>,
    dockerfile: Option<String>,
    #[serde(rename = "args")]
    args: Option<BTreeMap<String, String>>,
}

#[derive(JsonSchema)]
enum MachineRestartPolicy {
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "on-failure")]
    OnFailure,
    #[serde(rename = "remove")]
    Remove,
}

#[derive(JsonSchema)]
struct MachineResources {
    cpu: u8,
    memory: u64,
}

#[derive(JsonSchema)]
enum MachineMode {
    #[serde(rename = "regular")]
    Regular,
    #[serde(rename = "flash")]
    Flash {
        strategy: MachineSnapshotStrategy,
        timeout: Option<u64>,
    },
}

#[derive(JsonSchema)]
enum MachineSnapshotStrategy {
    #[serde(rename = "first-listen")]
    WaitForFirstListen,
    #[serde(rename = "nth-listen")]
    WaitForNthListen(u32),
    #[serde(rename = "listen-on-port")]
    WaitForListenOnPort(u16),
    #[serde(rename = "user-space-ready")]
    WaitForUserSpaceReady,
    #[serde(rename = "manual")]
    Manual,
}

#[derive(JsonSchema)]
struct MachineVolumeBinding {
    name: String,
    #[serde(default)]
    namespace: Option<String>,
    path: String,
}

#[derive(JsonSchema)]
struct MachineDependency {
    name: String,
    #[serde(default)]
    namespace: Option<String>,
}

#[derive(JsonSchema)]
struct Status {
    hash: u64,
    phase: MachinePhase,
    image_id: Option<String>,
    image_resolved_reference: Option<String>,
    machine_id: Option<String>,
    machine_ip: Option<String>,
    machine_tap: Option<String>,
    machine_image_volume_id: Option<String>,
    last_boot_time_us: Option<u64>,
    first_boot_time_us: Option<u64>,
    last_restarting_time_us: Option<u64>,
    last_exit_code: Option<i32>,
}

#[derive(JsonSchema)]
enum MachinePhase {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "pulling-image")]
    PullingImage,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "creating")]
    Creating,
    #[serde(rename = "booting")]
    Booting,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "suspending")]
    Suspending,
    #[serde(rename = "suspended")]
    Suspended,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "restarting")]
    Restarting,
    #[serde(rename = "error")]
    Error { message: String },
}

impl ToString for MachinePhase {
    fn to_string(&self) -> String {
        match self {
            MachinePhase::Idle => "idle".to_string(),
            MachinePhase::PullingImage => "pulling-image".to_string(),
            MachinePhase::Creating => "creating".to_string(),
            MachinePhase::Waiting => "waiting".to_string(),
            MachinePhase::Booting => "booting".to_string(),
            MachinePhase::Ready => "ready".to_string(),
            MachinePhase::Suspending => "suspending".to_string(),
            MachinePhase::Suspended => "suspended".to_string(),
            MachinePhase::Stopping => "stopping".to_string(),
            MachinePhase::Stopped => "stopped".to_string(),
            MachinePhase::Restarting => "restarting".to_string(),
            MachinePhase::Error { message } => format!("error ({})", message),
        }
    }
}

impl ToString for MachineRestartPolicy {
    fn to_string(&self) -> String {
        match self {
            MachineRestartPolicy::Never => "never".to_string(),
            MachineRestartPolicy::Always => "always".to_string(),
            MachineRestartPolicy::OnFailure => "on-failure".to_string(),
            MachineRestartPolicy::Remove => "remove".to_string(),
        }
    }
}

#[derive(JsonSchema)]
struct Log {
    message: String,
    timestamp: u64,
    target: LogTarget,
}

#[derive(JsonSchema)]
enum LogTarget {
    #[serde(rename = "stdout")]
    Stdout,
    #[serde(rename = "stderr")]
    Stderr,
}

#[derive(JsonSchema)]
struct LogRequest {
    follow: bool,
    since: Option<u64>,
    until: Option<u64>,
}

const COMPAT_VERSION: &str = "1";

fn main() {
    let spec = Spec::new("ignition")
        .description("Ignition is a platform for building and deploying machines.")
        .organization("ignition")
        .website("https://ignition.com")
        .docs("https://docs.ignition.com")
        .repository("https://github.com/ignition/ignition")
        .header("x-ignition-compat", header_value!(COMPAT_VERSION))
        .header(
            "x-ignition-token",
            header_value!("Bearer {token}" use token: String),
        )
        .service("machines", |service| {
            service
                .get("get", path!("machines", name: String), |endpoint| {
                    endpoint
                        .response(type_of_tuple!(MachineV1, Status))
                        .header("x-ignition-namespace", header_value!(namespace: String))
                })
                .get("list", path!("machines"), |endpoint| {
                    endpoint
                        .response(type_of_tuple!(MachineV1, Status).wrap_list())
                        .header(
                            "x-ignition-namespace",
                            header_value!(namespace: Option<String>),
                        )
                })
                .get(
                    "status",
                    path!("machines", name: String, "status"),
                    |endpoint| {
                        endpoint
                            .response(type_of!(Status))
                            .header("x-ignition-namespace", header_value!(namespace: String))
                    },
                )
                .put("apply", path!("machines"), |endpoint| {
                    endpoint.body(type_of!(MachineV1))
                })
                .delete("delete", path!("machines", name: String), |endpoint| {
                    endpoint.header("x-ignition-namespace", header_value!(namespace: String))
                })
                .get(
                    "logs",
                    path!("machines", name: String, "logs"),
                    |endpoint| {
                        endpoint
                            .response(type_of!(Log).wrap_stream())
                            .upgrade(Upgrade::Ws)
                            .query(type_of!(LogRequest))
                            .header("x-ignition-namespace", header_value!(namespace: String))
                    },
                )
        });

    println!("=== Converting Spec to AAT ===");

    // Convert spec to AAT
    let aat = match AAT::from_spec(&spec) {
        Ok(aat) => {
            println!("✓ Successfully converted Spec to AAT");
            aat
        }
        Err(e) => {
            println!("✗ Failed to convert Spec to AAT: {}", e);
            return;
        }
    };

    println!("\n=== Validating AAT ===");
    match aat.validate() {
        Ok(_) => {
            println!("✓ AAT validation passed: all type references are valid");
        }
        Err(e) => {
            println!("✗ AAT validation failed: {}", e);
            return;
        }
    }

    println!("AAT: {:#?}", aat);

    println!("\n=== Generating TypeScript Client ===");
    match TypeScriptGenerator::generate(&aat) {
        Ok(ts) => {
            println!("✓ Successfully generated TypeScript client");
            std::fs::write("generated/ignition.ts", ts).expect("Failed to write ignition.ts");
            println!("✓ Written to generated/ignition.ts");
        }
        Err(e) => {
            println!("✗ Failed to generate TypeScript client: {}", e);
            return;
        }
    }
}
