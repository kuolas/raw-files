// SPDX-License-Identifier: MIT OR Apache-2.0

//! XMP packets built in code:
//!
//! - one of each form XMP Part 1 gives a value;
//! - one of the XML features the reader reads;
//! - three in the shapes measured files show: a Nikon Z 9's packet, a
//!   darktable sidecar and a Lightroom packet in a JPEG file.
//!
//! The three shapes are those of the files: their elements, attributes,
//! namespaces and padding. Every value is written here by hand. No text
//! names a person but a painter of the 19th century, as the examples of
//! the API docs do.

/// The `xmlns` declarations the built packets use.
const NS: &str = "xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#' \
    xmlns:xmp='http://ns.adobe.com/xap/1.0/' xmlns:dc='http://purl.org/dc/elements/1.1/' \
    xmlns:ex='urn:example:'";

/// `body` as the content of one `rdf:Description`, in `rdf:RDF` in
/// `x:xmpmeta`.
fn packet(body: &str) -> String {
    format!(
        "<x:xmpmeta xmlns:x='adobe:ns:meta/' x:xmptk='raw-files tests'><rdf:RDF {NS}>\
         <rdf:Description rdf:about=''>{body}</rdf:Description></rdf:RDF></x:xmpmeta>"
    )
}

/// Simple values: as attributes, as elements with text, and a URI.
pub fn simple() -> String {
    let text = packet(
        "<xmp:Rating>3</xmp:Rating><xmp:Label>Red</xmp:Label>\
         <xmp:BaseURL rdf:resource='https://example.com/'/><dc:format>image/jpeg</dc:format>",
    );
    text.replacen(
        "rdf:about=''",
        "rdf:about='' xmp:CreatorTool='raw-files tests'",
        1,
    )
}

/// A structure in each of its three forms, nested.
pub fn structures() -> String {
    packet(
        "<ex:a rdf:parseType='Resource'><ex:w>8.5</ex:w><ex:h>11.0</ex:h>\
           <ex:inner rdf:parseType='Resource'><ex:leaf>deep</ex:leaf></ex:inner></ex:a>\
         <ex:b><rdf:Description ex:w='8.5'><ex:h>11.0</ex:h></rdf:Description></ex:b>\
         <ex:c ex:w='8.5' ex:h='11.0' ex:unit='inch'/>",
    )
}

/// The three kinds of array, empty, nested and holding structures.
pub fn arrays() -> String {
    packet(
        "<dc:subject><rdf:Bag><rdf:li>harbor</rdf:li><rdf:li>sunrise</rdf:li></rdf:Bag></dc:subject>\
         <dc:creator><rdf:Seq><rdf:li>Claude Monet</rdf:li></rdf:Seq></dc:creator>\
         <dc:rights><rdf:Alt><rdf:li xml:lang='x-default'>Public domain</rdf:li></rdf:Alt></dc:rights>\
         <ex:empty><rdf:Bag/></ex:empty>\
         <ex:nested><rdf:Seq><rdf:li><rdf:Bag><rdf:li>inner</rdf:li></rdf:Bag></rdf:li>\
           <rdf:li rdf:parseType='Resource'><ex:f>x</ex:f></rdf:li><rdf:li ex:f='y'/></rdf:Seq></ex:nested>",
    )
}

/// Qualifiers: languages, `rdf:value` in each form, and typed nodes.
pub fn qualifiers() -> String {
    packet(
        "<dc:title><rdf:Alt><rdf:li xml:lang='x-default'>Impression</rdf:li>\
           <rdf:li xml:lang='fr'>Impression, soleil levant</rdf:li></rdf:Alt></dc:title>\
         <ex:a><rdf:Description><rdf:value>v</rdf:value><ex:q>w</ex:q></rdf:Description></ex:a>\
         <ex:b rdf:parseType='Resource' xml:lang='en'><rdf:value>v</rdf:value><ex:q>w</ex:q></ex:b>\
         <ex:c rdf:value='v' ex:q='w'/>\
         <ex:d><ex:Kind><ex:f>x</ex:f></ex:Kind></ex:d>\
         <dc:contributor><rdf:Bag><rdf:li><rdf:Description><rdf:value>Berthe Morisot</rdf:value>\
           <ex:role>painter</ex:role></rdf:Description></rdf:li></rdf:Bag></dc:contributor>",
    )
}

