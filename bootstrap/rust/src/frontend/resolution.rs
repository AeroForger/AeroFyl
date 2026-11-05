use std::collections::{BTreeMap, HashMap, HashSet};

use super::ast::{
    Block, EnumDeclaration, Expression, ExpressionKind, Function, Import, Module, Name,
    NamespaceDeclaration, StatementKind, StructDeclaration, Visibility,
};
use super::diagnostics::Diagnostic;
use super::source::{FileId, Span};
use super::types::Type;

/// Sentinel `FileId` for the built-in standard-library namespaces.
pub const STD_FILE: FileId = FileId(u32::MAX);

/// Builds the dotted symbol name used by diagnostics and code generation, for
/// example `file2.File2.function` becomes `file2__File2__function`.
pub fn mangle(path: &str) -> String {
    path.replace('.', "__")
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SymbolId(pub u32);

/// Index into `Resolution::namespaces`.
pub type NamespaceIdx = usize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    Function {
        parameters: Vec<Type>,
        return_type: Type,
    },
    Parameter {
        ty: Type,
    },
    Variable {
        ty: Type,
    },
    /// Builtin signatures and behavior have not been specified.
    Builtin,
    /// A struct or enum declaration; `path` is its canonical namespace path.
    NamedType {
        path: String,
    },
    /// A namespace reached through `use`, the always-present `std`, or a
    /// declaration in an enclosing namespace.
    Module {
        namespace: NamespaceIdx,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    pub id: SymbolId,
    /// The leaf name, without namespace segments.
    pub name: String,
    /// Full dotted namespace path for declared functions, named types, and
    /// standard-library builtins; `None` for locals and unprefixed builtins.
    pub path: Option<String>,
    pub kind: SymbolKind,
    pub declaration: Option<Span>,
    pub visibility: Option<Visibility>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamespaceKind {
    /// The implicit top-level namespace of one file.
    File,
    /// The always-present `std` namespace.
    StdRoot,
    /// A `std.*` module namespace such as `std.io`.
    StdModule,
    /// A nested `namespace` block declared inside a file.
    Nested,
}

#[derive(Clone, Debug)]
pub struct Namespace {
    pub name: String,
    pub path: String,
    pub file: FileId,
    pub kind: NamespaceKind,
    pub parent: Option<NamespaceIdx>,
    /// Functions, child namespaces (as `Module` symbols), and std builtins.
    pub members: HashMap<String, SymbolId>,
    /// Named-type members, kept apart so a function and a struct can share a
    /// leaf name in one namespace.
    pub types: HashMap<String, SymbolId>,
}

/// One import together with the namespace scope that declared it.
#[derive(Clone, Debug)]
pub struct ScopedImport {
    pub import: Import,
    pub namespace: NamespaceIdx,
}

#[derive(Clone, Debug, Default)]
pub struct Resolution {
    pub symbols: Vec<Symbol>,
    pub declarations: HashMap<Span, SymbolId>,
    pub references: HashMap<Span, SymbolId>,
    pub namespaces: Vec<Namespace>,
    /// Root namespace of each source file, keyed by `FileId`.
    pub file_namespaces: HashMap<FileId, NamespaceIdx>,
    /// The always-present `std` namespace root.
    pub std_root: NamespaceIdx,
    /// Canonical named-type path, mapped to the declaring name span.
    pub type_paths: HashMap<String, Span>,
    /// Imports in declaration order, each attached to its namespace scope.
    pub imports: Vec<ScopedImport>,
}

impl Resolution {
    pub fn symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id.0 as usize)
    }
    pub fn declaration(&self, span: Span) -> Option<SymbolId> {
        self.declarations.get(&span).copied()
    }
    pub fn reference(&self, span: Span) -> Option<SymbolId> {
        self.references.get(&span).copied()
    }

    pub fn namespace(&self, id: NamespaceIdx) -> &Namespace {
        self.namespaces.get(id).expect("namespace index is valid")
    }

    /// The dotted path of a namespace, for example `file2.Inner`.
    pub fn namespace_path(&self, id: NamespaceIdx) -> &str {
        &self.namespace(id).path
    }

    pub fn root_namespace_of_file(&self, file: FileId) -> Option<NamespaceIdx> {
        self.file_namespaces.get(&file).copied()
    }

    /// The namespace one import points at. The first dotted segment names the
    /// imported file or `std`; the rest name nested namespaces inside it.
    pub fn namespace_for_import(&self, import: &Import) -> Option<NamespaceIdx> {
        let segments: Vec<&str> = import.name.text.split('.').collect();
        let (mut current, skip) = if segments.first() == Some(&"std") {
            (self.std_root, 1)
        } else {
            (self.root_namespace_of_file(import.target_file?)?, 1)
        };
        for segment in segments.iter().skip(skip) {
            let symbol = self.namespace(current).members.get(*segment).copied()?;
            match self.symbol(symbol)?.kind {
                SymbolKind::Module { namespace } => current = namespace,
                _ => return None,
            }
        }
        Some(current)
    }

    /// Namespace indexes from `start` up to and including its file root,
    /// most specific first.
    pub fn namespace_ancestors_from(&self, start: NamespaceIdx) -> Vec<NamespaceIdx> {
        let mut chain = Vec::new();
        let mut current = Some(start);
        while let Some(id) = current {
            chain.push(id);
            current = self.namespaces[id].parent;
        }
        chain
    }
}

