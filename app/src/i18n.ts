// UI language. The English text is the key; i18n-it.ts lists every key, so a string
// without an Italian translation fails the type check (`npm run build`).

import { IT } from "./i18n-it";

export type Lang = "en" | "it";
/** An English UI string. A `|context` suffix tells apart words with two translations. */
export type Key = keyof typeof IT;
type Params = Record<string, string | number>;

const STORE = "omnipack.lang";

/** The saved choice, else the system language (Italian or English). */
function initialLang(): Lang {
  try {
    const saved = localStorage.getItem(STORE);
    if (saved === "en" || saved === "it") return saved;
  } catch {
    // Storage may be unavailable (private mode); fall back to the system language.
  }
  return navigator.language.toLowerCase().startsWith("it") ? "it" : "en";
}

let lang: Lang = initialLang();
document.documentElement.lang = lang;

export const getLang = (): Lang => lang;

export function setLang(l: Lang) {
  lang = l;
  document.documentElement.lang = l;
  try {
    localStorage.setItem(STORE, l);
  } catch {
    // The choice just isn't remembered.
  }
}

/** Locale for numbers and dates: Italian, or the system's for English (as before). */
export const locale = (): string | undefined => (lang === "it" ? "it-IT" : undefined);

const fill = (s: string, params?: Params) => (params ? s.replace(/\{(\w+)\}/g, (m, k: string) => (k in params ? String(params[k]) : m)) : s);

export function t(key: Key, params?: Params): string {
  return fill(lang === "it" ? IT[key] : key.replace(/\|.*$/, ""), params);
}

/** `one` for n = 1, else `other`; both get `{n}`. */
export const plural = (n: number, one: Key, other: Key, params: Params = {}) => t(n === 1 ? one : other, { n, ...params });

/** Text from the engine (preset names, notes, errors): translated when known, else unchanged. */
export function tr(text: string): string {
  return lang === "it" && Object.hasOwn(IT, text) ? IT[text as Key] : text;
}

/** Fills the static page from its data-i18n, data-i18n-title and data-i18n-placeholder attributes. */
export function translatePage(root: ParentNode = document) {
  const text = (key: string) => {
    if (!Object.hasOwn(IT, key)) console.warn(`i18n: no translation for "${key}"`);
    return Object.hasOwn(IT, key) ? t(key as Key) : key;
  };
  for (const el of root.querySelectorAll<HTMLElement>("[data-i18n]")) el.textContent = text(el.dataset.i18n!);
  for (const el of root.querySelectorAll<HTMLElement>("[data-i18n-title]")) el.title = text(el.dataset.i18nTitle!);
  for (const el of root.querySelectorAll<HTMLInputElement>("[data-i18n-placeholder]")) el.placeholder = text(el.dataset.i18nPlaceholder!);
}
