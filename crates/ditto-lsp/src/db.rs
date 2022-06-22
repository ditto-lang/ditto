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

type Module = serde_json::Value;

#[salsa::query_group(ModuleDatabaseStorage)]
trait ModuleDatabase: SourceDatabase {
    fn module(&self, url: Url) -> Module;
}

fn module(db: &dyn ModuleDatabase, url: Url) -> Module {
    let source = db.source(url);
    let cst_module = ditto_cst::Module::parse(&source).unwrap();
    let everything = ditto_checker::Everything::default();
    let (ast_module, _warnings) = ditto_checker::check_module(&everything, cst_module).unwrap();
    serde_json::to_value(ast_module).unwrap()
}

#[salsa::database(SourceDatabaseStorage, ModuleDatabaseStorage)]
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

        // TODO: Durability is higher for package sources!
        self.set_sources_with_durability(sources, salsa::Durability::LOW);
        self.set_source_with_durability(url, source, salsa::Durability::LOW);
    }

    pub fn get_module(&self, url: Url) -> ditto_ast::Module {
        serde_json::from_value(self.module(url)).unwrap()
    }
}
