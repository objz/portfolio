use crate::terminal::Terminal;
use wasm_bindgen_futures::spawn_local;

pub mod boot;

impl Terminal {
    pub async fn init_boot(&self) {
        self.clear_output();
        boot::boot(self).await;
        boot::logo(self).await;
        boot::login(self).await;
        self.prepare_for_input();

        spawn_local(async {
            let _ = crate::commands::processor::CommandHandler::sync_default_projects(false).await;
        });
    }
}
