"use client";
import { Input } from "@gongzzang/ui";
import { useTranslations } from "next-intl";
import { useState } from "react";

export function SearchBar() {
  const t = useTranslations("listings.search");
  const [value, setValue] = useState("");
  return (
    <div className="w-full max-w-md">
      <Input
        type="search"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={t("placeholder")}
        aria-label={t("placeholder")}
      />
    </div>
  );
}
