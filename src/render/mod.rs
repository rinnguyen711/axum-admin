pub mod context;

use minijinja::Environment;

pub struct AdminRenderer {
    env: Environment<'static>,
}

impl AdminRenderer {
    pub fn new() -> Self {
        let mut env = Environment::new();

        env.add_template_owned(
            "layout.html",
            include_str!("../../templates/layout.html").to_string(),
        ).unwrap();
        env.add_template_owned(
            "login.html",
            include_str!("../../templates/login.html").to_string(),
        ).unwrap();
        env.add_template_owned(
            "list.html",
            include_str!("../../templates/list.html").to_string(),
        ).unwrap();
        env.add_template_owned(
            "list_table.html",
            include_str!("../../templates/list_table.html").to_string(),
        ).unwrap();
        env.add_template_owned(
            "form.html",
            include_str!("../../templates/form.html").to_string(),
        ).unwrap();

        Self { env }
    }

    pub fn render<S: serde::Serialize>(&self, template: &str, ctx: S) -> String {
        self.env
            .get_template(template)
            .unwrap()
            .render(ctx)
            .unwrap_or_else(|e| format!("<pre>Template error: {e}</pre>"))
    }
}

impl Default for AdminRenderer {
    fn default() -> Self {
        Self::new()
    }
}
