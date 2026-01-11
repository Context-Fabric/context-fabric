import type { Metadata } from "next";
import { Source_Serif_4, Inter, JetBrains_Mono } from "next/font/google";
import { ThemeProvider } from "@/components/theme-provider";
import "./globals.css";

const sourceSerif = Source_Serif_4({
  subsets: ["latin"],
  variable: "--font-source-serif",
  display: "swap",
});

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

const jetbrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-jetbrains",
  display: "swap",
});

export const metadata: Metadata = {
  title: "Context-Fabric - Corpus Analysis Infrastructure",
  description: "Production-ready corpus analysis for the age of AI. Memory-efficient storage and querying for annotated text corpora.",
  keywords: ["corpus analysis", "text-fabric", "nlp", "digital humanities", "biblical studies", "mcp"],
  authors: [{ name: "Cody Kingham" }],
  openGraph: {
    title: "Context-Fabric",
    description: "Production-ready corpus analysis for the age of AI",
    type: "website",
    images: [
      {
        url: "/images/og-image.png",
        width: 1200,
        height: 630,
        alt: "Context-Fabric - AI-Native Corpus Engine",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Context-Fabric",
    description: "Production-ready corpus analysis for the age of AI",
    images: ["/images/og-image.png"],
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body
        className={`${sourceSerif.variable} ${inter.variable} ${jetbrainsMono.variable} antialiased`}
      >
        <ThemeProvider
          attribute="data-theme"
          defaultTheme="system"
          enableSystem={true}
          disableTransitionOnChange={false}
        >
          {children}
        </ThemeProvider>
      </body>
    </html>
  );
}