/// The XML features the reader reads:
///
/// - a byte order mark, an XML declaration and the packet wrapper;
/// - a comment, a processing instruction and CDATA;
/// - every entity, and both reference forms;
/// - CR LF line ends, and single quotes;
/// - a default namespace, and a prefix bound twice.
pub fn xml_features() -> String {
    "\u{FEFF}<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n\
     <?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\r\n\
     <!-- a comment --><x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\r\n\
     <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\r\n\
     <rdf:Description rdf:about='' xmlns='urn:example:' xmlns:p='http://ns.adobe.com/xap/1.0/'\r\n\
       p:Label='a&#9;tab &amp; &lt;angles&gt;'>\r\n\
     <Entities>&lt;&gt;&amp;&apos;&quot;&#65;&#x42;</Entities>\r\n\
     <Cdata><![CDATA[x < y ]] z]]></Cdata><?pi data?>\r\n\
     <p:Rating xmlns:p='urn:example:other'>rebound</p:Rating>\r\n\
     </rdf:Description></rdf:RDF></x:xmpmeta>\r\n\
     <?xpacket end=\"w\"?>"
        .to_owned()
}

/// A packet as a Nikon Z 9 writes it into IFD0's tag 0x02BC. It holds
/// the creator tool, the creation time and a rating of 0, then Adobe
/// Camera Raw's defaults in a second `rdf:Description`. Spaces inside the
/// wrapper pad it to 32,768 bytes.
pub fn nikon_z9() -> Vec<u8> {
    let defaults = [
        ("Exposure2012", "0.33"),
        ("Highlights2012", "-21"),
        ("Shadows2012", "10"),
        ("LuminanceSmoothing", "25"),
        ("LuminanceNoiseReductionDetail", "75"),
        ("LuminanceNoiseReductionContrast", "0"),
        ("ColorNoiseReduction", "10"),
        ("ColorNoiseReductionDetail", "50"),
        ("ColorNoiseReductionSmoothness", "50"),
        ("Sharpness", "44"),
        ("SharpenRadius", "2.00"),
        ("SharpenDetail", "25"),
        ("SharpenEdgeMasking", "0"),
        ("Contrast2012", "0"),
        ("Saturation", "4"),
        ("Clarity2012", "25"),
        ("Texture", "45"),
        ("CameraProfile", "Camera Standard"),
    ];
    let mut crd = String::new();
    for (name, value) in defaults {
        crd.push_str(&format!("<crd:{name}>{value}</crd:{name}>"));
    }
    let head = format!(
        "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
         <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
         <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\
         <xmp:CreatorTool>NIKON Z 9 Ver.05.20   </xmp:CreatorTool>\
         <xmp:CreateDate>2026-11-28T09:30:00.25</xmp:CreateDate>\
         <xmp:Rating>0</xmp:Rating> </rdf:Description>\
         <rdf:Description rdf:about=\"\" xmlns:crd=\"http://ns.adobe.com/camera-raw-defaults/1.0/\">\
         {crd}</rdf:Description></rdf:RDF></x:xmpmeta>"
    );
    let tail = "<?xpacket end=\"w\"?>";
    let mut bytes = head.into_bytes();
    bytes.resize(32_768 - tail.len(), b' ');
    bytes.extend_from_slice(tail.as_bytes());
    bytes
}

