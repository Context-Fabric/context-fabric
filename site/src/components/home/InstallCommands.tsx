"use client";

import { useState } from "react";

export interface InstallGroup {
  comment: string;
  commands: string[];
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <button
      onClick={handleCopy}
      aria-label={copied ? "Copied" : `Copy: ${text}`}
      className="ml-auto shrink-0 text-[#8A9099] hover:text-white transition-colors cursor-pointer"
    >
      {copied ? (
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
        </svg>
      ) : (
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
          />
        </svg>
      )}
    </button>
  );
}

/** A terminal-style panel of install commands, grouped under comment lines. */
export function InstallCommands({ groups }: { groups: InstallGroup[] }) {
  return (
    <div className="rounded-[10px] bg-[#1C1F23] px-4 md:px-5 py-4 [font-family:var(--font-jetbrains),monospace] text-[0.75rem] md:text-[0.875rem] leading-[1.7] text-left">
      {groups.map((group, index) => (
        <div key={group.comment} className={index > 0 ? "mt-3" : undefined}>
          <div className="text-[var(--color-accent)]"># {group.comment}</div>
          {group.commands.map((command) => (
            <div key={command} className="flex items-center gap-2 text-white">
              <span className="text-[#8A9099] select-none">$</span>
              <span className="whitespace-nowrap overflow-x-auto">{command}</span>
              <CopyButton text={command} />
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
