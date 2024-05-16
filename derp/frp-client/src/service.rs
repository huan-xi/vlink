pub struct FrpService {
    run_id: String,
}

impl FrpService {
    pub fn new(run_id: String) -> Self {
        Self {
            run_id,
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {

        Ok(())
    }
}