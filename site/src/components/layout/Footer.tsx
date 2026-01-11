import Link from "next/link";
import Image from "next/image";

const footerLinks = [
  { href: "/docs", label: "Documentation" },
  { href: "https://github.com/Context-Fabric/context-fabric", label: "GitHub", external: true },
  { href: "https://pypi.org/project/context-fabric/", label: "PyPI", external: true },
];

export function Footer() {
  return (
    <footer className="border-t border-[var(--color-border)] px-6 md:px-10 py-12">
      <div className="max-w-[1000px] mx-auto flex flex-col md:flex-row justify-between items-center gap-6">
        <Link href="/" className="flex items-center gap-3">
          <Image
            src="/images/brand/fabric_tan_mark.svg"
            alt="Context-Fabric"
            width={28}
            height={28}
            className="h-7 w-auto"
          />
          <span className="font-sans font-semibold text-[var(--color-text)]">
            Context-Fabric
          </span>
        </Link>

        <div className="flex items-center gap-8">
          {footerLinks.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              target={link.external ? "_blank" : undefined}
              rel={link.external ? "noopener noreferrer" : undefined}
              className="text-[0.875rem] text-[var(--color-text-secondary)] hover:text-[var(--color-text)] transition-colors"
            >
              {link.label}
            </Link>
          ))}
        </div>
      </div>
    </footer>
  );
}
