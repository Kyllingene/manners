use roff::{bold, italic, line_break, roman, Inline, Roff};
use rustdoc_types::{
    Abi, Crate, Enum, Header, Id, Impl, Item, ItemEnum, Module, Struct, StructKind, Trait, Type,
    Union, Variant, VariantKind, MacroKind,
};

use crate::markdown;

mod generics;
use generics::*;

fn get<'a>(cr: &'a Crate, id: &Id) -> &'a Item {
    cr.index
        .get(id)
        .unwrap_or_else(|| panic!("invalid ID: {}", id.0))
}

fn render_links(cr: &Crate, item: &Item, page: &mut Roff) {
    let paths: Vec<_> = item
        .links
        // .values()
        .iter()
        .flat_map(|(_, id)| {
            Some([
                cr.paths.get(id).map(|i| italic(i.path.join("::")))?,
                roman(", "),
            ])
        })
        .flatten()
        .collect();

    if !paths.is_empty() {
        page.control("SH", ["SEE ALSO"]);
        page.text(&paths[..paths.len() - 1]);
    }
}

fn render_items(cr: &Crate, items: &[Id], page: &mut Roff, max_width: Option<usize>) {
    render_item_kinds! {
        cr, items, page, max_width;
        "MODULES": mod Module;
        "UNIONS": union Union;
        "STRUCTS": struct Struct;
        "ENUMS": enum Enum;
        "FNS": fn Function;
        "TRAITS": trait Trait;
        "TRAIT ALIASES": trait TraitAlias;
        "TYPE ALIASES": type TypeAlias;
        "CONSTANTS": const Constant;
        "STATICS": static Static;
        "MACROS": macro Macro;
        "PROC MACROS": macro ProcMacro;
        "PRIMITIVES": primitive Primitive;
    }
}

fn render_fields(cr: &Crate, kind: &StructKind, page: &mut Vec<Inline>) {
    let mut depth = 0;
    match kind {
        StructKind::Unit => {}
        StructKind::Tuple(fields) => {
            let sep = if fields.len() < 3 {
                |page: &mut Vec<Inline>| page.push(roman(", "))
            } else {
                |page: &mut Vec<Inline>| {
                    page.push(roman(","));
                    page.push(line_break());
                    page.push(roman("  "));
                }
            };

            page.push(roman("("));
            if fields.len() > 2 {
                page.push(line_break());
                page.push(roman("  "));
                depth = 1;
            }

            let mut first = true;
            let mut private = false;
            for field in fields {
                if !first {
                    sep(page);
                }

                let Some(field) = field else {
                    private = true;
                    continue;
                };

                first = false;

                let Item {
                    inner: ItemEnum::StructField(typ),
                    ..
                } = get(cr, field)
                else {
                    unreachable!()
                };

                render_type(cr, typ, depth, page);
            }

            if private {
                if !first {
                    sep(page);
                }
                page.push(roman("/* private fields */"));
            }

            if fields.len() > 2 {
                page.push(line_break());
            }
            page.push(roman(")"));
        }
        StructKind::Plain {
            fields,
            fields_stripped,
        } => {
            let sep = |page: &mut Vec<Inline>| {
                page.push(roman(","));
                page.push(line_break());
                page.push(roman("  "));
            };

            page.push(roman(" {"));
            page.push(line_break());
            page.push(roman("  "));

            let mut first = true;
            for field in fields {
                if !first {
                    sep(page);
                }

                let Item {
                    name: Some(name),
                    docs,
                    inner: ItemEnum::StructField(typ),
                    ..
                } = get(cr, field)
                else {
                    panic!("invalid struct field");
                };

                if let Some(docs) = docs {
                    if !first {
                        page.push(line_break());
                        page.push(roman("    "));
                    } else {
                        page.push(roman("  "));
                    }
                    page.append(&mut markdown::to_roff(docs, 2));
                    page.push(roman("  "));
                }

                first = false;

                page.push(roman(name));
                page.push(roman(": "));
                render_type(cr, typ, 1, page);
            }

            if *fields_stripped {
                if !fields.is_empty() {
                    sep(page);
                }
                page.push(roman("/* private fields */"));
            }

            page.push(line_break());
            page.push(roman("}"));
        }
    }
}

