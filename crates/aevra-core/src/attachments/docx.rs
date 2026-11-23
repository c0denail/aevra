use crate::{Error, Result};
#[derive(Default)]
struct Node {
    name: String,
    text: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}
impl Node {
    fn local(&self) -> &str {
        self.name.rsplit(':').next().unwrap_or(&self.name)
    }
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|n| n.local() == name)
    }
    fn val(&self) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k.rsplit(':').next() == Some("val"))
            .map(|(_, v)| v.as_str())
    }
    fn math_child(&self, name: &str) -> String {
        self.child(name).map(math).unwrap_or_default()
    }
}
fn invalid() -> Error {
    Error::Validation("Word DOCX içeriği okunamadı.".into())
}
fn node(e: &quick_xml::events::BytesStart<'_>) -> Result<Node> {
    let mut node = Node {
        name: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
        ..Node::default()
    };
    for a in e.attributes() {
        let a = a.map_err(|_| invalid())?;
        node.attrs.push((
            String::from_utf8_lossy(a.key.as_ref()).into_owned(),
            a.unescape_value().map_err(|_| invalid())?.into_owned(),
        ));
    }
    Ok(node)
}
pub fn text(bytes: &[u8]) -> Result<String> {
    let mut reader = quick_xml::Reader::from_reader(bytes);
    let mut stack = vec![Node::default()];
    let mut count = 0;
    loop {
        use quick_xml::events::Event;
        match reader.read_event().map_err(|_| invalid())? {
            Event::Start(e) => {
                count += 1;
                if count > 200000 || stack.len() > 128 {
                    return Err(invalid());
                }
                stack.push(node(&e)?);
            }
            Event::Empty(e) => {
                count += 1;
                if count > 200000 {
                    return Err(invalid());
                }
                stack
                    .last_mut()
                    .ok_or_else(invalid)?
                    .children
                    .push(node(&e)?);
            }
            Event::End(_) => {
                if stack.len() < 2 {
                    return Err(invalid());
                }
                let node = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(node);
            }
            Event::Text(e) => stack
                .last_mut()
                .ok_or_else(invalid)?
                .text
                .push_str(&e.xml_content().map_err(|_| invalid())?),
            Event::GeneralRef(e) => {
                let name = e.decode().map_err(|_| invalid())?;
                let s = format!("&{name};");
                let v = quick_xml::escape::unescape(&s).map_err(|_| invalid())?;
                stack.last_mut().ok_or_else(invalid)?.text.push_str(&v);
            }
            Event::DocType(_) => return Err(invalid()),
            Event::Eof => break,
            _ => (),
        }
    }
    if stack.len() != 1 {
        return Err(invalid());
    }
    let mut out = String::new();
    extract(&stack[0], &mut out);
    Ok(out)
}
fn extract(node: &Node, out: &mut String) {
    if node.local() == "oMath" {
        let tex = math(node);
        if !tex.trim().is_empty() {
            out.push_str(&format!(" ${tex}$ "));
        }
        return;
    }
    if node.local() == "t" {
        out.push_str(&node.text);
        return;
    }
    if node.local() == "tab" {
        out.push('\t');
    }
    if node.local() == "br" {
        out.push('\n');
    }
    for child in &node.children {
        extract(child, out);
    }
    if node.local() == "p" {
        out.push('\n');
    }
}
fn symbols(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'α' => "\\alpha ".into(),
            'β' => "\\beta ".into(),
            'γ' => "\\gamma ".into(),
            'δ' => "\\delta ".into(),
            'θ' => "\\theta ".into(),
            'λ' => "\\lambda ".into(),
            'μ' => "\\mu ".into(),
            'π' => "\\pi ".into(),
            'σ' => "\\sigma ".into(),
            'φ' => "\\phi ".into(),
            'ω' => "\\omega ".into(),
            '∞' => "\\infty ".into(),
            '≤' => "\\le ".into(),
            '≥' => "\\ge ".into(),
            '≠' => "\\ne ".into(),
            '±' => "\\pm ".into(),
            '×' => "\\times ".into(),
            '∫' => "\\int ".into(),
            '∑' => "\\sum ".into(),
            '∏' => "\\prod ".into(),
            '²' => "^{2}".into(),
            '³' => "^{3}".into(),
            '_' => "\\_".into(),
            '{' => "\\{".into(),
            '}' => "\\}".into(),
            _ => c.to_string(),
        })
        .collect()
}
fn math(node: &Node) -> String {
    match node.local() {
        "t" => symbols(&node.text),
        "f" => format!(
            "\\frac{{{}}}{{{}}}",
            node.math_child("num"),
            node.math_child("den")
        ),
        "rad" => {
            let deg = node.math_child("deg");
            let value = node.math_child("e");
            if deg.is_empty() {
                format!("\\sqrt{{{value}}}")
            } else {
                format!("\\sqrt[{deg}]{{{value}}}")
            }
        }
        "sSup" => format!(
            "{{{}}}^{{{}}}",
            node.math_child("e"),
            node.math_child("sup")
        ),
        "sSub" => format!(
            "{{{}}}_{{{}}}",
            node.math_child("e"),
            node.math_child("sub")
        ),
        "sSubSup" => format!(
            "{{{}}}_{{{}}}^{{{}}}",
            node.math_child("e"),
            node.math_child("sub"),
            node.math_child("sup")
        ),
        "nary" => {
            let symbol = node
                .child("naryPr")
                .and_then(|n| n.child("chr"))
                .and_then(Node::val)
                .unwrap_or("∫");
            format!(
                "{} _{{{}}}^{{{}}} {}",
                symbols(symbol),
                node.math_child("sub"),
                node.math_child("sup"),
                node.math_child("e")
            )
        }
        "m" => format!(
            "\\begin{{matrix}}{}\\end{{matrix}}",
            node.children
                .iter()
                .filter(|n| n.local() == "mr")
                .map(|r| r
                    .children
                    .iter()
                    .filter(|n| n.local() == "e")
                    .map(math)
                    .collect::<Vec<_>>()
                    .join(" & "))
                .collect::<Vec<_>>()
                .join(" \\\\ ")
        ),
        "func" => format!(
            "\\operatorname{{{}}}{{{}}}",
            node.math_child("fName"),
            node.math_child("e")
        ),
        "limLow" => format!(
            "\\underset{{{}}}{{{}}}",
            node.math_child("lim"),
            node.math_child("e")
        ),
        "limUpp" => format!(
            "\\overset{{{}}}{{{}}}",
            node.math_child("lim"),
            node.math_child("e")
        ),
        "d" => format!(
            "\\left({}\\right)",
            node.children
                .iter()
                .filter(|n| n.local() == "e")
                .map(math)
                .collect::<Vec<_>>()
                .join(",")
        ),
        name if name.ends_with("Pr") => String::new(),
        _ => node.children.iter().map(math).collect(),
    }
}
