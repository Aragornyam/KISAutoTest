import init, { analyze_repository } from './pkg/rust_repo_autotester.js';

const scanBtn = document.getElementById('scanBtn');
const statusText = document.getElementById('statusText');
const statusDot = document.getElementById('statusDot');
const resultsArea = document.getElementById('resultsArea');
const statsGrid = document.getElementById('statsGrid');
const projectsCard = document.getElementById('projectsCard');
const projectsList = document.getElementById('projectsList');
const violationsBody = document.getElementById('violationsBody');
const summaryBody = document.getElementById('summaryBody');
const summaryTitle = document.getElementById('summaryTitle');
const ruleFilter = document.getElementById('ruleFilter');
const severityFilter = document.getElementById('severityFilter');
const fileSearch = document.getElementById('fileSearch');

let currentViolations = [];

const BINARY_EXTENSIONS = new Set([
  '.zip', '.tar', '.gz', '.tgz', '.rar', '.7z', '.bz2', '.xz', '.zst',
  '.exe', '.dll', '.so', '.dylib', '.bin', '.o', '.obj', '.a', '.lib',
  '.class', '.pyc', '.pyo', '.pyd', '.wasm', '.pdb', '.jar', '.war',
  '.png', '.jpg', '.jpeg', '.gif', '.ico', '.pdf', '.mp3', '.mp4', '.woff', '.woff2'
]);

function setStatus(msg, color = '#38bdf8') {
  statusText.textContent = msg;
  statusDot.style.background = color;
  statusDot.style.boxShadow = `0 0 8px ${color}`;
}

async function boot() {
  try {
    await init();
    scanBtn.disabled = false;
    scanBtn.textContent = 'Проверить репозиторий';
    setStatus('Готов к работе. Введите ссылку на репозиторий или подпапку практики.', '#10b981');
  } catch (err) {
    setStatus('Ошибка инициализации WebAssembly: ' + err.message, '#ef4444');
  }
}

function parseGithubUrl(url) {
  const cleaned = url.trim().replace(/\.git$/, '').replace(/\/+$/, '');
  const treeMatch = cleaned.match(/github\.com\/([^\/]+)\/([^\/]+)\/tree\/([^\/]+)(?:\/(.+))?/i);
  if (treeMatch) {
    return {
      owner: treeMatch[1],
      repo: treeMatch[2],
      branch: treeMatch[3],
      subpath: treeMatch[4] ? treeMatch[4].replace(/^\/+|\/+$/g, '') : ''
    };
  }
  const match = cleaned.match(/github\.com\/([^\/]+)\/([^\/]+)/i) || cleaned.match(/^([^\/]+)\/([^\/]+)$/);
  if (!match) return null;
  return { owner: match[1], repo: match[2], branch: '', subpath: '' };
}

function isKnownBinary(path) {
  const lower = path.toLowerCase();
  for (const ext of BINARY_EXTENSIONS) {
    if (lower.endsWith(ext)) return true;
  }
  return false;
}

async function fetchRepoData(parsed, token) {
  const { owner, repo, branch, subpath } = parsed;
  const headers = { 'Accept': 'application/vnd.github+json' };
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  setStatus(`Запрос истории коммитов ${owner}/${repo}...`);
  const shaParam = branch ? `&sha=${encodeURIComponent(branch)}` : '';
  const commitsResp = await fetch(
    `https://api.github.com/repos/${owner}/${repo}/commits?per_page=30${shaParam}`,
    { headers }
  );
  if (!commitsResp.ok) {
    throw new Error(`GitHub API вернул ${commitsResp.status}. Проверьте ссылку или укажите токен.`);
  }
  const commitsJson = await commitsResp.json();
  if (!Array.isArray(commitsJson) || commitsJson.length === 0) {
    throw new Error('Репозиторий пуст или коммиты не найдены.');
  }

  const headSha = commitsJson[0].sha;
  const commits = commitsJson.map(c => ({
    sha: c.sha,
    message: c.commit && c.commit.message ? c.commit.message : ''
  }));

  setStatus(`Получение дерева файлов (${headSha.slice(0, 7)})...`);
  const treeResp = await fetch(
    `https://api.github.com/repos/${owner}/${repo}/git/trees/${headSha}?recursive=1`,
    { headers }
  );
  if (!treeResp.ok) {
    throw new Error(`Не удалось загрузить дерево файлов (${treeResp.status})`);
  }
  const treeJson = await treeResp.json();
  let blobs = (treeJson.tree || []).filter(item => item.type === 'blob');

  if (subpath) {
    const prefix = subpath + '/';
    blobs = blobs.filter(item =>
      item.path.startsWith(prefix) ||
      item.path === '.gitignore' ||
      item.path.toLowerCase() === 'readme.md' ||
      item.path === 'Makefile' ||
      item.path === 'run.sh' ||
      item.path === 'run.bat'
    );
  }

  const files = [];
  const concurrency = 10;
  let completed = 0;

  async function worker(queue) {
    while (queue.length > 0) {
      const item = queue.shift();
      const binary = isKnownBinary(item.path);
      let content = null;

      if (!binary && (item.size || 0) < 500000) {
        try {
          const rawUrl = `https://raw.githubusercontent.com/${owner}/${repo}/${headSha}/${encodeURI(item.path)}`;
          const rawResp = await fetch(rawUrl);
          if (rawResp.ok) {
            content = await rawResp.text();
          }
        } catch (_) {
          content = null;
        }
      }

      files.push({
        path: item.path,
        content,
        is_binary: binary
      });

      completed++;
      setStatus(`Скачивание файлов: ${completed} из ${blobs.length}...`);
    }
  }

  const queue = [...blobs];
  const workers = Array.from({ length: concurrency }, () => worker(queue));
  await Promise.all(workers);

  return {
    repo_name: subpath ? `${owner}/${repo} (${subpath})` : `${owner}/${repo}`,
    files,
    commits
  };
}

