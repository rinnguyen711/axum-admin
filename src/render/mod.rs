pub mod context;

use minijinja::Environment;

pub struct AdminRenderer {
    env: Environment<'static>,
}

impl AdminRenderer {
    pub fn new() -> Self {
        Self::with_overrides(vec![])
    }

    pub fn with_overrides(overrides: Vec<(String, String)>) -> Self {
        let mut env = Environment::new();

        env.add_filter("basename", |value: String| -> String {
            value.rsplit('/').next().unwrap_or(&value).to_string()
        });

        // Built-in templates
        env.add_template_owned("layout.html", include_str!("../../templates/layout.html").to_string()).unwrap();
        env.add_template_owned("login.html", include_str!("../../templates/login.html").to_string()).unwrap();
        env.add_template_owned("list.html", include_str!("../../templates/list.html").to_string()).unwrap();
        env.add_template_owned("list_table.html", include_str!("../../templates/list_table.html").to_string()).unwrap();
        env.add_template_owned("form.html", include_str!("../../templates/form.html").to_string()).unwrap();
        env.add_template_owned("flash.html", include_str!("../../templates/flash.html").to_string()).unwrap();
        env.add_template_owned("home.html", include_str!("../../templates/home.html").to_string()).unwrap();
        env.add_template_owned("change_password.html", include_str!("../../templates/change_password.html").to_string()).unwrap();
        env.add_template_owned("users_list.html", include_str!("../../templates/users_list.html").to_string()).unwrap();
        env.add_template_owned("user_form.html", include_str!("../../templates/user_form.html").to_string()).unwrap();

        // Dev overrides — applied after defaults so they take precedence
        for (name, content) in overrides {
            env.add_template_owned(name, content).unwrap();
        }

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
