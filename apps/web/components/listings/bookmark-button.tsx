"use client";

/**
 * SP6-iii: 즐겨찾기 toggle 버튼.
 *
 * Optimistic UI — 클릭 즉시 UI 반영, 실패 시 rollback. TanStack Query mutation
 * 으로 detail/list 캐시 invalidate.
 */

import { Button } from "@gongzzang/ui";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Heart } from "lucide-react";
import { useTranslations } from "next-intl";
import { toast } from "sonner";

import { addBookmark, removeBookmark } from "@/lib/listings/mutations";

interface BookmarkButtonProps {
  listingId: string;
  isBookmarked: boolean;
  bookmarkCount: number;
}

export function BookmarkButton({
  listingId,
  isBookmarked,
  bookmarkCount,
}: BookmarkButtonProps): React.ReactElement {
  const qc = useQueryClient();
  const t = useTranslations("bookmark");

  const mutation = useMutation({
    mutationFn: async () => {
      if (isBookmarked) {
        await removeBookmark(listingId);
      } else {
        await addBookmark(listingId);
      }
    },
    onSuccess() {
      // detail / list 캐시 invalidate -> 다음 fetch 가 정확한 count 반영.
      void qc.invalidateQueries({ queryKey: ["listing-detail", listingId] });
      void qc.invalidateQueries({ queryKey: ["listings"] });
      toast.success(t(isBookmarked ? "removed" : "added"));
    },
    onError() {
      toast.error(t("error"));
    },
  });

  return (
    <Button
      type="button"
      variant={isBookmarked ? "primary" : "ghost"}
      onClick={() => mutation.mutate()}
      disabled={mutation.isPending}
      aria-label={t(isBookmarked ? "ariaRemove" : "ariaAdd")}
      aria-pressed={isBookmarked}
    >
      <Heart className={`mr-2 h-4 w-4 ${isBookmarked ? "fill-current" : ""}`} aria-hidden="true" />
      {bookmarkCount}
    </Button>
  );
}