/// Resolves the merged module with one namespace per file.
///
/// `file_names` maps every `FileId` that contributes declarations to its
/// sanitized file-stem namespace name chosen by the driver.
pub fn resolve(
    module: &Module,
    file_names: &HashMap<FileId, String>,
) -> Result<Resolution, Vec<Diagnostic>> {
    let mut resolver = Resolver::new(module, file_names);
    resolver.register_std();
    resolver.register_trees(module);
    resolver.register_functions(module);
    resolver.check_selections();
    resolver.check_std_alias_conflicts();
    resolver.resolve_bodies(module);
    if resolver.diagnostics.is_empty() {
        Ok(resolver.resolution)
    } else {
        Err(resolver.diagnostics)
    }
}

struct Resolver {
    resolution: Resolution,
    scopes: Vec<HashMap<String, SymbolId>>,
    diagnostics: Vec<Diagnostic>,
    /// Canonical type path to the declaring name span; keeps struct-value
    /// receivers free of identifier resolution, as before the refactor.
    type_names: HashMap<String, Span>,
    /// Enclosing namespace chain of the function being resolved,
    /// most specific first.
    ancestors: Vec<NamespaceIdx>,
    /// Sanitized file-stem namespace names chosen by the driver, per `FileId`.
    file_names: HashMap<FileId, String>,
    /// Cached universal builtin symbols shared by every file.
    builtins: HashMap<String, SymbolId>,
    /// Base scope of each namespace: universal builtins, `std`, and the
    /// imports declared by the namespace and its ancestors.
    namespace_scopes: HashMap<NamespaceIdx, HashMap<String, SymbolId>>,
    /// The always-bound `std` namespace root symbol.
    std_symbol: SymbolId,
}

impl Resolver {
    fn new(_module: &Module, file_names: &HashMap<FileId, String>) -> Self {
        Self {
            resolution: Resolution::default(),
            scopes: vec![HashMap::new()],
            diagnostics: Vec::new(),
            type_names: HashMap::new(),
            ancestors: Vec::new(),
            file_names: file_names.clone(),
            builtins: HashMap::new(),
            namespace_scopes: HashMap::new(),
            std_symbol: SymbolId(u32::MAX),
        }
    }

    // ---------- namespace construction ----------

    fn create_namespace(
        &mut self,
        parent: Option<NamespaceIdx>,
        name: String,
        path: String,
        file: FileId,
        kind: NamespaceKind,
    ) -> NamespaceIdx {
        let id = self.resolution.namespaces.len();
        self.resolution.namespaces.push(Namespace {
            name,
            path,
            file,
            kind,
            parent,
            members: HashMap::new(),
            types: HashMap::new(),
        });
        id
    }

    fn make_symbol(
        name: &str,
        path: Option<String>,
        kind: SymbolKind,
        declaration: Option<Span>,
        visibility: Option<Visibility>,
    ) -> Symbol {
        Symbol {
            id: SymbolId(u32::MAX),
            name: name.to_owned(),
            path,
            kind,
            declaration,
            visibility,
        }
    }

    fn push_symbol(&mut self, mut symbol: Symbol) -> SymbolId {
        let id = SymbolId(self.resolution.symbols.len() as u32);
        symbol.id = id;
        self.resolution.symbols.push(symbol);
        id
    }

    /// Builds the always-present standard-library namespace tree.
    fn register_std(&mut self) {
        let std_root = self.create_namespace(
            None,
            "std".to_owned(),
            "std".to_owned(),
            STD_FILE,
            NamespaceKind::StdRoot,
        );
        self.resolution.std_root = std_root;
        self.std_symbol = self.push_symbol(Self::make_symbol(
            "std",
            Some("std".to_owned()),
            SymbolKind::Module {
                namespace: std_root,
            },
            None,
            None,
        ));
        self.register_std_module("io", &["print", "println", "eprint", "eprintln", "input"]);
        self.register_std_module(
            "fs",
            &["readFile", "writeFile", "readBytes", "writeBytes", "exists"],
        );
        self.register_std_module("os", &["os", "process"]);
        // `Process` is the compiler-built struct injected by the driver into
        // `std.os`.
        let id = self.push_symbol(Self::make_symbol(
            "Process",
            Some("std.os.Process".to_owned()),
            SymbolKind::NamedType {
                path: "std.os.Process".to_owned(),
            },
            None,
            None,
        ));
        if let Some(os) = self.std_child("os") {
            self.resolution.namespaces[os]
                .types
                .insert("Process".to_owned(), id);
        }
    }

    /// Registers one `std.<module>` namespace with its builtin members.
    fn register_std_module(&mut self, name: &str, members: &[&str]) {
        let parent = self.resolution.std_root;
        let path = format!("std.{name}");
        let id = self.create_namespace(
            Some(parent),
            name.to_owned(),
            path.clone(),
            STD_FILE,
            NamespaceKind::StdModule,
        );
        let child = self.push_symbol(Self::make_symbol(
            name,
            Some(path.clone()),
            SymbolKind::Module { namespace: id },
            None,
            None,
        ));
        self.resolution.namespaces[parent]
            .members
            .insert(name.to_owned(), child);
        for member in members {
            let symbol = self.push_symbol(Self::make_symbol(
                member,
                Some(format!("{path}.{member}")),
                SymbolKind::Builtin,
                None,
                None,
            ));
            self.resolution.namespaces[id]
                .members
                .insert((*member).to_owned(), symbol);
        }
    }

