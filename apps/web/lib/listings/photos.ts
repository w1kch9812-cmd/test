import { API } from "@/lib/routes";

type ListingPhotoRef = {
  photo_id: string;
  r2_key: string;
};

export function listingPhotoImageSrc(listingId: string, photo: ListingPhotoRef): string {
  return `${API.proxy.base}/listings/${encodeURIComponent(listingId)}/photos/${encodeURIComponent(
    photo.photo_id,
  )}`;
}