fn render_type(cr: &Crate, ty: &Type, mut depth: usize, page: &mut Vec<Inline>) {
    match ty {
        Type::ResolvedPath(path) => {
            if let Some(args) = &path.args {
                render_generics_args(cr, &path.name, args, depth, page);
            } else {
                page.push(bold(&path.name));
            }
        }
        Type::DynTrait(obj) => {
            page.push(roman("dyn "));
            let mut first = true;
            for tr in &obj.traits {
                if !first {
                    page.push(roman(" + "));
                }
                first = false;

                if !tr.generic_params.is_empty() {
                    render_generics(cr, "for", &tr.generic_params, depth, page);
                    page.push(roman(" "));
                }

                if let Some(args) = &tr.trait_.args {
                    render_generics_args(cr, &tr.trait_.name, args, depth, page);
                } else {
                    page.push(bold(&tr.trait_.name));
                }
            }

            if let Some(lt) = &obj.lifetime {
                page.push(roman(" + "));
                page.push(roman(lt));
            }
        }
        Type::Generic(s) | Type::Primitive(s) => page.push(bold(s)),
        Type::FunctionPointer(func) => {
            if !func.generic_params.is_empty() {
                render_generics(cr, "for", &func.generic_params, depth, page);
                page.push(roman(" "));
            }

            let Header {
                const_,
                unsafe_,
                async_,
                ..
            } = &func.header;
            if *const_ {
                page.push(roman("const "));
            }
            if *unsafe_ {
                page.push(roman("unsafe "));
            }
            if *async_ {
                page.push(roman("async "));
            }

            if let Some(abi) = match &func.header.abi {
                Abi::Rust => None,
                Abi::C { .. } => Some("\"C\""),
                Abi::Cdecl { .. } => Some("\"cdecl\""),
                Abi::Stdcall { .. } => Some("\"stdcall\""),
                Abi::Fastcall { .. } => Some("\"fastcall\""),
                Abi::Aapcs { .. } => Some("\"aapcs\""),
                Abi::Win64 { .. } => Some("\"win64\""),
                Abi::SysV64 { .. } => Some("\"sysv64\""),
                Abi::System { .. } => Some("\"system\""),
                Abi::Other(s) => Some(&**s),
            } {
                page.push(roman(abi));
                page.push(roman(" "));
            }

            let inputs = &func.decl.inputs;

            page.push(roman("fn("));
            if inputs.len() >= 3 {
                depth += 1;
                page.push(line_break());
                page.push(roman("  ".repeat(depth)));
            }

            let sep = if inputs.len() < 3 {
                |page: &mut Vec<Inline>, _| page.push(roman(", "))
            } else {
                |page: &mut Vec<Inline>, depth| {
                    page.push(roman(","));
                    page.push(line_break());
                    page.push(roman("  ".repeat(depth)));
                }
            };

            let mut first = true;
            for (_, arg) in inputs {
                if !first {
                    sep(page, depth + 1);
                }
                first = false;
                render_type(cr, arg, depth + 1, page);
            }

            if func.decl.c_variadic {
                page.push(roman("..."));
            }

            if inputs.len() < 3 {
                page.push(roman(")"));
            } else {
                page.push(line_break());
                page.push(roman(")"));
                depth -= 1;
            }

            if let Some(output) = &func.decl.output {
                page.push(roman(" -> "));
                render_type(cr, output, depth + 1, page);
            }
        }
        Type::Tuple(types) => {
            page.push(roman("("));
            if types.len() >= 3 {
                depth += 1;
                page.push(line_break());
                page.push(roman("  ".repeat(depth)));
            }

            let sep = if types.len() < 3 {
                |page: &mut Vec<Inline>, _| page.push(roman(", "))
            } else {
                |page: &mut Vec<Inline>, depth| {
                    page.push(roman(","));
                    page.push(line_break());
                    page.push(roman("  ".repeat(depth)));
                }
            };

            let mut first = true;
            for arg in types {
                if !first {
                    sep(page, depth + 1);
                }
                first = false;
                render_type(cr, arg, depth + 1, page);
            }

            if types.len() < 3 {
                page.push(roman(")"));
            } else {
                page.push(line_break());
                page.push(roman(")"));
            }
        }
        Type::Slice(typ) => {
            page.push(roman("["));
            render_type(cr, typ, depth, page);
            page.push(roman("]"));
        }
        Type::Array { type_, len } => {
            page.push(roman("["));
            render_type(cr, type_, depth, page);
            page.push(roman(format!("; {len}]")));
        }
        Type::Pat { .. } => eprintln!("pattern types aren't supported (yet)"),
        Type::ImplTrait(traits) => {
            page.push(roman("impl "));
            render_generics_bounds(cr, traits, depth, page);
        }
        Type::Infer => page.push(roman("_")),
        Type::RawPointer { mutable, type_ } => {
            page.push(roman(if *mutable { "*mut " } else { "*const " }));
            render_type(cr, type_, depth, page);
        }
        Type::BorrowedRef {
            lifetime,
            mutable,
            type_,
        } => {
            page.push(roman("&"));
            if let Some(lt) = lifetime.as_ref() {
                page.push(roman(lt));
                page.push(roman(" "));
            }
            if *mutable {
                page.push(roman("mut "));
            }
            render_type(cr, type_, depth, page);
        }
        // TODO: what is args?
        Type::QualifiedPath {
            name,
            args: _,
            self_type,
            trait_,
        } => {
            if let Some(tr) = trait_ {
                page.push(roman("<"));
                render_type(cr, self_type, depth, page);
                page.push(roman(" as "));
                if let Some(args) = &tr.args {
                    render_generics_args(cr, &tr.name, args, depth, page);
                } else {
                    page.push(bold(&tr.name));
                }
                page.push(roman(">::"));
                page.push(italic(name));
            } else {
                render_type(cr, self_type, depth, page);
                page.push(roman("::"));
                page.push(italic(name));
            }
        }
    }
}

