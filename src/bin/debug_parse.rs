use evento::engine::state::ExecutionPlan;

fn main() {
    let yaml = std::fs::read_to_string("tests/multi_protocol.yaml").unwrap();
    match serde_yaml::from_str::<ExecutionPlan>(&yaml) {
        Ok(_) => println!("Parse OK"),
        Err(e) => println!("Parse error: {}", e),
    }
}
