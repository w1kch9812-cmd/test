import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "공짱",
  description: "산업용 부동산 정보 플랫폼",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="ko">
      <body>{children}</body>
    </html>
  );
}
