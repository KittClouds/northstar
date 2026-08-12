use super::*;

impl Drop for OperatingApp {
    fn drop(&mut self) {
        if let Some(stop) = self.stop_sender.take() {
            let _ = stop.try_send(());
        }
        self.snapshot_task.take();
        self.snapshot_thread.take();
    }
}

impl Render for OperatingApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compact = f32::from(window.viewport_size().width) < 1_720.0;
        let page = match self.page {
            OperatingPage::Desk => self.render_desk(compact, cx).into_any_element(),
            OperatingPage::Macro => self.render_macro_page(compact).into_any_element(),
            OperatingPage::Fund => self.render_fund_board(compact).into_any_element(),
            OperatingPage::Systems => self.render_systems_control_room(compact).into_any_element(),
            OperatingPage::Ledger => self.render_ledger_page(compact, cx).into_any_element(),
        };
        div()
            .size_full()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(CANVAS))
            .text_color(rgb(TEXT))
            .child(self.render_topbar(compact, cx))
            .child(page)
    }
}
