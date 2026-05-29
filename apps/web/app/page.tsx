import { redirect } from "next/navigation";
import { ROUTES } from "@/lib/routes";

export default function Home() {
  redirect(ROUTES.listings.index);
}
