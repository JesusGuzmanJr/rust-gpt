(function () {
  const locale = encodeURIComponent(navigator.language);
  const timezone = encodeURIComponent(
    Intl.DateTimeFormat().resolvedOptions().timeZone
  );

  document.cookie = `locale=${locale}; path=/; SameSite=Strict`;
  document.cookie = `timezone=${timezone}; path=/; SameSite=Strict`;
})();
