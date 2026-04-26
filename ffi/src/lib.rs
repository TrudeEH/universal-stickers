use std::path::PathBuf;

use universal_stickers_core::{ImportRequest, StickerStore};

#[cxx::bridge(namespace = "universal_stickers")]
mod ffi {
    struct StickerRecord {
        id: u64,
        name: String,
        kind: String,
        original_filename: String,
        asset_path: String,
        thumb_path: String,
        mime_type: String,
        width: u32,
        height: u32,
        created_at: i64,
        updated_at: i64,
    }

    extern "Rust" {
        type StickerLibrary;

        fn init_library(data_dir: String) -> Result<Box<StickerLibrary>>;
        fn list_items(self: &StickerLibrary, query: String) -> Result<Vec<StickerRecord>>;
        fn import_items(
            self: &StickerLibrary,
            paths: Vec<String>,
            names: Vec<String>,
        ) -> Result<Vec<StickerRecord>>;
        fn delete_item(self: &StickerLibrary, id: u64) -> Result<()>;
        fn delete_all_items(self: &StickerLibrary) -> Result<usize>;
        fn get_item(self: &StickerLibrary, id: u64) -> Result<StickerRecord>;
        fn rename_item(self: &StickerLibrary, id: u64, new_name: String) -> Result<StickerRecord>;
        fn export_backup(self: &StickerLibrary, target_dir: String) -> Result<String>;
        fn import_backup(self: &StickerLibrary, source_dir: String) -> Result<usize>;
    }
}

pub struct StickerLibrary {
    store: StickerStore,
}

fn init_library(data_dir: String) -> Result<Box<StickerLibrary>> {
    let store = StickerStore::initialize(PathBuf::from(data_dir))?;
    Ok(Box::new(StickerLibrary { store }))
}

impl StickerLibrary {
    fn list_items(&self, query: String) -> Result<Vec<ffi::StickerRecord>> {
        let items = self.store.list_items(&query)?;
        Ok(items.into_iter().map(map_record).collect())
    }

    fn import_items(
        &self,
        paths: Vec<String>,
        names: Vec<String>,
    ) -> Result<Vec<ffi::StickerRecord>> {
        let requests = paths
            .into_iter()
            .enumerate()
            .map(|(index, path)| ImportRequest {
                path: PathBuf::from(path),
                name: names.get(index).cloned(),
                original_filename: None,
            });

        let items = self.store.import_items(requests)?;
        Ok(items.into_iter().map(map_record).collect())
    }

    fn delete_item(&self, id: u64) -> Result<()> {
        self.store.delete_item(id)?;
        Ok(())
    }

    fn delete_all_items(&self) -> Result<usize> {
        Ok(self.store.delete_all_items()?)
    }

    fn get_item(&self, id: u64) -> Result<ffi::StickerRecord> {
        let item = self.store.get_item(id)?;
        Ok(map_record(item))
    }

    fn rename_item(&self, id: u64, new_name: String) -> Result<ffi::StickerRecord> {
        let item = self.store.rename_item(id, &new_name)?;
        Ok(map_record(item))
    }

    fn export_backup(&self, target_dir: String) -> Result<String> {
        let backup_dir = self.store.export_backup(PathBuf::from(target_dir))?;
        Ok(backup_dir.to_string_lossy().into_owned())
    }

    fn import_backup(&self, source_dir: String) -> Result<usize> {
        Ok(self.store.import_backup(PathBuf::from(source_dir))?)
    }
}

fn map_record(record: universal_stickers_core::StickerRecord) -> ffi::StickerRecord {
    ffi::StickerRecord {
        id: record.id,
        name: record.name,
        kind: record.kind,
        original_filename: record.original_filename,
        asset_path: record.asset_path,
        thumb_path: record.thumb_path,
        mime_type: record.mime_type,
        width: record.width,
        height: record.height,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
