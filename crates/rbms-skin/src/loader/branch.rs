//! The two document features that run before anything is deserialised: conditional branches and
//! includes.
//!
//! A document may write an object as a list of guarded clauses -- `[{ "if": [901], "value": {...} },
//! { "value": {...} }]` -- and the first clause whose condition holds is the one that survives. It
//! may also write `{ "include": "parts/gauge.json" }` in an object's place and have that file's
//! contents take over. Both are how a skin offers the player choices, and both are resolved on the
//! parsed tree before the mirror in [`crate::model`] ever sees it (`JsonSkinSerializer`'s
//! `ObjectSerializer`).
//!
//! A list-typed field is the other half, and it reads its elements quite differently
//! (`JsonSkinSerializer`'s `ArraySerializer`): there, an element carrying `if` alongside `value` or
//! `values` is one *conditional element* which stays where it is when its condition holds, and an
//! element carrying `include` splices the included list into its place. Which of the two rules
//! applies is decided by the field, not by the shape of the value, so the field names that hold a
//! list of records are listed here the way the reference lists its array classes.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::SkinError;
use crate::model::PropertyDef;

/// The option value the reference stores for "pick one at random"
/// (`SkinProperty.OPTION_RANDOM_VALUE`). The generated property table carries the same constant.
pub const OPTION_RANDOM_VALUE: i32 = -1;

/// The key a guarded clause states its condition under.
const BRANCH_CONDITION_KEY: &str = "if";

/// The key a guarded clause holds its payload under.
const BRANCH_VALUE_KEY: &str = "value";

/// The key a conditional element of a list holds several payloads under, each spliced in where the
/// element stood.
const BRANCH_VALUES_KEY: &str = "values";

/// The key an object names another file under.
const INCLUDE_KEY: &str = "include";

/// How deep includes may nest before the document is treated as circular.
pub const MAX_INCLUDE_DEPTH: usize = 8;

/// How many included files one load may expand in total.
///
/// Depth alone does not bound the work. A file that includes eight files at each of eight levels
/// never nests deeper than the limit and still expands `8^8` times, which is a document of a few
/// kilobytes holding the loader for as long as it likes. This is the ceiling on the whole tree, and
/// a document that reaches it stops expanding and says so.
pub const MAX_INCLUDE_EXPANSIONS: usize = 1_024;

/// The document's own top-level fields that hold a list of records rather than one record.
///
/// These are the reference's `array_classes` read at the document root: every field whose Java type
/// is a `T[]` of a class the serializer knows. Kept apart from [`NESTED_ARRAY_FIELDS`] because a
/// handful of names mean a list at the root and a single record further down -- `graph` is a list of
/// bar graphs on the document and one destination inside a song list.
const ROOT_ARRAY_FIELDS: &[&str] = &[
    "bpmgraph",
    "category",
    "customEvents",
    "customTimers",
    "destination",
    "filepath",
    "floatvalue",
    "font",
    "gaugegraph",
    "graph",
    "hiddenCover",
    "hiterrorvisualizer",
    "image",
    "imageset",
    "judge",
    "judgegraph",
    "liftCover",
    "offset",
    "pmchara",
    "property",
    "slider",
    "source",
    "text",
    "timingdistributiongraph",
    "timingvisualizer",
    "value",
];

/// The fields below the document root that hold a list of records.
///
/// A name that reaches a plain list of strings or numbers is left out: those cannot carry a clause
/// or an include, so naming them would only widen the chance of colliding with a single-record
/// field of the same name somewhere else in the mirror.
const NESTED_ARRAY_FIELDS: &[&str] = &[
    "bpm",
    "dst",
    "fallback",
    "group",
    "images",
    "item",
    "label",
    "lamp",
    "level",
    "listoff",
    "liston",
    "numbers",
    "offset",
    "op",
    "playerlamp",
    "rivallamp",
    "stop",
    "text",
    "time",
    "trophy",
];

