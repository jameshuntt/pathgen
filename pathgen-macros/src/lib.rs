//! pathgen-macros
//! ===============
//!
//! Declaration-only proc macros for pathgen artifact/receipt contracts.
//!
//! These macros generate metadata trait implementations. They intentionally do
//! not resolve paths, open files, write SQLite rows, sign payloads, or perform
//! any other side effect.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, DeriveInput, Expr, ExprLit, ExprPath, Lit,
    LitInt, LitStr, Path, Result, Token,
};

/// Derives [`ArtifactContract`](pathgen_core::ArtifactContract) for a type from one
/// `#[artifact(...)]` attribute.
///
/// The macro only fills in the trait's associated constants. It never resolves a
/// path, opens a file or touches a database.
///
/// # Options
///
/// | option      | required | value |
/// |-------------|----------|-------|
/// | `schema`    | yes      | string literal, the schema id (`"my.config.v1"`) |
/// | `route`     | yes      | string literal, the route id |
/// | `kind`      | yes      | string literal, the artifact kind |
/// | `version`   | yes      | integer literal, a `u32` |
/// | `canonical` | yes      | one endpoint, `canonical(file::toml)` or `canonical = file::toml` |
/// | `imports`   | no       | list of endpoints, defaults to the canonical one |
/// | `exports`   | no       | list of endpoints, defaults to the canonical one |
/// | `core`      | no       | path of the crate that exports the trait, defaults to `pathgen_core`; write `core = pathgen` when you depend on the facade |
///
/// The canonical endpoint must appear in `imports` or `exports`. It does by default.
///
/// # Endpoints
///
/// An endpoint is a path, or the same text as a string literal (`"file::yaml"`):
/// `file::json`, `file::yaml`, `file::toml`, `file::txt`, `file::jsonl`, `file::xml`,
/// `file::markdown` (alias `file::md`), `file::cbor`, `file::docx`, `file::pdf`,
/// `file::npy`, `file::npz`, `sql::sqlite(table = "name")` and
/// `chain::target(route = "route.id")`.
///
/// # Example
///
/// ```
/// use pathgen_core::{ArtifactContract as _, ArtifactEndpoint};
/// use pathgen_macros::ArtifactContract;
///
/// #[derive(ArtifactContract)]
/// #[artifact(
///     schema = "example.config.v1",
///     route = "example.config",
///     kind = "config",
///     version = 1,
///     canonical(file::toml),
///     exports(file::toml, file::json, sql::sqlite(table = "artifact_index"))
/// )]
/// struct Config;
///
/// assert_eq!(Config::ROUTE_ID, "example.config");
/// assert_eq!(Config::CANONICAL, ArtifactEndpoint::FileToml);
/// assert_eq!(Config::IMPORTS, &[ArtifactEndpoint::FileToml]);
/// ```
///
/// A missing required option, a second `#[artifact]` attribute, an unknown option
/// or an unknown endpoint is a compile error naming the problem.
#[proc_macro_derive(ArtifactContract, attributes(artifact))]
pub fn derive_artifact_contract(input: TokenStream) -> TokenStream {
    derive_contract(input, "artifact", "ArtifactContract")
}

/// Derives [`ArtifactContract`](pathgen_core::ArtifactContract) for a receipt type
/// from one `#[receipt(...)]` attribute.
///
/// Identical to [`ArtifactContract`](macro@ArtifactContract) except for the attribute
/// name, so a receipt and the artifact it describes can carry contracts on the same
/// type without the attributes clashing. It takes the same options and endpoints and
/// implements the same trait; `pathgen-core` has no separate receipt trait.
///
/// ```
/// use pathgen_core::{ArtifactContract as _, ArtifactEndpoint};
/// use pathgen_macros::ReceiptContract;
///
/// #[derive(ReceiptContract)]
/// #[receipt(
///     schema = "example.receipt.v1",
///     route = "example.receipt",
///     kind = "receipt",
///     version = 7,
///     canonical = file::json,
///     exports(file::json, file::jsonl, chain::target(route = "chain.example.receipt"))
/// )]
/// struct Receipt;
///
/// assert_eq!(Receipt::VERSION, 7);
/// assert_eq!(Receipt::CANONICAL, ArtifactEndpoint::FileJson);
/// ```
#[proc_macro_derive(ReceiptContract, attributes(receipt))]
pub fn derive_receipt_contract(input: TokenStream) -> TokenStream {
    derive_contract(input, "receipt", "ReceiptContract")
}

