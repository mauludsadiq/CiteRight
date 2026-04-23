use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

pub fn read_document(path: &Path) -> Result<String> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "md" => std::fs::read_to_string(path).with_context(|| format!("read text input {}", path.display())),
        "docx" => read_docx(path),
        "pdf" => read_pdf(path),
        other => anyhow::bail!("unsupported input extension '{}'; supported: txt, md, docx, pdf", other),
    }
}

fn read_docx(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("open docx {}", path.display()))?;
    let mut zip = ZipArchive::new(file).context("open docx zip container")?;
    let mut xml = String::new();
    zip.by_name("word/document.xml")?.read_to_string(&mut xml)?;
    let mut reader = Reader::from_str(&xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut out = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => out.push_str(&e.unescape()?.to_string()),
            Ok(Event::Start(e)) if e.name().as_ref() == b"w:p" => out.push('\n'),
            Ok(Event::Eof) => break,
            Err(e) => anyhow::bail!("docx xml parse error: {e}"),
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

fn read_pdf(path: &Path) -> Result<String> {
    let doc = lopdf::Document::load(path).with_context(|| format!("load pdf {}", path.display()))?;
    let mut text = String::new();
    for (page_no, _page_id) in doc.get_pages() {
        match doc.extract_text(&[page_no]) {
            Ok(t) => {
                text.push_str(&t);
                text.push('\n');
            }
            Err(e) => text.push_str(&format!("\n[page {page_no}: text extraction failed: {e}]\n")),
        }
    }
    Ok(text)
}
