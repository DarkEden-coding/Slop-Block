"use client";

import Script from "next/script";
import { useEffect, useRef, useState } from "react";
import type { CaptchaPublicProvider, SessionCaptchaConfig } from "../lib/api";

const TURNSTILE_SCRIPT = "https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit";
const HCAPTCHA_SCRIPT = "https://js.hcaptcha.com/1/api.js?render=explicit";
const RECAPTCHA_SCRIPT = "https://www.google.com/recaptcha/api.js?render=explicit";

declare global {
  interface Window {
    turnstile?: {
      ready: (callback: () => void) => void;
      render: (
        el: HTMLElement | string,
        opts: {
          sitekey: string;
          theme?: "light" | "dark" | "auto";
          callback?: (token: string) => void;
          "error-callback"?: () => void;
          "expired-callback"?: () => void;
        },
      ) => string;
      remove: (widgetId: string) => void;
    };
    hcaptcha?: {
      render: (
        el: HTMLElement | string,
        opts: { sitekey: string; callback: (token: string) => void },
      ) => string;
      remove?: (widgetId: string) => void;
    };
    grecaptcha?: {
      ready: (callback: () => void) => void;
      render: (
        el: HTMLElement | string,
        opts: { sitekey: string; callback: (token: string) => void },
      ) => number;
      reset?: (widgetId?: number) => void;
    };
  }
}

type CaptchaWidgetProps = {
  config: SessionCaptchaConfig;
  onToken: (token: string, provider: string) => void;
  onError?: (message: string) => void;
};

type ProviderWidgetProps = {
  siteKey: string;
  providerId: string;
  onToken: (token: string) => void;
  onError?: (message: string) => void;
};

function useStableHandler<T extends (...args: never[]) => void>(handler: T) {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;
  return handlerRef;
}

function TurnstileWidget({ siteKey, onToken, onError }: Omit<ProviderWidgetProps, "providerId">) {
  const containerRef = useRef<HTMLDivElement>(null);
  const widgetIdRef = useRef<string | null>(null);
  const onTokenRef = useStableHandler(onToken);
  const onErrorRef = useStableHandler(onError ?? (() => undefined));
  const [scriptReady, setScriptReady] = useState(
    () => typeof window !== "undefined" && typeof window.turnstile !== "undefined",
  );

  useEffect(() => {
    if (!scriptReady || !containerRef.current) return;

    let cancelled = false;

    const mount = () => {
      if (cancelled || !containerRef.current || !window.turnstile) return;
      if (widgetIdRef.current) {
        window.turnstile.remove(widgetIdRef.current);
        widgetIdRef.current = null;
      }
      widgetIdRef.current = window.turnstile.render(containerRef.current, {
        sitekey: siteKey,
        theme: "light",
        callback: (token) => onTokenRef.current(token),
        "error-callback": () =>
          onErrorRef.current?.("Turnstile could not run. Disable ad blockers for this page and try again."),
        "expired-callback": () =>
          onErrorRef.current?.("Turnstile expired. Complete the challenge again."),
      });
    };

    window.turnstile?.ready(mount);

    return () => {
      cancelled = true;
      if (widgetIdRef.current && window.turnstile) {
        window.turnstile.remove(widgetIdRef.current);
        widgetIdRef.current = null;
      }
    };
  }, [scriptReady, siteKey, onErrorRef, onTokenRef]);

  return (
    <>
      <Script
        src={TURNSTILE_SCRIPT}
        strategy="afterInteractive"
        onLoad={() => setScriptReady(true)}
        onError={() => onErrorRef.current?.("Failed to load Cloudflare Turnstile.")}
      />
      <div ref={containerRef} className="min-h-[65px]" />
    </>
  );
}