fn derive_contract(input: TokenStream, attr_name: &str, macro_name: &str) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_contract(&input, attr_name) {
        Ok(tokens) => tokens.into(),
        Err(err) => {
            let name = &input.ident;
            let msg = format!("{} derive failed for `{}`", macro_name, name);
            let compile_error = err.to_compile_error();
            quote! {
                compile_error!(#msg);
                #compile_error
            }
            .into()
        }
    }
}

fn expand_contract(input: &DeriveInput, attr_name: &str) -> Result<proc_macro2::TokenStream> {
    let ident = &input.ident;
    let attr = find_single_attr(&input.attrs, attr_name)?;
    let config = ContractConfig::parse(attr)?;

    let core = &config.core_path;
    let schema = &config.schema;
    let route = &config.route;
    let kind = &config.kind;
    let version = config.version;
    let canonical = config.canonical.to_tokens(core);
    let imports = config
        .imports
        .iter()
        .map(|endpoint| endpoint.to_tokens(core))
        .collect::<Vec<_>>();
    let exports = config
        .exports
        .iter()
        .map(|endpoint| endpoint.to_tokens(core))
        .collect::<Vec<_>>();

    Ok(quote! {
        impl ::#core::ArtifactContract for #ident {
            const SCHEMA: &'static str = #schema;
            const ROUTE_ID: &'static str = #route;
            const KIND: &'static str = #kind;
            const VERSION: u32 = #version;

            const CANONICAL: ::#core::ArtifactEndpoint = #canonical;
            const IMPORTS: &'static [::#core::ArtifactEndpoint] = &[#(#imports),*];
            const EXPORTS: &'static [::#core::ArtifactEndpoint] = &[#(#exports),*];
        }
    })
}

fn find_single_attr<'a>(attrs: &'a [Attribute], name: &str) -> Result<&'a Attribute> {
    let mut found = None;

    for attr in attrs {
        if attr.path().is_ident(name) {
            if found.is_some() {
                return Err(syn::Error::new(
                    attr.span(),
                    format!("expected exactly one #[{}(...)] attribute", name),
                ));
            }
            found = Some(attr);
        }
    }

    found.ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            format!("missing #[{}(...)] attribute", name),
        )
    })
}

#[derive(Debug)]
struct ContractConfig {
    core_path: Path,
    schema: LitStr,
    route: LitStr,
    kind: LitStr,
    version: u32,
    canonical: EndpointSpec,
    imports: Vec<EndpointSpec>,
    exports: Vec<EndpointSpec>,
}

impl ContractConfig {
    fn parse(attr: &Attribute) -> Result<Self> {
        let mut core_path: Option<Path> = None;
        let mut schema: Option<LitStr> = None;
        let mut route: Option<LitStr> = None;
        let mut kind: Option<LitStr> = None;
        let mut version: Option<u32> = None;
        let mut canonical: Option<EndpointSpec> = None;
        let mut imports: Option<Vec<EndpointSpec>> = None;
        let mut exports: Option<Vec<EndpointSpec>> = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("core") {
                let value = meta.value()?;
                core_path = Some(value.parse::<Path>()?);
                return Ok(());
            }

            if meta.path.is_ident("schema") {
                schema = Some(parse_lit_str_value(&meta)?);
                return Ok(());
            }

            if meta.path.is_ident("route") {
                route = Some(parse_lit_str_value(&meta)?);
                return Ok(());
            }

            if meta.path.is_ident("kind") {
                kind = Some(parse_lit_str_value(&meta)?);
                return Ok(());
            }

            if meta.path.is_ident("version") {
                let value = meta.value()?;
                let lit = value.parse::<LitInt>()?;
                version = Some(lit.base10_parse::<u32>()?);
                return Ok(());
            }

            if meta.path.is_ident("canonical") {
                canonical = Some(parse_endpoint_value_or_list(&meta)?);
                return Ok(());
            }

            if meta.path.is_ident("imports") {
                imports = Some(parse_endpoint_list(&meta)?);
                return Ok(());
            }

            if meta.path.is_ident("exports") {
                exports = Some(parse_endpoint_list(&meta)?);
                return Ok(());
            }

            Err(meta.error("unsupported artifact contract option"))
        })?;

        let schema = required(schema, attr, "schema")?;
        let route = required(route, attr, "route")?;
        let kind = required(kind, attr, "kind")?;
        let version = required(version, attr, "version")?;
        let canonical = required(canonical, attr, "canonical")?;
        let imports = imports.unwrap_or_else(|| vec![canonical.clone()]);
        let exports = exports.unwrap_or_else(|| vec![canonical.clone()]);
        let core_path = core_path.unwrap_or_else(default_core_path);

        if !endpoints_contain(&imports, &canonical) && !endpoints_contain(&exports, &canonical) {
            return Err(syn::Error::new(
                canonical.span,
                "canonical endpoint should appear in imports and/or exports",
            ));
        }

        Ok(Self {
            core_path,
            schema,
            route,
            kind,
            version,
            canonical,
            imports,
            exports,
        })
    }
}

