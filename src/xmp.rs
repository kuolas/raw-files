// SPDX-License-Identifier: MIT OR Apache-2.0

//! XMP: the properties a program adds to a picture, in XMP Part 1's data
//! model.
//!
//! XMP, the Extensible Metadata Platform, is XML. A packet of it lies
//! inside a file, or in a sidecar file beside it. Editors write ratings,
//! keywords, titles and who made the picture there; cameras write a few
//! values of their own. XMP Part 1, which is ISO 16684-1, defines the
//! data model, and this module gives it as types:
//!
//! - [`Xmp`]: every property of one packet, in the packet's order.
//! - [`XmpProperty`]: one property, a structure's field or a qualifier, a
//!   value under a name. The name is a namespace URI and a local name,
//!   never the prefix the packet writes.
//! - [`XmpValue`]: a value, in one of the [`XmpForm`]s, with its
//!   qualifiers, such as its language.
//! - [`XmpForm`]: simple text, a URI, a structure of named fields, or an
//!   array of items of one [`XmpArrayKind`].
//!
//! [`namespaces`](crate::namespaces) gives the URIs of the namespaces of
//! the rating, the label, IPTC Core and two qualifiers, so that a lookup
//! says what it looks for.
//!
//! Every value can be built without a packet, for a caller's tests, as
//! every record of raw-files can.

/// Every property of an XMP packet, in XMP Part 1's data model.
///
/// A property is named by its namespace URI and its local name, never by
/// the prefix the packet writes. Where both prefixes are bound to
/// [`namespaces::DC`](crate::namespaces::DC), `dc:subject` and
/// `x:subject` are one property. [`property`](Self::property) looks one
/// up by the two. [`iter`](Self::iter) gives every one, in the packet's
/// order, as [`Entries`](crate::Entries) gives a file's entries.
///
/// XMP Part 1 allows a property's name once in a packet. Where a packet
/// repeats one, the first stands, and the later ones are left out.
///
/// `Xmp` has no order and no text form: properties have no order but the
/// packet's, and a tree of values has no one way to print. It is equal to
/// another when both hold the same properties in the same order, each
/// value as recorded, and the same [`toolkit`](Self::toolkit) and
/// [`about`](Self::about).
///
/// `Xmp::default()` is an empty packet. A caller's tests start from it
/// and add properties with [`with_property`](Self::with_property), each
/// replacing one of the same name.
///
/// # Examples
///
/// Build a packet and look up its values:
///
/// ```
/// use raw_files::{Xmp, XmpArrayKind, XmpValue, namespaces};
///
/// let title = XmpValue::array(
///     XmpArrayKind::Alternative,
///     [XmpValue::simple("Impression, soleil levant").with_language("x-default")],
/// );
/// let xmp = Xmp::default()
///     .with_toolkit("XMP Core 6.0.0")
///     .with_property(namespaces::XMP, "Rating", "5")
///     .with_property(namespaces::DC, "title", title);
/// assert_eq!(xmp.len(), 2);
/// assert_eq!(xmp.toolkit(), Some("XMP Core 6.0.0"));
/// let rating = xmp.property(namespaces::XMP, "Rating");
/// assert_eq!(rating.and_then(|value| value.text()), Some("5"));
/// for property in &xmp {
///     println!("{} {}", property.namespace(), property.name());
/// }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Xmp {
    properties: Vec<XmpProperty>,
    toolkit: Option<String>,
    about: Option<String>,
}

