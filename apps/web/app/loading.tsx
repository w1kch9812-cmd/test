export default function Loading() {
  return (
    <main className="flex min-h-screen items-center justify-center p-8">
      <div className="flex flex-col items-center gap-3" role="status" aria-live="polite">
        <div className="h-12 w-12 animate-spin rounded-full border-4 border-[var(--color-muted)] border-t-[var(--color-brand-600)]" />
        <span className="sr-only">불러오는 중이에요</span>
      </div>
    </main>
  );
}
