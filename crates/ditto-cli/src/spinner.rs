use crate::common;
use indicatif::{ProgressBar, ProgressStyle};
use std::borrow::Cow;

pub struct Spinner {
    progress: Option<ProgressBar>,
    prefix: Option<String>,
}

impl Spinner {
    pub fn new() -> Self {
        Self::new_impl(None)
    }

    pub fn new_with_prefix(prefix: String) -> Self {
        Self::new_impl(Some(prefix))
    }

    fn new_impl(prefix: Option<String>) -> Self {
        if common::is_plain() {
            return Self {
                progress: None,
                prefix,
            };
        }
        let progress = ProgressBar::new_spinner();
        if let Some(ref prefix) = prefix {
            progress.set_style(
                ProgressStyle::default_spinner()
                    .template("{prefix:.bold.dim} {spinner} {msg:.cyan}"),
            );
            progress.set_prefix(prefix.clone());
        } else {
            progress.set_style(ProgressStyle::default_spinner().template("{spinner} {msg:.cyan}"));
        }
        progress.enable_steady_tick(10);
        progress.set_draw_rate(25);
        Self {
            progress: Some(progress),
            prefix,
        }
    }

    pub fn set_message(&mut self, message: impl Into<Cow<'static, str>>) {
        if let Some(progress) = self.progress.as_ref() {
            progress.set_message(message);
        } else {
            self.print_plain_message(message);
        }
    }

    pub fn println<I: AsRef<str>>(&mut self, message: I) {
        if let Some(progress) = self.progress.as_ref() {
            progress.println(message);
        } else if let Some(ref prefix) = self.prefix {
            println!("{}: {}", prefix, message.as_ref());
        } else {
            println!("{}", message.as_ref());
        }
    }

    pub fn success(self, message: impl Into<Cow<'static, str>>) {
        if let Some(progress) = self.progress {
            if let Some(prefix) = self.prefix {
                progress
                    .with_style(
                        ProgressStyle::default_spinner()
                            .template("{prefix:.bold.green} {spinner} {msg}"),
                    )
                    .finish_with_message(prefix);
            } else {
                progress
                    .with_style(ProgressStyle::default_spinner().template("{msg:.bold.green}"))
                    .finish_with_message(message);
            }
        } else {
            self.print_plain_message(message);
        }
    }

    pub fn _warning(self, message: impl Into<Cow<'static, str>>) {
        if let Some(progress) = self.progress {
            if let Some(prefix) = self.prefix {
                progress
                    .with_style(
                        ProgressStyle::default_spinner()
                            .template("{prefix:.bold.yellow} {spinner} {msg}"),
                    )
                    .finish_with_message(prefix);
            } else {
                progress
                    .with_style(ProgressStyle::default_spinner().template("{msg:.bold.yellow}"))
                    .finish_with_message(message);
            }
        } else {
            self.print_plain_message(message);
        }
    }

    pub fn _fail(self, message: impl Into<Cow<'static, str>>) {
        if let Some(progress) = self.progress {
            if let Some(prefix) = self.prefix {
                progress
                    .with_style(
                        ProgressStyle::default_spinner()
                            .template("{prefix:.bold.red} {spinner} {msg}"),
                    )
                    .finish_with_message(prefix);
            } else {
                progress
                    .with_style(ProgressStyle::default_spinner().template("{msg:.bold.red}"))
                    .finish_with_message(message);
            }
        } else {
            self.print_plain_message(message);
        }
    }

    pub fn finish(self) {
        if let Some(progress) = self.progress {
            progress.finish_and_clear()
        }
    }

    fn print_plain_message(&self, message: impl Into<Cow<'static, str>>) {
        if let Some(ref prefix) = self.prefix {
            println!("{}: {}", prefix, message.into())
        } else {
            println!("{}", message.into())
        }
    }
}
