use serde::{Deserialize,Serialize};
use std::{fs,io::{Cursor,Read,Write},path::{Path,PathBuf},collections::BTreeSet};
use tauri::Manager;
use crate::{cloud::catalog::{Catalog,Game,Skill,Version},storage::{mods,paths}};
use super::models::SkillDefinition;
#[derive(Clone,Deserialize,Serialize)]
#[serde(rename_all="camelCase")]
pub struct ConfirmedVersion {pub skill_id:String,pub name:String,pub description:String,pub version:Version}
#[derive(Clone,Default,Deserialize,Serialize)]
pub struct Confirmed {pub games:Vec<Game>,pub versions:Vec<ConfirmedVersion>}
pub fn root(app:&tauri::AppHandle)->Result<PathBuf,String>{Ok(app.path().app_local_data_dir().map_err(|e|e.to_string())?.join("official"))}
fn read<T:serde::de::DeserializeOwned+Default>(root:&Path,name:&str)->Result<T,String>{
    let path=root.join(name);
    if !path.exists(){return Ok(T::default());}
    paths::reject_redirect(root)?;paths::reject_redirect(&path)?;
    serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())
}
pub fn confirmed_on(root:&Path)->Result<Confirmed,String>{read(root,"confirmed.json")}
pub fn catalog_on(root:&Path)->Result<Catalog,String>{read(root,"catalog.json")}
fn save<T:Serialize>(root:&Path,name:&str,value:&T)->Result<(),String>{fs::create_dir_all(root).map_err(|e|e.to_string())?;paths::reject_redirect(root)?;mods::write_json(&root.join(name),value)}
pub fn save_catalog_on(root:&Path,value:&Catalog)->Result<(),String>{save(root,"catalog.json",value)}
pub fn save_confirmed_on(root:&Path,value:&Confirmed)->Result<(),String>{save(root,"confirmed.json",value)}
pub fn confirm_games_on(root:&Path,games:Vec<Game>)->Result<(),String>{
    let mut confirmed=confirmed_on(root)?;
    for game in games {confirmed.games.retain(|old|old.id!=game.id);confirmed.games.push(game);}
    save_confirmed_on(root,&confirmed)
}
pub fn definition(root:&Path,entry:&ConfirmedVersion)->Result<SkillDefinition,String>{
    if !super::registry::valid_name(&entry.name){return Err("官方 Skill 名称无效".into());}
    let version=crate::changes::relative_path(&entry.version.version)?;
    if version.components().count()!=1{return Err("版本号不是有效的目录名称".into());}
    let manifest=&entry.version.manifest;
    Ok(SkillDefinition{name:entry.name.clone(),description:entry.description.clone(),category:manifest.category.clone(),game_id:manifest.game_id.clone(),version:manifest.version.clone(),dependencies:manifest.dependencies.clone(),path:root.join(&entry.name).join(version).to_string_lossy().into_owned(),exclude_paths:manifest.exclude_paths.clone(),required_command_scripts:manifest.required_command_scripts.clone(),inspection_file:manifest.inspection_file.clone()})
}
pub fn available_on(root:&Path)->Result<Vec<ConfirmedVersion>,String>{
    let confirmed=confirmed_on(root)?;let catalog=catalog_on(root)?;
    Ok(confirmed.versions.into_iter().filter(|old|catalog.skills.iter().any(|skill|skill.id==old.skill_id && skill.name==old.name && skill.versions.iter().any(|version|version.id==old.version.id && version.content_hash==old.version.content_hash && version.file_id==old.version.file_id))).collect())
}
pub fn install_on(root:&Path,skill:&Skill,version:&Version,bytes:&[u8])->Result<(),String>{
    if version.skill_id!=skill.id || version.manifest.version!=version.version || version.manifest.category!=skill.kind || version.manifest.game_id!=skill.game_id{return Err("官方版本清单关联不一致".into());}
    let entry=ConfirmedVersion{skill_id:skill.id.clone(),name:skill.name.clone(),description:skill.description.clone(),version:version.clone()};
    let definition=definition(root,&entry)?;let target=PathBuf::from(&definition.path);
    fs::create_dir_all(root).map_err(|e|e.to_string())?;paths::reject_redirect(root)?;
    let parent=root.join(&skill.name);fs::create_dir_all(&parent).map_err(|e|e.to_string())?;paths::reject_redirect(&parent)?;
    let temporary=tempfile::Builder::new().prefix(".download-").tempdir_in(&parent).map_err(|e|e.to_string())?;
    let mut archive=zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e|e.to_string())?;let mut names=BTreeSet::new();
    for index in 0..archive.len(){
        let mut file=archive.by_index(index).map_err(|e|e.to_string())?;
        let name=file.name().to_owned();let relative=crate::changes::relative_path(name.trim_end_matches('/'))?;
        if file.unix_mode().is_some_and(|mode|mode&0o170000==0o120000)||!names.insert(name.to_lowercase()){return Err("zip 含链接或重复路径".into());}
        if name=="registry.json"{return Err("官方 zip 不得携带 registry".into());}
        let output=temporary.path().join(relative);
        if file.is_dir(){fs::create_dir_all(output).map_err(|e|e.to_string())?;continue;}
        fs::create_dir_all(output.parent().ok_or("文件路径无效")?).map_err(|e|e.to_string())?;
        let mut destination=fs::File::create(output).map_err(|e|e.to_string())?;let mut content=vec![];file.read_to_end(&mut content).map_err(|e|e.to_string())?;destination.write_all(&content).map_err(|e|e.to_string())?;
    }
    let (hash,_)=super::registry::validate(&definition,temporary.path())?;
    if hash!=version.content_hash{return Err("官方 Skill 内容哈希不一致".into());}
    if target.exists(){
        let (existing,_)=super::registry::validate(&definition,&target)?;
        if existing!=hash{return Err("本机同版本目录内容不同，保留原目录，请发布新版本号".into());}
    }else{fs::rename(temporary.path(),&target).map_err(|e|e.to_string())?;}
    let mut confirmed=confirmed_on(root)?;confirmed.versions.retain(|old|old.version.id!=version.id);confirmed.versions.push(entry);save_confirmed_on(root,&confirmed)
}
