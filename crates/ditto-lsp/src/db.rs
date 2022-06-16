use url::Url;

// https://salsa-rs.github.io/salsa/common_patterns/on_demand_inputs.html
// https://github.com/ChristopherBiscardi/salsa-file-watch-example/blob/f968dc8ea13a90373f91d962f173de3fe6ae24cd/main.rs
#[salsa::query_group(SourceDatabaseStorage)]
pub trait SourceDatabase: salsa::Database {
    #[salsa::input]
    fn source(&self, url: Url) -> String;
}

#[salsa::database(SourceDatabaseStorage)]
pub struct Database {
    storage: salsa::Storage<Self>,
}

impl<'a> salsa::Database for Database {}

impl Database {
    pub fn new() -> Self {
        Self {
            storage: salsa::Storage::default(),
        }
    }
}
