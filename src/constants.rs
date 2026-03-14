pub const FAVICON_URL: &str = "https://perosar.eu.org/imgs/rustbin.ico";
pub const LOGO_URL: &str = "https://perosar.eu.org/imgs/rust.png";
pub const PREVIEW_IMAGE_BASE_URL: &str =
    "https://perosar.tilde.team/cgi-bin/rustbin-preview.sh?id=";
pub const FONT_URL: &str =
    "https://fonts.googleapis.com/css2?family=DM+Mono:wght@300;400;500&display=swap";
pub const PASTE_JS: &str = r##"
(function(){
    function applyRange(){
        document.querySelectorAll(".code-line.is-selected").forEach(function(el){
            el.classList.remove("is-selected");
        });
        var m = location.hash.match(/^#L(\d+)-L?(\d+)$/);
        if (!m) return;
        var lo = Math.min(+m[1], +m[2]), hi = Math.max(+m[1], +m[2]);
        for (var i = lo; i <= hi; i++) {
            var el = document.getElementById("L" + i);
            if (el) el.classList.add("is-selected");
        }
        var first = document.getElementById("L" + lo);
        if (first) first.scrollIntoView({ block: "center" });
    }
    document.addEventListener("click", function(e) {
        var a = e.target.closest(".line-link");
        if (!a || !e.shiftKey) return;
        e.preventDefault();
        var m = location.hash.match(/^#L(\d+)/);
        if (!m) return;
        var s = +m[1], c = +a.dataset.lineNumber;
        location.hash = "L" + Math.min(s, c) + "-L" + Math.max(s, c);
    });
    addEventListener("hashchange", applyRange);
    applyRange();
})();
"##;

pub const APP_CSS: &str = r#"
:root {
    --bg: #0a0c10;
    --panel: #161b22;
    --panel-2: #0f141b;
    --fg: #f0f3f6;
    --muted: #9ea7b3;
    --accent: #71b7ff;
    --accent-strong: #26cd4d;
    --accent-soft: #76e3ea;
    --selection: #ffffff33;
    --selection-border: #26cd4d40;
    --anchor-selection: #bb800926;
    --anchor-selection-border: #d29922;
    --anchor-selection-number: #f0f6fc;
    --danger: #ff9492;
    --border: #272b33;
    --hover: #71b7ff1f;
}

* {
    padding: 0;
    color: var(--fg);
    margin: 0;
    box-sizing: border-box;
    scrollbar-width: none;
    font-family: 'DM Mono', monospace;
}

*::-webkit-scrollbar {
    display: none;
}

::selection {
    background-color: var(--selection);
}

pre {
    height: 100%;
    width: 100%;
    margin: 0;
    overflow: auto;
    font-family: inherit;
    font-size: 1rem;
    line-height: inherit;
}

.paste-content {
    margin-left: 0;
    margin-right: 0;
}

.code-grid {
    min-width: 100%;
    width: max-content;
}

pre code {
    display: grid;
    grid-template-columns: auto 1fr;
    min-width: 100%;
    width: 100%;
    min-height: 1.5em;
    scroll-margin-top: 20vh;
}

.line-link {
    display: inline-block;
    min-width: 3.8em;
    padding: 0 1em 0.3em 0;
    margin-right: .2em;
    color: var(--muted);
    -webkit-user-select: none;
    user-select: none;
    text-align: right;
    text-decoration: none;
}

.line-link:hover {
    color: var(--accent);
    text-decoration: underline;
}

.line-content {
    min-width: 0;
    white-space: pre;
    overflow-wrap: normal;
    word-break: normal;
}

.code-line.is-selected {
    background-color: var(--anchor-selection);
    box-shadow: inset 2px 0 0 var(--anchor-selection-border);
}

.code-line.is-selected .line-link {
    color: var(--anchor-selection-number);
    text-decoration: none;
}

.code-line:target {
    background-color: var(--anchor-selection);
    box-shadow: inset 2px 0 0 var(--anchor-selection-border);
}

.code-line:target .line-link {
    color: var(--anchor-selection-number);
    text-decoration: none;
}

footer {
    font-family: inherit;
}

.foot-minibuf {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    width: 100%;
    background-color: var(--panel);
    user-select: none;
}

.foot {
    height: 40px;
    background: linear-gradient(90deg, var(--panel-2), var(--panel));
    border-top: 1px solid var(--border);
    border-left: 6px solid var(--accent);
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: start;
    gap: 12px;
    padding-left: 16px;
    padding-right: 20px;
    font-weight: bold;
}

.foot-end {
    margin-left: auto;
    height: 100%;
    display: flex;
    flex-direction: row-reverse;
    align-items: center;
    justify-content: start;
    gap: 16px;
    color: #f0f3f6;
    font-weight: normal;
    opacity: 0.72;
}

.kopirite {
    height: 32px;
    font-size: 0.84rem;
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: start;
    padding-left: 8px;
    padding-right: 8px;
}

.foot-hover {
    cursor: pointer;
    height: 100%;
    display: flex;
    align-items: center;
}

.foot-hover:hover {
    background-color: var(--hover);
    color: var(--accent);
    text-decoration: underline;
}

textarea {
    background: 0 0;
    border: 0;
    color: var(--fg);
    padding: 0;
    width: 100%;
    height: 100vh;
    font-family: inherit;
    outline: none;
    resize: vertical;
    font-size: 1rem;
    line-height: 1.5;
    padding-top: 20px;
    padding-left: 40px;
    margin-top: 0;
    margin-bottom: 0;
    display: block;
}

textarea::placeholder {
    color: var(--muted);
    opacity: 0.5;
}

#prompt {
    color: var(--muted);
    z-index: -1000;
    position: absolute;
    top: 20px;
    left: 0;
    width: 30px;
    font-size: 1rem;
    line-height: 1.5;
    font-family: inherit;
    text-align: right;
}

h1, h2, h3, p, a, pre, code, hr, body {
    line-height: 1.5;
}

body > h1,
body > h2,
body > h3,
body > p,
body > pre,
body > hr,
body > div.notice {
    margin-left: 16px;
    margin-right: 16px;
}

body > h1 {
    margin-top: 16px;
}

body > hr {
    border: 0;
    border-top: 1px solid var(--border);
}

.notice {
    color: var(--danger);
    margin-top: 16px;
}

.app-body {
    background-color: var(--bg);
}

.title-accent {
    color: var(--accent-strong);
}

.foot-spacer {
    height: 80px;
    display: block;
}

.foot-logo {
    margin-right: 12px;
}

.link-reset {
    text-decoration: none;
}

.foot-btn {
    background: none;
    border: none;
    cursor: pointer;
    font: inherit;
    color: inherit;
    padding: 0;
}

.markdown-body {
    padding: 32px 40px;
    line-height: 1.6;
    word-wrap: break-word;
    max-width: 900px;
}

.markdown-body h1,
.markdown-body h2,
.markdown-body h3,
.markdown-body h4,
.markdown-body h5,
.markdown-body h6 {
    margin-top: 24px;
    margin-bottom: 16px;
    font-weight: 600;
    line-height: 1.25;
}

.markdown-body h1 { font-size: 2em; border-bottom: 1px solid var(--border); padding-bottom: .3em; }
.markdown-body h2 { font-size: 1.5em; border-bottom: 1px solid var(--border); padding-bottom: .3em; }
.markdown-body h3 { font-size: 1.25em; }
.markdown-body h4 { font-size: 1em; }
.markdown-body h5 { font-size: .875em; }
.markdown-body h6 { font-size: .85em; color: var(--muted); }

.markdown-body p {
    margin-top: 0;
    margin-bottom: 16px;
}

.markdown-body a {
    color: var(--accent);
    text-decoration: none;
}

.markdown-body a:hover {
    text-decoration: underline;
}

.markdown-body ul,
.markdown-body ol {
    margin-top: 0;
    margin-bottom: 16px;
    padding-left: 2em;
}

.markdown-body li + li {
    margin-top: .25em;
}

.markdown-body blockquote {
    margin: 0 0 16px 0;
    padding: 0 1em;
    color: var(--muted);
    border-left: .25em solid var(--border);
}

.markdown-body pre {
    background-color: var(--panel-2);
    border-radius: 6px;
    padding: 16px;
    overflow: auto;
    margin-bottom: 16px;
    height: auto;
}

.markdown-body pre code {
    display: block;
    background: none;
    padding: 0;
    border: none;
    border-radius: 0;
    font-size: .85em;
    white-space: pre;
}

.markdown-body code {
    background-color: var(--panel);
    border-radius: 4px;
    padding: .2em .4em;
    font-size: .85em;
}

.markdown-body hr {
    height: .25em;
    padding: 0;
    margin: 24px 0;
    background-color: var(--border);
    border: 0;
}

.markdown-body table {
    border-spacing: 0;
    border-collapse: collapse;
    margin-bottom: 16px;
    width: auto;
}

.markdown-body table th,
.markdown-body table td {
    padding: 6px 13px;
    border: 1px solid var(--border);
}

.markdown-body table th {
    font-weight: 600;
    background-color: var(--panel-2);
}

.markdown-body table tr:nth-child(2n) {
    background-color: var(--panel-2);
}

.markdown-body img {
    max-width: 100%;
    height: auto;
}

.markdown-body input[type="checkbox"] {
    margin-right: .5em;
}

.markdown-body del {
    color: var(--muted);
}

.markdown-body sup {
    font-size: .75em;
}

.markdown-body .footnote-definition {
    margin-bottom: 8px;
    font-size: .9em;
}

.markdown-body .footnote-definition p {
    display: inline;
}"#;
