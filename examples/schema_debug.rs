use damascus::JsonSchema;
use schemars::schema_for;

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
enum MachineMode {
    #[serde(rename = "regular")]
    Regular,
    #[serde(rename = "flash")]
    Flash {
        strategy: MachineSnapshotStrategy,
        timeout: Option<u64>,
    },
}

fn main() {
    let schema = schema_for!(MachineMode);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
