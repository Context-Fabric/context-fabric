"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import type { NavItem } from "@/types/docs";
import { clsx } from "clsx";
import { useState } from "react";

function NavItemComponent({
  item,
  currentPath,
  depth = 0,
}: {
  item: NavItem;
  currentPath: string;
  depth?: number;
}) {
  const isActive = currentPath === item.path;
  const hasChildren = item.children && item.children.length > 0;
  const isParentOfActive =
    hasChildren && item.children!.some((child) => currentPath.startsWith(child.path));
  const [isOpen, setIsOpen] = useState(isActive || isParentOfActive);

  return (
    <li>
      <div className="flex items-center">
        {hasChildren && (
          <button
            onClick={() => setIsOpen(!isOpen)}
            className="mr-1 p-1 hover:bg-[var(--color-border)] rounded"
            aria-label={isOpen ? "Collapse" : "Expand"}
          >
            <svg
              className={clsx("w-3 h-3 transition-transform", isOpen && "rotate-90")}
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
            </svg>
          </button>
        )}
        <Link
          href={item.path}
          className={clsx(
            "block py-1.5 px-2 rounded text-[0.875rem] transition-colors flex-1",
            isActive
              ? "bg-[var(--color-accent)] text-white font-medium"
              : "text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]"
          )}
          style={{ paddingLeft: `${depth * 12 + 8}px` }}
        >
          {item.title}
        </Link>
      </div>

      {hasChildren && isOpen && (
        <ul className="mt-1">
          {item.children!.map((child) => (
            <NavItemComponent
              key={child.path}
              item={child}
              currentPath={currentPath}
              depth={depth + 1}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

interface SidebarProps {
  navigation: NavItem[];
}

export function Sidebar({ navigation }: SidebarProps) {
  const pathname = usePathname();

  return (
    <nav className="w-64 flex-shrink-0 border-r border-[var(--color-border)] bg-[var(--color-bg-alt)] h-[calc(100vh-4rem)] sticky top-16 overflow-y-auto">
      <div className="p-4">
        <h2 className="text-sm font-semibold uppercase tracking-wider text-[var(--color-text-secondary)] mb-4">
          API Reference
        </h2>
        <ul className="space-y-1">
          {navigation.map((item) => (
            <NavItemComponent key={item.path} item={item} currentPath={pathname} />
          ))}
        </ul>
      </div>
    </nav>
  );
}
