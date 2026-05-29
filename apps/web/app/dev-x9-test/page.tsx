import { notFound } from "next/navigation";
import { DevX9TestClient } from "./dev-x9-test-client";

export default function DevX9TestPage() {
  if (process.env.NODE_ENV === "production") {
    notFound();
  }

  return <DevX9TestClient />;
}