/// Where in the document a value sits, which is what decides how a field name is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The document's own top level.
    Root,
    /// Anywhere inside it, an included file's contents included.
    Nested,
}

/// Whether `key` names a list of records at this point in the document.
fn holds_records(scope: Scope, key: &str) -> bool {
    NESTED_ARRAY_FIELDS.binary_search(&key).is_ok() || (scope == Scope::Root && ROOT_ARRAY_FIELDS.binary_search(&key).is_ok())
}

/// The option a customisation row resolves to.
///
/// The player's choice wins when it is one this row offers; otherwise the row's own `def` names an
/// item, and failing that its first item stands (`SkinHeader.CustomOption.getDefaultOption`).
pub fn selected_option(property: &PropertyDef, chosen: Option<i32>) -> i32 {
    if let Some(chosen) = chosen
        && property.item.iter().any(|item| item.op == chosen)
    {
        return chosen;
    }
    if let Some(item) = property.item.iter().find(|item| Some(item.name.as_str()) == property.def.as_deref()) {
        return item.op;
    }
    property.item.first().map_or(OPTION_RANDOM_VALUE, |item| item.op)
}

/// Every option id a document's customisation rows currently turn on
/// (`JSONSkinLoader.getEnabledOptions`).
pub fn enabled_options(properties: &[PropertyDef], chosen: &BTreeMap<String, i32>) -> BTreeSet<i32> {
    properties.iter().map(|property| selected_option(property, chosen.get(&property.name).copied())).collect()
}

/// Every option id a document's customisation rows can turn on, whether or not they currently do.
///
/// These are the document's own options, so no build-side registry declares them; the loader treats
/// them as implemented on the document's word alone. Without this a skin's customisation rows would
/// gate nothing, and every variant of a customised object would draw at once.
pub fn declared_options(properties: &[PropertyDef]) -> BTreeSet<i32> {
    properties.iter().flat_map(|property| property.item.iter()).map(|item| item.op.saturating_abs()).filter(|op| *op != 0).collect()
}

/// Whether one option id holds, negative meaning "and it must be off".
pub fn option_holds(id: i32, enabled: &BTreeSet<i32>) -> bool {
    if id >= 0 { enabled.contains(&id) } else { !enabled.contains(&-id) }
}

/// Whether a clause's condition holds.
///
/// A bare number is one option. A list is a conjunction, and a list nested inside it is a
/// disjunction, so `[[901, 902], 911]` reads as "901 or 902, and 911". A missing condition always
/// holds; anything else never does.
pub fn test_condition(condition: Option<&Value>, enabled: &BTreeSet<i32>) -> bool {
    let Some(condition) = condition else { return true };
    match condition {
        Value::Null => true,
        Value::Number(number) => number.as_i64().is_some_and(|id| option_holds(id as i32, enabled)),
        Value::Array(clauses) => clauses.iter().all(|clause| match clause {
            Value::Number(number) => number.as_i64().is_some_and(|id| option_holds(id as i32, enabled)),
            Value::Array(alternatives) => alternatives.iter().any(|alternative| alternative.as_i64().is_some_and(|id| option_holds(id as i32, enabled))),
            _ => false,
        }),
        _ => false,
    }
}

/// Whether an array is a list of guarded clauses rather than ordinary data.
///
/// Every element has to be an object and at least one has to carry a condition, which no ordinary
/// object list in a skin document does.
fn is_branch(items: &[Value]) -> bool {
    !items.is_empty() && items.iter().all(Value::is_object) && items.iter().any(|item| item.get(BRANCH_CONDITION_KEY).is_some())
}

/// Whether one element of a list is a conditional element rather than a record of its own.
///
/// The reference asks for a condition *and* a payload, so a record that happens to carry an `if`
/// field of its own is still read as a record.
fn is_conditional_element(object: &Map<String, Value>) -> bool {
    object.contains_key(BRANCH_CONDITION_KEY) && (object.contains_key(BRANCH_VALUE_KEY) || object.contains_key(BRANCH_VALUES_KEY))
}