fn render_fn(cr: &Crate, id: &Id, mut depth: usize, page: &mut Vec<Inline>) {
    let Item {
        inner: ItemEnum::Function(func),
        name,
        ..
    } = get(cr, id)
    else {
        unreachable!()
    };

    let Header {
        const_,
        unsafe_,
        async_,
        ..
    } = &func.header;
    if *const_ {
        page.push(roman("const "));
    }
    if *unsafe_ {
        page.push(roman("unsafe "));
    }
    if *async_ {
        page.push(roman("async "));
    }

    if let Some(abi) = match &func.header.abi {
        Abi::Rust => None,
        Abi::C { .. } => Some("\"C\""),
        Abi::Cdecl { .. } => Some("\"cdecl\""),
        Abi::Stdcall { .. } => Some("\"stdcall\""),
        Abi::Fastcall { .. } => Some("\"fastcall\""),
        Abi::Aapcs { .. } => Some("\"aapcs\""),
        Abi::Win64 { .. } => Some("\"win64\""),
        Abi::SysV64 { .. } => Some("\"sysv64\""),
        Abi::System { .. } => Some("\"system\""),
        Abi::Other(s) => Some(&**s),
    } {
        page.push(roman(abi));
        page.push(roman(" "));
    }

    let inputs = &func.decl.inputs;

    page.push(roman("fn "));
    render_generics(
        cr,
        name.as_ref().unwrap(),
        &func.generics.params,
        depth,
        page,
    );
    page.push(roman("("));
    if inputs.len() >= 3 {
        depth += 1;
        page.push(line_break());
        page.push(roman("  ".repeat(depth)));
    }

    let sep = if inputs.len() < 3 {
        |page: &mut Vec<Inline>, _| page.push(roman(", "))
    } else {
        |page: &mut Vec<Inline>, depth| {
            page.push(roman(","));
            page.push(line_break());
            page.push(roman("  ".repeat(depth)));
        }
    };

    let mut first = true;
    for (name, arg) in inputs {
        if !first {
            sep(page, depth + 1);
        }
        first = false;

        page.push(roman(name));
        page.push(roman(": "));
        render_type(cr, arg, depth + 1, page);
    }

    if func.decl.c_variadic {
        page.push(roman("..."));
    }

    if inputs.len() < 3 {
        page.push(roman(")"));
    } else {
        page.push(line_break());
        page.push(roman(")"));
        depth -= 1;
    }

    if let Some(output) = &func.decl.output {
        page.push(roman(" -> "));
        render_type(cr, output, depth + 1, page);
    }

    page.push(line_break());
    page.push(roman("  ".repeat(depth)));
    render_where(cr, &func.generics.where_predicates, depth + 1, page);
}

fn render_impls(cr: &Crate, impls: &[Id], page: &mut Roff) {
    let mut buf = Vec::new();

    let mut first = true;
    for id in impls {
        let Item {
            inner: ItemEnum::Impl(imp),
            ..
        } = get(cr, id)
        else {
            unreachable!()
        };
        if !imp.synthetic && imp.blanket_impl.is_none() {
            if first {
                page.text(buf.clone());
                buf.clear();
                page.control("SH", ["IMPLS"]);
                first = false;
            } else {
                buf.extend_from_slice(&[
                    line_break(),
                    bold("|=========|"),
                    line_break(),
                    line_break(),
                ]);
            }

            render_impl(cr, imp, true, &mut buf);
        }
    }

    first = true;
    for id in impls {
        let Item {
            inner: ItemEnum::Impl(imp),
            ..
        } = get(cr, id)
        else {
            unreachable!()
        };
        if imp.blanket_impl.is_some() {
            if first {
                page.text(buf.clone());
                buf.clear();
                page.control("SH", ["BLANKET IMPLS"]);
                first = false;
            } else {
                buf.extend_from_slice(&[
                    line_break(),
                    bold("|=========|"),
                    line_break(),
                    line_break(),
                ]);
            }

            render_impl(cr, imp, false, &mut buf);
        }
    }

    first = true;
    for id in impls {
        let Item {
            inner: ItemEnum::Impl(imp),
            ..
        } = get(cr, id)
        else {
            unreachable!()
        };
        if imp.blanket_impl.is_none() && imp.synthetic {
            if first {
                page.text(buf.clone());
                buf.clear();
                page.control("SH", ["AUTO TRAIT IMPLS"]);
                first = false;
            } else {
                buf.extend_from_slice(&[
                    line_break(),
                    bold("|=========|"),
                    line_break(),
                    line_break(),
                ]);
            }

            render_impl(cr, imp, false, &mut buf);
        }
    }

    page.text(buf);
}

