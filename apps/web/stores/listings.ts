"use client";
import { create } from "zustand";
import type { ListingFilters, SortKey } from "@/lib/listings/filters";

interface ListingsState {
  filters: ListingFilters;
  setFilters: (next: ListingFilters) => void;
  patchFilters: (patch: Partial<ListingFilters>) => void;
}

const DEFAULT_FILTERS: ListingFilters = {
  types: [],
  transactions: [],
  minAreaM2: undefined,
  maxAreaM2: undefined,
  minPriceKrw: undefined,
  maxPriceKrw: undefined,
  sort: "created_at_desc" as SortKey,
  adminCode: undefined,
  landUseType: undefined,
};

export const useListingsStore = create<ListingsState>((set) => ({
  filters: DEFAULT_FILTERS,
  setFilters: (next) => set({ filters: next }),
  patchFilters: (patch) => set((state) => ({ filters: { ...state.filters, ...patch } })),
}));