impl Xmp {
    /// Every property, in the packet's order: the order of the elements
    /// that hold them, each `rdf:Description`'s attributes before its
    /// elements.
    pub fn iter(&self) -> std::slice::Iter<'_, XmpProperty> {
        self.properties.iter()
    }

    /// The number of properties. A structure's fields, an array's items
    /// and a value's qualifiers are not counted: they belong to their
    /// property.
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Whether the packet holds no property, as the packets of some Nikon
    /// bodies hold none.
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// The value of the property named `name` in the namespace whose URI
    /// is `namespace`, such as [`namespaces::DC`](crate::namespaces::DC)
    /// and `"subject"`. `None` when the packet holds no such property.
    ///
    /// The two are compared byte for byte, as XMP Part 1 compares names:
    /// a namespace URI with or without its final slash names two
    /// namespaces.
    pub fn property(&self, namespace: &str, name: &str) -> Option<&XmpValue> {
        find(&self.properties, namespace, name)
    }

    /// The properties in the namespace whose URI is `namespace`, in the
    /// packet's order; none when the packet holds none of them.
    pub fn in_namespace<'a>(
        &'a self,
        namespace: &'a str,
    ) -> impl Iterator<Item = &'a XmpProperty> + 'a {
        self.properties
            .iter()
            .filter(move |property| property.namespace == namespace)
    }

    /// The toolkit that wrote the packet, as it names itself in the
    /// `x:xmptk` attribute of the `x:xmpmeta` element around the
    /// properties, such as `XMP Core 4.4.0-Exiv2`. `None` when the packet
    /// names none, or names it with an empty text.
    pub fn toolkit(&self) -> Option<&str> {
        self.toolkit.as_deref()
    }

    /// The resource the packet describes, from the `rdf:about` attribute
    /// of its `rdf:Description` elements, the first that is not empty.
    /// `None` when every one is empty or missing, as in most packets: the
    /// packet describes the file that holds it.
    pub fn about(&self) -> Option<&str> {
        self.about.as_deref()
    }

    /// Sets the property `name` of the namespace `namespace`, in place of
    /// one of the same namespace and name, which keeps its position; else
    /// after the others. A text may be given for a simple value.
    ///
    /// For a caller's tests, as the `with_…` methods of every record.
    #[must_use]
    pub fn with_property(
        mut self,
        namespace: impl Into<String>,
        name: impl Into<String>,
        value: impl Into<XmpValue>,
    ) -> Self {
        set(
            &mut self.properties,
            XmpProperty::new(namespace, name, value),
        );
        self
    }

    /// Sets the toolkit that [`toolkit`](Self::toolkit) gives, in place of
    /// one set before.
    #[must_use]
    pub fn with_toolkit(mut self, toolkit: impl Into<String>) -> Self {
        self.toolkit = Some(toolkit.into());
        self
    }

    /// Sets the resource that [`about`](Self::about) gives, in place of
    /// one set before.
    #[must_use]
    pub fn with_about(mut self, about: impl Into<String>) -> Self {
        self.about = Some(about.into());
        self
    }
}

impl<'a> IntoIterator for &'a Xmp {
    type Item = &'a XmpProperty;
    type IntoIter = std::slice::Iter<'a, XmpProperty>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A value under a name: a property of an [`Xmp`], a field of a
/// structure, or a qualifier of a value.
///
/// The name is an XML expanded name: a namespace URI, such as
/// [`namespaces::DC`](crate::namespaces::DC), and a local name, such as
/// `"creator"`. The prefix a packet writes for the namespace is not kept:
/// it only stands for the URI.
///
/// # Examples
///
/// ```
/// use raw_files::{XmpProperty, namespaces};
///
/// let property = XmpProperty::new(namespaces::XMP, "Label", "Red");
/// assert_eq!(property.namespace(), namespaces::XMP);
/// assert_eq!(property.name(), "Label");
/// assert_eq!(property.value().text(), Some("Red"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct XmpProperty {
    namespace: String,
    name: String,
    value: XmpValue,
}

impl XmpProperty {
    /// A property of the namespace whose URI is `namespace`, named `name`,
    /// with `value`. A text may be given for a simple value.
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
        value: impl Into<XmpValue>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            value: value.into(),
        }
    }

    /// The URI of the property's namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The property's local name, its name within its namespace.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The property's value.
    pub fn value(&self) -> &XmpValue {
        &self.value
    }
}

