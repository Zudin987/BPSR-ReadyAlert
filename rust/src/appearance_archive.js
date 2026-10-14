// Enhance the real local roster after the existing filters and click handlers run.
// No game-specific AION metrics or network resources are added to the report.
(() => {
  const original = renderPlayerList;
  const hues = {
    iaido:'#cda5ff', iai:'#cda5ff', moonstrike:'#9f8cff',
    icicle:'#7de3db', 'ice spear':'#7de3db', frostbeam:'#5cc7d1', crystal:'#5cc7d1',
    formless:'#ffad72', voidflame:'#ffad72', crimson:'#ff7f6b', blazecrimson:'#ff7f6b',
    vanguard:'#69d5e2', skyward:'#6cafe8', smite:'#a6e178', lifebind:'#72c985',
    earthfort:'#e1be69', block:'#c7a96b', wildpack:'#d9cf70', taming:'#d9cf70',
    falconry:'#f0b84e', recovery:'#99d3d7', shield:'#80afc3', dissonance:'#e893af', concerto:'#e47d8a'
  };
  renderPlayerList = function(encounter, rows) {
    original(encounter, rows);
    const largest = Math.max(1, ...rows.map(row => Math.max(0, Number(row.damage) || 0)));
    document.querySelectorAll('#players .player').forEach((button, index) => {
      const row = rows[index];
      if (!row) return;
      const damage = Math.max(0, Number(row.damage) || 0);
      button.style.setProperty('--fill', `${Math.min(100, damage * 100 / largest)}%`);
      button.style.setProperty('--hue', hues[String(row.subprofession_name || '').trim().toLowerCase()] || '#becda7');
      const metric = document.createElement('span');
      metric.className = 'player-rate';
      const dps = Number(row.dps);
      metric.textContent = Number.isFinite(dps) ? `${compact(Math.max(0, dps))} /s` : '—';
      button.appendChild(metric);
    });
  };
  const current = selectedEncounter();
  if (current && !compareMode) renderPlayerList(current, byDamage(current));
})();
