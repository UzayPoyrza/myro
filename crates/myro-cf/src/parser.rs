use crate::types::{ProblemStatement, TestExample};
use anyhow::{bail, Result};
use scraper::{Html, Selector};

/// Parse a Codeforces problem page HTML into a structured ProblemStatement.
pub fn parse_problem(html: &str, contest_id: i64, index: &str) -> Result<ProblemStatement> {
    let doc = Html::parse_document(html);

    let problem_stmt = sel(".problem-statement");

    // Check that the problem statement exists
    if doc.select(&problem_stmt).next().is_none() {
        bail!("No .problem-statement found in HTML");
    }

    let title = extract_text(&doc, ".problem-statement .header .title")
        .unwrap_or_default()
        .trim()
        .to_string();

    let time_limit = extract_text(&doc, ".problem-statement .header .time-limit")
        .map(|s| s.replace("time limit per test", "").trim().to_string())
        .unwrap_or_default();

    let memory_limit = extract_text(&doc, ".problem-statement .header .memory-limit")
        .map(|s| s.replace("memory limit per test", "").trim().to_string())
        .unwrap_or_default();

    // Description: the div(s) between header and input-specification
    let description = extract_section_html(&doc, ".problem-statement .header")
        .map(|_| {
            // Get all direct children divs of .problem-statement that aren't named sections
            extract_description(&doc)
        })
        .unwrap_or_default();

    let input_spec =
        extract_section_text(&doc, ".problem-statement .input-specification").unwrap_or_default();

    let output_spec =
        extract_section_text(&doc, ".problem-statement .output-specification").unwrap_or_default();

    let examples = extract_examples(&doc);

    let note = extract_section_text(&doc, ".problem-statement .note");

    Ok(ProblemStatement {
        contest_id,
        index: index.to_string(),
        title,
        time_limit,
        memory_limit,
        description,
        input_spec,
        output_spec,
        examples,
        note,
    })
}

fn sel(s: &str) -> Selector {
    Selector::parse(s).unwrap()
}

fn extract_text(doc: &Html, selector: &str) -> Option<String> {
    let s = sel(selector);
    doc.select(&s).next().map(|el| {
        el.text()
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string()
    })
}

fn extract_section_html(doc: &Html, selector: &str) -> Option<String> {
    let s = sel(selector);
    doc.select(&s)
        .next()
        .map(|el| el.inner_html().trim().to_string())
}

fn extract_section_text(doc: &Html, selector: &str) -> Option<String> {
    let s = sel(selector);
    doc.select(&s).next().map(|el| {
        let mut text = String::new();
        collect_text_with_breaks(&el, &mut text);
        // Remove the section title (first line like "Input" or "Output" or "Note")
        let trimmed = text.trim().to_string();
        if let Some(rest) = trimmed
            .strip_prefix("Input")
            .or_else(|| trimmed.strip_prefix("Output"))
            .or_else(|| trimmed.strip_prefix("Note"))
        {
            rest.trim().to_string()
        } else {
            trimmed
        }
    })
}

fn extract_description(doc: &Html) -> String {
    // The description is typically in divs directly after .header inside .problem-statement
    // We look for div children of .problem-statement that are not known sections
    let ps = sel(".problem-statement");
    let ps_el = match doc.select(&ps).next() {
        Some(el) => el,
        None => return String::new(),
    };

    let known_classes = [
        "header",
        "input-specification",
        "output-specification",
        "sample-tests",
        "note",
    ];

    let mut parts = Vec::new();
    for child in ps_el.children() {
        if let Some(el) = child.value().as_element() {
            let classes: Vec<&str> = el.classes().collect();
            let is_known = classes.iter().any(|c| known_classes.contains(c));
            if !is_known {
                let child_ref = scraper::ElementRef::wrap(child).unwrap();
                let mut text = String::new();
                collect_text_with_breaks(&child_ref, &mut text);
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    parts.push(trimmed);
                }
            }
        }
    }

    parts.join("\n\n")
}