/// A value of XMP, in one of the four [`XmpForm`]s, with its qualifiers.
///
/// A qualifier is a value about the value, under a name of its own, such
/// as the language of a text, `xml:lang`. Any value may have qualifiers,
/// and a qualifier may have qualifiers in turn (XMP Part 1, §6.4).
///
/// [`form`](Self::form) gives the form, to match on. The getters beside it
/// read one form each, and give nothing for the others, so a lookup needs
/// no match:
///
/// - [`text`](Self::text) gives a simple value's text;
/// - [`fields`](Self::fields) gives a structure's fields;
/// - [`items`](Self::items) gives an array's items.
///
/// `XmpValue` has no order and no text form, as [`Xmp`] has none. It is
/// equal to another when both have the same form, the same contents in
/// the same order, and the same qualifiers in the same order.
///
/// # Examples
///
/// The creators of a picture, an ordered array of names, and the text
/// of its description in the default language:
///
/// ```
/// use raw_files::{XmpArrayKind, XmpValue};
///
/// let creators = XmpValue::array(
///     XmpArrayKind::Ordered,
///     ["Claude Monet".into(), "Berthe Morisot".into()],
/// );
/// let names: Vec<&str> = creators.items().iter().filter_map(XmpValue::text).collect();
/// assert_eq!(names, ["Claude Monet", "Berthe Morisot"]);
///
/// let description = XmpValue::array(
///     XmpArrayKind::Alternative,
///     [
///         XmpValue::simple("A harbor at sunrise").with_language("x-default"),
///         XmpValue::simple("Un port au lever du soleil").with_language("fr"),
///     ],
/// );
/// let default = description
///     .items()
///     .iter()
///     .find(|item| item.language() == Some("x-default"))
///     .and_then(XmpValue::text);
/// assert_eq!(default, Some("A harbor at sunrise"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct XmpValue {
    form: XmpForm,
    qualifiers: Vec<XmpProperty>,
}

impl XmpValue {
    /// A simple value: a text, which may be empty.
    pub fn simple(text: impl Into<String>) -> Self {
        Self::of(XmpForm::Simple(text.into()))
    }

    /// A simple value that is a URI, written with `rdf:resource` in a
    /// packet.
    pub fn uri(uri: impl Into<String>) -> Self {
        Self::of(XmpForm::Uri(uri.into()))
    }

    /// A structure of named fields, in the order given.
    ///
    /// XMP Part 1 allows a name once in a structure. So a field whose
    /// namespace and name repeat an earlier field's is left out, as the
    /// reader leaves it out.
    pub fn structure(fields: impl IntoIterator<Item = XmpProperty>) -> Self {
        let mut kept: Vec<XmpProperty> = Vec::new();
        for field in fields {
            if find(&kept, &field.namespace, &field.name).is_none() {
                kept.push(field);
            }
        }
        Self::of(XmpForm::Structure(kept))
    }

    /// An array of `kind`, of `items` in the order given. The items may be
    /// of any form.
    pub fn array(kind: XmpArrayKind, items: impl IntoIterator<Item = XmpValue>) -> Self {
        Self::of(XmpForm::Array {
            kind,
            items: items.into_iter().collect(),
        })
    }

    /// A value of `form`, with no qualifier.
    fn of(form: XmpForm) -> Self {
        Self {
            form,
            qualifiers: Vec::new(),
        }
    }

    /// Sets the value's language, the qualifier `xml:lang`, in place of
    /// one set before. `language` is a language tag, such as `en-US`, or
    /// `x-default` for the default item of an alternative array; it is
    /// stored as given.
    #[must_use]
    pub fn with_language(self, language: impl Into<String>) -> Self {
        self.with_qualifier(crate::namespaces::XML, "lang", Self::simple(language))
    }

    /// Sets the qualifier `name` of the namespace `namespace`, in place of
    /// one of the same namespace and name, which keeps its position; else
    /// after the others. A text may be given for a simple value.
    #[must_use]
    pub fn with_qualifier(
        mut self,
        namespace: impl Into<String>,
        name: impl Into<String>,
        value: impl Into<XmpValue>,
    ) -> Self {
        set(
            &mut self.qualifiers,
            XmpProperty::new(namespace, name, value),
        );
        self
    }

    /// The value's form, to match on. Every form is listed, so a match
    /// needs no wildcard arm.
    pub fn form(&self) -> &XmpForm {
        &self.form
    }

    /// The text of a simple value, or the URI of a URI value. `None` for a
    /// structure or an array.
    pub fn text(&self) -> Option<&str> {
        match &self.form {
            XmpForm::Simple(text) | XmpForm::Uri(text) => Some(text),
            XmpForm::Structure(_) | XmpForm::Array { .. } => None,
        }
    }

    /// The fields of a structure, in the packet's order; empty for any
    /// other form. [`form`](Self::form) tells an empty structure from a
    /// value that is none.
    pub fn fields(&self) -> &[XmpProperty] {
        match &self.form {
            XmpForm::Structure(fields) => fields,
            _ => &[],
        }
    }

