import { marked, type Renderer } from "marked";
import markedKatex from "marked-katex-extension";
import "katex/dist/katex.min.css";

let renderer: Renderer | null = null;

export function useMarkdownRenderer() {
  if (!renderer) {
    renderer = new marked.Renderer();

    renderer.heading = ({ text, depth }) => {
      if (depth < 3) {
        depth = 3;
      }
      return `<h${depth}>${text}</h${depth}>`;
    };

    renderer.html = () => "";

    marked.use(
      markedKatex({
        throwOnError: false,
      }),
    );

    marked.use({
      breaks: true,
      gfm: true,
      renderer: renderer,
    });
  }

  const renderMarkdown = (text: string): string => {
    return marked(text) as string;
  };

  return {
    renderMarkdown,
  };
}
