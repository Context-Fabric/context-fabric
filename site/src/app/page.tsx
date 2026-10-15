import { Header, Footer } from "@/components/layout";
import { Hero, Features, WhatsNew, CodeExamples, CTA } from "@/components/home";

export default function Home() {
  return (
    <>
      <Header />
      <main>
        <Hero />
        <Features />
        <WhatsNew />
        <CodeExamples />
        <CTA />
      </main>
      <Footer />
    </>
  );
}
