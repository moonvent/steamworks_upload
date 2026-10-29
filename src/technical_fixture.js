// Synthetic offline editor. No Steam requests or account data.
window.fixtureRecords = {};
window.fixtureCreates = 0;
const statFields = {
  type:['stattype','select'], api_name:['statapiname','text'], permission:['permission','select'],
  increment_only:['incrementonly','checkbox'], max_change:['maxchange','text'], min:['min','text'],
  max:['max','text'], window:['windowsize','text'], default:['default','text'],
  aggregated:['aggregated','checkbox'], display_name:['displayname','text']
};
const achievementFields = {
  api_name:['apiname','text'], permission:['permission','select'], hidden:['hidden','checkbox'],
  progress_stat:['progress','select'], progress_min:['minval','text'], progress_max:['maxval','text'],
  display_name:['displayname','form'], description:['description','form']
};
function render(section, raw) {
  const stats=section==='stats', id=stats?'s1':'a2_0', prefix=stats?'stat1':'ach2_0';
  const table=document.querySelector(stats?'#statTable':'#achievementTable');
  let row=document.getElementById(id);
  if(!row){row=table.insertRow();row.id=id;}
  row.replaceChildren();row.className=stats?'dirty':'selected';
  for(const [key,[suffix,type]] of Object.entries(stats?statFields:achievementFields)) {
    const cell=row.insertCell(), e=document.createElement(type==='form'?'form':type==='select'?'select':'input');
    e.id=prefix+'_'+suffix;
    if(type==='form') {
      const source=typeof raw[key]==='string'?{english:raw[key]}:raw[key]||{};
      for(const lang of ['english','german','token']) {
        const input=document.createElement('input');input.name=lang;input.value=source[lang]||'';
        if(lang!=='english')input.type='hidden';e.append(input);
      }
    } else if(type==='select') {
      const options=key==='type'?['INT','FLOAT','AVGRATE']:key==='permission'?['0','1','2']:['-1','stat_rounds'];
      for(const v of options){const o=document.createElement('option');o.value=v;o.text=v==='-1'?'None':v;e.add(o);}
      if(key==='type')e.value=raw.type||'INT';
    } else if(type!=='text') {e.type=type;}
    cell.append(e);
  }
  if(!stats) {
    for(const gray of [false,true]) {
      const f=document.createElement('form');f.method='POST';f.enctype='multipart/form-data';
      f.action='https://partner.steamgames.com/images/uploadachievement';
      f.setAttribute('onsubmit',`return AIM.submit(this, {'onComplete': AchievementImageUploadCallbackClosure( 0, 2, 0, ${gray} ) })`);
      for(const [name,value] of Object.entries({appID:'0',statID:'2',bit:'0',requestType:gray?'achievement_gray':'achievement',MAX_FILE_SIZE:'3000000',sessionid:'offline-fixture'})) {
        const e=document.createElement('input');e.type='hidden';e.name=name;e.value=value;f.append(e);
      }
      const image=document.createElement('img');image.id=id+'_'+(gray?'icon_gray':'icon');f.append(image);
      const file=document.createElement('input');file.type='file';file.name='image';f.append(file);
      const upload=document.createElement('input');upload.type='submit';upload.value='Upload';f.append(upload);
      row.insertCell().append(f);
    }
  }
  const save=document.createElement('input');save.type='submit';save.value='Save';
  save.onclick=()=>{
    const v=s=>document.getElementById(prefix+'_'+s).value;
    const checked=s=>document.getElementById(prefix+'_'+s).checked?'1':'0';
    let result;
    if(stats) {
      result={stat_id:'1',name:v('statapiname'),type:v('stattype'),permission:v('permission'),incrementonly:checked('incrementonly'),aggregated:checked('aggregated'),display:{name:v('displayname')}};
      for(const k of ['min','max','maxchange','windowsize','default'])if(v(k)!=='')result[k]=v(k);
    }else {
      const map=s=>Object.fromEntries(Array.from(document.getElementById(prefix+'_'+s).elements).filter(e=>e.value).map(e=>[e.name,e.value]));
      result={...raw,stat_id:'2',bit_id:'0',api_name:v('apiname'),permission:v('permission'),hidden:checked('hidden'),display_name:map('displayname'),description:map('description')};
      result.progress=v('progress')==='-1'?null:{value:{operand1:v('progress')},min_val:v('minval'),max_val:v('maxval')};
    }
    window.fixtureRecords[section]=result;row.className='';row.replaceChildren();row.insertCell().textContent=stats?result.name:result.api_name;
  };
  row.insertCell().append(save);
}
window.EditStat=(_,raw)=>render('stats',raw);
window.EditAchievement=(_,raw)=>render('achievements',raw);
window.PerformNewStat=()=>{window.fixtureCreates++;render('stats',{});};
window.PerformNewAchievement=()=>{window.fixtureCreates++;render('achievements',{});};
