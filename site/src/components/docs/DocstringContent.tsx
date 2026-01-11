import { isRenderableString } from "@/lib/render";

interface DocstringContentProps {
  content: string;
  className?: string;
}

/**
 * Renders docstring content with proper code block formatting.
 * Detects Python REPL examples (>>> ) and indented code blocks.
 */
export function DocstringContent({ content, className = "" }: DocstringContentProps) {
  if (!isRenderableString(content)) {
    return null;
  }

  // Split content into segments (text and code blocks)
  const segments = parseDocstring(content);

  return (
    <div className={className}>
      {segments.map((segment, index) => {
        if (segment.type === "code") {
          return (
            <pre
              key={index}
              className="my-4 p-4 bg-[#1F1F1F] text-white/85 rounded-lg overflow-x-auto text-sm font-mono leading-relaxed"
            >
              <code>{segment.content}</code>
            </pre>
          );
        }
        return (
          <p
            key={index}
            className="text-[var(--color-text-secondary)] leading-relaxed mb-3 last:mb-0"
          >
            {segment.content}
          </p>
        );
      })}
    </div>
  );
}

interface Segment {
  type: "text" | "code";
  content: string;
}

// Headers that typically precede code blocks
const CODE_SECTION_HEADERS = /^(usage|example|examples|synopsis):\s*$/i;

function parseDocstring(content: string): Segment[] {
  const lines = content.split("\n");
  const segments: Segment[] = [];
  let currentSegment: Segment | null = null;
  let inCodeSection = false; // Track if we're after a code section header

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmedLine = line.trimStart();
    const indentLevel = line.length - trimmedLine.length;

    // Check if this line is a code section header (e.g., "Usage:")
    if (CODE_SECTION_HEADERS.test(line.trim())) {
      // Save current segment
      if (currentSegment) {
        segments.push(currentSegment);
      }
      // Add the header as text
      segments.push({ type: "text", content: line.trim() });
      currentSegment = null;
      inCodeSection = true;
      continue;
    }

    // Determine if this is a code line
    const isReplLine = trimmedLine.startsWith(">>>") || trimmedLine.startsWith("...");
    const isIndentedCode = indentLevel >= 4 && trimmedLine.length > 0;
    const isCodeContinuation = currentSegment?.type === "code" && isIndentedCode;

    const isCodeLine = isReplLine ||
      (inCodeSection && isIndentedCode) ||
      isCodeContinuation;

    if (isCodeLine) {
      const codeContent = indentLevel >= 4 ? line.slice(4) : line; // Remove 4-space indent

      if (currentSegment?.type === "code") {
        currentSegment.content += "\n" + codeContent;
      } else {
        if (currentSegment) {
          segments.push(currentSegment);
        }
        currentSegment = { type: "code", content: codeContent };
      }
    } else {
      // Not a code line
      if (line.trim() === "") {
        // Empty line
        if (currentSegment?.type === "code") {
          // Empty line might continue a code block if next line is also indented
          const nextLine = lines[i + 1];
          if (nextLine && (nextLine.startsWith("    ") || nextLine.trimStart().startsWith(">>>"))) {
            currentSegment.content += "\n";
            continue;
          }
        }
        // End current segment
        if (currentSegment) {
          segments.push(currentSegment);
          currentSegment = null;
        }
        inCodeSection = false;
      } else {
        // Regular text line
        inCodeSection = false;
        if (currentSegment?.type === "text") {
          currentSegment.content += " " + line.trim();
        } else {
          if (currentSegment) {
            segments.push(currentSegment);
          }
          currentSegment = { type: "text", content: line.trim() };
        }
      }
    }
  }

  if (currentSegment) {
    segments.push(currentSegment);
  }

  return segments;
}