function HCaptchaWidget({ siteKey, onToken, onError }: Omit<ProviderWidgetProps, "providerId">) {
  const containerRef = useRef<HTMLDivElement>(null);
  const widgetIdRef = useRef<string | null>(null);
  const onTokenRef = useStableHandler(onToken);
  const onErrorRef = useStableHandler(onError ?? (() => undefined));
  const [scriptReady, setScriptReady] = useState(
    () => typeof window !== "undefined" && typeof window.hcaptcha !== "undefined",
  );

  useEffect(() => {
    if (!scriptReady || !containerRef.current || !window.hcaptcha) return;
    if (widgetIdRef.current) {
      window.hcaptcha.remove?.(widgetIdRef.current);
      widgetIdRef.current = null;
    }
    widgetIdRef.current = window.hcaptcha.render(containerRef.current, {
      sitekey: siteKey,
      callback: (token) => onTokenRef.current(token),
    });
    return () => {
      if (widgetIdRef.current && window.hcaptcha?.remove) {
        window.hcaptcha.remove(widgetIdRef.current);
        widgetIdRef.current = null;
      }
    };
  }, [scriptReady, siteKey, onTokenRef]);

  return (
    <>
      <Script
        src={HCAPTCHA_SCRIPT}
        strategy="afterInteractive"
        onLoad={() => setScriptReady(true)}
        onError={() => onErrorRef.current?.("Failed to load hCaptcha.")}
      />
      <div ref={containerRef} className="min-h-[65px]" />
    </>
  );
}

function RecaptchaWidget({ siteKey, onToken, onError }: Omit<ProviderWidgetProps, "providerId">) {
  const containerRef = useRef<HTMLDivElement>(null);
  const widgetIdRef = useRef<number | null>(null);
  const onTokenRef = useStableHandler(onToken);
  const onErrorRef = useStableHandler(onError ?? (() => undefined));
  const [scriptReady, setScriptReady] = useState(
    () => typeof window !== "undefined" && typeof window.grecaptcha !== "undefined",
  );

  useEffect(() => {
    if (!scriptReady || !containerRef.current || !window.grecaptcha) return;

    let cancelled = false;
    const mount = () => {
      if (cancelled || !containerRef.current || !window.grecaptcha) return;
      if (widgetIdRef.current !== null) {
        window.grecaptcha.reset?.(widgetIdRef.current);
        widgetIdRef.current = null;
      }
      widgetIdRef.current = window.grecaptcha.render(containerRef.current, {
        sitekey: siteKey,
        callback: (token) => onTokenRef.current(token),
      });
    };

    window.grecaptcha.ready(mount);

    return () => {
      cancelled = true;
      widgetIdRef.current = null;
    };
  }, [scriptReady, siteKey, onTokenRef]);

  return (
    <>
      <Script
        src={RECAPTCHA_SCRIPT}
        strategy="afterInteractive"
        onLoad={() => setScriptReady(true)}
        onError={() => onErrorRef.current?.("Failed to load Google reCAPTCHA.")}
      />
      <div ref={containerRef} className="min-h-[65px]" />
    </>
  );
}

export function CaptchaWidget({ config, onToken, onError }: CaptchaWidgetProps) {
  const providerOptions: CaptchaPublicProvider[] = [
    { id: config.provider, label: config.label, site_key: config.site_key },
    ...(config.alternate_providers ?? []),
  ];
  const [selection, setSelection] = useState({
    provider: config.provider,
    siteKey: config.site_key,
    label: config.label,
  });

  useEffect(() => {
    setSelection({
      provider: config.provider,
      siteKey: config.site_key,
      label: config.label,
    });
  }, [config]);

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

  const widgetKey = `${selection.provider}:${selection.siteKey}`;

  return (
    <div className="space-y-4">
      {providerOptions.length > 1 && (
        <label className="block text-sm text-slate-600">
          <span className="mb-2 block font-semibold text-slate-800">CAPTCHA provider</span>
          <select
            className="h-11 w-full rounded-lg border border-slate-200 bg-white px-3 text-sm text-slate-900"
            value={selection.provider}
            onChange={(event) => {
              const provider = providerOptions.find((option) => option.id === event.target.value);
              if (!provider) return;
              setSelection({
                provider: provider.id,
                siteKey: provider.site_key,
                label: provider.label,
              });
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

      {selection.provider === "cloudflare-turnstile" && (
        <TurnstileWidget
          key={widgetKey}
          siteKey={selection.siteKey}
          onToken={(token) => onToken(token, selection.provider)}
          onError={onError}
        />
      )}
      {selection.provider === "hcaptcha" && (
        <HCaptchaWidget
          key={widgetKey}
          siteKey={selection.siteKey}
          onToken={(token) => onToken(token, selection.provider)}
          onError={onError}
        />
      )}
      {selection.provider === "google-recaptcha-v2" && (
        <RecaptchaWidget
          key={widgetKey}
          siteKey={selection.siteKey}
          onToken={(token) => onToken(token, selection.provider)}
          onError={onError}
        />
      )}
    </div>
  );
}
