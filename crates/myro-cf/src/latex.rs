/// A segment of text with a flag indicating whether it's math or prose.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledSegment {
    pub text: String,
    pub is_math: bool,
}

/// Convert Codeforces-style LaTeX math (`$$$...$$$`) to styled Unicode segments.
/// Math segments get `is_math: true` so the TUI can render them in a distinct color.
pub fn convert_cf_latex_styled(text: &str) -> Vec<StyledSegment> {
    let mut segments = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find("$$$") {
        // Prose before math
        if start > 0 {
            segments.push(StyledSegment {
                text: rest[..start].to_string(),
                is_math: false,
            });
        }
        let after_open = &rest[start + 3..];
        if let Some(end) = after_open.find("$$$") {
            let latex = &after_open[..end];
            segments.push(StyledSegment {
                text: latex_to_unicode(latex),
                is_math: true,
            });
            rest = &after_open[end + 3..];
        } else {
            // Unclosed delimiter — keep as prose
            segments.push(StyledSegment {
                text: "$$$".to_string(),
                is_math: false,
            });
            rest = after_open;
        }
    }
    if !rest.is_empty() {
        segments.push(StyledSegment {
            text: rest.to_string(),
            is_math: false,
        });
    }
    segments
}

/// Convert Codeforces-style LaTeX math (`$$$...$$$`) to Unicode for terminal display.
pub fn convert_cf_latex(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find("$$$") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 3..];
        if let Some(end) = after_open.find("$$$") {
            let latex = &after_open[..end];
            result.push_str(&latex_to_unicode(latex));
            rest = &after_open[end + 3..];
        } else {
            // Unclosed delimiter — keep as-is
            result.push_str("$$$");
            rest = after_open;
        }
    }
    result.push_str(rest);
    result
}