    fn std_child(&self, name: &str) -> Option<NamespaceIdx> {
        let symbol = self
            .resolution
            .namespaces
            .get(self.resolution.std_root)?
            .members
            .get(name)
            .copied()?;
        match self.resolution.symbol(symbol)?.kind.clone() {
            SymbolKind::Module { namespace } => Some(namespace),
            _ => None,
        }
    }

    /// Gets or creates the implicit root namespace of one file.
    fn root_namespace(&mut self, file: FileId) -> Option<NamespaceIdx> {
        if let Some(id) = self.resolution.file_namespaces.get(&file) {
            return Some(*id);
        }
        let name = self.file_names.get(&file)?.clone();
        let id = self.create_namespace(None, name.clone(), name.clone(), file, NamespaceKind::File);
        self.resolution.file_namespaces.insert(file, id);
        Some(id)
    }

    /// Gets or creates a nested namespace under `parent`. Reopening a
    /// namespace in one file adds members to the existing namespace; a
    /// namespace only ever opens inside the file that declared it.
    fn nested_namespace(&mut self, parent: NamespaceIdx, name: &str) -> NamespaceIdx {
        if let Some(existing) = self.resolution.namespaces[parent].members.get(name) {
            if let Some(SymbolKind::Module { namespace }) = self
                .resolution
                .symbol(*existing)
                .map(|symbol| symbol.kind.clone())
            {
                return namespace;
            }
        }
        let path = format!("{}.{}", self.resolution.namespaces[parent].path, name);
        let file = self.resolution.namespaces[parent].file;
        let id = self.create_namespace(
            Some(parent),
            name.to_owned(),
            path.clone(),
            file,
            NamespaceKind::Nested,
        );
        let symbol = self.push_symbol(Self::make_symbol(
            name,
            Some(path),
            SymbolKind::Module { namespace: id },
            None,
            None,
        ));
        self.resolution.namespaces[parent]
            .members
            .insert(name.to_owned(), symbol);
        id
    }

    /// Defines one named-type member of a namespace and records its canonical
    /// path in the shared type registry.
    fn define_type(&mut self, namespace: NamespaceIdx, name: &str, span: Span, path: String) {
        if self.type_names.contains_key(&path) {
            self.diagnostics.push(Diagnostic::error(
                format!("duplicate type declaration `{path}`"),
                span,
            ));
            return;
        }
        let id = self.push_symbol(Self::make_symbol(
            name,
            Some(path.clone()),
            SymbolKind::NamedType { path: path.clone() },
            Some(span),
            None,
        ));
        self.resolution.namespaces[namespace]
            .types
            .insert(name.to_owned(), id);
        self.resolution.declarations.insert(span, id);
        self.resolution.type_paths.insert(path.clone(), span);
        self.type_names.insert(path, span);
    }