fn render_impl(cr: &Crate, imp: &Impl, render_items: bool, page: &mut Vec<Inline>) {
    if imp.is_unsafe {
        page.push(roman("unsafe "));
    }

    page.push(roman("impl"));
    render_generics(cr, "", &imp.generics.params, 0, page);

    page.push(roman(" "));

    if let Some(tr) = &imp.trait_ {
        if imp.negative {
            page.push(roman("!"));
        }

        let path = if imp.synthetic {
            tr.name.to_string()
        } else {
            cr.paths
                .get(&tr.id)
                .map(|tr| tr.path.clone())
                .unwrap_or_else(|| {
                    eprintln!("failed to find trait {}", tr.name);
                    vec![tr.name.to_string()]
                })
                .join("::")
        };
        if let Some(args) = &tr.args {
            render_generics_args(cr, &path, args, 0, page);
        } else {
            page.push(bold(path));
        }
        page.push(roman(" for "));
    }

    render_type(cr, &imp.for_, 0, page);
    page.push(line_break());

    render_where(cr, &imp.generics.where_predicates, 0, page);
    page.push(line_break());

    if !render_items {
        return;
    }

    for id in &imp.items {
        let item = get(cr, id);

        page.push(line_break());
        page.push(italic("  +-----+"));
        page.push(line_break());
        page.push(line_break());

        if let Some(docs) = &item.docs {
            page.push(roman("    "));
            page.append(&mut markdown::to_roff(docs, 2));
        }

        match &item.inner {
            ItemEnum::Function(_) => {
                page.push(roman("  "));
                render_fn(cr, id, 1, page);
            }
            ItemEnum::AssocConst { type_, default } => {
                page.push(roman("  const "));
                page.push(roman(item.name.as_ref().unwrap()));
                page.push(roman(": "));
                render_type(cr, type_, 1, page);

                if let Some(default) = default {
                    page.push(roman(" = "));
                    page.push(roman(default));
                }
            }
            ItemEnum::AssocType {
                generics,
                bounds,
                default,
            } => {
                page.push(roman("  type "));
                render_generics(cr, item.name.as_ref().unwrap(), &generics.params, 0, page);

                if !bounds.is_empty() {
                    page.push(roman(": "));
                    render_generics_bounds(cr, bounds, 0, page);
                }

                if let Some(default) = default {
                    page.push(roman(" = "));
                    render_type(cr, default, 1, page);
                }

                page.push(line_break());
                render_where(cr, &generics.where_predicates, 0, page);
            }
            _ => panic!("unhandled item: {item:#?}"),
        }

        page.push(line_break());
        page.push(line_break());
    }

    page.push(line_break());
}

