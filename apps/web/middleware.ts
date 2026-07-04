import { NextRequest, NextResponse } from "next/server";

export function middleware(request: NextRequest) {
  const url = request.nextUrl;
  const decodedPathname = decodeURIComponent(url.pathname);

  // GitHub App setup URLs are easy to paste with a trailing space before the
  // query string. Browsers encode that as %20, which would otherwise miss the
  // real App Router page and show a 404 after installation.
  if (decodedPathname === "/setup/github/install-complete ") {
    const redirectUrl = url.clone();
    redirectUrl.pathname = "/setup/github/install-complete";
    return NextResponse.redirect(redirectUrl);
  }

  return NextResponse.next();
}

export const config = {
  matcher: ["/setup/github/:path*"],
};
