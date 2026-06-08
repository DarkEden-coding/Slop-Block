"use client";

import Script from "next/script";
import { useEffect, useRef, useState } from "react";
import type { CaptchaPublicProvider, SessionCaptchaConfig } from "../lib/api";

declare global {
  interface Window {
    turnstile?: {
      render: (el: HTMLElement | string, opts: { sitekey: string; callback: (token: string) => void }) => string;
      remove?: (widgetId: string) => void;
    };
    hcaptcha?: {
      render: (el: HTMLElement | string, opts: { sitekey: string; callback: (token: string) => void }) => string;
      remove?: (widgetId: string) => void;
    };
    grecaptcha?: {
      render: (el: HTMLElement | string, opts: { sitekey: string; callback: (token: string) => void }) => number;
      reset?: (widgetId?: number) => void;
    };
  }
}

const SCRIPT_BY_PROVIDER: Record<string, string> = {
  "cloudflare-turnstile": "https://challenges.cloudflare.com/turnstile/v0/api.js",
  hcaptcha: "https://js.hcaptcha.com/1/api.js",
  "google-recaptcha-v2": "https://www.google.com/recaptcha/api.js",
};

type CaptchaWidgetProps = {
  config: SessionCaptchaConfig;
  onToken: (token: string, provider: string) => void;
  onError?: (message: string) => void;
};

export function CaptchaWidget({ config, onToken, onError }: CaptchaWidgetProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const widgetIdRef = useRef<string | number | null>(null);
  const [activeProvider, setActiveProvider] = useState(config.provider);
  const [activeSiteKey, setActiveSiteKey] = useState(config.site_key);
  const [scriptReady, setScriptReady] = useState(false);

  const providerOptions: CaptchaPublicProvider[] = [
    { id: config.provider, label: config.label, site_key: config.site_key },
    ...(config.alternate_providers ?? []),
  ];

  useEffect(() => {
    setActiveProvider(config.provider);
    setActiveSiteKey(config.site_key);
    setScriptReady(false);
    widgetIdRef.current = null;
  }, [config]);

  useEffect(() => {
    if (!scriptReady || !containerRef.current || !activeSiteKey) return;
    const container = containerRef.current;
    container.innerHTML = "";

    if (activeProvider === "cloudflare-turnstile" && window.turnstile) {
      widgetIdRef.current = window.turnstile.render(container, {
        sitekey: activeSiteKey,
        callback: (token) => onToken(token, activeProvider),
      });
      return;
    }

    if (activeProvider === "hcaptcha" && window.hcaptcha) {
      widgetIdRef.current = window.hcaptcha.render(container, {
        sitekey: activeSiteKey,
        callback: (token) => onToken(token, activeProvider),
      });
      return;
    }

    if (activeProvider === "google-recaptcha-v2" && window.grecaptcha) {
      widgetIdRef.current = window.grecaptcha.render(container, {
        sitekey: activeSiteKey,
        callback: (token) => onToken(token, activeProvider),
      });
      return;
    }

    onError?.("CAPTCHA widget failed to load.");
  }, [activeProvider, activeSiteKey, onError, onToken, scriptReady]);

  function switchProvider(provider: CaptchaPublicProvider) {
    setScriptReady(false);
    widgetIdRef.current = null;
    setActiveProvider(provider.id);
    setActiveSiteKey(provider.site_key);
  }

  if (config.provider === "dev-bypass") {
    return (
      <div className="rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
        Development bypass is enabled. Submit with token <code className="font-mono">dev-pass</code>.
        <button
          type="button"
          className="mt-3 block rounded-lg bg-amber-700 px-4 py-2 font-semibold text-white"
          onClick={() => onToken("dev-pass", "dev-bypass")}
        >
          Use development bypass
        </button>
      </div>
    );
  }

  const scriptSrc = SCRIPT_BY_PROVIDER[activeProvider];

  return (
    <div className="space-y-4">
      {providerOptions.length > 1 && (
        <label className="block text-sm text-slate-600">
          <span className="mb-2 block font-semibold text-slate-800">CAPTCHA provider</span>
          <select
            className="h-11 w-full rounded-lg border border-slate-200 bg-white px-3 text-sm text-slate-900"
            value={activeProvider}
            onChange={(event) => {
              const provider = providerOptions.find((option) => option.id === event.target.value);
              if (provider) switchProvider(provider);
            }}
          >
            {providerOptions.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.label}
              </option>
            ))}
          </select>
        </label>
      )}
      {scriptSrc && (
        <Script
          key={activeProvider}
          src={scriptSrc}
          onLoad={() => setScriptReady(true)}
          onError={() => onError?.("Failed to load CAPTCHA provider script.")}
        />
      )}
      <div ref={containerRef} className="min-h-16" />
    </div>
  );
}
