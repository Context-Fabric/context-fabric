import { INTERFACES, navigate, type Ui } from "./ui";

/** The page of an interface that is planned but not built yet. */
export function NotYet({ ui }: { ui: Ui }) {
  const info = INTERFACES.find((i) => i.id === ui)!;
  return (
    <main className="mx-auto max-w-[36rem] p-10 text-[15px] leading-relaxed">
      <h1 className="mb-2 text-[20px] font-semibold">{info.name}</h1>
      <p className="mb-1">{info.about}.</p>
      <p className="mb-4 text-stone">This interface is planned but not built yet.</p>
      <button
        type="button"
        onClick={() => navigate("/?ui=parallel")}
        className="rounded bg-ink px-3 py-1.5 text-[14px] text-vellum hover:opacity-90"
      >
        Open Parallel
      </button>
    </main>
  );
}