/// A sidecar as darktable writes one beside a picture: a whole XML
/// document with no packet wrapper, every simple value an attribute of
/// one `rdf:Description`, then its arrays.
pub fn darktable_sidecar() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
     <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"XMP Core 4.4.0-Exiv2\">\n \
     <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n  \
     <rdf:Description rdf:about=\"\"\n    \
       xmlns:exif=\"http://ns.adobe.com/exif/1.0/\"\n    \
       xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n    \
       xmlns:xmpMM=\"http://ns.adobe.com/xap/1.0/mm/\"\n    \
       xmlns:darktable=\"http://darktable.sf.net/\"\n    \
       xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n    \
       xmlns:lr=\"http://ns.adobe.com/lightroom/1.0/\"\n   \
      exif:DateTimeOriginal=\"2026:11:28 09:30:00\"\n   \
      xmp:Rating=\"1\"\n   \
      xmpMM:DerivedFrom=\"DSC_0001.NEF\"\n   \
      darktable:import_timestamp=\"63898765432000000\"\n   \
      darktable:change_timestamp=\"-1\"\n   \
      darktable:export_timestamp=\"-1\"\n   \
      darktable:print_timestamp=\"-1\"\n   \
      darktable:xmp_version=\"5\"\n   \
      darktable:raw_params=\"0\"\n   \
      darktable:auto_presets_applied=\"1\"\n   \
      darktable:history_end=\"0\"\n   \
      darktable:iop_order_version=\"4\">\n   \
      <darktable:masks_history>\n    <rdf:Seq/>\n   </darktable:masks_history>\n   \
      <darktable:history>\n    <rdf:Seq/>\n   </darktable:history>\n   \
      <dc:creator>\n    <rdf:Seq>\n     <rdf:li>Claude Monet</rdf:li>\n    </rdf:Seq>\n   </dc:creator>\n   \
      <dc:description>\n    <rdf:Alt>\n     <rdf:li xml:lang=\"x-default\">A harbor at sunrise</rdf:li>\n    </rdf:Alt>\n   </dc:description>\n   \
      <dc:rights>\n    <rdf:Alt>\n     <rdf:li xml:lang=\"x-default\">Public domain</rdf:li>\n    </rdf:Alt>\n   </dc:rights>\n   \
      <dc:subject>\n    <rdf:Bag>\n     <rdf:li>darktable|format|nef</rdf:li>\n     <rdf:li>harbor</rdf:li>\n     <rdf:li>sunrise</rdf:li>\n    </rdf:Bag>\n   </dc:subject>\n   \
      <lr:hierarchicalSubject>\n    <rdf:Bag>\n     <rdf:li>darktable|format|nef</rdf:li>\n    </rdf:Bag>\n   </lr:hierarchicalSubject>\n  \
     </rdf:Description>\n \
     </rdf:RDF>\n\
     </x:xmpmeta>\n"
        .to_owned()
}

