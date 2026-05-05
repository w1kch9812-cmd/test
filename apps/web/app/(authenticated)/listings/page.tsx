import { getTranslations } from "next-intl/server";
import { FilterBar } from "@/components/listings/filter-bar";
import { ListingCardList } from "@/components/listings/listing-card-list";
import { ListingMap } from "@/components/listings/listing-map";
import { SearchBar } from "@/components/listings/search-bar";

export default async function ListingsPage() {
  const t = await getTranslations("listings.page");

  return (
    <main className="flex h-screen flex-col">
      <header
        className="flex items-center justify-between p-4"
        style={{ borderBottom: "1px solid var(--color-border)" }}
      >
        <h1 className="text-xl font-bold">{t("title")}</h1>
        <SearchBar />
      </header>
      <div className="grid flex-1 grid-cols-1 overflow-hidden md:grid-cols-[280px_1fr_400px]">
        <aside
          className="hidden overflow-y-auto md:block"
          style={{ borderRight: "1px solid var(--color-border)" }}
        >
          <FilterBar />
        </aside>
        <section className="relative h-full">
          <ListingMap />
        </section>
        <aside className="overflow-y-auto" style={{ borderLeft: "1px solid var(--color-border)" }}>
          <ListingCardList />
        </aside>
      </div>
    </main>
  );
}
