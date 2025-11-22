import markdownItKatex from "@traptitech/markdown-it-katex";
import MarkdownIt from "markdown-it";
import "katex/dist/katex.min.css";

let md: MarkdownIt | null = null;

export function useMarkdownRenderer() {
  if (!md) {
    md = new MarkdownIt({
      html: false, // no raw HTML
      breaks: true, // \n to <br>
      linkify: true, // URL -> link
    });

    // katex
    md.use(markdownItKatex, {
      throwOnError: false,
      errorColor: "#cc0000",
    });

    // minimal heading = h3
    md.renderer.rules.heading_open = (tokens, idx) => {
      const token = tokens[idx];
      if (!token) {
        return `ERR null token`
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
        return `ERR null token`
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
      return `ERR null md render`
    }
    return md.render(text);
  };

  return {
    renderMarkdown,
  };
}
