use crate::terminal::Terminal;

pub mod boot;

impl Terminal {
    pub async fn init_boot(&self) {
        self.clear_output();
        boot::boot(self).await;
        boot::logo(self).await;
        boot::login(self).await;
        self.prepare_for_input();
    }
}
