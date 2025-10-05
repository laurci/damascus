use damascus::{JsonSchema, aat::AAT, path, spec::Spec, type_of};

#[derive(JsonSchema)]
struct ComplexType {
    field: String,
}

#[derive(JsonSchema)]
enum Operation {
    Add,
    Subtract,
}

fn main() {
    // Test 1: Object in path parameter (should fail)
    println!("=== Test 1: Object in path parameter ===");
    let spec1 = Spec::new("test").service("test", |service| {
        service.get("test", path!("items", item: ComplexType), |endpoint| {
            endpoint.response(type_of!(String))
        })
    });

    match AAT::from_spec(&spec1) {
        Ok(aat) => match aat.validate() {
            Ok(_) => println!("✗ Validation should have failed but passed"),
            Err(e) => println!("✓ Correctly rejected: {}", e),
        },
        Err(e) => println!("✗ Conversion failed: {}", e),
    }

    // Test 2: String enum in path parameter (should pass)
    println!("\n=== Test 2: String enum in path parameter ===");
    let spec2 = Spec::new("test").service("test", |service| {
        service.get("test", path!("math", operation: Operation), |endpoint| {
            endpoint.response(type_of!(String))
        })
    });

    match AAT::from_spec(&spec2) {
        Ok(aat) => match aat.validate() {
            Ok(_) => println!("✓ Validation passed (string enums are allowed)"),
            Err(e) => println!("✗ Validation should have passed: {}", e),
        },
        Err(e) => println!("✗ Conversion failed: {}", e),
    }
}
