"use client";

import { useEffect, useMemo, useState } from "react";
import {
  apiFetch,
  CAPTCHA_PROVIDER_OPTIONS,
  type CaptchaProviderCredentialsUpdate,
  type CaptchaProviderInfo,
  type CaptchaSettings,
  type CaptchaSettingsUpdate,
} from "../lib/api";

type ProviderDraft = {
  siteKey: string;
  secret: string;
  enabled: boolean;
};

function emptyDrafts(): Record<string, ProviderDraft> {
  return Object.fromEntries(
    CAPTCHA_PROVIDER_OPTIONS.map((provider) => [
      provider.id,
      { siteKey: "", secret: "", enabled: false },
    ]),
  );
}

function labelFor(providerId: string, providers: CaptchaProviderInfo[]) {
  return (
    providers.find((provider) => provider.id === providerId)?.label
    ?? CAPTCHA_PROVIDER_OPTIONS.find((provider) => provider.id === providerId)?.label
    ?? providerId
  );
}

export function CaptchaSettingsEditor() {
  const [settings, setSettings] = useState<CaptchaSettings | null>(null);
  const [drafts, setDrafts] = useState<Record<string, ProviderDraft>>(emptyDrafts);
  const [defaultProvider, setDefaultProvider] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiFetch<CaptchaSettings>("/api/settings/captcha")
      .then((loaded) => {
        setSettings(loaded);
        setDefaultProvider(loaded.default_provider ?? loaded.enabled_providers[0] ?? "");
        setDrafts(() => {
          const next = emptyDrafts();
          for (const provider of loaded.available_providers) {
            if (provider.id === "dev-bypass") continue;
            next[provider.id] = {
              siteKey: provider.site_key ?? "",
              secret: "",
              enabled: loaded.enabled_providers.includes(provider.id),
            };
          }
          return next;
        });
      })
      .catch((err: Error) => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  const enabledProviderIds = useMemo(
    () => Object.entries(drafts).filter(([, draft]) => draft.enabled).map(([id]) => id),
    [drafts],
  );

  function updateDraft(providerId: string, patch: Partial<ProviderDraft>) {
    setDrafts((current) => ({
      ...current,
      [providerId]: { ...current[providerId], ...patch },
    }));
  }

  function toggleEnabled(providerId: string) {
    setDrafts((current) => {
      const enabled = !current[providerId]?.enabled;
      const next = {
        ...current,
        [providerId]: { ...current[providerId], enabled },
      };
      const nextEnabled = Object.entries(next)
        .filter(([, draft]) => draft.enabled)
        .map(([id]) => id);
      if (defaultProvider === providerId && !enabled) {
        setDefaultProvider(nextEnabled[0] ?? "");
      } else if (!defaultProvider && enabled) {
        setDefaultProvider(providerId);
      }
      return next;
    });
  }

  async function save() {
    if (!settings) return;
    setSaving(true);
    setError(null);
    setMessage(null);

    const providers: Record<string, CaptchaProviderCredentialsUpdate> = {};
    for (const [providerId, draft] of Object.entries(drafts)) {
      const existing = settings.available_providers.find((provider) => provider.id === providerId);
      const siteKey = draft.siteKey.trim();
      const secret = draft.secret.trim();
      const credentials: CaptchaProviderCredentialsUpdate = {};
      if (siteKey) credentials.site_key = siteKey;
      if (secret) credentials.secret = secret;
      if (!siteKey && !secret && !existing?.configured) continue;
      if (Object.keys(credentials).length > 0) providers[providerId] = credentials;
    }

    for (const providerId of enabledProviderIds) {
      const draft = drafts[providerId];
      const existing = settings.available_providers.find((provider) => provider.id === providerId);
      if (!draft?.siteKey.trim() && !existing?.site_key) {
        setError(`Enter a site key for ${labelFor(providerId, settings.available_providers)}.`);
        setSaving(false);
        return;
      }
      if (!draft?.secret.trim() && !existing?.secret_set) {
        setError(`Enter a secret key for ${labelFor(providerId, settings.available_providers)}.`);
        setSaving(false);
        return;
      }
    }

    const payload: CaptchaSettingsUpdate = {
      enabled_providers: enabledProviderIds,
      default_provider: defaultProvider || null,
      providers,
    };

    try {
      const saved = await apiFetch<CaptchaSettings>("/api/settings/captcha", {
        method: "PUT",
        body: JSON.stringify(payload),
      });
      setSettings(saved);
      setDefaultProvider(saved.default_provider ?? saved.enabled_providers[0] ?? "");
      setDrafts(() => {
        const next = emptyDrafts();
        for (const provider of saved.available_providers) {
          if (provider.id === "dev-bypass") continue;
          next[provider.id] = {
            siteKey: provider.site_key ?? "",
            secret: "",
            enabled: saved.enabled_providers.includes(provider.id),
          };
        }
        return next;
      });
      setMessage("CAPTCHA settings saved.");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Save failed");
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return <div className="rounded-2xl border border-white/10 bg-white/5 p-6 text-slate-300 shadow-xl shadow-black/20">Loading CAPTCHA settings…</div>;
  }

  if (!settings) {
    return <div className="rounded-2xl border border-red-400/30 bg-red-950/30 p-6 text-red-200">{error ?? "Unable to load CAPTCHA settings."}</div>;
  }

  const setupProviders = settings.available_providers.filter((provider) => provider.id !== "dev-bypass");

  return (
    <section className="space-y-6">
      <div className="rounded-3xl border border-white/10 bg-white/[0.04] p-6 shadow-2xl shadow-black/30 backdrop-blur">
        <h2 className="text-2xl font-bold text-white">Configure CAPTCHA providers</h2>
        <p className="mt-2 max-w-3xl text-sm leading-6 text-slate-400">
          Enter the site key and secret for each provider you want to use. For Cloudflare Turnstile, create a widget in the Cloudflare dashboard and paste both keys here.
        </p>

        {settings.dev_bypass && (
          <div className="mt-5 rounded-2xl border border-amber-300/20 bg-amber-300/10 p-4 text-sm text-amber-100">
            Development bypass is enabled for this deployment. It is only available when secure cookies are disabled.
          </div>
        )}

        <div className="mt-6 space-y-4">
          {setupProviders.map((provider) => {
            const draft = drafts[provider.id] ?? { siteKey: "", secret: "", enabled: false };
            return (
              <div key={provider.id} className="rounded-2xl border border-white/10 bg-slate-950/40 p-5">
                <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
                  <div>
                    <h3 className="text-lg font-semibold text-white">{provider.label}</h3>
                    <p className="mt-1 text-sm text-slate-400">
                      {provider.configured
                        ? provider.source === "environment"
                          ? "Configured from deployment environment variables."
                          : "Configured from dashboard settings."
                        : "Not configured yet."}
                      {provider.secret_set ? " Secret is stored." : ""}
                    </p>
                  </div>
                  <label className="flex cursor-pointer items-center gap-3 text-sm font-semibold text-slate-200">
                    <span
                      className={`relative h-7 w-12 shrink-0 rounded-full border transition ${
                        draft.enabled ? "border-cyan-300/50 bg-cyan-300/80" : "border-white/15 bg-white/10"
                      }`}
                    >
                      <input
                        type="checkbox"
                        className="peer sr-only"
                        checked={draft.enabled}
                        onChange={() => toggleEnabled(provider.id)}
                      />
                      <span className={`absolute top-1 h-5 w-5 rounded-full bg-white shadow transition ${draft.enabled ? "left-6" : "left-1"}`} />
                    </span>
                    Enabled
                  </label>
                </div>

                <div className="mt-4 grid gap-4 md:grid-cols-2">
                  <label className="block">
                    <span className="text-sm font-semibold text-white">Site key</span>
                    <input
                      className="mt-2 h-11 w-full rounded-xl border border-white/10 bg-slate-950 px-3 text-sm text-white outline-none ring-cyan-300/40 transition placeholder:text-slate-600 focus:ring-4"
                      placeholder="Public site key"
                      value={draft.siteKey}
                      onChange={(event) => updateDraft(provider.id, { siteKey: event.target.value })}
                    />
                  </label>
                  <label className="block">
                    <span className="text-sm font-semibold text-white">Secret key</span>
                    <input
                      type="password"
                      className="mt-2 h-11 w-full rounded-xl border border-white/10 bg-slate-950 px-3 text-sm text-white outline-none ring-cyan-300/40 transition placeholder:text-slate-600 focus:ring-4"
                      placeholder={provider.secret_set ? "Leave blank to keep the current secret" : "Private secret key"}
                      value={draft.secret}
                      onChange={(event) => updateDraft(provider.id, { secret: event.target.value })}
                    />
                  </label>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <div className="rounded-3xl border border-white/10 bg-white/[0.04] p-6 shadow-2xl shadow-black/30 backdrop-blur">
        <label className="block">
          <span className="font-semibold text-white">Default provider</span>
          <p className="mt-1 text-sm text-slate-400">Used for verification unless a repository overrides it.</p>
          <select
            className="mt-3 h-11 w-full rounded-xl border border-white/10 bg-slate-950 px-3 text-sm text-white outline-none ring-cyan-300/40 transition focus:ring-4"
            value={defaultProvider}
            onChange={(event) => setDefaultProvider(event.target.value)}
          >
            {enabledProviderIds.length === 0 && <option value="">Enable a provider first</option>}
            {enabledProviderIds.map((providerId) => (
              <option key={providerId} value={providerId}>
                {labelFor(providerId, settings.available_providers)}
              </option>
            ))}
          </select>
        </label>

        {error && <p className="mt-4 text-sm font-medium text-red-300">{error}</p>}
        {message && <p className="mt-4 text-sm font-medium text-emerald-300">{message}</p>}
        <button
          onClick={save}
          disabled={saving || enabledProviderIds.length === 0}
          className="mt-6 inline-flex h-11 items-center justify-center rounded-xl bg-cyan-300 px-6 text-sm font-bold leading-none text-slate-950 shadow-xl shadow-cyan-950/30 transition hover:-translate-y-0.5 hover:bg-cyan-200 disabled:translate-y-0 disabled:opacity-60"
        >
          {saving ? "Saving…" : "Save CAPTCHA settings"}
        </button>
      </div>
    </section>
  );
}
