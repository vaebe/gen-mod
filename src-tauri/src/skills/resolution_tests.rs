use super::{registry,official,models::*};
use crate::cloud::catalog::*;
use std::{fs,path::Path,io::{Cursor,Write}};
fn publish(root:&Path,catalog:&mut Catalog,id:&str,name:&str,category:&str,game:Option<&str>,version:&str,date:&str){
    let manifest=Manifest{category:category.into(),game_id:game.map(str::to_owned),version:version.into(),dependencies:vec![],exclude_paths:vec![],required_command_scripts:vec![],inspection_file:None};
    let mut skill=Skill{id:id.into(),name:name.into(),description:"desc".into(),kind:category.into(),game_id:game.map(str::to_owned),versions:vec![]};
    let temp=tempfile::tempdir().unwrap();let markdown=format!("---\nname: {name}\ndescription: desc\n---\n{version}");fs::write(temp.path().join("SKILL.md"),&markdown).unwrap();
    let definition=SkillDefinition{name:name.into(),description:"desc".into(),category:category.into(),game_id:game.map(str::to_owned),version:version.into(),dependencies:vec![],exclude_paths:vec![],required_command_scripts:vec![],inspection_file:None,path:String::new()};
    let hash=registry::validate(&definition,temp.path()).unwrap().0;
    let record=Version{id:format!("{id}-{version}"),skill_id:id.into(),version:version.into(),content_hash:hash,file_id:format!("file-{id}-{version}"),uploaded_at:date.into(),manifest};
    let mut zip=zip::ZipWriter::new(Cursor::new(vec![]));zip.start_file("SKILL.md",zip::write::SimpleFileOptions::default()).unwrap();zip.write_all(markdown.as_bytes()).unwrap();let bytes=zip.finish().unwrap().into_inner();
    official::install_on(root,&skill,&record,&bytes).unwrap();skill.versions.push(record);
    if let Some(old)=catalog.skills.iter_mut().find(|old|old.id==id){old.versions.extend(skill.versions);}else{catalog.skills.push(skill);}
    official::save_catalog_on(root,catalog).unwrap();
}
fn import(root:&Path,category:&str,game:Option<&str>){
    fs::create_dir_all(root.join("imported")).unwrap();fs::write(root.join("imported/SKILL.md"),"---\nname: imported\ndescription: desc\n---\nlocal").unwrap();
    let definition=SkillDefinition{name:"imported".into(),description:"desc".into(),category:category.into(),game_id:game.map(str::to_owned),version:"local".into(),dependencies:vec![],exclude_paths:vec![],required_command_scripts:vec![],inspection_file:None,path:"imported".into()};
    crate::storage::mods::write_json(&root.join("registry.json"),&UserRegistry{imported:vec![definition],disabled:vec![]}).unwrap();
}
#[test] fn selects_each_skills_latest_confirmed_version_and_all_enabled_common_and_imports(){
    let root=tempfile::tempdir().unwrap();let official=root.path().join("official");let user=root.path().join("user");let mut catalog=Catalog::default();
    publish(&official,&mut catalog,"a","game-a","game",Some("duckov"),"1","2026-10-20T00:00:00Z");
    publish(&official,&mut catalog,"a","game-a","game",Some("duckov"),"2","2026-10-23T00:00:00Z");
    publish(&official,&mut catalog,"b","game-b","game",Some("duckov"),"9","2026-10-21T00:00:00Z");
    publish(&official,&mut catalog,"c","common","common",None,"3","2026-10-22T00:00:00Z");
    publish(&official,&mut catalog,"d","other","game",Some("other"),"1","2026-10-24T00:00:00Z");
    import(&user,"common",None);
    let views=registry::resolve_views_for_game(registry::list_on(&official,&user).unwrap(),"duckov").unwrap();
    assert_eq!(views.iter().map(|view|(view.definition.name.as_str(),view.definition.version.as_str())).collect::<Vec<_>>(),vec![("common","3"),("game-a","2"),("game-b","9"),("imported","local")]);
    assert!(registry::resolve_views_for_game(registry::list_on(&official,&user).unwrap(),"missing").is_err());
}
#[test] fn imported_game_alone_or_disabled_official_game_cannot_create_sessions(){
    let root=tempfile::tempdir().unwrap();let official=root.path().join("official");let user=root.path().join("user");import(&user,"game",Some("duckov"));
    assert!(registry::resolve_views_for_game(registry::list_on(&official,&user).unwrap(),"duckov").is_err());
    let mut catalog=Catalog::default();publish(&official,&mut catalog,"a","game-a","game",Some("duckov"),"1","2026-10-20");
    let mut views=registry::list_on(&official,&user).unwrap();views.iter_mut().filter(|v|v.source=="official").for_each(|v|v.enabled=false);
    assert!(registry::resolve_views_for_game(views,"duckov").is_err());
}
#[test] fn confirming_updates_makes_old_sessions_read_only_but_pinned_task_paths_and_hashes_remain(){
    let root=tempfile::tempdir().unwrap();let official=root.path().join("official");let user=root.path().join("user");let mut catalog=Catalog::default();
    publish(&official,&mut catalog,"a","game-a","game",Some("duckov"),"1","2026-10-20");
    let before=registry::resolve_views_for_game(registry::list_on(&official,&user).unwrap(),"duckov").unwrap();let old_path=before[0].definition.path.clone();
    let task=root.path().join("task");fs::create_dir(&task).unwrap();registry::pin_task_on(&task,&before).unwrap();
    let mut sessions=vec![crate::storage::models::SessionReference{session_id:"s".into(),session_file:String::new(),skills:registry::snapshots(&before),read_only:false,name:String::new()}];
    publish(&official,&mut catalog,"a","game-a","game",Some("duckov"),"2","2026-10-23");
    let after=registry::resolve_views_for_game(registry::list_on(&official,&user).unwrap(),"duckov").unwrap();
    registry::refresh_snapshots(&mut sessions,Some(&registry::snapshots(&after)));assert!(sessions[0].read_only);
    let pinned=registry::read_task_on(&task).unwrap();assert_eq!(pinned[0].definition.path,old_path);assert_eq!(pinned[0].content_hash,before[0].content_hash);assert!(Path::new(&old_path).join("SKILL.md").exists());
}