fn endpoints_contain(endpoints: &[EndpointSpec], expected: &EndpointSpec) -> bool {
    endpoints.iter().any(|endpoint| endpoint_kind_eq(&endpoint.kind, &expected.kind))
}

fn required<T>(value: Option<T>, attr: &Attribute, name: &'static str) -> Result<T> {
    value.ok_or_else(|| syn::Error::new(attr.span(), format!("missing required `{}`", name)))
}

fn default_core_path() -> Path {
    syn::parse_quote!(pathgen_core)
}


fn endpoint_kind_eq(left: &EndpointSpecKind, right: &EndpointSpecKind) -> bool {
    match (left, right) {
        (EndpointSpecKind::FileJson, EndpointSpecKind::FileJson)
        | (EndpointSpecKind::FileYaml, EndpointSpecKind::FileYaml)
        | (EndpointSpecKind::FileToml, EndpointSpecKind::FileToml)
        | (EndpointSpecKind::FileTxt, EndpointSpecKind::FileTxt)
        | (EndpointSpecKind::FileJsonl, EndpointSpecKind::FileJsonl)
        | (EndpointSpecKind::FileXml, EndpointSpecKind::FileXml)
        | (EndpointSpecKind::FileMarkdown, EndpointSpecKind::FileMarkdown)
        | (EndpointSpecKind::FileCbor, EndpointSpecKind::FileCbor)
        | (EndpointSpecKind::FileDocx, EndpointSpecKind::FileDocx)
        | (EndpointSpecKind::FilePdf, EndpointSpecKind::FilePdf)
        | (EndpointSpecKind::FileNpy, EndpointSpecKind::FileNpy)
        | (EndpointSpecKind::FileNpz, EndpointSpecKind::FileNpz) => true,
        (EndpointSpecKind::SqliteTable { table: left }, EndpointSpecKind::SqliteTable { table: right }) => {
            left.value() == right.value()
        }
        (EndpointSpecKind::ChainTarget { route_id: left }, EndpointSpecKind::ChainTarget { route_id: right }) => {
            left.value() == right.value()
        }
        _ => false,
    }
}

