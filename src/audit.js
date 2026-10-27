const content = arguments[0];
const visible = e => !!e.getClientRects().length && getComputedStyle(e).visibility !== 'hidden';
const text = e => (e?.innerText || e?.textContent || '').replace(/\s+/g,' ').trim();
const title = el => {
  const row=el.closest('.formrow,.section,tr');
  const labelled=el.getAttribute('aria-label') || Array.from(el.labels || []).map(text).join(' ');
  return labelled || text(row?.querySelector('h2,h3,.formlabel,.title,label')) || el.name || el.id || 'Unnamed control';
};
const required = Array.from(content.querySelectorAll('.required_status')).filter(visible).map(e=>({
  label:text(e.closest('h1,h2,h3,.formlabel') || e.parentElement).replace(/[*✔]/g,'').trim(),
  state:e.classList.contains('required_set')?'complete':'missing',
  source:'steamworks-required-status'
}));
const fields = Array.from(content.querySelectorAll('input,textarea,select')).filter(e=>
  visible(e) && !['hidden','password','submit','button','reset','file'].includes(e.type)
  && (e.id || e.name)
).map(e=>({
  field:e.id?'id:'+e.id:'name:'+e.name,
  label:title(e),
  type:e.type,
  required:!!e.required || e.getAttribute('aria-required')==='true',
  empty: ['checkbox','radio'].includes(e.type) ? null : !String(e.value||'').trim(),
  editable: !e.disabled && !e.readOnly,
  options:e.tagName==='SELECT'?Array.from(e.options).map(o=>({value:o.value,label:o.text})):undefined,
  note:/_compl$/.test(e.name)?'autocomplete: choose a suggestion in the browser':undefined
}));
const assets=Array.from(content.querySelectorAll('.graphical_assets_tab')).map(e=>({label:text(e),source:'steamworks-asset-counter'}));
const widgets=Array.from(content.querySelectorAll('[data-featuretarget]')).map(e=>({
  widget:e.dataset.featuretarget,
  editor_type:(()=>{try{return JSON.parse(e.dataset.props||'{}').editorType || null;}catch{return null;}})(),
  visible:visible(e)
}));
const uploads=Array.from(content.querySelectorAll('input[type=file]')).map(e=>({
  field:e.id?'id:'+e.id:e.name?'name:'+e.name:null,
  accept:e.accept||null,
  available:!!(e.id||e.name),
  note:'An explicit upload may send files immediately; unnamed widgets need their own adapter.'
}));
return {required,fields,assets,widgets,uploads,
  empty_fields:fields.filter(f=>f.empty===true),
  unnamed_controls:Array.from(content.querySelectorAll('input,textarea,select')).filter(e=>visible(e)&&!e.id&&!e.name&&!['hidden','password','submit','button','reset'].includes(e.type)).length
};
