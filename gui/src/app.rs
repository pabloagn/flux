use ratatui::{prelude::*, widgets::*};

pub struct App {
    pub last_msg: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            last_msg: "…waiting…".into(),
        }
    }

    pub fn ui(&self, f: &mut Frame) {
        let area = f.size();
        let block = Block::default()
            .title("Flux Control-Room")
            .borders(Borders::ALL);
        f.render_widget(block, area);

        // convert &str → Text explicitly to silence the type-inference issue
        let para = Paragraph::new(self.last_msg.as_str()).block(
            Block::default()
                .title("Latest message")
                .border_style(Style::new().cyan()),
        );

        f.render_widget(para, area);
    }
}
