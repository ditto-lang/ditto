use crate::common::{
    parse_error_into_lsp_diagnostic, type_error_into_lsp_diagnostic, warning_into_lsp_diagnostic,
};
use ditto_ast::{self as ast, FullyQualifiedModuleName};
use ditto_checker as checker;
use ditto_cst as cst;
use ropey::Rope;
use tower_lsp::lsp_types::{Diagnostic as LspDiagnostic, Url};

#[salsa::jar(db = Db)]
pub struct Jar(
    Document,
    Diagnostics,
    Imports,
    parse_and_check,
    prepare_checking_environment,
);

pub trait Db: salsa::DbWithJar<Jar> {
    fn get_document(&self, key: &FullyQualifiedModuleName) -> Option<Document>;
}

#[salsa::db(Jar)]
#[derive(Default)]
pub struct Database {
    storage: salsa::Storage<Self>,
    documents: Documents,
}

pub type Documents = dashmap::DashMap<FullyQualifiedModuleName, Document>;

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Database")
    }
}

impl salsa::Database for Database {}

impl salsa::ParallelDatabase for Database {
    fn snapshot(&self) -> salsa::Snapshot<Self> {
        salsa::Snapshot::new(Database {
            storage: self.storage.snapshot(),
            documents: self.documents.clone(),
        })
    }
}

impl Db for Database {
    fn get_document(&self, key: &FullyQualifiedModuleName) -> Option<Document> {
        self.storage.runtime().report_untracked_read(); // do we need to do this?
        Some(self.documents.get(key)?.to_owned())
    }
}

impl Database {
    pub(crate) fn insert_document(&mut self, key: FullyQualifiedModuleName, value: Document) {
        self.documents.insert(key, value);
    }

    pub(crate) fn update_document(&mut self, key: FullyQualifiedModuleName, value: Document) {
        for item in self.documents.iter() {
            let document = item.value();
            if document.uri(self) == value.uri(self) {
                let current_key = item.key();
                if *current_key != key {
                    self.documents.remove(current_key);
                    self.documents.insert(key, *document);
                    return;
                }
            }
        }
    }

    pub(crate) fn remove_document(&mut self, key: &FullyQualifiedModuleName) {
        self.documents.remove(key);
    }
}

pub type DocumentVersion = Option<i32>;

#[salsa::input(jar = Jar)]
pub struct Document {
    #[return_ref]
    pub version: DocumentVersion,
    #[return_ref]
    pub uri: Url,
    #[return_ref]
    pub rope: Rope,
}

#[salsa::accumulator(jar = Jar)]
pub struct Diagnostics(Diagnostic);

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub uri: Url,
    pub version: DocumentVersion,
    pub diagnostic: Option<LspDiagnostic>,
}

#[salsa::tracked(jar = Jar)]
pub fn parse_and_check(
    db: &dyn Db,
    source: Document,
    package: Option<ast::PackageName>,
) -> Option<ast::Module> {
    let uri = source.uri(db);
    let version = source.version(db);
    let rope = source.rope(db);
    let input = rope.to_string();
    match cst::Module::parse(&input) {
        Err(err) => {
            let diagnostic = parse_error_into_lsp_diagnostic(err, uri, rope);
            Diagnostics::push(
                db,
                Diagnostic {
                    uri: uri.clone(),
                    version: *version,
                    diagnostic,
                },
            );
            None
        }
        Ok(cst_module) => {
            let imports = extract_imports(db, &cst_module);
            let everything = prepare_checking_environment(db, imports, package);
            match checker::check_module(&everything, cst_module) {
                Err(err) => {
                    let diagnostic = type_error_into_lsp_diagnostic(err, uri, rope);
                    Diagnostics::push(
                        db,
                        Diagnostic {
                            uri: uri.clone(),
                            version: *version,
                            diagnostic,
                        },
                    );
                    None
                }
                Ok((module, warnings)) => {
                    if warnings.is_empty() {
                        Diagnostics::push(
                            db,
                            Diagnostic {
                                uri: uri.clone(),
                                version: *version,
                                diagnostic: None,
                            },
                        );
                        Some(module)
                    } else {
                        for warning in warnings {
                            let diagnostic = warning_into_lsp_diagnostic(warning, uri, rope);
                            Diagnostics::push(
                                db,
                                Diagnostic {
                                    uri: uri.clone(),
                                    version: *version,
                                    diagnostic,
                                },
                            );
                        }
                        Some(module)
                    }
                }
            }
        }
    }
}

#[salsa::interned (jar = Jar)]
struct Imports {
    #[return_ref]
    imports: Vec<FullyQualifiedModuleName>,
}

fn extract_imports(db: &dyn Db, cst_module: &cst::Module) -> Imports {
    let mut imports = vec![];
    for cst::ImportLine {
        package,
        module_name,
        ..
    } in cst_module.imports.iter()
    {
        let package_name: Option<ast::PackageName> =
            package.as_ref().map(|parens| parens.value.clone().into());
        let module_name: ast::ModuleName = module_name.clone().into();
        let key: FullyQualifiedModuleName = (package_name, module_name);
        imports.push(key)
    }
    Imports::new(db, imports)
}

#[salsa::tracked(jar = Jar)]
fn prepare_checking_environment(
    db: &dyn Db,
    imports: Imports,
    package: Option<ast::PackageName>,
) -> checker::Everything {
    let mut everything = checker::Everything::default();
    for (import_package, import_module_name) in imports.imports(db) {
        // FIXME: lots of cloning below...
        let package_name: Option<ast::PackageName> =
            import_package.as_ref().or(package.as_ref()).cloned();
        let module_name: ast::ModuleName = import_module_name.clone();
        let key: FullyQualifiedModuleName = (package_name, module_name);

        if let Some(document) = db.get_document(&key) {
            if let Some(module) = parse_and_check(db, document, key.0) {
                if let Some(ref package_name) = import_package {
                    if let Some(packages) = everything.packages.get_mut(package_name) {
                        packages.insert(key.1, module.exports);
                    } else {
                        let mut packages = std::collections::HashMap::new();
                        packages.insert(key.1, module.exports);
                        everything.packages.insert(package_name.clone(), packages);
                    }
                } else {
                    everything.modules.insert(key.1, module.exports);
                }
            }
        }
    }
    everything
}
