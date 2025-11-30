import { katex } from "@mdit/plugin-katex";
import hljs from "highlight.js";
import MarkdownIt from "markdown-it";
import "highlight.js/styles/github-dark.css";

let md: MarkdownIt | null = null;

export function useMarkdownRenderer() {
  if (!md) {
    md = new MarkdownIt({
      html: false,
      breaks: true,
      linkify: true,
      highlight: (str, lang) => {
        if (lang && hljs.getLanguage(lang)) {
          try {
            return `<pre><code class="hljs language-${lang}">${hljs.highlight(str, { language: lang }).value}</code></pre>`;
          } catch (__) { }
        }
        return `<pre><code class="hljs">${md?.utils.escapeHtml(str)}</code></pre>`;
      },
    });

    md.use(katex, {
      allowInlineWithSpace: true,
    });

    // minimal heading = h3
    md.renderer.rules.heading_open = (tokens, idx) => {
      const token = tokens[idx];
      if (!token) {
        return `ERR null token`;
      }
      let level = parseInt(token.tag.substring(1), 10);
      if (level < 3) {
        level = 3;
      }
      return `<h${level}>`;
    };

    md.renderer.rules.heading_close = (tokens, idx) => {
      const token = tokens[idx];
      if (!token) {
        return `ERR null token`;
      }
      let level = parseInt(token.tag.substring(1), 10);
      if (level < 3) {
        level = 3;
      }
      return `</h${level}>`;
    };
  }

  const renderMarkdown = (text: string): string => {
    if (!md) {
      return `ERR null md render`;
    }
    return md.render(text);
  };

  return {
    renderMarkdown,
  };
}