    /// The value of a structure's field named `name` in the namespace
    /// `namespace`. `None` when the value is no structure, or has no such
    /// field.
    pub fn field(&self, namespace: &str, name: &str) -> Option<&XmpValue> {
        find(self.fields(), namespace, name)
    }

    /// The items of an array, in the packet's order; empty for any other
    /// form. [`form`](Self::form) tells an empty array from a value that
    /// is none.
    pub fn items(&self) -> &[XmpValue] {
        match &self.form {
            XmpForm::Array { items, .. } => items,
            _ => &[],
        }
    }

    /// Every qualifier of the value, in the packet's order, `xml:lang`
    /// among them.
    pub fn qualifiers(&self) -> &[XmpProperty] {
        &self.qualifiers
    }

    /// The value of the qualifier named `name` in the namespace
    /// `namespace`. `None` when the value has no such qualifier.
    pub fn qualifier(&self, namespace: &str, name: &str) -> Option<&XmpValue> {
        find(&self.qualifiers, namespace, name)
    }

    /// The value's language: the text of its qualifier `xml:lang`, as
    /// written, such as `en-US` or `x-default`. `None` when the value has
    /// no language of its own.
    ///
    /// The text is given as the packet writes it. Language tags compare
    /// without regard to case (XMP Part 1, §6.4), so a caller compares
    /// them with [`str::eq_ignore_ascii_case`]. A language set on a
    /// structure or an array is the default of its fields or items, and
    /// is not repeated on them.
    pub fn language(&self) -> Option<&str> {
        self.qualifier(crate::namespaces::XML, "lang")
            .and_then(XmpValue::text)
    }
}

/// A simple value of the text.
impl From<&str> for XmpValue {
    fn from(text: &str) -> Self {
        Self::simple(text)
    }
}

/// A simple value of the text.
impl From<String> for XmpValue {
    fn from(text: String) -> Self {
        Self::simple(text)
    }
}

/// The form of an [`XmpValue`], one of XMP Part 1's value forms (§6.3).
///
/// ISO 16684-1 fixes the data model at these four forms, so the enum is
/// not marked `#[non_exhaustive]`: a match on it needs no wildcard arm.
///
/// # Examples
///
/// ```
/// use raw_files::{XmpForm, XmpValue};
///
/// /// A short description of a value's form, as a viewer's tree shows it.
/// fn describe(value: &XmpValue) -> String {
///     match value.form() {
///         XmpForm::Simple(text) => format!("\"{text}\""),
///         XmpForm::Uri(uri) => format!("<{uri}>"),
///         XmpForm::Structure(fields) => format!("{} fields", fields.len()),
///         XmpForm::Array { kind, items } => format!("{kind:?} of {}", items.len()),
///     }
/// }
///
/// assert_eq!(describe(&"3".into()), "\"3\"");
/// assert_eq!(describe(&XmpValue::uri("https://example.com/")), "<https://example.com/>");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum XmpForm {
    /// A text, which may be empty.
    Simple(String),
    /// A text that is a URI, written with `rdf:resource` in a packet.
    Uri(String),
    /// Named fields, in the packet's order, each name once.
    Structure(Vec<XmpProperty>),
    /// Items of any form, in the packet's order.
    Array {
        /// What the order of the items means.
        kind: XmpArrayKind,
        /// The items.
        items: Vec<XmpValue>,
    },
}

/// The kind of an XMP array: what the order of its items means (XMP
/// Part 1, §6.3.4).
///
/// XMP Part 1 defines these three kinds and no other, so the enum is not
/// marked `#[non_exhaustive]`: a match on it needs no wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum XmpArrayKind {
    /// An unordered array, `rdf:Bag`: the order of its items means
    /// nothing, as for keywords.
    Unordered,
    /// An ordered array, `rdf:Seq`: the order of its items means
    /// something, as for the authors of a work.
    Ordered,
    /// An alternative array, `rdf:Alt`: one of its items is chosen, as one
    /// of a text's languages. The first item is the default, where one is.
    Alternative,
}

/// The value of the first of `properties` named `name` in the namespace
/// `namespace`.
fn find<'a>(properties: &'a [XmpProperty], namespace: &str, name: &str) -> Option<&'a XmpValue> {
    properties
        .iter()
        .find(|property| property.namespace == namespace && property.name == name)
        .map(XmpProperty::value)
}

