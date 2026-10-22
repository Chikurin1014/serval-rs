use dioxus_primitives::toast::{ToastOptions, ToastType, Toasts, use_toast};

/// Shows toasts of a title and a description, as Dioxus' own do: each one.
#[derive(Clone, Copy)]
pub struct Toaster {
    toasts: Toasts,
}

impl Toaster {
    pub fn success(&self, title: &str, description: &str) {
        self.show(ToastType::Success, title, description);
    }

    pub fn info(&self, title: &str, description: &str) {
        self.show(ToastType::Info, title, description);
    }

    pub fn error(&self, title: &str, description: &str) {
        self.show(ToastType::Error, title, description);
    }

    fn show(&self, kind: ToastType, title: &str, description: &str) {
        self.toasts.show(
            title.to_string(),
            kind,
            ToastOptions::new().description(description),
        );
    }
}

pub fn use_toaster() -> Toaster {
    Toaster {
        toasts: use_toast(),
    }
}
