import { Header, Footer } from "@/components/layout";
import { Hero, Features, Stats, CodeExamples, CTA } from "@/components/home";

export default function Home() {
  return (
    <>
      <Header />
      <main>
        <Hero />
        <Features />
        <Stats />
        <CodeExamples />
        <CTA />
      </main>
      <Footer />
    </>
  );
}