fn collect_text_with_breaks(el: &scraper::ElementRef, out: &mut String) {
    for child in el.children() {
        match child.value() {
            scraper::Node::Text(t) => {
                out.push_str(t.trim());
                if !t.trim().is_empty() {
                    out.push(' ');
                }
            }
            scraper::Node::Element(e) => {
                let tag = e.name();
                if tag == "br" {
                    out.push('\n');
                } else if tag == "p" || tag == "div" {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                    if let Some(child_ref) = scraper::ElementRef::wrap(child) {
                        collect_text_with_breaks(&child_ref, out);
                    }
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                } else if tag == "pre" {
                    // Preserve whitespace in <pre> blocks
                    if let Some(child_ref) = scraper::ElementRef::wrap(child) {
                        let pre_text: String = child_ref.text().collect();
                        out.push_str(pre_text.trim());
                        out.push('\n');
                    }
                } else if tag == "ul" || tag == "ol" {
                    if let Some(child_ref) = scraper::ElementRef::wrap(child) {
                        collect_text_with_breaks(&child_ref, out);
                    }
                } else if tag == "li" {
                    out.push_str("  - ");
                    if let Some(child_ref) = scraper::ElementRef::wrap(child) {
                        collect_text_with_breaks(&child_ref, out);
                    }
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                } else if let Some(child_ref) = scraper::ElementRef::wrap(child) {
                    collect_text_with_breaks(&child_ref, out);
                }
            }
            _ => {}
        }
    }
}

/// Extract text from a `<pre>` element, handling three CF formats:
/// 1. Plain text nodes with literal newlines
/// 2. `<br>` elements as line separators
/// 3. Newer CF format: each line wrapped in `<div class="test-example-line ...">`
fn extract_pre_text(el: scraper::ElementRef<'_>) -> String {
    let mut text = String::new();
    for child in el.children() {
        match child.value() {
            scraper::Node::Text(t) => text.push_str(t),
            scraper::Node::Element(e) if e.name() == "br" => text.push('\n'),
            scraper::Node::Element(_) => {
                if let Some(child_ref) = scraper::ElementRef::wrap(child) {
                    let t: String = child_ref.text().collect();
                    if !t.is_empty() {
                        text.push_str(&t);
                        if !text.ends_with('\n') {
                            text.push('\n');
                        }
                    }
                }
            }
            _ => {}
        }
    }
    text.trim().to_string()
}

