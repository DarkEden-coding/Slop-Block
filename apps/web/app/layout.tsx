import type { Metadata } from "next";
import "./globals.css";
import { Toaster } from "../components/Toaster";

export const metadata: Metadata = {
  title: "GitHub Human Auth",
  description: "Self-hostable human verification for GitHub issues and pull requests.",
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body className="antialiased">
        {children}
        <Toaster />
      </body>
    </html>
  );
}