fn parse_lit_str_value(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<LitStr> {
    let value = meta.value()?;
    value.parse::<LitStr>()
}

fn parse_endpoint_value_or_list(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<EndpointSpec> {
    if meta.input.peek(Token![=]) {
        let value = meta.value()?;
        let expr = value.parse::<Expr>()?;
        return endpoint_from_expr(expr);
    }

    let mut parsed: Option<EndpointSpec> = None;
    meta.parse_nested_meta(|nested| {
        if parsed.is_some() {
            return Err(nested.error("canonical(...) accepts exactly one endpoint"));
        }

        parsed = Some(endpoint_from_nested_meta(&nested)?);
        Ok(())
    })?;

    parsed.ok_or_else(|| meta.error("canonical requires one endpoint"))
}

fn parse_endpoint_list(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<Vec<EndpointSpec>> {
    let mut endpoints = Vec::new();

    meta.parse_nested_meta(|nested| {
        endpoints.push(endpoint_from_nested_meta(&nested)?);
        Ok(())
    })?;

    if endpoints.is_empty() {
        return Err(meta.error("endpoint list cannot be empty"));
    }

    Ok(endpoints)
}

#[derive(Debug, Clone)]
enum EndpointSpecKind {
    FileJson,
    FileYaml,
    FileToml,
    FileTxt,
    FileJsonl,
    FileXml,
    FileMarkdown,
    FileCbor,
    FileDocx,
    FilePdf,
    FileNpy,
    FileNpz,
    SqliteTable { table: LitStr },
    ChainTarget { route_id: LitStr },
}

#[derive(Debug, Clone)]
struct EndpointSpec {
    kind: EndpointSpecKind,
    span: Span,
}

impl EndpointSpec {
    fn to_tokens(&self, core: &Path) -> proc_macro2::TokenStream {
        match &self.kind {
            EndpointSpecKind::FileJson => quote!(::#core::ArtifactEndpoint::FileJson),
            EndpointSpecKind::FileYaml => quote!(::#core::ArtifactEndpoint::FileYaml),
            EndpointSpecKind::FileToml => quote!(::#core::ArtifactEndpoint::FileToml),
            EndpointSpecKind::FileTxt => quote!(::#core::ArtifactEndpoint::FileTxt),
            EndpointSpecKind::FileJsonl => quote!(::#core::ArtifactEndpoint::FileJsonl),
            EndpointSpecKind::FileXml => quote!(::#core::ArtifactEndpoint::FileXml),
            EndpointSpecKind::FileMarkdown => quote!(::#core::ArtifactEndpoint::FileMarkdown),
            EndpointSpecKind::FileCbor => quote!(::#core::ArtifactEndpoint::FileCbor),
            EndpointSpecKind::FileDocx => quote!(::#core::ArtifactEndpoint::FileDocx),
            EndpointSpecKind::FilePdf => quote!(::#core::ArtifactEndpoint::FilePdf),
            EndpointSpecKind::FileNpy => quote!(::#core::ArtifactEndpoint::FileNpy),
            EndpointSpecKind::FileNpz => quote!(::#core::ArtifactEndpoint::FileNpz),
            EndpointSpecKind::SqliteTable { table } => {
                quote!(::#core::ArtifactEndpoint::SqliteTable { table: #table })
            }
            EndpointSpecKind::ChainTarget { route_id } => {
                quote!(::#core::ArtifactEndpoint::ChainTarget { route_id: #route_id })
            }
        }
    }
}

fn endpoint_from_expr(expr: Expr) -> Result<EndpointSpec> {
    match expr {
        Expr::Path(ExprPath { path, .. }) => endpoint_from_path(path, None),
        Expr::Lit(ExprLit { lit: Lit::Str(label), .. }) => endpoint_from_label(&label),
        other => Err(syn::Error::new(
            other.span(),
            "endpoint must be a path like file::json or a string like \"file::json\"",
        )),
    }
}

fn endpoint_from_label(label: &LitStr) -> Result<EndpointSpec> {
    let path: Path = label.parse()?;
    endpoint_from_path(path, Some(label.span()))
}

fn endpoint_from_nested_meta(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<EndpointSpec> {
    let path = meta.path.clone();
    let path_string = path_to_string(&path);

    if path_string == "sql::sqlite" {
        let mut table = None;
        meta.parse_nested_meta(|nested| {
            if nested.path.is_ident("table") {
                table = Some(parse_lit_str_value(&nested)?);
                Ok(())
            } else {
                Err(nested.error("sql::sqlite accepts only table = \"...\""))
            }
        })?;

        let table = table.ok_or_else(|| meta.error("sql::sqlite requires table = \"...\""))?;
        return Ok(EndpointSpec {
            kind: EndpointSpecKind::SqliteTable { table },
            span: path.span(),
        });
    }

    if path_string == "chain::target" {
        let mut route_id = None;
        meta.parse_nested_meta(|nested| {
            if nested.path.is_ident("route") || nested.path.is_ident("route_id") {
                route_id = Some(parse_lit_str_value(&nested)?);
                Ok(())
            } else {
                Err(nested.error("chain::target accepts only route = \"...\""))
            }
        })?;

        let route_id = route_id.ok_or_else(|| meta.error("chain::target requires route = \"...\""))?;
        return Ok(EndpointSpec {
            kind: EndpointSpecKind::ChainTarget { route_id },
            span: path.span(),
        });
    }

    endpoint_from_path(path, None)
}

fn endpoint_from_path(path: Path, forced_span: Option<Span>) -> Result<EndpointSpec> {
    let span = forced_span.unwrap_or_else(|| path.span());
    let path_string = path_to_string(&path);

    let kind = match path_string.as_str() {
        "file::json" => EndpointSpecKind::FileJson,
        "file::yaml" => EndpointSpecKind::FileYaml,
        "file::toml" => EndpointSpecKind::FileToml,
        "file::txt" => EndpointSpecKind::FileTxt,
        "file::jsonl" => EndpointSpecKind::FileJsonl,
        "file::xml" => EndpointSpecKind::FileXml,
        "file::markdown" | "file::md" => EndpointSpecKind::FileMarkdown,
        "file::cbor" => EndpointSpecKind::FileCbor,
        "file::docx" => EndpointSpecKind::FileDocx,
        "file::pdf" => EndpointSpecKind::FilePdf,
        "file::npy" => EndpointSpecKind::FileNpy,
        "file::npz" => EndpointSpecKind::FileNpz,
        "sql::sqlite" => {
            return Err(syn::Error::new(span, "sql::sqlite requires table = \"...\""));
        }
        "chain::target" => {
            return Err(syn::Error::new(span, "chain::target requires route = \"...\""));
        }
        _ => {
            return Err(syn::Error::new(
                span,
                format!("unsupported endpoint `{}`", path_string),
            ));
        }
    };

    Ok(EndpointSpec { kind, span })
}

fn path_to_string(path: &Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}
