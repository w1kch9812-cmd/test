"use client";
import { create } from "zustand";
import type { ListingFilters, SortKey } from "@/lib/listings/filters";

export interface MapBounds {
  south: number;
  west: number;
  north: number;
  east: number;
}

interface ListingsState {
  bounds: MapBounds | undefined;
  filters: ListingFilters;
  selectedListingId: string | null;
  setBounds: (b: MapBounds) => void;
  setFilters: (next: ListingFilters) => void;
  patchFilters: (patch: Partial<ListingFilters>) => void;
  setSelectedListingId: (id: string | null) => void;
}

const DEFAULT_FILTERS: ListingFilters = {
  types: [],
  transactions: [],
  minAreaM2: undefined,
  maxAreaM2: undefined,
  minPriceKrw: undefined,
  maxPriceKrw: undefined,
  sort: "created_at_desc" as SortKey,
  pnu: undefined,
  adminCode: undefined,
  landUseType: undefined,
};

export const useListingsStore = create<ListingsState>((set) => ({
  bounds: undefined,
  filters: DEFAULT_FILTERS,
  selectedListingId: null,
  setBounds: (b) => set({ bounds: b }),
  setFilters: (next) => set({ filters: next }),
  patchFilters: (patch) => set((state) => ({ filters: { ...state.filters, ...patch } })),
  setSelectedListingId: (id) => set({ selectedListingId: id }),
}));