fn render_variant(cr: &Crate, variant: &Variant, page: &mut Vec<Inline>) {
    match &variant.kind {
        VariantKind::Plain => {}
        VariantKind::Tuple(fields) => {
            page.push(roman("("));
            let mut depth = 3;
            let sep = if fields.len() < 3 {
                |page: &mut Vec<Inline>| page.push(roman(", "))
            } else {
                page.push(line_break());
                page.push(roman("    "));
                depth += 1;

                |page: &mut Vec<Inline>| {
                    page.push(roman(","));
                    page.push(line_break());
                    page.push(roman("    "));
                }
            };

            let mut first = true;
            for item in fields {
                if !first {
                    sep(page);
                }
                first = false;

                let Some(id) = item else {
                    page.push(roman("/* hidden */"));
                    continue;
                };

                let Item {
                    inner: ItemEnum::StructField(ty),
                    ..
                } = get(cr, id)
                else {
                    panic!("invalid variant type");
                };

                render_type(cr, ty, depth, page);
            }

            if fields.len() < 3 {
                page.push(roman(")"));
            } else {
                page.push(line_break());
                page.push(roman("  )"));
            }
        }
        VariantKind::Struct {
            fields,
            fields_stripped,
        } => {
            page.push(roman(" {"));
            page.push(line_break());
            page.push(roman("    "));

            let sep = |page: &mut Vec<Inline>| {
                page.push(roman(","));
                page.push(line_break());
                page.push(roman("    "));
            };

            let mut first = true;
            for id in fields {
                if !first {
                    sep(page);
                }

                let Item {
                    name,
                    docs,
                    inner: ItemEnum::StructField(ty),
                    ..
                } = get(cr, id)
                else {
                    panic!("invalid variant type");
                };

                if let Some(docs) = docs {
                    if !first {
                        page.push(line_break());
                        page.push(roman("      "));
                    } else {
                        page.push(roman("  "));
                    }
                    page.append(&mut markdown::to_roff(docs, 3));
                    page.push(roman("    "));
                }

                first = false;

                page.push(roman(name.as_ref().unwrap()));
                page.push(roman(": "));
                render_type(cr, ty, 4, page);
            }

            if *fields_stripped {
                if !first {
                    sep(page);
                }
                page.push(roman("/* hidden fields */"));
            }

            page.push(line_break());
            page.push(roman("  }"));
        }
    }

    if let Some(disc) = &variant.discriminant {
        page.push(roman(" = "));
        page.push(roman(&*disc.expr));
    }
}

fn render_variants(cr: &Crate, variants: &[Id], stripped: bool, page: &mut Vec<Inline>) {
    page.push(roman(" {"));
    page.push(line_break());
    page.push(roman("  "));

    let sep = |page: &mut Vec<Inline>| {
        page.extend_from_slice(&[roman(","), line_break(), line_break(), roman("  ")])
    };

    let mut first = true;
    for id in variants {
        let item = get(cr, id);
        let Item {
            name,
            docs,
            inner: ItemEnum::Variant(var),
            ..
        } = item
        else {
            panic!("invalid variant")
        };

        if !first {
            sep(page);
        }
        first = false;

        if let Some(docs) = docs {
            page.push(roman("  "));
            page.append(&mut markdown::to_roff(docs, 2));
            page.push(roman("  "));
        }

        page.push(italic("variant "));
        page.push(bold(name.as_ref().unwrap()));
        render_variant(cr, var, page);
    }

    if stripped {
        if !first {
            sep(page);
        }
        page.push(roman("/* hidden variants */"));
    }

    page.push(line_break());
    page.push(roman("}"));
}

fn r#enum(cr: &Crate, id: &Id, page: &mut Roff) {
    let en = get(cr, id);
    let ItemEnum::Enum(Enum {
        generics,
        variants_stripped,
        variants,
        impls,
    }) = &en.inner
    else {
        panic!("expected struct")
    };

    let name = en.name.as_ref().unwrap();
    page.control("SH", ["SIGNATURE"]);

    let mut buf = Vec::new();

    for attr in &en.attrs {
        if attr.starts_with("#[repr") {
            buf.push(roman(attr));
            buf.push(line_break());
        }
    }

    render_generics(cr, name, &generics.params, 0, &mut buf);
    render_where(cr, &generics.where_predicates, 0, &mut buf);
    render_variants(cr, variants, *variants_stripped, &mut buf);
    page.text(&buf[..]);

    if let Some(docs) = &en.docs {
        if let Some((synopsis, rest)) = docs.split_once("\n\n") {
            page.control("SH", ["SYNOPSIS"]);
            page.text(markdown::to_roff(synopsis, 0));
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(rest, 0));
        } else {
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(docs, 0));
        }
    }

    render_impls(cr, impls, page);
}

fn module(cr: &Crate, id: &Id, max_width: usize, page: &mut Roff) {
    let module = get(cr, id);
    let ItemEnum::Module(Module { items, .. }) = &module.inner else {
        panic!("expected module")
    };

    let name = module.name.as_ref().unwrap();
    page.control("SH", ["NAME"]);
    page.text([roman(name)]);

    if let Some(docs) = &module.docs {
        if let Some((synopsis, rest)) = docs.split_once("\n\n") {
            page.control("SH", ["SYNOPSIS"]);
            page.text(markdown::to_roff(synopsis, 0));
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(rest, 0));
        } else {
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(docs, 0));
        }
    }

    render_items(cr, items, page, Some(max_width));
}