fn extract_examples(doc: &Html) -> Vec<TestExample> {
    let input_sel = sel(".problem-statement .sample-tests .input pre");
    let output_sel = sel(".problem-statement .sample-tests .output pre");

    let inputs: Vec<String> = doc
        .select(&input_sel)
        .map(|el| extract_pre_text(el))
        .collect();

    let outputs: Vec<String> = doc
        .select(&output_sel)
        .map(|el| extract_pre_text(el))
        .collect();

    inputs
        .into_iter()
        .zip(outputs)
        .map(|(input, output)| TestExample { input, output })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cf_html(description: &str, input_spec: &str, output_spec: &str) -> String {
        format!(
            r#"<html><body>
            <div class="problem-statement">
                <div class="header">
                    <div class="title">A. Test Problem</div>
                    <div class="time-limit">time limit per test2 seconds</div>
                    <div class="memory-limit">memory limit per test256 megabytes</div>
                </div>
                <div>{description}</div>
                <div class="input-specification"><div class="section-title">Input</div>{input_spec}</div>
                <div class="output-specification"><div class="section-title">Output</div>{output_spec}</div>
                <div class="sample-tests">
                    <div class="input"><pre>3</pre></div>
                    <div class="output"><pre>YES</pre></div>
                </div>
            </div>
            </body></html>"#
        )
    }

    #[test]
    fn parser_preserves_dollar_delimiters() {
        let html = make_cf_html(
            "<p>You are given an integer $$$n$$$ ($$$1 \\le n \\le 10^5$$$). Find $$$\\frac{n}{2}$$$.</p>",
            "<p>The first line contains $$$t$$$ ($$$1 \\le t \\le 1000$$$) — the number of test cases.</p>",
            "<p>Print $$$n$$$ integers.</p>",
        );
        let ps = parse_problem(&html, 42, "A").unwrap();
        assert!(ps.description.contains("$$$"), "description should have $$$ delimiters");
        assert!(ps.input_spec.contains("$$$"), "input_spec should have $$$ delimiters");
        assert!(ps.output_spec.contains("$$$"), "output_spec should have $$$ delimiters");
    }

    #[test]
    fn parser_styled_segments_work() {
        let html = make_cf_html(
            "<p>Given $$$n$$$ and $$$k$$$, find the answer.</p>",
            "<p>One integer $$$n$$$.</p>",
            "<p>One integer.</p>",
        );
        let ps = parse_problem(&html, 42, "A").unwrap();
        let segments = crate::latex::convert_cf_latex_styled(&ps.description);
        let math_count = segments.iter().filter(|s| s.is_math).count();
        assert!(math_count >= 2, "should have at least 2 math segments, got {}", math_count);
    }

    #[test]
    fn parser_complex_math_renders() {
        let html = make_cf_html(
            "<p>Find $$$\\frac{n}{2}$$$ where $$$1 \\le n \\le 10^5$$$.</p>",
            "<p>Input $$$t$$$ tests.</p>",
            "<p>Print answer.</p>",
        );
        let ps = parse_problem(&html, 42, "A").unwrap();
        let segments = crate::latex::convert_cf_latex_styled(&ps.description);
        let math_texts: Vec<&str> = segments.iter().filter(|s| s.is_math).map(|s| s.text.as_str()).collect();
        assert!(math_texts.iter().any(|t| t.contains("≤")), "should convert \\le to ≤");
        assert!(math_texts.iter().any(|t| t.contains("/")), "should convert \\frac to /");
    }

    #[test]
    fn parser_example_multiline_div_format() {
        // Newer CF format: each line in <pre> wrapped in <div class="test-example-line ...">
        let html = format!(
            r#"<html><body>
            <div class="problem-statement">
                <div class="header">
                    <div class="title">D. Test</div>
                    <div class="time-limit">time limit per test2 seconds</div>
                    <div class="memory-limit">memory limit per test256 megabytes</div>
                </div>
                <div><p>desc</p></div>
                <div class="input-specification"><div class="section-title">Input</div><p>spec</p></div>
                <div class="output-specification"><div class="section-title">Output</div><p>spec</p></div>
                <div class="sample-tests">
                    <div class="input"><pre><div class="test-example-line test-example-line-even">4</div><div class="test-example-line test-example-line-odd">1 2 4 8</div></pre></div>
                    <div class="output"><pre><div class="test-example-line test-example-line-even">2</div></pre></div>
                </div>
            </div>
            </body></html>"#
        );
        let ps = parse_problem(&html, 1950, "D").unwrap();
        assert_eq!(ps.examples.len(), 1);
        assert_eq!(ps.examples[0].input, "4\n1 2 4 8");
        assert_eq!(ps.examples[0].output, "2");
    }

    #[test]
    fn parser_example_br_format() {
        // Older CF format: lines separated by <br>
        let html = format!(
            r#"<html><body>
            <div class="problem-statement">
                <div class="header">
                    <div class="title">A. Test</div>
                    <div class="time-limit">time limit per test2 seconds</div>
                    <div class="memory-limit">memory limit per test256 megabytes</div>
                </div>
                <div><p>desc</p></div>
                <div class="input-specification"><div class="section-title">Input</div><p>spec</p></div>
                <div class="output-specification"><div class="section-title">Output</div><p>spec</p></div>
                <div class="sample-tests">
                    <div class="input"><pre>4<br>1 2 4 8</pre></div>
                    <div class="output"><pre>2</pre></div>
                </div>
            </div>
            </body></html>"#
        );
        let ps = parse_problem(&html, 1950, "A").unwrap();
        assert_eq!(ps.examples[0].input, "4\n1 2 4 8");
    }
}

