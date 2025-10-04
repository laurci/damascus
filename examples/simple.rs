use damascus::{JsonSchema, aat::AAT, path, spec::Spec, type_of};

#[derive(JsonSchema)]
struct AddOutput {
    a: u32,
    b: u32,
    output: u32,
}

#[derive(JsonSchema)]
enum Output {
    Add(AddOutput),
}

#[derive(JsonSchema)]
enum Operation {
    Add,
}

#[derive(JsonSchema)]
pub struct Input {
    a: u32,
    b: u32,
}

fn main() {
    let spec = Spec::new("lttle.cloud")
        .description("Simply deploy Docker image as serverless.")
        .organization("lttle.cloud")
        .website("https://lttle.cloud")
        .docs("https://docs.lttle.cloud")
        .repository("https://github.com/lttle-cloud/ignition")
        .service("math", |service| {
            service.post(
                "operation",
                path!("math", operation: Operation),
                |endpoint| endpoint.body(type_of!(Input)).response(type_of!(Output)),
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

    println!("\n=== AAT (Abstract API Tree) ===");
    println!("{:#?}", aat);
}