fn latex_to_unicode(latex: &str) -> String {
    let chars: Vec<char> = latex.chars().collect();
    let mut out = String::new();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '\\' => {
                let cmd = read_command(&chars, i + 1);
                if cmd.is_empty() {
                    // Escaped character like \{ \} \  \, etc.
                    if i + 1 < chars.len() {
                        match chars[i + 1] {
                            '{' | '}' => {
                                out.push(chars[i + 1]);
                                i += 2;
                            }
                            ',' | ';' | '!' => {
                                out.push(' ');
                                i += 2;
                            }
                            ' ' => {
                                out.push(' ');
                                i += 2;
                            }
                            '\\' => {
                                out.push('\n');
                                i += 2;
                            }
                            _ => {
                                out.push('\\');
                                i += 1;
                            }
                        }
                    } else {
                        out.push('\\');
                        i += 1;
                    }
                } else {
                    let cmd_end = i + 1 + cmd.len();
                    i = handle_command(&cmd, &chars, cmd_end, &mut out);
                }
            }
            '^' => {
                let (content, next) = read_arg(&chars, i + 1);
                let inner = latex_to_unicode(&content);
                if let Some(sup) = try_superscript(&inner) {
                    out.push_str(&sup);
                } else {
                    out.push('^');
                    if inner.chars().count() > 1 {
                        out.push('(');
                        out.push_str(&inner);
                        out.push(')');
                    } else {
                        out.push_str(&inner);
                    }
                }
                i = next;
            }
            '_' => {
                let (content, next) = read_arg(&chars, i + 1);
                let inner = latex_to_unicode(&content);
                if let Some(sub) = try_subscript(&inner) {
                    out.push_str(&sub);
                } else {
                    out.push('_');
                    if inner.chars().count() > 1 {
                        out.push('(');
                        out.push_str(&inner);
                        out.push(')');
                    } else {
                        out.push_str(&inner);
                    }
                }
                i = next;
            }
            ' ' => {
                // Collapse whitespace
                if !out.ends_with(' ') {
                    out.push(' ');
                }
                i += 1;
            }
            _ => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

/// Read an alphabetic command name starting at position `start`.
fn read_command(chars: &[char], start: usize) -> String {
    let mut cmd = String::new();
    let mut i = start;
    while i < chars.len() && chars[i].is_ascii_alphabetic() {
        cmd.push(chars[i]);
        i += 1;
    }
    cmd
}

/// Read a brace-delimited `{...}` argument or a single character.
/// Returns (content, next_index).
fn read_arg(chars: &[char], start: usize) -> (String, usize) {
    if start >= chars.len() {
        return (String::new(), start);
    }
    if chars[start] == '{' {
        let mut depth = 1;
        let mut i = start + 1;
        let mut content = String::new();
        while i < chars.len() && depth > 0 {
            if chars[i] == '{' {
                depth += 1;
                if depth > 1 {
                    content.push('{');
                }
            } else if chars[i] == '}' {
                depth -= 1;
                if depth > 0 {
                    content.push('}');
                }
            } else {
                content.push(chars[i]);
            }
            i += 1;
        }
        (content, i)
    } else {
        (chars[start].to_string(), start + 1)
    }
}

/// Skip optional whitespace and read a brace arg if present; used for commands
/// that take an argument like \frac{a}{b}.
fn read_arg_skip_ws(chars: &[char], start: usize) -> (String, usize) {
    let mut i = start;
    while i < chars.len() && chars[i] == ' ' {
        i += 1;
    }
    read_arg(chars, i)
}

fn handle_command(cmd: &str, chars: &[char], cmd_end: usize, out: &mut String) -> usize {
    match cmd {
        // Comparison / relations
        "le" | "leq" => {
            out.push('≤');
            cmd_end
        }
        "ge" | "geq" => {
            out.push('≥');
            cmd_end
        }
        "ne" | "neq" => {
            out.push('≠');
            cmd_end
        }
        "lt" => {
            out.push('<');
            cmd_end
        }
        "gt" => {
            out.push('>');
            cmd_end
        }
        "approx" => {
            out.push('≈');
            cmd_end
        }
        "equiv" => {
            out.push('≡');
            cmd_end
        }
        "sim" => {
            out.push('∼');
            cmd_end
        }

        // Arrows
        "to" | "rightarrow" => {
            out.push('→');
            cmd_end
        }
        "leftarrow" => {
            out.push('←');
            cmd_end
        }
        "leftrightarrow" => {
            out.push('↔');
            cmd_end
        }
        "Rightarrow" => {
            out.push('⇒');
            cmd_end
        }
        "Leftarrow" => {
            out.push('⇐');
            cmd_end
        }
        "Leftrightarrow" => {
            out.push('⇔');
            cmd_end
        }
        "uparrow" => {
            out.push('↑');
            cmd_end
        }
        "downarrow" => {
            out.push('↓');
            cmd_end
        }

        // Binary operators
        "cdot" => {
            out.push('·');
            cmd_end
        }
        "times" => {
            out.push('×');
            cmd_end
        }
        "div" => {
            out.push('÷');
            cmd_end
        }
        "pm" => {
            out.push('±');
            cmd_end
        }
        "mp" => {
            out.push('∓');
            cmd_end
        }
        "oplus" => {
            out.push('⊕');
            cmd_end
        }
        "otimes" => {
            out.push('⊗');
            cmd_end
        }
        "circ" => {
            out.push('∘');
            cmd_end
        }
        "ast" | "star" => {
            out.push('∗');
            cmd_end
        }

        // Set theory / logic
        "in" => {
            out.push('∈');
            cmd_end
        }
        "notin" => {
            out.push('∉');
            cmd_end
        }
        "subset" => {
            out.push('⊂');
            cmd_end
        }
        "subseteq" => {
            out.push('⊆');
            cmd_end
        }
        "supset" => {
            out.push('⊃');
            cmd_end
        }
        "supseteq" => {
            out.push('⊇');
            cmd_end
        }
        "cup" => {
            out.push('∪');
            cmd_end
        }
        "cap" => {
            out.push('∩');
            cmd_end
        }
        "emptyset" | "varnothing" => {
            out.push('∅');
            cmd_end
        }
        "forall" => {
            out.push('∀');
            cmd_end
        }
        "exists" => {
            out.push('∃');
            cmd_end
        }
        "neg" | "lnot" => {
            out.push('¬');
            cmd_end
        }
        "land" | "wedge" => {
            out.push('∧');
            cmd_end
        }
        "lor" | "vee" => {
            out.push('∨');
            cmd_end
        }

        // Big operators
        "sum" => {
            out.push('Σ');
            cmd_end
        }
        "prod" => {
            out.push('Π');
            cmd_end
        }
        "int" => {
            out.push('∫');
            cmd_end
        }
        "infty" => {
            out.push('∞');
            cmd_end
        }
        "partial" => {
            out.push('∂');
            cmd_end
        }
        "nabla" => {
            out.push('∇');
            cmd_end
        }

        // Dots
        "dots" | "ldots" | "cdots" | "hdots" => {
            out.push('…');
            cmd_end
        }
        "vdots" => {
            out.push('⋮');
            cmd_end
        }
        "ddots" => {
            out.push('⋱');
            cmd_end
        }

        // Delimiters
        "lfloor" => {
            out.push('⌊');
            cmd_end
        }
        "rfloor" => {
            out.push('⌋');
            cmd_end
        }
        "lceil" => {
            out.push('⌈');
            cmd_end
        }
        "rceil" => {
            out.push('⌉');
            cmd_end
        }
        "lvert" | "vert" => {
            out.push('|');
            cmd_end
        }
        "rvert" => {
            out.push('|');
            cmd_end
        }
        "langle" => {
            out.push('⟨');
            cmd_end
        }
        "rangle" => {
            out.push('⟩');
            cmd_end
        }
        "left" | "right" | "big" | "Big" | "bigg" | "Bigg" | "middle" => {
            // Size modifiers — skip, let the next delimiter render naturally
            cmd_end
        }

        // Misc symbols
        "sqrt" => {
            let (arg, next) = read_arg_skip_ws(chars, cmd_end);
            out.push('√');
            let inner = latex_to_unicode(&arg);
            if inner.len() > 1 {
                out.push('(');
                out.push_str(&inner);
                out.push(')');
            } else {
                out.push_str(&inner);
            }
            next
        }

        // Fractions
        "frac" | "dfrac" | "tfrac" => {
            let (num, after_num) = read_arg_skip_ws(chars, cmd_end);
            let (den, after_den) = read_arg_skip_ws(chars, after_num);
            let num_u = latex_to_unicode(&num);
            let den_u = latex_to_unicode(&den);
            // Simple fraction: if both are short, use a/b; otherwise (a)/(b)
            if num_u.chars().count() <= 1 && den_u.chars().count() <= 1 {
                out.push_str(&num_u);
                out.push('/');
                out.push_str(&den_u);
            } else {
                out.push('(');
                out.push_str(&num_u);
                out.push_str(")/(");
                out.push_str(&den_u);
                out.push(')');
            }
            after_den
        }

        // Modular arithmetic
        "bmod" | "mod" | "pmod" => {
            out.push_str(" mod ");
            cmd_end
        }

        // Text-style commands — just render content
        "text" | "texttt" | "textrm" | "textbf" | "textit" | "textsf" | "mathrm" | "mathit"
        | "mathbf" | "mathsf" | "mathtt" | "operatorname" | "mathcal" | "mathbb" => {
            let (arg, next) = read_arg_skip_ws(chars, cmd_end);
            out.push_str(&latex_to_unicode(&arg));
            next
        }
        "overline" | "bar" => {
            let (arg, next) = read_arg_skip_ws(chars, cmd_end);
            out.push_str(&latex_to_unicode(&arg));
            out.push('\u{0305}'); // combining overline
            next
        }
        "hat" => {
            let (arg, next) = read_arg_skip_ws(chars, cmd_end);
            out.push_str(&latex_to_unicode(&arg));
            out.push('\u{0302}'); // combining circumflex
            next
        }
        "tilde" => {
            let (arg, next) = read_arg_skip_ws(chars, cmd_end);
            out.push_str(&latex_to_unicode(&arg));
            out.push('\u{0303}'); // combining tilde
            next
        }
        "vec" => {
            let (arg, next) = read_arg_skip_ws(chars, cmd_end);
            out.push_str(&latex_to_unicode(&arg));
            out.push('\u{20D7}'); // combining right arrow above
            next
        }
        "underline" => {
            let (arg, next) = read_arg_skip_ws(chars, cmd_end);
            out.push_str(&latex_to_unicode(&arg));
            next
        }

        // Greek lowercase
        "alpha" => {
            out.push('α');
            cmd_end
        }
        "beta" => {
            out.push('β');
            cmd_end
        }
        "gamma" => {
            out.push('γ');
            cmd_end
        }
        "delta" => {
            out.push('δ');
            cmd_end
        }
        "epsilon" | "varepsilon" => {
            out.push('ε');
            cmd_end
        }
        "zeta" => {
            out.push('ζ');
            cmd_end
        }
        "eta" => {
            out.push('η');
            cmd_end
        }
        "theta" | "vartheta" => {
            out.push('θ');
            cmd_end
        }
        "iota" => {
            out.push('ι');
            cmd_end
        }
        "kappa" => {
            out.push('κ');
            cmd_end
        }
        "lambda" => {
            out.push('λ');
            cmd_end
        }
        "mu" => {
            out.push('μ');
            cmd_end
        }
        "nu" => {
            out.push('ν');
            cmd_end
        }
        "xi" => {
            out.push('ξ');
            cmd_end
        }
        "pi" | "varpi" => {
            out.push('π');
            cmd_end
        }
        "rho" | "varrho" => {
            out.push('ρ');
            cmd_end
        }
        "sigma" | "varsigma" => {
            out.push('σ');
            cmd_end
        }
        "tau" => {
            out.push('τ');
            cmd_end
        }
        "upsilon" => {
            out.push('υ');
            cmd_end
        }
        "phi" | "varphi" => {
            out.push('φ');
            cmd_end
        }
        "chi" => {
            out.push('χ');
            cmd_end
        }
        "psi" => {
            out.push('ψ');
            cmd_end
        }
        "omega" => {
            out.push('ω');
            cmd_end
        }

        // Greek uppercase
        "Gamma" => {
            out.push('Γ');
            cmd_end
        }
        "Delta" => {
            out.push('Δ');
            cmd_end
        }
        "Theta" => {
            out.push('Θ');
            cmd_end
        }
        "Lambda" => {
            out.push('Λ');
            cmd_end
        }
        "Xi" => {
            out.push('Ξ');
            cmd_end
        }
        "Pi" => {
            out.push('Π');
            cmd_end
        }
        "Sigma" => {
            out.push('Σ');
            cmd_end
        }
        "Phi" => {
            out.push('Φ');
            cmd_end
        }
        "Psi" => {
            out.push('Ψ');
            cmd_end
        }
        "Omega" => {
            out.push('Ω');
            cmd_end
        }

        // Spacing
        "quad" => {
            out.push_str("  ");
            cmd_end
        }
        "qquad" => {
            out.push_str("    ");
            cmd_end
        }

        // Unknown command — output the name as-is
        _ => {
            out.push_str(cmd);
            cmd_end
        }
    }
}

/// Try to convert every character in `s` to a Unicode superscript.
/// Returns `None` if any character lacks a superscript form.
fn try_superscript(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match superscript_char(c) {
            Some(sc) => out.push(sc),
            None => return None,
        }
    }
    Some(out)
}

