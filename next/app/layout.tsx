import type { Metadata } from "next";
import localFont from "next/font/local";
import "./globals.css";
const outfit = localFont({
  src: "../public/fonts/outfit-latin.woff2",
  variable: "--font-outfit",
  display: "swap",
});
export const metadata: Metadata = {
  title: "Internet Swap · A small Wi-Fi companion for Linux",
  description:
    "A small Linux app that watches saved Wi-Fi networks and uses recent measurements to choose a better connection. Built with Tauri and NetworkManager.",
  metadataBase: new URL("https://internet-swap.vercel.app"),
  openGraph: {
    title: "Internet Swap",
    description: "A little less babysitting your Wi-Fi.",
    type: "website",
  },
};
export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={outfit.variable} suppressHydrationWarning>
      <head>
        <script
          dangerouslySetInnerHTML={{
            __html:
              "try{document.documentElement.dataset.mode=localStorage.getItem('internet-swap-mode')||(matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light')}catch{document.documentElement.dataset.mode='dark'}",
          }}
        />
      </head>
      <body>{children}</body>
    </html>
  );
}
