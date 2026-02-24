use crate::terminal::Terminal;
use wasm_bindgen_futures::spawn_local;

pub mod sequence;

impl Terminal {
    pub async fn init_boot(&self) {
        self.clear_output();
        sequence::boot(self).await;
        sequence::logo(self).await;
        sequence::login(self).await;
        self.prepare_for_input();

        spawn_local(async {
            let _ = crate::commands::processor::CommandHandler::sync_default_projects(false).await;
        });
    }
}