fn superscript_char(c: char) -> Option<char> {
    match c {
        '0' => Some('⁰'),
        '1' => Some('¹'),
        '2' => Some('²'),
        '3' => Some('³'),
        '4' => Some('⁴'),
        '5' => Some('⁵'),
        '6' => Some('⁶'),
        '7' => Some('⁷'),
        '8' => Some('⁸'),
        '9' => Some('⁹'),
        '+' => Some('⁺'),
        '-' | '−' => Some('⁻'),
        '=' => Some('⁼'),
        '(' => Some('⁽'),
        ')' => Some('⁾'),
        'a' => Some('ᵃ'),
        'b' => Some('ᵇ'),
        'c' => Some('ᶜ'),
        'd' => Some('ᵈ'),
        'e' => Some('ᵉ'),
        'f' => Some('ᶠ'),
        'g' => Some('ᵍ'),
        'h' => Some('ʰ'),
        'i' => Some('ⁱ'),
        'j' => Some('ʲ'),
        'k' => Some('ᵏ'),
        'l' => Some('ˡ'),
        'm' => Some('ᵐ'),
        'n' => Some('ⁿ'),
        'o' => Some('ᵒ'),
        'p' => Some('ᵖ'),
        'r' => Some('ʳ'),
        's' => Some('ˢ'),
        't' => Some('ᵗ'),
        'u' => Some('ᵘ'),
        'v' => Some('ᵛ'),
        'w' => Some('ʷ'),
        'x' => Some('ˣ'),
        'y' => Some('ʸ'),
        'z' => Some('ᶻ'),
        _ => None,
    }
}

