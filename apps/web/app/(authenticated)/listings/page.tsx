import { Separator } from "@gongzzang/ui";
import { getTranslations } from "next-intl/server";
import { FilterBar } from "@/components/listings/filter-bar";
import { ListingCardList } from "@/components/listings/listing-card-list";
import { ListingMap } from "@/components/listings/listing-map";
import { SearchBar } from "@/components/listings/search-bar";

export default async function ListingsPage() {
  const t = await getTranslations("listings.page");

  return (
    <main className="flex h-screen flex-col bg-[var(--color-canvas)]">
      <header className="flex items-center justify-between gap-6 px-6 py-4">
        <h1 className="whitespace-nowrap text-[length:var(--text-title-lg)] font-semibold tracking-[var(--tracking-display-sm)] text-[var(--color-ink)]">
          {t("title")}
        </h1>
        <div className="max-w-md flex-1">
          <SearchBar />
        </div>
      </header>
      <Separator />
      <FilterBar />
      <Separator />
      <div className="grid flex-1 grid-cols-1 overflow-hidden md:grid-cols-[1fr_420px]">
        <section className="relative h-full">
          <ListingMap />
        </section>
        <aside className="overflow-y-auto border-l border-[var(--color-hairline)] bg-[var(--color-canvas)]">
          <ListingCardList />
        </aside>
      </div>
    </main>
  );
}
