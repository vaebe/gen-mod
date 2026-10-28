use super::{models::*,registry,official};
use crate::cloud::catalog::{Catalog,Game,Skill};
use std::{fs,path::Path};
fn fixture(root:&Path,category:&str,game:Option<&str>,name:&str)->SkillDefinition{
    fs::create_dir_all(root).unwrap();fs::write(root.join("SKILL.md"),format!("---\nname: {name}\ndescription: desc\n---\ncontent")).unwrap();
    SkillDefinition{name:name.into(),description:"desc".into(),category:category.into(),game_id:game.map(str::to_owned),version:"1".into(),dependencies:vec![],path:String::new(),exclude_paths:vec![],required_command_scripts:vec![],inspection_file:None}
}
#[test] fn common_import_succeeds_without_any_game(){
    let root=tempfile::tempdir().unwrap();let source=root.path().join("source");let definition=fixture(&source,"common",None,"local-common");
    registry::validate_import_on(&definition,&source,&root.path().join("official"),&root.path().join("user")).unwrap();assert!(source.join("SKILL.md").exists());
}
#[test] fn game_import_requires_confirmed_synced_game_and_keeps_original_files(){
    let root=tempfile::tempdir().unwrap();let source=root.path().join("source");let official=root.path().join("official");let user=root.path().join("user");let definition=fixture(&source,"game",Some("duckov"),"local-game");
    let original=fs::read(source.join("SKILL.md")).unwrap();
    let catalog=Catalog{games:vec![Game{id:"duckov".into(),name:"Duckov".into(),description:String::new()}],skills:vec![]};official::save_catalog_on(&official,&catalog).unwrap();
    assert!(registry::validate_import_on(&definition,&source,&official,&user).unwrap_err().contains("已同步"));
    official::confirm_games_on(&official,catalog.games).unwrap();registry::validate_import_on(&definition,&source,&official,&user).unwrap();assert_eq!(fs::read(source.join("SKILL.md")).unwrap(),original);
}
#[test] fn official_name_conflicts_are_rejected_even_without_confirmed_versions(){
    let root=tempfile::tempdir().unwrap();let source=root.path().join("source");let official=root.path().join("official");let definition=fixture(&source,"common",None,"official-name");
    official::save_catalog_on(&official,&Catalog{games:vec![],skills:vec![Skill{id:"s".into(),name:"official-name".into(),description:"desc".into(),kind:"common".into(),game_id:None,versions:vec![]}]}).unwrap();
    assert!(registry::validate_import_on(&definition,&source,&official,&root.path().join("user")).unwrap_err().contains("官方"));
}
#[test] fn existing_import_name_is_not_overwritten(){
    let root=tempfile::tempdir().unwrap();let source=root.path().join("source");let user=root.path().join("user");let mut definition=fixture(&source,"common",None,"local-name");fs::create_dir_all(&user).unwrap();definition.path="local-name".into();
    crate::storage::mods::write_json(&user.join("registry.json"),&UserRegistry{imported:vec![definition.clone()],disabled:vec![]}).unwrap();
    assert!(registry::validate_import_on(&definition,&source,&root.path().join("official"),&user).unwrap_err().contains("已存在"));assert!(source.join("SKILL.md").exists());
}
#[test] fn bundle_does_not_include_skills_resource(){
    let config:serde_json::Value=serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    let resources=config["bundle"]["resources"].as_object().unwrap();
    assert!(!resources.keys().any(|key|key.contains("skills")));
}
