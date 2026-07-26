use crate::engine::state::{ExecutionPlan, RunState};
use crate::engine::worker::VuWorker;
use crate::engine::storage::sled_store::SledStore;
use anyhow::Result;
use std::sync::Arc;
use tokio::task;

pub struct RunManager {
    pub plan: Arc<ExecutionPlan>,
    pub store: Arc<SledStore>,
}

impl RunManager {
    pub fn new(plan: ExecutionPlan, store: Arc<SledStore>) -> Self {
        Self {
            plan: Arc::new(plan),
            store,
        }
    }

    /// Spawns all virtual users and orchestrates the run
    pub async fn execute(&self) -> Result<()> {
        let run_id = self.plan.run_id.clone();

        // 1. Mark as Running
        let running_state = serde_json::to_string(&RunState::Running)?;
        self.store.set_run_state(&run_id, &running_state)?;

        // 2. Spawn VU Workers
        let vus = self.plan.config.virtual_users;
        let mut handles = Vec::new();

        for vu_id in 0..vus {
            let plan_clone = self.plan.clone();
            let store_clone = self.store.clone();
            
            let handle = task::spawn(async move {
                let mut worker = VuWorker::new(vu_id, plan_clone, store_clone);
                if let Err(e) = worker.run().await {
                    eprintln!("VU {} failed: {:?}", vu_id, e);
                }
            });
            handles.push(handle);
        }

        // 3. Wait for all VUs to complete
        for handle in handles {
            let _ = handle.await;
        }

        // 4. Check if Cancelled or Mark Completed
        let current_state = self.store.get_run_state(&run_id)?;
        if let Some(state_json) = current_state {
            if !state_json.contains("Cancelled") && !state_json.contains("Failed") {
                let completed_state = serde_json::to_string(&RunState::Completed)?;
                self.store.set_run_state(&run_id, &completed_state)?;
            }
        } else {
            let completed_state = serde_json::to_string(&RunState::Completed)?;
            self.store.set_run_state(&run_id, &completed_state)?;
        }

        Ok(())
    }
}