    /// Defines one function member. The language has no overloading, so any
    /// second declaration of one name in one namespace is an error.
    fn define_function(&mut self, namespace: NamespaceIdx, function: &Function) {
        if self.resolution.namespaces[namespace]
            .members
            .contains_key(&function.name.text)
        {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "duplicate declaration of `{}` in the same namespace",
                    function.name.text
                ),
                function.name.span,
            ));
            return;
        }
        let ancestors = self.namespace_ancestors(namespace);
        let parameters = self.canonicalize(
            function
                .parameters
                .iter()
                .map(|parameter| &parameter.ty.kind)
                .collect(),
            &ancestors,
        );
        let return_type = self.canonical_type(&function.return_type.kind, &ancestors);
        let path = format!(
            "{}.{}",
            self.resolution.namespaces[namespace].path, function.name.text
        );
        let id = self.push_symbol(Self::make_symbol(
            &function.name.text,
            Some(path),
            SymbolKind::Function {
                parameters,
                return_type,
            },
            Some(function.name.span),
            Some(function.visibility),
        ));
        self.resolution.namespaces[namespace]
            .members
            .insert(function.name.text.clone(), id);
        self.resolution.declarations.insert(function.name.span, id);
    }

    /// Qualifies one declared type so a signature or field can be looked up
    /// without its enclosing context. Unqualified names resolve through the
    /// namespace chain of the declaring file, then through `using` selections
    /// visible in that chain. Anything else stays as-is.
    fn canonical_type(&self, ty: &Type, context: &[NamespaceIdx]) -> Type {
        match ty {
            Type::Named(name) if !name.contains('.') => {
                if let Some(path) = self.type_path_in_context(name, context) {
                    return Type::Named(path);
                }
                Type::Named(name.clone())
            }
            other => other.clone(),
        }
    }

    fn type_path_in_context(&self, name: &str, context: &[NamespaceIdx]) -> Option<String> {
        for namespace in context {
            let candidate = format!("{}.{}", self.resolution.namespaces[*namespace].path, name);
            if self.type_names.contains_key(&candidate) {
                return Some(candidate);
            }
        }
        for namespace in context {
            for scoped in &self.resolution.imports {
                if scoped.namespace != *namespace {
                    continue;
                }
                let Some(selected) = &scoped.import.selected else {
                    continue;
                };
                if !selected.iter().any(|item| item.text == name) {
                    continue;
                }
                let Some(target) = self.resolution.namespace_for_import(&scoped.import) else {
                    continue;
                };
                let candidate = format!("{}.{}", self.resolution.namespaces[target].path, name);
                if self.type_names.contains_key(&candidate) {
                    return Some(candidate);
                }
            }
        }
        None
    }

    fn canonicalize(&self, types: Vec<&Type>, context: &[NamespaceIdx]) -> Vec<Type> {
        types
            .iter()
            .map(|ty| self.canonical_type(ty, context))
            .collect()
    }

    /// Namespaces from the most specific enclosing namespace back to the file
    /// root, all inside one file.
    fn namespace_ancestors(&self, namespace: NamespaceIdx) -> Vec<NamespaceIdx> {
        let mut chain = Vec::new();
        let mut current = Some(namespace);
        while let Some(id) = current {
            chain.push(id);
            current = self.resolution.namespaces[id].parent;
        }
        chain
    }

    /// The `FileId` of a namespace, used to skip cross-file visibility work
    /// for references inside the declaring file.
    fn member_of_namespace(&self, namespace: NamespaceIdx, name: &str) -> Option<SymbolId> {
        let space = &self.resolution.namespaces[namespace];
        space
            .types
            .get(name)
            .or_else(|| space.members.get(name))
            .copied()
    }

    /// True when the injected `std.os` `Process` struct matches this span.
    fn is_injected_process(&self, module: &Module, span: Span) -> bool {
        span == Span::new(STD_FILE, 0, 0)
            || module
                .imports
                .iter()
                .any(|import| import.name.text == "std.os" && import.name.span == span)
    }

    // ---------- registration: trees then functions ----------

    /// Builds namespace trees and named types. Types come before function
    /// declarations so parameter signatures can qualify without forward
    /// context, and returns the ordered file set.
    fn register_trees(&mut self, module: &Module) {
        let mut files: Vec<FileId> = Vec::new();
        for function in &module.functions {
            files.push(function.name.span.file);
        }
        for declaration in &module.structs {
            let span = declaration.name.span;
            if self.is_injected_process(module, span) {
                continue;
            }
            files.push(span.file);
        }
        for declaration in &module.enums {
            files.push(declaration.name.span.file);
        }
        for namespace in &module.namespaces {
            files.push(namespace.name.span.file);
            let mut nested = Vec::new();
            collect_namespace_files(namespace, &mut nested);
            files.extend(nested);
        }
        for import in &module.imports {
            files.push(import.span.file);
        }

        let mut seen = HashSet::new();
        for file in &files {
            if seen.insert(*file) {
                self.root_namespace(*file);
            }
        }

        for import in &module.imports {
            let Some(namespace) = self.root_namespace(import.span.file) else {
                continue;
            };
            self.resolution.imports.push(ScopedImport {
                import: import.clone(),
                namespace,
            });
        }

        // Nested namespace blocks live under their file's root namespace.
        let trees: Vec<_> = module.namespaces.iter().collect();
        for declaration in &trees {
            let Some(parent) = self.root_namespace(declaration.name.span.file) else {
                continue;
            };
            self.build_namespace_tree(parent, declaration);
        }

        for declaration in &module.structs {
            let span = declaration.name.span;
            if self.is_injected_process(module, span) {
                continue;
            }
            let Some(namespace) = self.root_namespace(span.file) else {
                continue;
            };
            self.declare_struct(namespace, declaration);
            self.check_member_names(
                declaration.fields.iter().map(|field| &field.name),
                "struct field",
            );
        }
        for declaration in &module.enums {
            let namespace_ = declaration.name.span.file;
            let Some(namespace) = self.root_namespace(namespace_) else {
                continue;
            };
            self.declare_enum(namespace, declaration);
            self.check_member_names(
                declaration.variants.iter().map(|variant| &variant.name),
                "enum variant",
            );
        }
    }

    /// Registers the namespace tree and named types of one block.
    fn build_namespace_tree(&mut self, parent: NamespaceIdx, declaration: &NamespaceDeclaration) {
        let namespace = self.nested_namespace(parent, &declaration.name.text);
        for import in &declaration.imports {
            self.resolution.imports.push(ScopedImport {
                import: import.clone(),
                namespace,
            });
        }
        for child in &declaration.namespaces {
            self.build_namespace_tree(namespace, child);
        }
        for type_declaration in &declaration.structs {
            self.declare_struct(namespace, type_declaration);
        }
        for type_declaration in &declaration.enums {
            self.declare_enum(namespace, type_declaration);
        }
    }

    fn declare_struct(&mut self, namespace: NamespaceIdx, declaration: &StructDeclaration) {
        let path = format!(
            "{}.{}",
            self.resolution.namespaces[namespace].path, declaration.name.text
        );
        self.define_type(
            namespace,
            &declaration.name.text,
            declaration.name.span,
            path,
        );
    }

    fn declare_enum(&mut self, namespace: NamespaceIdx, declaration: &EnumDeclaration) {
        let path = format!(
            "{}.{}",
            self.resolution.namespaces[namespace].path, declaration.name.text
        );
        self.define_type(
            namespace,
            &declaration.name.text,
            declaration.name.span,
            path,
        );
    }

    /// Declares top-level and namespace-block functions after all types are
    /// known, walking the same structures the parser produced.
    fn register_functions(&mut self, module: &Module) {
        for function in &module.functions {
            let Some(namespace) = self.root_namespace(function.name.span.file) else {
                continue;
            };
            self.define_function(namespace, function);
        }
        for declaration in &module.namespaces {
            let Some(parent) = self.root_namespace(declaration.name.span.file) else {
                continue;
            };
            self.register_namespace_functions(parent, declaration);
        }
    }

    fn register_namespace_functions(
        &mut self,
        parent: NamespaceIdx,
        declaration: &NamespaceDeclaration,
    ) {
        let Some(namespace) = self.namespace_of(parent, &declaration.name.text) else {
            return;
        };
        for function in &declaration.functions {
            self.define_function(namespace, function);
        }
        for child in &declaration.namespaces {
            self.register_namespace_functions(namespace, child);
        }
    }

    fn namespace_of(&self, parent: NamespaceIdx, name: &str) -> Option<NamespaceIdx> {
        let symbol = self.resolution.namespaces[parent]
            .members
            .get(name)
            .copied()?;
        match self.resolution.symbol(symbol)?.kind.clone() {
            SymbolKind::Module { namespace } => Some(namespace),
            _ => None,
        }
    }

    /// Duplicate name check used for struct fields and enum variants.
    fn check_member_names<'a>(&mut self, names: impl Iterator<Item = &'a Name>, description: &str) {
        let mut seen = HashMap::new();
        for name in names {
            if seen.insert(&name.text, name.span).is_some() {
                self.diagnostics.push(Diagnostic::error(
                    format!("duplicate {description} `{}`", name.text),
                    name.span,
                ));
            }
        }
    }

    // ---------- import selection ----------

    /// Validates `using` selections against the target namespace and records
    /// the leaf symbols the selection unqualifiedly binds.
    fn check_selections(&mut self) {
        let imports: Vec<Import> = self
            .resolution
            .imports
            .iter()
            .map(|scoped| scoped.import.clone())
            .collect();
        for import in imports {
            if !import.name.text.starts_with("std.") {
                self.validate_user_selection(&import);
            }
        }
    }

    fn validate_user_selection(&mut self, import: &Import) {
        let Some(selected) = &import.selected else {
            return;
        };
        let segments: Vec<&str> = import.name.text.split('.').collect();
        let Some(target_file) = import.target_file else {
            return;
        };
        let Some(mut namespace) = self.root_namespace(target_file) else {
            return;
        };
        for segment in segments.iter().skip(1) {
            match self.namespace_of(namespace, segment) {
                Some(child) => namespace = child,
                None => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("module does not declare `{segment}`"),
                        import.span,
                    ));
                    return;
                }
            }
        }
        for item in selected {
            let leaf = self.member_of_namespace(namespace, &item.text);
            let Some(symbol) = leaf else {
                self.diagnostics.push(Diagnostic::error(
                    format!("module does not declare `{}`", item.text),
                    item.span,
                ));
                continue;
            };
            let symbol_ref = &self
                .resolution
                .symbol(symbol)
                .expect("known member")
                .clone();
            if symbol_ref.visibility == Some(Visibility::Private) {
                self.diagnostics.push(Diagnostic::error(
                    format!("private function `{}` cannot be selected", item.text),
                    item.span,
                ));
            }
        }
    }

    fn namespace_scope(&mut self, namespace: NamespaceIdx) -> HashMap<String, SymbolId> {
        let mut scope = HashMap::new();
        for builtin in [
            "exit",
            "int",
            "char",
            "byte",
            "some",
            "none",
            "reference",
            "string",
            "bool",
            "float",
        ] {
            let id = self.builtins.get(builtin).copied().unwrap_or_else(|| {
                let id = self.push_symbol(Self::make_symbol(
                    builtin,
                    None,
                    SymbolKind::Builtin,
                    None,
                    None,
                ));
                self.builtins.insert(builtin.to_owned(), id);
                id
            });
            scope.insert(builtin.to_owned(), id);
        }
        scope.insert("std".to_owned(), self.std_symbol);
        let ancestors = self.namespace_ancestors(namespace);
        for ancestor in ancestors.iter().rev() {
            let imports: Vec<_> = self
                .resolution
                .imports
                .iter()
                .filter(|scoped| scoped.namespace == *ancestor)
                .map(|scoped| scoped.import.clone())
                .collect();
            for import in imports {
                self.bind_import(&import, &mut scope);
            }
        }
        scope
    }

    /// Resolves the namespace an import names, reporting the first missing
    /// segment for user modules.
    fn import_namespace(&mut self, import: &Import) -> Option<NamespaceIdx> {
        if import.name.text.starts_with("std.") {
            if let Some(namespace) = self.resolution.namespace_for_import(import) {
                return Some(namespace);
            }
            self.diagnostics.push(Diagnostic::error(
                format!("unknown standard-library module `{}`", import.name.text),
                import.span,
            ));
            return None;
        }
        let target_file = import.target_file?;
        let mut namespace = self.root_namespace(target_file)?;
        for segment in import.name.text.split('.').skip(1) {
            match self.namespace_of(namespace, segment) {
                Some(child) => namespace = child,
                None => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("module does not declare `{segment}`"),
                        import.span,
                    ));
                    return None;
                }
            }
        }
        Some(namespace)
    }

    fn bind_import(&mut self, import: &Import, scope: &mut HashMap<String, SymbolId>) {
        let Some(namespace) = self.import_namespace(import) else {
            return;
        };
        if let Some(selected) = &import.selected {
            for item in selected {
                let Some(symbol) = self.member_of_namespace(namespace, &item.text) else {
                    continue; // reported by check_selections
                };
                scope.entry(item.text.clone()).or_insert(symbol);
            }
            return;
        }
        if import.name.text.starts_with("std.") {
            // Standard-library members stay reachable as `std.<module>`. A
            // whole-module import also binds the module's last segment so
            // `use std.io;` allows `io.print`. File members and locals keep
            // precedence over the alias because `lookup_symbol` checks
            // namespaces before the import scope.
            let short = import.name.text.rsplit('.').next().unwrap_or("");
            let parent = self.resolution.namespaces[namespace].parent;
            let child = parent.and_then(|parent| {
                self.resolution.namespaces[parent]
                    .members
                    .get(short)
                    .copied()
                    .map(|sym| {
                        matches!(
                            self.resolution.symbol(sym).map(|s| &s.kind),
                            Some(SymbolKind::Module { .. })
                        )
                        .then_some(sym)
                    })
                    .flatten()
            });
            let Some(child) = child else { return };
            if let Some(existing) = scope.get(short) {
                if *existing == child {
                    return;
                }
            }
            scope.entry(short.to_owned()).or_insert(child);
            return;
        }
        let Some(target_file) = import.target_file else {
            return;
        };
        let Some(root) = self.root_namespace(target_file) else {
            return;
        };
        let Some(stem) = import.name.text.split('.').next() else {
            return;
        };
        let symbol = self.push_symbol(Self::make_symbol(
            stem,
            Some(self.resolution.namespaces[root].path.clone()),
            SymbolKind::Module { namespace: root },
            None,
            None,
        ));
        scope.entry(stem.to_owned()).or_insert(symbol);
    }

    /// Whole-module std aliases visible from a namespace: the short name,
    /// for example `io`, to the imported path, for example `std.io`.
    fn std_aliases(&self, namespace: NamespaceIdx) -> Vec<(String, String)> {
        let mut aliases = BTreeMap::new();
        for ancestor in self.namespace_ancestors(namespace) {
            for scoped in &self.resolution.imports {
                if scoped.namespace != ancestor || scoped.import.selected.is_some() {
                    continue;
                }
                let text = scoped.import.name.text.as_str();
                if !text.starts_with("std.") {
                    continue;
                }
                if let Some(short) = text.rsplit('.').next() {
                    aliases
                        .entry(short.to_owned())
                        .or_insert_with(|| text.to_owned());
                }
            }
        }
        aliases.into_iter().collect()
    }

    /// The imported std path whose short alias collides with `name` in the
    /// enclosing namespace chain, if any.
    fn std_alias_path(&self, name: &str) -> Option<String> {
        for &ancestor in &self.ancestors {
            for scoped in &self.resolution.imports {
                if scoped.namespace != ancestor || scoped.import.selected.is_some() {
                    continue;
                }
                let text = scoped.import.name.text.as_str();
                if text.starts_with("std.") && text.rsplit('.').next() == Some(name) {
                    return Some(text.to_owned());
                }
            }
        }
        None
    }

    /// Whole-module std aliases must not be hidden by declarations. A file
    /// member or local named like a bound alias, for example `io` next to
    /// `use std.io;`, is an error.
    fn check_std_alias_conflicts(&mut self) {
        let count = self.resolution.namespaces.len();
        for index in 0..count {
            let id = index;
            let aliases = self.std_aliases(id);
            if aliases.is_empty() {
                continue;
            }
            let namespace = &self.resolution.namespaces[index];
            let mut conflicts = Vec::new();
            for (name, path) in &aliases {
                if let Some(member) = namespace.members.get(name) {
                    conflicts.push((path.to_owned(), *member));
                } else if let Some(member) = namespace.types.get(name) {
                    conflicts.push((path.to_owned(), *member));
                }
            }
            for (path, member) in conflicts {
                let Some(symbol) = self.resolution.symbol(member) else {
                    continue;
                };
                let name = symbol.name.clone();
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "declaration `{name}` conflicts with the `{path}` alias from `use {path};`"
                    ),
                    symbol
                        .declaration
                        .unwrap_or_else(|| Span::new(FileId(0), 0, 0)),
                ));
            }
        }
    }

    // ---------- bodies ----------

    /// Resolves all function bodies with their file's base scope.
    fn resolve_bodies(&mut self, module: &Module) {
        for function in &module.functions {
            let Some(namespace) = self.root_namespace(function.name.span.file) else {
                continue;
            };
            self.resolve_function_body(namespace, function);
        }
        for declaration in &module.namespaces {
            let Some(parent) = self.root_namespace(declaration.name.span.file) else {
                continue;
            };
            self.resolve_namespace_bodies(parent, declaration);
        }
    }

    fn resolve_namespace_bodies(
        &mut self,
        parent: NamespaceIdx,
        declaration: &NamespaceDeclaration,
    ) {
        let Some(namespace) = self.namespace_of(parent, &declaration.name.text) else {
            return;
        };
        for function in &declaration.functions {
            self.resolve_function_body(namespace, function);
        }
        for child in &declaration.namespaces {
            self.resolve_namespace_bodies(namespace, child);
        }
    }

    fn resolve_function_body(&mut self, namespace: NamespaceIdx, function: &Function) {
        self.ancestors = self.namespace_ancestors(namespace);
        let base = self
            .namespace_scopes
            .get(&namespace)
            .cloned()
            .unwrap_or_else(|| {
                let scope = self.namespace_scope(namespace);
                self.namespace_scopes.insert(namespace, scope.clone());
                scope
            });
        self.scopes = vec![base];
        self.scopes.push(HashMap::new());
        for parameter in &function.parameters {
            self.declare_local(
                &parameter.name.text,
                SymbolKind::Parameter {
                    ty: parameter.ty.kind.clone(),
                },
                parameter.name.span,
            );
        }
        self.resolve_block(&function.body, false);
        self.scopes.pop();
    }

    fn declare_local(&mut self, name: &str, kind: SymbolKind, span: Span) {
        if let Some(path) = self.std_alias_path(name) {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "declaration `{name}` conflicts with the `{path}` alias from `use {path};`"
                ),
                span,
            ));
            return;
        }
        if self
            .scopes
            .last()
            .expect("function scope exists")
            .contains_key(name)
        {
            self.diagnostics.push(Diagnostic::error(
                format!("duplicate declaration of `{name}` in the same scope"),
                span,
            ));
            return;
        }
        let id = self.push_symbol(Self::make_symbol(name, None, kind, Some(span), None));
        self.resolution.declarations.insert(span, id);
        self.scopes
            .last_mut()
            .expect("function scope exists")
            .insert(name.to_owned(), id);
    }

    fn resolve_block(&mut self, block: &Block, nested: bool) {
        if nested {
            self.scopes.push(HashMap::new());
        }
        for statement in &block.statements {
            self.resolve_statement(statement);
        }
        if nested {
            self.scopes.pop();
        }
    }

    fn resolve_statement(&mut self, statement: &super::ast::Statement) {
        match &statement.kind {
            StatementKind::Variable(variable) => {
                self.resolve_expression(&variable.initializer);
                self.declare_local(
                    &variable.name.text,
                    SymbolKind::Variable {
                        ty: variable.ty.kind.clone(),
                    },
                    variable.name.span,
                );
            }
            StatementKind::Assignment(assignment) => {
                self.resolve_expression(&assignment.target);
                self.resolve_expression(&assignment.value);
            }
            StatementKind::Return(Some(value)) | StatementKind::Expression(value) => {
                self.resolve_expression(value);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                self.resolve_expression(condition);
                self.resolve_block(then_block, true);
                if let Some(else_block) = else_block {
                    self.resolve_block(else_block, true);
                }
            }
            StatementKind::While { condition, body } => {
                self.resolve_expression(condition);
                self.resolve_block(body, true);
            }
            StatementKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                self.scopes.push(HashMap::new());
                self.resolve_statement(initializer);
                self.resolve_expression(condition);
                self.resolve_statement(increment);
                self.resolve_block(body, true);
                self.scopes.pop();
            }
            StatementKind::Return(None) | StatementKind::Break | StatementKind::Continue => {}
        }
    }

    fn resolve_expression(&mut self, expression: &Expression) {
        match &expression.kind {
            ExpressionKind::Identifier(name) => self.resolve_name(&name.text, name.span),
            ExpressionKind::Call { callee, arguments } => {
                self.resolve_name(&callee.text, callee.span);
                for argument in arguments {
                    if callee.text == "input"
                        && matches!(
                            &argument.kind,
                            ExpressionKind::Identifier(name)
                                if matches!(
                                    name.text.as_str(),
                                    "string" | "int" | "bool" | "char" | "float"
                                )
                        )
                    {
                        continue;
                    }
                    self.resolve_expression(argument);
                }
            }
            ExpressionKind::Unary { operand, .. } => self.resolve_expression(operand),
            ExpressionKind::Binary { left, right, .. } => {
                self.resolve_expression(left);
                self.resolve_expression(right);
            }
            ExpressionKind::Collection(values) | ExpressionKind::Tuple(values) => {
                for value in values {
                    self.resolve_expression(value);
                }
            }
            ExpressionKind::StructLiteral { fields, name } => {
                let _ = name;
                for field in fields {
                    self.resolve_expression(&field.value);
                }
            }
            ExpressionKind::Member { base, name } => {
                if let Some((root, segments)) = self.chain_segments(base) {
                    self.resolve_chain(&root, segments, &name.text, name.span);
                    return;
                }
                if !self.receiver_is_type(base) {
                    self.resolve_expression(base);
                }
            }
            ExpressionKind::Index { base, index } => {
                self.resolve_expression(base);
                self.resolve_expression(index);
            }
            ExpressionKind::MethodCall {
                receiver,
                method,
                arguments,
            } => {
                if let Some((root, segments)) = self.chain_segments(receiver) {
                    self.resolve_chain(&root, segments, &method.text, method.span);
                    for argument in arguments {
                        self.resolve_expression(argument);
                    }
                    return;
                }
                if !self.receiver_is_type(receiver) {
                    self.resolve_expression(receiver);
                }
                for argument in arguments {
                    self.resolve_expression(argument);
                }
            }
            ExpressionKind::Literal(_) => {}
            ExpressionKind::Interpolated(parts) => {
                for part in parts {
                    if let super::token::InterpolationPart::Variable(name, span) = part {
                        self.resolve_name(name, *span);
                    }
                }
            }
        }
    }

    /// Flattens an unbroken Member chain into its root identifier and the
    /// member names after it.
    fn chain_segments(&self, expression: &Expression) -> Option<(Name, Vec<Name>)> {
        match &expression.kind {
            ExpressionKind::Identifier(name) => Some((name.clone(), Vec::new())),
            ExpressionKind::Member { base, name } => {
                let (root, mut segments) = self.chain_segments(base)?;
                segments.push(name.clone());
                Some((root, segments))
            }
            _ => None,
        }
    }

    /// Resolves a dotted path. Namespace segments resolve one at a time and
    /// the failing segment is named in the diagnostic. The final name is the
    /// member after the last explicit segment, so `file2.fn()` works even
    /// though the parser represents it as a method call.
    fn resolve_chain(&mut self, root: &Name, segments: Vec<Name>, leaf: &str, leaf_span: Span) {
        let Some(root_symbol) = self.lookup_symbol(&root.text) else {
            self.report_unknown(&root.text, root.span);
            return;
        };
        let root_symbol = self
            .resolution
            .symbol(root_symbol)
            .expect("known root symbol")
            .clone();
        let SymbolKind::Module {
            namespace: mut current,
        } = root_symbol.kind
        else {
            // The root is a value (variable, parameter, builtin alias, or
            // named type); record it and let semantic analysis interpret the
            // remaining segments as before the refactor.
            self.resolution.references.insert(root.span, root_symbol.id);
            return;
        };
        // Record the root alias so semantic analysis can ask what a chain
        // root resolved to. Module aliases are never values, so recording
        // them cannot make an identifier use valid.
        self.resolution.references.insert(root.span, root_symbol.id);
        let mut prefix = self.resolution.namespace_path(current).to_owned();
        let mut path = segments;
        path.push(Name {
            text: leaf.to_owned(),
            span: leaf_span,
        });
        for segment in path.iter() {
            let Some(member) = self.member_of_namespace(current, &segment.text) else {
                self.diagnostics.push(Diagnostic::error(
                    format!("`{prefix}` has no member `{}`", segment.text),
                    segment.span,
                ));
                return;
            };
            let symbol = self
                .resolution
                .symbol(member)
                .expect("member symbol recorded")
                .clone();
            match &symbol.kind {
                SymbolKind::Module { namespace } => {
                    current = *namespace;
                    prefix = self.resolution.namespace_path(current).to_owned();
                    // Intermediate module segments are recorded so semantic
                    // analysis can rebuild the chain when it needs to.
                    self.resolution.references.insert(segment.span, member);
                }
                SymbolKind::NamedType { .. } => {
                    // The chain reached a named type and still has segments
                    // after it (`pack.Tag.one`, `en.State.idle`). Record the
                    // type member and let semantic analysis interpret the
                    // remaining segments as enum variant access.
                    self.resolution.references.insert(segment.span, member);
                    return;
                }
                _ => {
                    // The segment is a function, builtin, or named type.
                    if let Some(declaration) = symbol.declaration {
                        if declaration.file != segment.span.file
                            && symbol.visibility == Some(Visibility::Private)
                        {
                            self.diagnostics.push(Diagnostic::error(
                                format!(
                                    "private function `{}` is not visible outside its module",
                                    segment.text
                                ),
                                segment.span,
                            ));
                            return;
                        }
                    }
                    self.resolution.references.insert(segment.span, member);
                    return;
                }
            }
        }
    }

    /// Resolves one unqualified name: locals first, then enclosing namespace
    /// members, then the file's import scope.
    fn resolve_name(&mut self, name: &str, span: Span) {
        if let Some(symbol) = self.lookup_symbol(name) {
            self.resolution.references.insert(span, symbol);
            return;
        }
        let suggestion = self.suggest_qualified(name);
        match suggestion {
            Some(candidate) => {
                self.diagnostics.push(Diagnostic::error(
                    format!("unknown identifier `{name}`; did you mean `{candidate}`?"),
                    span,
                ));
            }
            None => {
                self.diagnostics.push(Diagnostic::error(
                    format!("unknown identifier `{name}`"),
                    span,
                ));
            }
        }
    }

    fn report_unknown(&mut self, name: &str, span: Span) {
        self.resolve_name(name, span);
    }

    fn lookup_symbol(&self, name: &str) -> Option<SymbolId> {
        for scope in self.scopes.iter().skip(1).rev() {
            if let Some(id) = scope.get(name) {
                return Some(*id);
            }
        }
        for namespace in &self.ancestors {
            if let Some(id) = self.member_of_namespace(*namespace, name) {
                return Some(id);
            }
        }
        self.scopes
            .first()
            .and_then(|scope| scope.get(name))
            .copied()
    }

    fn suggest_qualified(&self, name: &str) -> Option<String> {
        let base = self.scopes.first()?;
        for symbol in base.values() {
            if let Some(SymbolKind::Module { namespace }) = self
                .resolution
                .symbol(*symbol)
                .map(|symbol| symbol.kind.clone())
            {
                if self.resolution.namespaces[namespace]
                    .types
                    .contains_key(name)
                    || self.resolution.namespaces[namespace]
                        .members
                        .contains_key(name)
                {
                    return Some(format!(
                        "{}.{}",
                        self.resolution.namespaces[namespace].path, name
                    ));
                }
            }
        }
        None
    }

    /// A receiver identifier is a struct or enum type name when the namespace
    /// chain or an imported selection declares it; those receivers stay
    /// unresolved so semantic analysis reads the member as a struct field.
    fn receiver_is_type(&self, base: &Expression) -> bool {
        match &base.kind {
            ExpressionKind::Identifier(name) => {
                if name.text.contains('.') {
                    return self.type_names.contains_key(&name.text);
                }
                for namespace in &self.ancestors {
                    if self.resolution.namespaces[*namespace]
                        .types
                        .contains_key(&name.text)
                    {
                        return true;
                    }
                }
                self.scopes
                    .first()
                    .and_then(|scope| scope.get(&name.text))
                    .and_then(|id| self.resolution.symbol(*id))
                    .map_or(false, |symbol| {
                        matches!(symbol.kind, SymbolKind::NamedType { .. })
                    })
            }
            _ => false,
        }
    }
}

/// Collects any file id a namespace block belongs to, including nested ones.
fn collect_namespace_files(declaration: &NamespaceDeclaration, files: &mut Vec<FileId>) {
    files.push(declaration.name.span.file);
    for child in &declaration.namespaces {
        collect_namespace_files(child, files);
    }
}
