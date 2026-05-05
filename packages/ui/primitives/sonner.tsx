"use client";

import { Toaster as Sonner } from "sonner";

export function Toaster() {
  return <Sonner position="top-right" theme="system" richColors closeButton duration={4000} />;
}

export { toast } from "sonner";
