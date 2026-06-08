"use client";

import { useEffect, useState } from "react";
import { apiFetch, type CaptchaSettings } from "../lib/api";

export function CaptchaSettingsEditor() {
  const [settings, setSettings] = useState<CaptchaSettings | null>(null);
  const [enabled, setEnabled] = useState<string[]>([]);
  const [defaultProvider, setDefaultProvider] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiFetch<CaptchaSettings>("/api/settings/captcha")
      .then((loaded) => {
        setSettings(loaded);
        setEnabled(loaded.enabled_providers);
        setDefaultProvider(loaded.default_provider ?? loaded.enabled_providers[0] ?? "");
      })
      .catch((err: Error) => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  function toggleProvider(id: string) {
    setEnabled((current) => {
      const next = current.includes(id) ? current.filter((item) => item !== id) : [...current, id];
      if (defaultProvider && !next.includes(defaultProvider)) {
        setDefaultProvider(next[0] ?? "");
      }
      return next;
    });
  }

  async function save() {
    if (!settings) return;
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      const saved = await apiFetch<CaptchaSettings>("/api/settings/captcha", {
        method: "PUT",
        body: JSON.stringify({
          enabled_providers: enabled,
          default_provider: defaultProvider || null,
        }),
      });
      setSettings(saved);
      setEnabled(saved.enabled_providers);
      setDefaultProvider(saved.default_provider ?? saved.enabled_providers[0] ?? "");
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

  const configuredProviders = settings.available_providers.filter((provider) => provider.configured);

  return (
    <section className="rounded-3xl border border-white/10 bg-white/[0.04] p-6 shadow-2xl shadow-black/30 backdrop-blur">
      <h2 className="text-2xl font-bold text-white">CAPTCHA providers</h2>
      <p className="mt-2 max-w-3xl text-sm leading-6 text-slate-400">
        Choose which configured providers contributors may use during verification. Provider secrets and site keys are supplied through deployment environment variables.
      </p>

      {settings.dev_bypass && (
        <div className="mt-5 rounded-2xl border border-amber-300/20 bg-amber-300/10 p-4 text-sm text-amber-100">
          Development bypass is enabled for this deployment. It is only available when secure cookies are disabled.
        </div>
      )}

      {configuredProviders.length === 0 ? (
        <p className="mt-5 rounded-2xl border border-white/10 bg-slate-950/40 p-4 text-sm text-slate-300">
          No CAPTCHA providers are configured yet. Add provider secrets and site keys to your deployment environment, then reload this page.
        </p>
      ) : (
        <>
          <div className="mt-5 divide-y divide-white/10 overflow-hidden rounded-2xl border border-white/10">
            {configuredProviders.map((provider) => {
              const checked = enabled.includes(provider.id);
              return (
                <label
                  key={provider.id}
                  className="flex cursor-pointer items-start justify-between gap-5 bg-slate-950/40 p-4 transition hover:bg-white/[0.06] sm:items-center"
                >
                  <span className="min-w-0 pr-2">
                    <span className="block font-semibold text-white">{provider.label}</span>
                    <span className="mt-1 block text-sm text-slate-400">
                      {provider.site_key ? `Site key ${provider.site_key.slice(0, 8)}…` : "Configured without a public site key"}
                    </span>
                  </span>
                  <span
                    className={`relative mt-0.5 h-7 w-12 shrink-0 rounded-full border transition sm:mt-0 ${
                      checked ? "border-cyan-300/50 bg-cyan-300/80 shadow-lg shadow-cyan-500/20" : "border-white/15 bg-white/10"
                    }`}
                  >
                    <input
                      type="checkbox"
                      className="peer sr-only"
                      checked={checked}
                      onChange={() => toggleProvider(provider.id)}
                    />
                    <span className={`absolute top-1 h-5 w-5 rounded-full bg-white shadow transition ${checked ? "left-6" : "left-1"}`} />
                  </span>
                </label>
              );
            })}
          </div>

          <label className="mt-5 block rounded-2xl border border-white/10 bg-slate-950/40 p-4">
            <span className="font-semibold text-white">Default provider</span>
            <select
              className="mt-3 h-11 w-full rounded-xl border border-white/10 bg-slate-950 px-3 text-sm text-white outline-none ring-cyan-300/40 transition focus:ring-4"
              value={defaultProvider}
              onChange={(event) => setDefaultProvider(event.target.value)}
            >
              {enabled.map((providerId) => {
                const provider = configuredProviders.find((item) => item.id === providerId);
                return (
                  <option key={providerId} value={providerId}>
                    {provider?.label ?? providerId}
                  </option>
                );
              })}
            </select>
          </label>
        </>
      )}

      {error && <p className="mt-4 text-sm font-medium text-red-300">{error}</p>}
      {message && <p className="mt-4 text-sm font-medium text-emerald-300">{message}</p>}
      <button
        onClick={save}
        disabled={saving || configuredProviders.length === 0 || enabled.length === 0}
        className="mt-6 inline-flex h-11 items-center justify-center rounded-xl bg-cyan-300 px-6 text-sm font-bold leading-none text-slate-950 shadow-xl shadow-cyan-950/30 transition hover:-translate-y-0.5 hover:bg-cyan-200 disabled:translate-y-0 disabled:opacity-60"
      >
        {saving ? "Saving…" : "Save CAPTCHA settings"}
      </button>
    </section>
  );
}
