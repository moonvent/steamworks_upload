const recipe = arguments[0], app = arguments[1];
const scripts = Array.from(document.scripts).filter(s => !s.src && s.textContent.includes(recipe.bootstrap_marker));
if (scripts.length !== 1) throw Error('Missing or ambiguous Tag Wizard bootstrap');
const text = scripts[0].textContent;
const start = text.indexOf('{', text.indexOf(recipe.bootstrap_marker) + recipe.bootstrap_marker.length);
let depth = 0, quoted = false, escaped = false, end = -1;
for (let i = start; i >= 0 && i < text.length; i++) {
    const c = text[i];
    if (quoted) {
        if (escaped) escaped = false;
        else if (c === '\\') escaped = true;
        else if (c === '"') quoted = false;
    } else if (c === '"') quoted = true;
    else if (c === '{' || c === '[') depth++;
    else if (c === '}' || c === ']') { if (--depth === 0) { end = i + 1; break; } }
}
if (end < 0) throw Error('Invalid Tag Wizard bootstrap');
const data = JSON.parse(text.slice(start, end));
if (data.appid !== app || !Array.isArray(data.devTags) || !Array.isArray(data.communityTags)
    || !data.tagNames || typeof data.tagNames !== 'object' || Array.isArray(data.tagNames))
    throw Error('Tag Wizard schema mismatch');
function tags(rows, weightKey) {
    const seen = new Set();
    return rows.map(row => {
        if (!Number.isInteger(row.tagid) || typeof row.name !== 'string'
            || !Number.isFinite(row[weightKey]) || seen.has(row.tagid)) throw Error('Invalid tag record');
        seen.add(row.tagid);
        return {tag_id: row.tagid, name: row.name, weight: row[weightKey]};
    });
}
const catalog = Object.entries(data.tagNames).map(([id, name]) => {
    if (!/^\d+$/.test(id) || typeof name !== 'string') throw Error('Invalid tag catalog');
    return {tag_id: Number(id), name, steam_description: null};
});
const questions = document.querySelectorAll('.question');
if (questions.length > 1) throw Error('Ambiguous wizard step');
const choices = Array.from(document.querySelectorAll(recipe.choices)).map(e => {
    const labels = document.querySelectorAll('label[for="' + e.id + '"]');
    if (e.type !== 'checkbox' || labels.length !== 1 || !/^\d+$/.test(e.value)) throw Error('Changed tag choice semantics');
    return {tag_id: Number(e.value), name: labels[0].textContent.trim(), selected: e.checked, steam_description: null};
});
return {app_id: app, developer_tags: tags(data.devTags, 'weight'), community_tags: tags(data.communityTags, 'count'),
    catalog, current_step: questions.length ? questions[0].id : null, choices,
    description_status: 'No per-tag descriptions extracted; null does not imply none exist elsewhere.',
    note: 'Developer tag order and survey choices are separate. This read does not verify public store visibility.'};
