import type { Route } from "next";
import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession } from "@/lib/session/store";

// cast: typed route generation (next build) 전 단계에서도 타입 오류 없이 redirect 가능
const LOGIN_ROUTE = "/login" as Route;

export default async function AuthenticatedLayout({ children }: { children: React.ReactNode }) {
  const cookieStore = await cookies();
  const sid = cookieStore.get(SID_COOKIE_NAME)?.value;
  if (!sid) {
    redirect(LOGIN_ROUTE);
  }
  const session = await getSession(sid);
  if (!session) {
    redirect(LOGIN_ROUTE);
  }
  return <>{children}</>;
}
