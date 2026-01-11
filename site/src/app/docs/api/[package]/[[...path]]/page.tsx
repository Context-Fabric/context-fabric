import { notFound } from "next/navigation";
import { getModule, packages, generateAllDocParams } from "@/lib/docs";
import { ModuleDoc } from "@/components/docs";

interface PageProps {
  params: Promise<{
    package: string;
    path?: string[];
  }>;
}

export async function generateStaticParams() {
  return generateAllDocParams();
}

export default async function ApiDocPage({ params }: PageProps) {
  const { package: packageName, path = [] } = await params;

  // Check if package exists
  if (!packages[packageName]) {
    notFound();
  }

  const module = getModule(packageName, path);

  if (!module) {
    notFound();
  }

  const basePath = `/docs/api/${packageName}${path.length > 0 ? "/" + path.join("/") : ""}`;

  return <ModuleDoc module={module} basePath={basePath} />;
}

export async function generateMetadata({ params }: PageProps) {
  const { package: packageName, path = [] } = await params;
  const module = getModule(packageName, path);

  if (!module) {
    return { title: "Not Found" };
  }

  const title = path.length > 0 ? `${module.name} - ${packageName}` : packageName;

  return {
    title: `${title} | Context-Fabric API`,
    description: module.docstring?.summary || `API documentation for ${title}`,
  };
}
