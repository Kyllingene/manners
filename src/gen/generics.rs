use roff::{bold, italic, line_break, roman, Inline};
use rustdoc_types::{
    Crate, GenericArg, GenericArgs, GenericBound, GenericParamDef, GenericParamDefKind as GPDK,
    Term, TraitBoundModifier, WherePredicate,
};

use super::render_type;

pub fn render_generics(
    cr: &Crate,
    name: &str,
    generics: &[GenericParamDef],
    mut depth: usize,
    page: &mut Vec<Inline>,
) {
    if generics.is_empty() {
        page.push(bold(name));
        return;
    }

    page.append(&mut if generics.len() < 3 {
        vec![bold(name), roman("<")]
    } else {
        depth += 1;
        vec![
            bold(name),
            roman("<"),
            line_break(),
            roman("  ".repeat(depth)),
        ]
    });

    let sep = if generics.len() < 3 {
        |page: &mut Vec<Inline>, _| page.push(roman(", "))
    } else {
        |page: &mut Vec<Inline>, depth| {
            page.push(roman(","));
            page.push(line_break());
            page.push(roman("  ".repeat(depth)));
        }
    };

    let mut first = true;
    for param in generics {
        if !first {
            sep(page, depth);
        }
        first = false;

        match &param.kind {
            GPDK::Lifetime { outlives } => {
                page.push(italic(&param.name));

                let mut first = true;
                for lt in outlives {
                    if first {
                        page.push(roman(": "));
                    } else {
                        page.push(roman(" + "));
                    }
                    first = false;

                    page.push(roman(lt));
                }
            }
            GPDK::Type {
                bounds, default, ..
            } => {
                page.push(italic(&param.name));

                let mut first = true;
                for bound in bounds {
                    if first {
                        page.push(roman(": "));
                    } else {
                        page.push(roman(" + "));
                    }
                    first = false;

                    match bound {
                        GenericBound::TraitBound {
                            trait_,
                            modifier,
                            .. // TODO: when is this populated?
                        } => {
                            if *modifier == TraitBoundModifier::Maybe {
                                page.push(roman("?"));
                            }

                            if let Some(args) = &trait_.args {
                                render_generics_args(cr, &trait_.name, args, depth + 1, page);
                            } else {
                                page.push(bold(&trait_.name));
                            }
                        }
                        GenericBound::Outlives(lt) => {
                            page.push(roman(lt));
                        }
                    }

                    if let Some(default) = default {
                        page.push(roman(" = "));
                        render_type(cr, default, depth + 1, page);
                    }
                }
            }
            GPDK::Const { type_, default } => {
                render_type(cr, type_, depth + 1, page);
                if let Some(default) = default {
                    page.push(roman(" = "));
                    page.push(roman(default));
                }
            }
        }
    }

    if generics.len() < 3 {
        page.push(roman(">"));
    } else {
        page.push(line_break());
        page.push(roman(">"));
    }
}

pub fn render_generics_args(
    cr: &Crate,
    name: &str,
    generics: &GenericArgs,
    mut depth: usize,
    page: &mut Vec<Inline>,
) {
    match generics {
        // TODO: handle bindings
        GenericArgs::AngleBracketed { args, .. } => {
            if args.is_empty() {
                page.push(bold(name));
                return;
            }

            page.append(&mut if args.len() < 3 {
                vec![bold(name), roman("<")]
            } else {
                depth += 1;
                vec![
                    bold(name),
                    roman("<"),
                    line_break(),
                    roman("  ".repeat(depth)),
                ]
            });

            let sep = if args.len() < 3 {
                |page: &mut Vec<Inline>, _| page.push(roman(", "))
            } else {
                |page: &mut Vec<Inline>, depth| {
                    page.push(roman(","));
                    page.push(line_break());
                    page.push(roman("  ".repeat(depth)));
                }
            };

            let mut first = true;
            for param in args {
                if !first {
                    sep(page, depth);
                }
                first = false;

                match param {
                    GenericArg::Lifetime(lt) => page.push(roman(lt)),
                    GenericArg::Type(typ) => render_type(cr, typ, depth + 1, page),
                    GenericArg::Const(co) => {
                        page.push(roman(format!("const {}{:?}", co.expr, co.value)));
                    }
                    GenericArg::Infer => page.push(roman("_")),
                }
            }

            if args.len() < 3 {
                page.push(roman(">"));
            } else {
                page.push(line_break());
                page.push(roman(">"));
            }
        }
        GenericArgs::Parenthesized { inputs, output } => {
            page.append(&mut if inputs.len() < 3 {
                vec![bold(name), roman("(")]
            } else {
                depth += 1;
                vec![
                    bold(name),
                    roman("("),
                    line_break(),
                    roman("  ".repeat(depth)),
                ]
            });

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
            for arg in inputs {
                if !first {
                    sep(page, depth);
                }
                first = false;
                render_type(cr, arg, depth + 1, page);
            }

            if inputs.len() < 3 {
                page.push(roman(")"));
            } else {
                page.push(line_break());
                page.push(roman(")"));
                depth -= 1;
            }

            if let Some(output) = output {
                page.push(roman(" -> "));
                render_type(cr, output, depth + 1, page);
            }
        }
    }
}

pub fn render_generics_bounds(
    cr: &Crate,
    bounds: &[GenericBound],
    depth: usize,
    page: &mut Vec<Inline>,
) {
    let mut first = true;
    for bound in bounds {
        if !first {
            page.push(roman(" + "));
        }
        first = false;

        match bound {
            GenericBound::TraitBound {
                trait_,
                modifier,
                .. // TODO: when is this populated?
            } => {
                if *modifier == TraitBoundModifier::Maybe {
                    page.push(roman("?"));
                }

                if let Some(args) = &trait_.args {
                    render_generics_args(cr, &trait_.name, args, depth + 1, page);
                } else {
                    page.push(bold(&trait_.name));
                }
            }
            GenericBound::Outlives(lt) => {
                page.push(roman(lt));
            }
        }
    }
}

pub fn render_where(cr: &Crate, bounds: &[WherePredicate], depth: usize, page: &mut Vec<Inline>) {
    if bounds.is_empty() {
        return;
    }
    page.push(roman("where"));

    let mut first = true;
    for pred in bounds {
        if !first {
            page.push(roman(","));
        }
        first = false;
        page.push(line_break());
        page.push(roman("  ".repeat(depth + 1)));

        match pred {
            WherePredicate::BoundPredicate {
                type_,
                bounds,
                generic_params,
            } => {
                if !generic_params.is_empty() {
                    // I've yet to discover what these are for
                    panic!("generics params: {generic_params:#?}");
                }
                render_type(cr, type_, depth + 2, page);
                page.push(roman(": "));
                render_generics_bounds(cr, bounds, depth + 2, page);
            }
            WherePredicate::RegionPredicate { lifetime, bounds } => {
                page.push(roman(lifetime));
                render_generics_bounds(cr, bounds, depth + 2, page);
            }
            WherePredicate::EqPredicate { lhs, rhs } => {
                render_type(cr, lhs, depth + 2, page);
                page.push(roman(" = "));

                match rhs {
                    Term::Type(t) => render_type(cr, t, depth + 2, page),
                    Term::Constant(c) => page.push(roman(c.value.as_ref().unwrap())),
                }
            }
        }
    }

    page.push(line_break());
}