fn trate(cr: &Crate, id: &Id, page: &mut Roff) {
    let tr = get(cr, id);
    let ItemEnum::Trait(Trait {
        is_auto,
        is_unsafe,
        is_object_safe,
        items,
        generics,
        bounds,
        implementations,
    }) = &tr.inner
    else {
        panic!("expected struct")
    };

    let name = tr.name.as_ref().unwrap();
    page.control("SH", ["SIGNATURE"]);

    let mut buf = Vec::new();

    if *is_unsafe {
        buf.push(roman("unsafe "));
    }

    if *is_auto {
        buf.push(roman("auto "));
    }

    buf.push(roman("trait "));
    render_generics(cr, name, &generics.params, 0, &mut buf);
    if !bounds.is_empty() {
        buf.push(roman(": "));
    }
    render_generics_bounds(cr, bounds, 0, &mut buf);
    render_where(cr, &generics.where_predicates, 0, &mut buf);
    page.text(&buf[..]);
    render_items(cr, items, page, None);

    page.control("SH", ["OBJECT SAFETY"]);
    if *is_object_safe {
        page.text([roman("This trait is object-safe.")]);
    } else {
        page.text([roman("This trait is "), bold("not"), roman(" object-safe.")]);
    }

    if let Some(docs) = &tr.docs {
        if let Some((synopsis, rest)) = docs.split_once("\n\n") {
            page.control("SH", ["SYNOPSIS"]);
            page.text(markdown::to_roff(synopsis, 0));
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(rest, 0));
        } else {
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(docs, 0));
        }
    }

    if !implementations.is_empty() {
        page.control("SH", ["IMPLEMENTORS"]);

        for id in implementations {
            let Item {
                inner: ItemEnum::Impl(imp),
                ..
            } = get(cr, id)
            else {
                panic!("invalid impl")
            };

            let mut buf = Vec::new();
            render_impl(cr, imp, false, &mut buf);
            page.text(buf);
        }
    }
}

fn strukt(cr: &Crate, id: &Id, page: &mut Roff) {
    let strukt = get(cr, id);
    let ItemEnum::Struct(Struct {
        kind,
        generics,
        impls,
    }) = &strukt.inner
    else {
        panic!("expected struct")
    };

    let name = strukt.name.as_ref().unwrap();
    page.control("SH", ["SIGNATURE"]);

    let mut buf = Vec::new();

    for attr in &strukt.attrs {
        if attr.starts_with("#[repr") {
            buf.push(roman(attr));
            buf.push(line_break());
        }
    }

    render_generics(cr, name, &generics.params, 0, &mut buf);
    render_where(cr, &generics.where_predicates, 0, &mut buf);
    render_fields(cr, kind, &mut buf);
    page.text(&buf[..]);

    if let Some(docs) = &strukt.docs {
        if let Some((synopsis, rest)) = docs.split_once("\n\n") {
            page.control("SH", ["SYNOPSIS"]);
            page.text(markdown::to_roff(synopsis, 0));
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(rest, 0));
        } else {
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(docs, 0));
        }
    }

    render_impls(cr, impls, page);
}

fn onion(cr: &Crate, id: &Id, page: &mut Roff) {
    let onion = get(cr, id);
    let ItemEnum::Union(Union {
        generics,
        fields_stripped,
        fields,
        impls,
    }) = &onion.inner
    else {
        panic!("expected struct")
    };

    let name = onion.name.as_ref().unwrap();
    page.control("SH", ["SIGNATURE"]);

    let mut buf = Vec::new();

    for attr in &onion.attrs {
        if attr.starts_with("#[repr") {
            buf.push(roman(attr));
            buf.push(line_break());
        }
    }

    render_generics(cr, name, &generics.params, 0, &mut buf);
    render_where(cr, &generics.where_predicates, 0, &mut buf);

    buf.push(roman(" {"));
    buf.push(line_break());
    buf.push(roman("    "));

    let sep = |buf: &mut Vec<Inline>| {
        buf.push(roman(","));
        buf.push(line_break());
        buf.push(roman("    "));
    };

    let mut first = true;
    for id in fields {
        if !first {
            sep(&mut buf);
        }

        let Item {
            name,
            docs,
            inner: ItemEnum::StructField(ty),
            ..
        } = get(cr, id)
        else {
            panic!("invalid variant type");
        };

        if let Some(docs) = docs {
            if !first {
                buf.push(line_break());
                buf.push(roman("      "));
            } else {
                buf.push(roman("  "));
            }
            buf.append(&mut markdown::to_roff(docs, 3));
            buf.push(roman("    "));
        }

        first = false;

        buf.push(roman(name.as_ref().unwrap()));
        buf.push(roman(": "));
        render_type(cr, ty, 4, &mut buf);
    }

    if *fields_stripped {
        if !first {
            sep(&mut buf);
        }
        buf.push(roman("/* hidden fields */"));
    }

    buf.push(line_break());
    buf.push(roman("  }"));

    page.text(&buf[..]);

    if let Some(docs) = &onion.docs {
        if let Some((synopsis, rest)) = docs.split_once("\n\n") {
            page.control("SH", ["SYNOPSIS"]);
            page.text(markdown::to_roff(synopsis, 0));
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(rest, 0));
        } else {
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(docs, 0));
        }
    }

    render_impls(cr, impls, page);
}