/// Try to convert every character in `s` to a Unicode subscript.
/// Returns `None` if any character lacks a subscript form.
fn try_subscript(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match subscript_char(c) {
            Some(sc) => out.push(sc),
            None => return None,
        }
    }
    Some(out)
}

fn subscript_char(c: char) -> Option<char> {
    match c {
        '0' => Some('₀'),
        '1' => Some('₁'),
        '2' => Some('₂'),
        '3' => Some('₃'),
        '4' => Some('₄'),
        '5' => Some('₅'),
        '6' => Some('₆'),
        '7' => Some('₇'),
        '8' => Some('₈'),
        '9' => Some('₉'),
        '+' => Some('₊'),
        '-' | '−' => Some('₋'),
        '=' => Some('₌'),
        '(' => Some('₍'),
        ')' => Some('₎'),
        'a' => Some('ₐ'),
        'e' => Some('ₑ'),
        'h' => Some('ₕ'),
        'i' => Some('ᵢ'),
        'j' => Some('ⱼ'),
        'k' => Some('ₖ'),
        'l' => Some('ₗ'),
        'm' => Some('ₘ'),
        'n' => Some('ₙ'),
        'o' => Some('ₒ'),
        'p' => Some('ₚ'),
        'r' => Some('ᵣ'),
        's' => Some('ₛ'),
        't' => Some('ₜ'),
        'u' => Some('ᵤ'),
        'v' => Some('ᵥ'),
        'x' => Some('ₓ'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_variable() {
        assert_eq!(convert_cf_latex("$$$n$$$"), "n");
    }

    #[test]
    fn subscript() {
        assert_eq!(convert_cf_latex("$$$x_i$$$"), "xᵢ");
        assert_eq!(convert_cf_latex("$$$a_{ij}$$$"), "aᵢⱼ");
    }

    #[test]
    fn superscript() {
        assert_eq!(convert_cf_latex("$$$10^5$$$"), "10⁵");
        assert_eq!(convert_cf_latex("$$$10^{18}$$$"), "10¹⁸");
        assert_eq!(convert_cf_latex("$$$2^{n-1}$$$"), "2ⁿ⁻¹");
    }

    #[test]
    fn comparison_operators() {
        assert_eq!(convert_cf_latex("$$$1 \\le n \\le 10^5$$$"), "1 ≤ n ≤ 10⁵");
    }

    #[test]
    fn absolute_value() {
        assert_eq!(convert_cf_latex("$$$|x_i - x_0|$$$"), "|xᵢ - x₀|");
    }

    #[test]
    fn xor_operator() {
        assert_eq!(convert_cf_latex("$$$a \\oplus b$$$"), "a ⊕ b");
    }

    #[test]
    fn fraction() {
        assert_eq!(convert_cf_latex("$$$\\frac{a}{b}$$$"), "a/b");
        assert_eq!(
            convert_cf_latex("$$$\\frac{n(n+1)}{2}$$$"),
            "(n(n+1))/(2)"
        );
    }

    #[test]
    fn dots() {
        assert_eq!(
            convert_cf_latex("$$$x_1, x_2, \\dots, x_n$$$"),
            "x₁, x₂, …, xₙ"
        );
    }

    #[test]
    fn mixed_text() {
        let input = "There are $$$n$$$ people and $$$m \\le 10^5$$$ edges.";
        assert_eq!(input.contains("$$$"), true);
        let result = convert_cf_latex(input);
        assert_eq!(result, "There are n people and m ≤ 10⁵ edges.");
    }

    #[test]
    fn floor_ceil() {
        assert_eq!(
            convert_cf_latex("$$$\\lfloor x \\rfloor$$$"),
            "⌊ x ⌋"
        );
    }

    #[test]
    fn text_command() {
        assert_eq!(convert_cf_latex("$$$\\text{MEX}$$$"), "MEX");
    }

    #[test]
    fn sqrt() {
        assert_eq!(convert_cf_latex("$$$\\sqrt{n}$$$"), "√n");
        assert_eq!(convert_cf_latex("$$$\\sqrt{n+1}$$$"), "√(n+1)");
    }

    #[test]
    fn no_math() {
        assert_eq!(convert_cf_latex("no math here"), "no math here");
    }

    #[test]
    fn unclosed_delimiter() {
        assert_eq!(convert_cf_latex("$$$oops"), "$$$oops");
    }

    #[test]
    fn braces() {
        assert_eq!(convert_cf_latex("$$$\\{1, 2, 3\\}$$$"), "{1, 2, 3}");
    }

    #[test]
    fn greek() {
        assert_eq!(convert_cf_latex("$$$\\alpha + \\beta$$$"), "α + β");
    }

    #[test]
    fn negative_superscript() {
        assert_eq!(convert_cf_latex("$$$10^{-6}$$$"), "10⁻⁶");
    }

    #[test]
    fn superscript_letters() {
        // Extended superscript coverage: a-z modifier letters
        assert_eq!(convert_cf_latex("$$$2^k$$$"), "2ᵏ");
        assert_eq!(convert_cf_latex("$$$n^{k+1}$$$"), "nᵏ⁺¹");
        // 'q' still has no Unicode superscript — falls back
        assert_eq!(convert_cf_latex("$$$2^q$$$"), "2^q");
    }

    #[test]
    fn subscript_fallback() {
        // 'b' has no Unicode subscript — falls back to _b
        assert_eq!(convert_cf_latex("$$$a_b$$$"), "a_b");
        assert_eq!(convert_cf_latex("$$$x_{bc}$$$"), "x_(bc)");
    }

    #[test]
    fn mixed_convertible_superscript() {
        // all chars in {12} are convertible
        assert_eq!(convert_cf_latex("$$$x^{12}$$$"), "x¹²");
        // 'k' is now convertible too
        assert_eq!(convert_cf_latex("$$$x^{k2}$$$"), "xᵏ²");
        // 'q' is not — entire group falls back
        assert_eq!(convert_cf_latex("$$$x^{q2}$$$"), "x^(q2)");
    }

    #[test]
    fn styled_segments() {
        let segments = convert_cf_latex_styled("There are $$$n$$$ people.");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], StyledSegment { text: "There are ".into(), is_math: false });
        assert_eq!(segments[1], StyledSegment { text: "n".into(), is_math: true });
        assert_eq!(segments[2], StyledSegment { text: " people.".into(), is_math: false });
    }

    #[test]
    fn styled_no_math() {
        let segments = convert_cf_latex_styled("plain text");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].is_math, false);
    }
}
