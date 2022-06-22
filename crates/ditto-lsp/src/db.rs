use std::collections::HashSet;
use url::Url;

// https://salsa-rs.github.io/salsa/common_patterns/on_demand_inputs.html
// https://github.com/ChristopherBiscardi/salsa-file-watch-example/blob/f968dc8ea13a90373f91d962f173de3fe6ae24cd/main.rs
#[salsa::query_group(SourceDatabaseStorage)]
trait SourceDatabase: salsa::Database {
    #[salsa::input]
    fn source(&self, url: Url) -> String;

    // NOTE: we need to do this because I don't think it's possible to retrieve
    // the available keys for the SourceQuery
    #[salsa::input]
    fn sources(&self) -> HashSet<Url>;
}

#[salsa::database(SourceDatabaseStorage)]
pub struct Database {
    storage: salsa::Storage<Self>,
}

impl<'a> salsa::Database for Database {}

impl Database {
    pub fn new() -> Self {
        let mut db = Self {
            storage: salsa::Storage::default(),
        };
        db.set_sources_with_durability(HashSet::new(), salsa::Durability::LOW);
        db
    }

    pub fn get_source(&self, url: Url) -> String {
        self.source(url)
    }

    pub fn set_source(&mut self, url: Url, source: String) {
        let mut sources = self.sources();
        sources.insert(url.clone());

        self.set_sources_with_durability(sources, salsa::Durability::LOW);
        self.set_source_with_durability(url, source, salsa::Durability::LOW);
    }
}