function escapeHtml(str) {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function renderViolations() {
  const ruleVal = ruleFilter.value;
  const sevVal = severityFilter.value;
  const query = fileSearch.value.trim().toLowerCase();

  const filtered = currentViolations.filter(v => {
    if (ruleVal !== 'ALL' && v.rule_id !== ruleVal) return false;
    if (sevVal !== 'ALL' && v.severity !== sevVal) return false;
    if (query) {
      const hay = `${v.file} ${v.snippet} ${v.description} ${v.rule_id}`.toLowerCase();
      if (!hay.includes(query)) return false;
    }
    return true;
  });

  if (filtered.length === 0) {
    violationsBody.innerHTML = '<tr><td colspan="4">Нарушений по заданным фильтрам не найдено.</td></tr>';
    return;
  }

  violationsBody.innerHTML = filtered.map(v => {
    const isWarn = v.severity === 'warning';
    const badgeClass = isWarn ? 'badge-warn' : 'badge-err';
    const sevText = isWarn ? 'Некритично' : 'Критично';

    let loc = escapeHtml(v.file);
    if (v.line) {
      loc += v.end_line && v.end_line !== v.line
        ? `:${v.line}-${v.end_line}`
        : `:${v.line}`;
    }

    return `
      <tr>
        <td><span class="badge ${badgeClass}">${escapeHtml(v.rule_id)}</span></td>
        <td><span class="badge ${badgeClass}">${sevText}</span></td>
        <td><code>${loc}</code></td>
        <td>
          <div><strong>${escapeHtml(v.description)}</strong></div>
          <div class="snippet-box"><code>${escapeHtml(v.snippet)}</code></div>
        </td>
      </tr>
    `;
  }).join('');
}

ruleFilter.addEventListener('change', renderViolations);
severityFilter.addEventListener('change', renderViolations);
fileSearch.addEventListener('input', renderViolations);

scanBtn.addEventListener('click', async () => {
  const parsed = parseGithubUrl(document.getElementById('repoUrl').value);
  const token = document.getElementById('ghToken').value.trim();

  if (!parsed) {
    setStatus('Введите корректную ссылку вида https://github.com/owner/repo', '#ef4444');
    return;
  }

  scanBtn.disabled = true;
  resultsArea.classList.add('hidden');

  try {
    const payload = await fetchRepoData(parsed, token);
    setStatus('Выполнение статического анализа в WebAssembly...');

    const reportJson = analyze_repository(JSON.stringify(payload));
    const report = JSON.parse(reportJson);

    if (report.error) {
      throw new Error(report.error);
    }

    currentViolations = report.violations || [];

    statsGrid.innerHTML = `
      <div class="metric-card">
        <div class="metric-label">Файлов и коммитов</div>
        <div class="metric-value">${report.total_files}</div>
        <div class="metric-sub">Коммитов проверено: ${report.total_commits}</div>
      </div>
      <div class="metric-card">
        <div class="metric-label">Правил пройдено</div>
        <div class="metric-value" style="color:var(--ok)">${report.passed_rules} / ${report.total_rules}</div>
        <div class="metric-sub">Нарушено правил: ${report.failed_rules + report.warning_rules}</div>
      </div>
      <div class="metric-card">
        <div class="metric-label">Критичные ошибки</div>
        <div class="metric-value" style="color:var(--err)">${report.critical_violations}</div>
        <div class="metric-sub">Требуют исправления</div>
      </div>
      <div class="metric-card">
        <div class="metric-label">Некритичные (src / tests)</div>
        <div class="metric-value" style="color:var(--warn)">${report.warning_violations}</div>
        <div class="metric-sub">Предупреждения структуры</div>
      </div>
    `;

    if (report.detected_projects && report.detected_projects.length > 0) {
      projectsCard.classList.remove('hidden');
      projectsList.innerHTML = report.detected_projects
        .map(p => `<span class="badge badge-info">${escapeHtml(p)}</span>`)
        .join('');
    } else {
      projectsCard.classList.add('hidden');
    }

    const violatedRuleIds = [...new Set(currentViolations.map(v => v.rule_id))];
    ruleFilter.innerHTML = `<option value="ALL">Все правила (${currentViolations.length})</option>` +
      violatedRuleIds.map(id => `<option value="${id}">Правило ${id}</option>`).join('');

    renderViolations();

    summaryTitle.textContent = `Сводка по всем ${report.total_rules} правилам (С1–И7)`;
    summaryBody.innerHTML = report.rules_summary.map(r => {
      let badgeHtml = '<span class="badge badge-ok">Пройдено</span>';
      if (!r.passed) {
        badgeHtml = r.severity === 'warning'
          ? `<span class="badge badge-warn">Некритично: ${r.count}</span>`
          : `<span class="badge badge-err">Ошибок: ${r.count}</span>`;
      }
      return `
        <tr>
          <td><strong>${escapeHtml(r.rule_id)}</strong></td>
          <td>${escapeHtml(r.category)}</td>
          <td>${escapeHtml(r.description)}</td>
          <td>${badgeHtml}</td>
        </tr>
      `;
    }).join('');

    resultsArea.classList.remove('hidden');
    setStatus(`Проверка репозитория ${report.repo_name} завершена.`, '#10b981');
  } catch (err) {
    setStatus('Ошибка: ' + err.message, '#ef4444');
  } finally {
    scanBtn.disabled = false;
  }
});

boot();