/// Sets `property` among `properties`, in place of one of the same
/// namespace and name, else after the others.
fn set(properties: &mut Vec<XmpProperty>, property: XmpProperty) {
    match properties
        .iter_mut()
        .find(|old| old.namespace == property.namespace && old.name == property.name)
    {
        Some(old) => *old = property,
        None => properties.push(property),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::namespaces::{DC, RDF, XML, XMP};

    #[test]
    fn a_simple_value_gives_its_text_and_nothing_of_another_form() {
        let value = XmpValue::simple("3");
        assert_eq!(value.form(), &XmpForm::Simple("3".to_owned()));
        assert_eq!(value.text(), Some("3"));
        assert!(value.fields().is_empty());
        assert!(value.items().is_empty());
        assert!(value.qualifiers().is_empty());
        assert_eq!(value.language(), None);
        assert_eq!(XmpValue::from("3"), value);
        assert_eq!(XmpValue::from(String::from("3")), value);
    }

    #[test]
    fn a_uri_gives_its_text_and_is_not_a_simple_value() {
        let value = XmpValue::uri("https://example.com/");
        assert_eq!(
            value.form(),
            &XmpForm::Uri("https://example.com/".to_owned())
        );
        assert_eq!(value.text(), Some("https://example.com/"));
        assert_ne!(value, XmpValue::simple("https://example.com/"));
    }

    #[test]
    fn a_structure_gives_its_fields_and_nothing_of_another_form() {
        let value = XmpValue::structure([
            XmpProperty::new(DC, "w", "8.5"),
            XmpProperty::new(DC, "h", "11.0"),
        ]);
        assert_eq!(value.text(), None);
        assert!(value.items().is_empty());
        let names: Vec<&str> = value.fields().iter().map(XmpProperty::name).collect();
        assert_eq!(names, ["w", "h"]);
        assert_eq!(value.field(DC, "h").and_then(XmpValue::text), Some("11.0"));
        assert_eq!(value.field(XMP, "h"), None);
        assert_eq!(value.field(DC, "unit"), None);
    }

    #[test]
    fn a_structure_keeps_the_first_of_a_repeated_field() {
        let value = XmpValue::structure([
            XmpProperty::new(DC, "w", "first"),
            XmpProperty::new(XMP, "w", "other namespace"),
            XmpProperty::new(DC, "w", "repeat"),
        ]);
        assert_eq!(value.fields().len(), 2);
        assert_eq!(value.field(DC, "w").and_then(XmpValue::text), Some("first"));
        assert_eq!(
            value.field(XMP, "w").and_then(XmpValue::text),
            Some("other namespace")
        );
    }

    #[test]
    fn an_array_gives_its_items_and_nothing_of_another_form() {
        let value = XmpValue::array(XmpArrayKind::Ordered, ["a".into(), "b".into()]);
        assert_eq!(value.text(), None);
        assert!(value.fields().is_empty());
        assert_eq!(value.field(DC, "a"), None);
        let items: Vec<&str> = value.items().iter().filter_map(XmpValue::text).collect();
        assert_eq!(items, ["a", "b"]);
        match value.form() {
            XmpForm::Array { kind, items } => {
                assert_eq!(*kind, XmpArrayKind::Ordered);
                assert_eq!(items.len(), 2);
            }
            other => panic!("not an array: {other:?}"),
        }
    }

    #[test]
    fn an_empty_array_and_an_empty_structure_differ_from_each_other() {
        let array = XmpValue::array(XmpArrayKind::Unordered, []);
        let structure = XmpValue::structure([]);
        assert!(array.items().is_empty() && structure.fields().is_empty());
        assert_ne!(array, structure);
        assert_ne!(array, XmpValue::array(XmpArrayKind::Ordered, []));
    }

    #[test]
    fn qualifiers_are_kept_in_order_and_a_name_set_again_replaces_its_value() {
        let value = XmpValue::simple("Monet")
            .with_qualifier(RDF, "type", XmpValue::uri("https://example.com/Person"))
            .with_language("fr")
            .with_qualifier(DC, "role", "painter")
            .with_language("en");
        let names: Vec<&str> = value.qualifiers().iter().map(XmpProperty::name).collect();
        assert_eq!(names, ["type", "lang", "role"]);
        assert_eq!(value.language(), Some("en"));
        assert_eq!(
            value.qualifier(XML, "lang").and_then(XmpValue::text),
            Some("en")
        );
        assert_eq!(
            value.qualifier(DC, "role").and_then(XmpValue::text),
            Some("painter")
        );
        assert_eq!(value.qualifier(DC, "lang"), None);
        // The value itself is unchanged by its qualifiers.
        assert_eq!(value.text(), Some("Monet"));
    }

    #[test]
    fn language_gives_the_text_as_written_and_compares_nothing() {
        let value = XmpValue::simple("Hello").with_language("EN-us");
        assert_eq!(value.language(), Some("EN-us"));
        assert_ne!(value, XmpValue::simple("Hello").with_language("en-US"));
    }

    #[test]
    fn an_empty_packet_has_no_property_toolkit_or_resource() {
        let xmp = Xmp::default();
        assert!(xmp.is_empty());
        assert_eq!(xmp.len(), 0);
        assert_eq!(xmp.iter().count(), 0);
        assert_eq!(xmp.property(XMP, "Rating"), None);
        assert_eq!(xmp.toolkit(), None);
        assert_eq!(xmp.about(), None);
    }

    #[test]
    fn properties_iterate_in_the_order_built() {
        let xmp = Xmp::default()
            .with_property(XMP, "Rating", "3")
            .with_property(DC, "format", "image/jpeg")
            .with_property(XMP, "Label", "Red");
        let names: Vec<&str> = xmp.iter().map(XmpProperty::name).collect();
        assert_eq!(names, ["Rating", "format", "Label"]);
        let by_ref: Vec<&str> = (&xmp).into_iter().map(XmpProperty::name).collect();
        assert_eq!(by_ref, names);
        assert_eq!(xmp.len(), 3);
        assert!(!xmp.is_empty());
    }

    #[test]
    fn with_property_replaces_a_property_of_the_same_name_in_place() {
        let xmp = Xmp::default()
            .with_property(XMP, "Rating", "3")
            .with_property(DC, "format", "image/jpeg")
            .with_property(XMP, "Rating", "5");
        let names: Vec<&str> = xmp.iter().map(XmpProperty::name).collect();
        assert_eq!(names, ["Rating", "format"]);
        assert_eq!(
            xmp.property(XMP, "Rating").and_then(XmpValue::text),
            Some("5")
        );
    }

    #[test]
    fn property_finds_by_a_constant_or_the_same_uri_written_out_never_by_prefix() {
        let xmp = Xmp::default().with_property(XMP, "Rating", "3");
        let written_out = String::from("http://ns.adobe.com/xap/1.0/");
        assert_eq!(
            xmp.property(XMP, "Rating"),
            xmp.property(&written_out, "Rating")
        );
        assert!(xmp.property(XMP, "Rating").is_some());
        assert_eq!(xmp.property("xmp", "Rating"), None);
        assert_eq!(xmp.property("http://ns.adobe.com/xap/1.0", "Rating"), None);
        assert_eq!(xmp.property(XMP, "rating"), None);
    }

    #[test]
    fn in_namespace_gives_one_namespace_in_order() {
        let xmp = Xmp::default()
            .with_property(XMP, "Rating", "3")
            .with_property(DC, "format", "image/jpeg")
            .with_property(XMP, "Label", "Red");
        let names: Vec<&str> = xmp.in_namespace(XMP).map(XmpProperty::name).collect();
        assert_eq!(names, ["Rating", "Label"]);
        assert_eq!(xmp.in_namespace(RDF).count(), 0);
    }

    #[test]
    fn with_toolkit_and_with_about_set_what_their_getters_give() {
        let xmp = Xmp::default()
            .with_toolkit("first")
            .with_toolkit("XMP Core 4.4.0-Exiv2")
            .with_about("uuid:1");
        assert_eq!(xmp.toolkit(), Some("XMP Core 4.4.0-Exiv2"));
        assert_eq!(xmp.about(), Some("uuid:1"));
        assert!(xmp.is_empty());
        assert_ne!(xmp, Xmp::default());
    }

    #[test]
    fn a_property_gives_its_name_and_value() {
        let property = XmpProperty::new(DC, "format", "image/jpeg");
        assert_eq!(property.namespace(), DC);
        assert_eq!(property.name(), "format");
        assert_eq!(property.value(), &XmpValue::simple("image/jpeg"));
    }
}
