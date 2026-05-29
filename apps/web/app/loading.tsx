import { tStatic } from "@/lib/i18n/static";

export default function Loading() {
  return (
    <main className="flex min-h-screen items-center justify-center p-8">
      <div className="flex flex-col items-center gap-3" role="status" aria-live="polite">
        <div className="h-12 w-12 animate-spin rounded-full border-4 border-[var(--color-hairline)] border-t-[var(--color-primary)]" />
        <span className="sr-only">{tStatic("common.loading")}</span>
      </div>
    </main>
  );
}