fn function(cr: &Crate, id: &Id, page: &mut Roff) {
    let item = get(cr, id);
    let ItemEnum::Function(_) = &item.inner else {
        panic!("expected function")
    };

    page.control("SH", ["SIGNATURE"]);
    let mut buf = Vec::new();
    render_fn(cr, id, 0, &mut buf);
    page.text(buf);

    if let Some(docs) = &item.docs {
        if let Some((synopsis, rest)) = docs.split_once("\n\n") {
            page.control("SH", ["SYNOPSIS"]);
            page.text(markdown::to_roff(synopsis, 0));
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(rest, 0));
        } else {
            page.control("SH", ["DESCRIPTION"]);
            page.text(markdown::to_roff(docs, 0));
        }
    }
}

pub fn gen(cr: &Crate, id: &Id, max_width: usize) -> Option<(String, Roff)> {
    let item = get(cr, id);
    if matches!(item.inner, ItemEnum::Import(_)) {
        return None;
    }

    let mut path = cr
        .paths
        .get(id)
        .map(|i| i.path.join("::"))
        .unwrap_or_else(|| {
            eprintln!("invalid ID: {}\n{:#?}", id.0, get(cr, id));
            item.name.clone().unwrap()
        });

    let mut page = Roff::new();
    page.control("TH", [&path, "3r"]);

    if let Some(dep) = &item.deprecation {
        page.control("SH", ["DEPRECATED"]);
        if let Some(since) = &dep.since {
            page.text([italic("since "), italic(since), line_break()]);
        }
        if let Some(note) = &dep.note {
            page.text([roman(note), line_break()]);
        }
    }

    let prefix = match &item.inner {
        ItemEnum::Module(_) => {
            module(cr, id, max_width, &mut page);
            "mod"
        }
        ItemEnum::Union(_) => {
            onion(cr, id, &mut page);
            "union"
        }
        ItemEnum::Struct(_) => {
            strukt(cr, id, &mut page);
            "struct"
        }
        ItemEnum::Enum(_) => {
            r#enum(cr, id, &mut page);
            "enum"
        }
        ItemEnum::Function(_) => {
            function(cr, id, &mut page);
            "fn"
        }
        ItemEnum::Macro(_) => {
            page.control("SH", ["NAME"]);
            page.text([roman("macro "), bold(item.name.as_ref().unwrap())]);

            if let Some(docs) = &item.docs {
                if let Some((synopsis, rest)) = docs.split_once("\n\n") {
                    page.control("SH", ["SYNOPSIS"]);
                    page.text(markdown::to_roff(synopsis, 0));
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(rest, 0));
                } else {
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(docs, 0));
                }
            }

            "macro"
        }
        ItemEnum::Trait(_) => {
            trate(cr, id, &mut page);
            "trait"
        }
        ItemEnum::Primitive(pr) => {
            page.control("SH", ["NAME"]);
            page.text([roman("primitive "), bold(&pr.name)]);

            if let Some(docs) = &item.docs {
                if let Some((synopsis, rest)) = docs.split_once("\n\n") {
                    page.control("SH", ["SYNOPSIS"]);
                    page.text(markdown::to_roff(synopsis, 0));
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(rest, 0));
                } else {
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(docs, 0));
                }
            }

            render_impls(cr, &pr.impls, &mut page);

            "primitive"
        }
        ItemEnum::TypeAlias(alias) => {
            page.control("SH", ["SIGNATURE"]);

            let mut buf = vec![roman("type ")];
            render_generics(cr, item.name.as_ref().unwrap(), &alias.generics.params, 0, &mut buf);
            buf.push(roman(" = "));
            render_type(cr, &alias.type_, 0, &mut buf);

            if !alias.generics.where_predicates.is_empty() {
                buf.push(line_break());
                render_where(cr, &alias.generics.where_predicates, 0, &mut buf);
            }

            page.text(buf);

            if let Some(docs) = &item.docs {
                if let Some((synopsis, rest)) = docs.split_once("\n\n") {
                    page.control("SH", ["SYNOPSIS"]);
                    page.text(markdown::to_roff(synopsis, 0));
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(rest, 0));
                } else {
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(docs, 0));
                }
            }

            "type"
        }
        ItemEnum::Constant(co) => {
            page.control("SH", ["SIGNATURE"]);

            let mut buf = vec![roman("const "), bold(item.name.as_ref().unwrap()), roman(": ")];
            render_type(cr, &co.type_, 0, &mut buf);
            page.text(buf);

            if let Some(docs) = &item.docs {
                if let Some((synopsis, rest)) = docs.split_once("\n\n") {
                    page.control("SH", ["SYNOPSIS"]);
                    page.text(markdown::to_roff(synopsis, 0));
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(rest, 0));
                } else {
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(docs, 0));
                }
            }

            "const"
        }
        ItemEnum::Static(st) => {
            page.control("SH", ["SIGNATURE"]);

            let mut buf = vec![if st.mutable {
                roman("static mut ")
            } else {
                roman("static ")
            }, bold(item.name.as_ref().unwrap()), roman(": ")];
            render_type(cr, &st.type_, 0, &mut buf);
            page.text(buf);

            if let Some(docs) = &item.docs {
                if let Some((synopsis, rest)) = docs.split_once("\n\n") {
                    page.control("SH", ["SYNOPSIS"]);
                    page.text(markdown::to_roff(synopsis, 0));
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(rest, 0));
                } else {
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(docs, 0));
                }
            }

            "static"
        }
        ItemEnum::ProcMacro(mac) => {
            page.control("SH", ["SIGNATURE"]);
            let name = item.name.as_ref().unwrap();
            match mac.kind {
                MacroKind::Bang => page.text([roman("proc macro "), bold(name)]),
                MacroKind::Attr => page.text([roman("#["), bold(name), roman("]")]),
                MacroKind::Derive => page.text([roman("#[derive("), bold(name), roman("]")]),
            };

            if !mac.helpers.is_empty() {
                page.control("SH", ["ATTRS"]);
                for helper in &mac.helpers {
                    page.text([roman("#["), italic(helper), roman("]")]);
                }
            }

            if let Some(docs) = &item.docs {
                if let Some((synopsis, rest)) = docs.split_once("\n\n") {
                    page.control("SH", ["SYNOPSIS"]);
                    page.text(markdown::to_roff(synopsis, 0));
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(rest, 0));
                } else {
                    page.control("SH", ["DESCRIPTION"]);
                    page.text(markdown::to_roff(docs, 0));
                }
            }

            "macro"
        }

        _ => panic!("failed to catch {item:#?}"),
    };

    render_links(cr, item, &mut page);

    path.insert(0, '.');
    path.insert_str(0, prefix);
    Some((path, page))
}

fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        s.len()
    } else {
        let lower_bound = index.saturating_sub(3);
        let mut new_index = 0;
        for i in lower_bound..=index {
            if s.is_char_boundary(i) {
                new_index = i;
            }
        }

        new_index
    }
}

macro_rules! render_item_kinds {
    (
        $cr:expr, $items:expr, $page:expr, $max_width:expr;
        $( $header:literal : $name:ident $kind:ident );+
    $(;)? ) => {$(
        let mut first = true;
        for id in $items {
            let item = get($cr, id);
            #[allow(unused)]
            if let ItemEnum::$kind(inner) = &item.inner {
                if first {
                    $page.control("SH", [$header]);
                }
                first = false;

                let path = $cr.paths.get(id)
                    .map(|i| i.path.join("::"))
                    .unwrap_or_else(|| {
                        eprintln!(
                            concat!(
                                "no path for ",
                                stringify!($name),
                                " {}",
                            ),
                            item.name.as_ref().unwrap(),
                        );
                        item.name.clone().unwrap()
                    });

                if $max_width.is_some() {
                    $page.text([
                        roman(concat!(stringify!($name), " ")),
                        italic(&path),
                    ]);
                }

                if let Some(docs) = &item.docs {
                    if let Some(max_width) = $max_width {
                        let synopsis = docs.split_once("\n\n")
                            .or_else(|| docs.split_once("\n"))
                            .map(|s| s.0)
                            .unwrap_or(docs);

                        let width = stringify!($name).len() + path.len() + 5;
                        let remaining = max_width - width;

                        let end = floor_char_boundary(synopsis, if synopsis.len() >= remaining {
                            remaining.saturating_sub(3)
                        } else {
                            synopsis.len()
                        });

                        $page.text([
                            bold("// "),
                            roman(&synopsis[..end]), // TODO: parse this markdown
                            roman(if synopsis.len() >= remaining {
                                "..."
                            } else {
                                ""
                            })
                        ]);
                    } else {
                        let mut buf = markdown::to_roff(docs, 1);
                        buf.insert(0, roman("  "));
                        $page.text(buf);
                    }
                }

                if $max_width.is_none() {
                    $page.text([
                        roman(concat!(stringify!($name), " ")),
                        italic(&path),
                    ]);
                }

                $page.text([
                    line_break(),
                ]);
            }
        })+
    };
}
use render_item_kinds;
