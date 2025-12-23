<script lang="ts">
  import { locales, localeNames, getCurrentLocale, setLocale, type Locale } from './i18n';

  let currentLocale = $state<Locale>(getCurrentLocale());

  function handleChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const newLocale = target.value as Locale;
    currentLocale = newLocale;
    setLocale(newLocale);
  }
</script>

<select
  value={currentLocale}
  onchange={handleChange}
  class="h-9 px-2 rounded-md border border-border bg-bg text-fg text-sm focus:outline-none focus:ring-2 focus:ring-accent"
>
  {#each locales as locale}
    <option value={locale}>{localeNames[locale]}</option>
  {/each}
</select>