/// Whether an object stands in for another file's contents.
fn include_target(object: &Map<String, Value>) -> Option<&str> {
    object.get(INCLUDE_KEY)?.as_str()
}

/// What the transform needs while it walks the tree.
pub struct BranchContext<'a> {
    /// The option ids the player's choices turn on.
    enabled: &'a BTreeSet<i32>,
    /// Reads and parses one included file, already checked to be inside the skin root.
    include: &'a mut dyn FnMut(&str) -> Result<Value, SkinError>,
    /// Where a skipped include is recorded.
    warnings: &'a mut Vec<String>,
    /// Each include target as it was parsed, so a file named from several places is read once.
    cache: BTreeMap<String, Value>,
    /// How many includes are left in this load's budget.
    remaining: usize,
    /// Whether the budget has already been reported, so one exhausted document costs one line.
    reported: bool,
}

impl<'a> BranchContext<'a> {
    /// A walk over one document, with `include` reading the files it names.
    pub fn new(enabled: &'a BTreeSet<i32>, include: &'a mut dyn FnMut(&str) -> Result<Value, SkinError>, warnings: &'a mut Vec<String>) -> BranchContext<'a> {
        BranchContext { enabled, include, warnings, cache: BTreeMap::new(), remaining: MAX_INCLUDE_EXPANSIONS, reported: false }
    }

    /// The option ids this walk resolves conditions against.
    pub fn enabled(&self) -> &BTreeSet<i32> {
        self.enabled
    }

    /// One included file's parsed contents, or `None` when it was skipped and warned about.
    ///
    /// A target already read this load comes back from the cache, so a part file named by fifty
    /// objects costs one read and one parse rather than fifty.
    fn expand(&mut self, target: &str, depth: usize) -> Option<Value> {
        if depth >= MAX_INCLUDE_DEPTH {
            self.warnings.push(format!("include {target:?} nests deeper than {MAX_INCLUDE_DEPTH} files and was skipped"));
            return None;
        }
        if self.remaining == 0 {
            if !self.reported {
                self.reported = true;
                self.warnings.push(format!("the document expanded more than {MAX_INCLUDE_EXPANSIONS} includes; the rest were skipped"));
            }
            return None;
        }
        self.remaining -= 1;
        if let Some(cached) = self.cache.get(target) {
            return Some(cached.clone());
        }
        match (self.include)(target) {
            Ok(value) => {
                self.cache.insert(target.to_owned(), value.clone());
                Some(value)
            }
            Err(error) => {
                self.warnings.push(format!("include {target:?} was skipped: {error}"));
                None
            }
        }
    }
}

/// Resolves every guarded clause and include in a parsed document, in place.
///
/// A clause list with no satisfied clause resolves to nothing, and so does an include that cannot
/// be read, which is warned about first. Nothing means the key is dropped where it stands in an
/// object and the element is dropped where it stands in a list, so an object nobody selected costs
/// one entry rather than the whole document, and the mirror never meets a null where it expects a
/// record.
pub fn transform(value: &mut Value, context: &mut BranchContext<'_>) -> Result<(), SkinError> {
    transform_at(value, context, 0, Scope::Root, false)
}

/// [`transform`], carrying how deep the includes have nested, where in the document this value
/// sits, and whether it is one of the fields that holds a list of records.
fn transform_at(value: &mut Value, context: &mut BranchContext<'_>, depth: usize, scope: Scope, records: bool) -> Result<(), SkinError> {
    match value {
        Value::Array(_) if records => transform_record_list(value, context, depth),
        Value::Array(items) if is_branch(items) => {
            let chosen =
                items.iter_mut().find(|item| test_condition(item.get(BRANCH_CONDITION_KEY), context.enabled())).map(|item| item[BRANCH_VALUE_KEY].take());
            *value = chosen.unwrap_or(Value::Null);
            transform_at(value, context, depth, Scope::Nested, false)
        }
        Value::Array(items) => {
            items.iter_mut().try_for_each(|item| transform_at(item, context, depth, Scope::Nested, false))?;
            items.retain(|item| !item.is_null());
            Ok(())
        }
        Value::Object(object) => {
            if let Some(target) = include_target(object) {
                let target = target.to_owned();
                *value = match context.expand(&target, depth) {
                    Some(mut included) => {
                        transform_at(&mut included, context, depth + 1, Scope::Nested, records)?;
                        included
                    }
                    None => Value::Null,
                };
                return Ok(());
            }
            object.iter_mut().try_for_each(|(key, child)| {
                let records = holds_records(scope, key);
                transform_at(child, context, depth, Scope::Nested, records)
            })?;
            object.retain(|_, child| !child.is_null());
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Resolves a list whose elements are records, keeping each surviving element where it stood.
///
/// Three element shapes are special and everything else is a record: a nested list is a guarded
/// clause list picking one record, an `include` splices the file's own list in, and an element with
/// a condition and a payload keeps or drops just itself.
fn transform_record_list(value: &mut Value, context: &mut BranchContext<'_>, depth: usize) -> Result<(), SkinError> {
    let Value::Array(items) = value else {
        return Ok(());
    };
    let taken = std::mem::take(items);
    let mut out: Vec<Value> = Vec::with_capacity(taken.len());
    for mut item in taken {
        match &mut item {
            Value::Object(object) if include_target(object).is_some() => {
                let target = include_target(object).unwrap_or_default().to_owned();
                let Some(mut included) = context.expand(&target, depth) else {
                    continue;
                };
                transform_at(&mut included, context, depth + 1, Scope::Nested, true)?;
                extend_with(&mut out, included);
            }
            Value::Object(object) if is_conditional_element(object) => {
                if !test_condition(object.get(BRANCH_CONDITION_KEY), context.enabled()) {
                    continue;
                }
                if let Some(mut one) = object.get_mut(BRANCH_VALUE_KEY).map(Value::take) {
                    transform_at(&mut one, context, depth, Scope::Nested, false)?;
                    if !one.is_null() {
                        out.push(one);
                    }
                }
                if let Some(mut many) = object.get_mut(BRANCH_VALUES_KEY).map(Value::take) {
                    transform_at(&mut many, context, depth, Scope::Nested, true)?;
                    extend_with(&mut out, many);
                }
            }
            other => {
                transform_at(other, context, depth, Scope::Nested, false)?;
                if !other.is_null() {
                    out.push(other.take());
                }
            }
        }
    }
    *value = Value::Array(out);
    Ok(())
}

/// Appends a resolved payload: a list is spliced element by element, anything else stands alone.
fn extend_with(out: &mut Vec<Value>, value: Value) {
    match value {
        Value::Null => {}
        Value::Array(elements) => out.extend(elements.into_iter().filter(|element| !element.is_null())),
        other => out.push(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_field_tables_are_sorted_so_the_lookup_can_bisect_them() {
        let mut root = ROOT_ARRAY_FIELDS.to_vec();
        root.sort_unstable();
        assert_eq!(root, ROOT_ARRAY_FIELDS, "ROOT_ARRAY_FIELDS is searched with binary_search");

        let mut nested = NESTED_ARRAY_FIELDS.to_vec();
        nested.sort_unstable();
        assert_eq!(nested, NESTED_ARRAY_FIELDS, "NESTED_ARRAY_FIELDS is searched with binary_search");
    }

    #[test]
    fn a_name_that_means_a_list_only_at_the_root_is_read_as_one_record_below_it() {
        assert!(holds_records(Scope::Root, "graph"), "the document's own graph field is a list of bar graphs");
        assert!(!holds_records(Scope::Nested, "graph"), "a song list's graph field is one destination");
        assert!(holds_records(Scope::Nested, "dst"), "keyframes are a list wherever they appear");
    }
}
