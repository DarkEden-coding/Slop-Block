import type { Metadata } from "next";
import "./globals.css";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { Toaster } from "../components/Toaster";

export const metadata: Metadata = {
  title: "GitHub Human Auth",
  description: "Self-hostable human verification for GitHub issues and pull requests.",
  icons: {
    icon: [
      { url: "/favicon.ico" },
      { url: "/icon.png", type: "image/png", sizes: "512x512" },
    ],
    apple: [{ url: "/apple-icon.png", type: "image/png", sizes: "180x180" }],
  },
  openGraph: {
    title: "GitHub Human Auth",
    description: "Self-hostable human verification for GitHub issues and pull requests.",
    images: [{ url: "/icon.png", width: 512, height: 512, alt: "GitHub Human Auth icon" }],
  },
  twitter: {
    card: "summary",
    title: "GitHub Human Auth",
    description: "Self-hostable human verification for GitHub issues and pull requests.",
    images: ["/icon.png"],
  },
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body className="antialiased">
        {children}
        <Toaster />
        <ConfirmDialog />
      </body>
    </html>
  );
}
