import os
import random
import subprocess
import yaml
import time
import shutil

NAMESPACES = ["sales.buyflow.mock", "analytics.tracking.mock", "infra.health.mock"]
HTTP_METHODS = ["GET", "POST", "PUT", "DELETE"]
STATUS_CODES = [200, 201, 202, 204, 400, 401, 403, 404, 500, 502, 503]
LATENCIES = ["10ms", "50ms", "100ms", "200ms", "500ms", "1s", "${$random.int(10,100)}ms"]

TEST_SUITE_DIR = ".test_suite"

def generate_step(step_idx, all_previous_steps):
    step_name = f"step_{step_idx}"
    is_async = random.random() < 0.2
    
    # Randomly decide to wait for a previous step
    wait_for = None
    if step_idx > 0 and random.random() < 0.3:
        wait_for = [random.choice(all_previous_steps)]

    step = {
        "name": step_name,
        "protocol": random.choice(["http", "kafka", "database"]),
        "mock": {
            "response": {
                "status": random.choice(STATUS_CODES),
                "latency": random.choice(LATENCIES)
            }
        }
    }
    
    if is_async:
        step["async"] = True
    if wait_for:
        step["wait_for"] = wait_for

    # Add some dummy endpoint or topic based on protocol
    if step["protocol"] == "http":
        step["endpoint"] = f"/api/v1/resource/{random.randint(1, 1000)}"
        step["method"] = random.choice(HTTP_METHODS)
    elif step["protocol"] == "kafka":
        step["topic"] = f"events.{random.randint(1, 100)}"
    elif step["protocol"] == "database":
        step["query"] = f"SELECT * FROM table_{random.randint(1, 10)}"

    return step

def generate_test_plan(test_idx):
    namespace = random.choice(NAMESPACES)
    
    num_steps = random.randint(1, 10)
    steps = []
    step_names = []
    
    features = []
    
    for i in range(num_steps):
        step = generate_step(i, step_names)
        steps.append(step)
        step_names.append(step["name"])
        if step.get("async"): features.append("async")
        if step.get("wait_for"): features.append("sync")
        features.append(step["protocol"])
        features.append(f"mock{step['mock']['response']['status']}")

    feature_desc = "_".join(list(set(features))[:3])
    test_name = f"{namespace}.validate_{feature_desc}_{test_idx}"

    plan = {
        "test": test_name,
        "description": f"Validating {feature_desc} behaviors at scale",
        "config": {
            "virtual_users": 1,
            "mock_strategy": "required"
        },
        "scenario": steps
    }
    
    return plan

def main():
    if os.path.exists(TEST_SUITE_DIR):
        shutil.rmtree(TEST_SUITE_DIR)
    os.makedirs(TEST_SUITE_DIR)

    print(f"Generating 100 test plans in {TEST_SUITE_DIR}...")
    yaml_files = []
    for i in range(1, 101):
        plan = generate_test_plan(i)
        filename = os.path.join(TEST_SUITE_DIR, f"run_{i:03d}.yaml")
        with open(filename, "w") as f:
            yaml.dump(plan, f, sort_keys=False)
        yaml_files.append(filename)
        
    print(f"Generated {len(yaml_files)} test plans.")
    print("Executing tests via evento-client sequentially...")
    
    success_count = 0
    failure_count = 0
    
    for idx, filepath in enumerate(yaml_files):
        print(f"Running [{idx+1}/100]: {filepath}")
        
        # Call cargo run --bin evento-client -- run --plan filepath
        # Assuming the server is running on localhost:8080
        cmd = ["cargo", "run", "--bin", "evento-client", "--", "run", "--plan", filepath]
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode == 0:
            success_count += 1
        else:
            failure_count += 1
            print(f"Failed to submit {filepath}: {result.stderr}")
            
    print("\n--- Execution Summary ---")
    print(f"Total Submitted: {success_count + failure_count}")
    print(f"Success: {success_count}")
    print(f"Failures: {failure_count}")

if __name__ == "__main__":
    main()
