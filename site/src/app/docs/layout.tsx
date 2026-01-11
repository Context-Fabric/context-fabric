import { Header } from "@/components/layout";
import { Sidebar } from "@/components/docs";
import { navigation } from "@/lib/docs";

export default function DocsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <>
      <Header />
      <div className="flex min-h-screen pt-16">
        <Sidebar navigation={navigation} />
        <main className="flex-1 p-8 overflow-auto bg-[var(--color-bg)]">
          {children}
        </main>
      </div>
    </>
  );
}
