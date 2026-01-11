"use client";

import Link from "next/link";
import Image from "next/image";
import { ThemeToggle } from "./ThemeToggle";
import { useState } from "react";

const navLinks = [
  { href: "/#features", label: "Features" },
  { href: "/#code", label: "Examples" },
  { href: "/docs", label: "Documentation" },
  { href: "https://github.com/Context-Fabric/context-fabric", label: "GitHub", external: true },
];

export function Header() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  return (
    <nav className="fixed top-0 left-0 right-0 px-6 md:px-10 py-5 flex justify-between items-center bg-[var(--color-nav-bg)] backdrop-blur-[10px] z-50">
      {/* Logo */}
      <Link href="/" className="flex items-center gap-3.5">
        <Image
          src="/images/brand/fabric_tan_mark.svg"
          alt="Context-Fabric"
          width={34}
          height={34}
          className="h-[34px] w-auto"
        />
        <span className="font-sans text-[1.0625rem] font-semibold text-[var(--color-text)]">
          Context-Fabric
        </span>
      </Link>

      {/* Desktop Navigation */}
      <div className="hidden md:flex items-center gap-10">
        {navLinks.map((link) => (
          <Link
            key={link.href}
            href={link.href}
            target={link.external ? "_blank" : undefined}
            rel={link.external ? "noopener noreferrer" : undefined}
            className="text-[0.9375rem] font-medium text-[var(--color-text-secondary)] hover:text-[var(--color-text)] transition-colors"
          >
            {link.label}
          </Link>
        ))}
        <ThemeToggle />
        <Link
          href="/docs"
          className="btn-primary px-4 py-2 rounded-md text-[0.875rem] font-medium transition-opacity"
        >
          Get Started
        </Link>
      </div>

      {/* Mobile Menu Button */}
      <button
        className="md:hidden p-2 text-[var(--color-text-secondary)]"
        onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
        aria-label="Toggle menu"
      >
        <svg
          className="w-6 h-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          {mobileMenuOpen ? (
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M6 18L18 6M6 6l12 12"
            />
          ) : (
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M4 6h16M4 12h16M4 18h16"
            />
          )}
        </svg>
      </button>

      {/* Mobile Menu */}
      {mobileMenuOpen && (
        <div className="absolute top-full left-0 right-0 bg-[var(--color-bg)] border-b border-[var(--color-border)] md:hidden">
          <div className="flex flex-col p-4 gap-4">
            {navLinks.map((link) => (
              <Link
                key={link.href}
                href={link.href}
                target={link.external ? "_blank" : undefined}
                rel={link.external ? "noopener noreferrer" : undefined}
                className="text-[0.9375rem] font-medium text-[var(--color-text-secondary)] hover:text-[var(--color-text)]"
                onClick={() => setMobileMenuOpen(false)}
              >
                {link.label}
              </Link>
            ))}
            <div className="flex items-center justify-between pt-2 border-t border-[var(--color-border)]">
              <ThemeToggle />
              <Link
                href="/docs"
                className="btn-primary px-4 py-2 rounded-md text-[0.875rem] font-medium"
                onClick={() => setMobileMenuOpen(false)}
              >
                Get Started
              </Link>
            </div>
          </div>
        </div>
      )}
    </nav>
  );
}
