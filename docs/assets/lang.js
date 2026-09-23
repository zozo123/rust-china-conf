/* Shared language switch for the static site. Default is zh-CN.
   Persists in localStorage (`rcc-lang`) and honors ?lang=zh|en. */
(function (global) {
  "use strict";

  function requested() {
    var q = new URLSearchParams(location.search).get("lang");
    if (q === "en" || q === "zh") return q;
    try {
      var stored = localStorage.getItem("rcc-lang");
      if (stored === "en" || stored === "zh") return stored;
    } catch (e) { /* private mode */ }
    return "zh";
  }

  function setDoc(lang) {
    document.documentElement.lang = lang === "en" ? "en" : "zh-CN";
    try { localStorage.setItem("rcc-lang", lang); } catch (e) { /* ignore */ }
    document.querySelectorAll("[data-lang-btn]").forEach(function (btn) {
      btn.setAttribute("aria-pressed", btn.getAttribute("data-lang-btn") === lang ? "true" : "false");
    });
  }

  function fill(dict) {
    var t = dict[requested()] || dict.zh;
    document.querySelectorAll("[data-i18n]").forEach(function (el) {
      var v = t[el.getAttribute("data-i18n")];
      if (v != null) el.textContent = v;
    });
    document.querySelectorAll("[data-i18n-html]").forEach(function (el) {
      var v = t[el.getAttribute("data-i18n-html")];
      if (v != null) el.innerHTML = v;
    });
    document.querySelectorAll("[data-i18n-attr]").forEach(function (el) {
      var parts = el.getAttribute("data-i18n-attr").split(";");
      parts.forEach(function (item) {
        var kv = item.split(":");
        if (kv.length !== 2) return;
        var v = t[kv[1].trim()];
        if (v != null) el.setAttribute(kv[0].trim(), v);
      });
    });
  }

  global.rccLang = {
    current: requested,
    apply: function (dict, after) {
      setDoc(requested());
      fill(dict);
      if (after) after(requested());
    },
    bind: function (dict, after) {
      document.querySelectorAll("[data-lang-btn]").forEach(function (btn) {
        btn.addEventListener("click", function () {
          var lang = btn.getAttribute("data-lang-btn");
          try { localStorage.setItem("rcc-lang", lang); } catch (e) { /* ignore */ }
          var url = new URL(location.href);
          url.searchParams.set("lang", lang);
          history.replaceState(null, "", url);
          setDoc(lang);
          fill(dict);
          if (after) after(lang);
        });
      });
    }
  };
})(window);