/// A packet as Lightroom Classic writes it into a JPEG file's APP1
/// segment, padded with spaces inside the wrapper. It holds:
///
/// - simple values as attributes, among them Adobe Camera Raw's settings;
/// - the history, structures written as attributes of `rdf:li`;
/// - a structure written as attributes, and the tone curves.
pub fn lightroom_jpeg() -> Vec<u8> {
    let settings = [
        ("Version", "16.0"),
        ("ProcessVersion", "15.4"),
        ("WhiteBalance", "As Shot"),
        ("Exposure2012", "+0.50"),
        ("Contrast2012", "0"),
        ("Highlights2012", "-40"),
        ("Shadows2012", "+30"),
        ("Texture", "0"),
        ("Clarity2012", "+10"),
        ("Vibrance", "+15"),
        ("Saturation", "0"),
        ("ToneCurveName2012", "Linear"),
        ("CameraProfile", "Adobe Standard"),
        ("HasSettings", "True"),
        ("HasCrop", "False"),
        ("AlreadyApplied", "True"),
    ];
    let mut crs = String::new();
    for (name, value) in settings {
        crs.push_str(&format!("\n   crs:{name}=\"{value}\""));
    }
    let curve = |name: &str| {
        format!(
            "\n   <crs:{name}>\n    <rdf:Seq>\n     <rdf:li>0, 0</rdf:li>\n     \
             <rdf:li>255, 255</rdf:li>\n    </rdf:Seq>\n   </crs:{name}>"
        )
    };
    let curves: String = [
        "ToneCurvePV2012",
        "ToneCurvePV2012Red",
        "ToneCurvePV2012Green",
        "ToneCurvePV2012Blue",
    ]
    .iter()
    .map(|name| curve(name))
    .collect();
    let text = format!(
        "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Adobe XMP Core 7.0-c000 1.000000, 0000/00/00-00:00:00        \">\n \
         <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n  \
         <rdf:Description rdf:about=\"\"\n    \
           xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n    \
           xmlns:aux=\"http://ns.adobe.com/exif/1.0/aux/\"\n    \
           xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\"\n    \
           xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n    \
           xmlns:xmpMM=\"http://ns.adobe.com/xap/1.0/mm/\"\n    \
           xmlns:stEvt=\"http://ns.adobe.com/xap/1.0/sType/ResourceEvent#\"\n    \
           xmlns:stRef=\"http://ns.adobe.com/xap/1.0/sType/ResourceRef#\"\n    \
           xmlns:crs=\"http://ns.adobe.com/camera-raw-settings/1.0/\"\n   \
          xmp:CreateDate=\"2026-11-28T09:30:00.25\"\n   \
          xmp:CreatorTool=\"Adobe Photoshop Lightroom Classic 14.0 (Windows)\"\n   \
          xmp:ModifyDate=\"2026-11-28T10:00:00-03:00\"\n   \
          xmp:MetadataDate=\"2026-11-28T10:00:00-03:00\"\n   \
          xmp:Rating=\"3\"\n   \
          aux:Lens=\"NIKKOR Z 24-120mm f/4 S\"\n   \
          photoshop:DateCreated=\"2026-11-28T09:30:00.25\"\n   \
          dc:format=\"image/jpeg\"\n   \
          xmpMM:DocumentID=\"xmp.did:00000000-0000-0000-0000-000000000001\"\n   \
          xmpMM:OriginalDocumentID=\"00000000000000000000000000000001\"\n   \
          xmpMM:InstanceID=\"xmp.iid:00000000-0000-0000-0000-000000000002\"\n   \
          crs:RawFileName=\"DSC_0001.NEF\"{crs}>\n   \
          <dc:description>\n    <rdf:Alt>\n     <rdf:li xml:lang=\"x-default\">                               </rdf:li>\n    </rdf:Alt>\n   </dc:description>\n   \
          <xmpMM:History>\n    <rdf:Seq>\n     \
            <rdf:li\n      stEvt:action=\"derived\"\n      stEvt:parameters=\"converted from image/x-nikon-nef to image/jpeg, saved to new location\"/>\n     \
            <rdf:li\n      stEvt:action=\"saved\"\n      stEvt:instanceID=\"xmp.iid:00000000-0000-0000-0000-000000000002\"\n      \
              stEvt:when=\"2026-11-28T10:00:00-03:00\"\n      stEvt:softwareAgent=\"Adobe Photoshop Lightroom Classic 14.0 (Windows)\"\n      stEvt:changed=\"/\"/>\n    \
          </rdf:Seq>\n   </xmpMM:History>\n   \
          <xmpMM:DerivedFrom\n    stRef:documentID=\"00000000000000000000000000000001\"\n    \
            stRef:originalDocumentID=\"00000000000000000000000000000001\"/>{curves}\n  \
         </rdf:Description>\n \
         </rdf:RDF>\n\
         </x:xmpmeta>\n"
    );
    let mut bytes = text.into_bytes();
    for _ in 0..20 {
        bytes.extend_from_slice(&[b' '; 99]);
        bytes.push(b'\n');
    }
    bytes.extend_from_slice(b"<?xpacket end=\"w\"?>");
    bytes
}
