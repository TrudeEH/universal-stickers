use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use image::GenericImageView;
use image::ImageReader;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const THUMB_SIZE: u32 = 160;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickerRecord {
    pub id: u64,
    pub name: String,
    pub kind: String,
    pub original_filename: String,
    pub asset_path: String,
    pub thumb_path: String,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct ImportRequest {
    pub path: PathBuf,
    pub name: Option<String>,
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StickerStore {
    data_dir: PathBuf,
    db_path: PathBuf,
    assets_dir: PathBuf,
    thumbs_dir: PathBuf,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("unsupported file type: {0}")]
    UnsupportedFormat(String),
    #[error("sticker name cannot be empty")]
    InvalidName,
    #[error("backup directory is missing manifest.json or stickers.db")]
    InvalidBackup,
    #[error("record not found")]
    NotFound,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupManifest {
    version: u32,
    items: Vec<BackupItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupItem {
    name: String,
    asset_file: String,
    original_filename: String,
}

impl StickerStore {
    pub fn initialize(data_dir: impl AsRef<Path>) -> Result<Self, StoreError> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let assets_dir = data_dir.join("assets");
        let thumbs_dir = data_dir.join("thumbs");
        let db_path = data_dir.join("stickers.db");

        fs::create_dir_all(&assets_dir)?;
        fs::create_dir_all(&thumbs_dir)?;

        let store = Self {
            data_dir,
            db_path,
            assets_dir,
            thumbs_dir,
        };

        let connection = store.connection()?;
        store.migrate(&connection)?;
        store.prune_missing_assets(&connection)?;
        drop(connection);

        Ok(store)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn list_items(&self, query: &str) -> Result<Vec<StickerRecord>, StoreError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT
                id,
                name,
                kind,
                original_filename,
                asset_path,
                thumb_path,
                mime_type,
                width,
                height,
                created_at,
                updated_at
             FROM stickers
             WHERE (?1 = '' OR lower(name) LIKE '%' || lower(?1) || '%')
             ORDER BY lower(name), id",
        )?;

        let rows = statement.query_map([query], Self::row_to_record)?;
        let items = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }

    pub fn get_item(&self, id: u64) -> Result<StickerRecord, StoreError> {
        let connection = self.connection()?;
        let record = connection
            .query_row(
                "SELECT
                    id,
                    name,
                    kind,
                    original_filename,
                    asset_path,
                    thumb_path,
                    mime_type,
                    width,
                    height,
                    created_at,
                    updated_at
                 FROM stickers
                 WHERE id = ?1",
                [id as i64],
                Self::row_to_record,
            )
            .optional()?;

        record.ok_or(StoreError::NotFound)
    }

    pub fn import_items(
        &self,
        requests: impl IntoIterator<Item = ImportRequest>,
    ) -> Result<Vec<StickerRecord>, StoreError> {
        let connection = self.connection()?;
        let mut imported = Vec::new();

        for request in requests {
            imported.push(self.import_one(&connection, request)?);
        }

        Ok(imported)
    }

    pub fn delete_item(&self, id: u64) -> Result<(), StoreError> {
        let connection = self.connection()?;
        let record = self.get_item(id)?;

        connection.execute("DELETE FROM stickers WHERE id = ?1", [id as i64])?;

        self.remove_if_exists(Path::new(&record.asset_path))?;
        self.remove_if_exists(Path::new(&record.thumb_path))?;

        Ok(())
    }

    pub fn delete_all_items(&self) -> Result<usize, StoreError> {
        let connection = self.connection()?;
        let records = self.list_items("")?;
        let deleted_count = records.len();

        connection.execute("DELETE FROM stickers", [])?;

        for record in records {
            self.remove_if_exists(Path::new(&record.asset_path))?;
            self.remove_if_exists(Path::new(&record.thumb_path))?;
        }

        Ok(deleted_count)
    }

    pub fn rename_item(&self, id: u64, new_name: &str) -> Result<StickerRecord, StoreError> {
        let trimmed_name = new_name.trim();
        if trimmed_name.is_empty() {
            return Err(StoreError::InvalidName);
        }

        let connection = self.connection()?;
        let updated_rows = connection.execute(
            "UPDATE stickers
             SET name = ?1, updated_at = ?2
             WHERE id = ?3",
            params![trimmed_name, unix_now(), id as i64],
        )?;

        if updated_rows == 0 {
            return Err(StoreError::NotFound);
        }

        self.get_item(id)
    }

    pub fn export_backup(&self, target_root: impl AsRef<Path>) -> Result<PathBuf, StoreError> {
        let target_root = target_root.as_ref();
        fs::create_dir_all(target_root)?;

        let backup_dir = target_root.join(format!("universal-stickers-backup-{}", unix_now()));
        let backup_assets_dir = backup_dir.join("assets");
        fs::create_dir_all(&backup_assets_dir)?;

        let items = self.list_items("")?;
        let mut manifest_items = Vec::with_capacity(items.len());

        for item in items {
            let source_asset = PathBuf::from(&item.asset_path);
            if !source_asset.exists() {
                return Err(StoreError::NotFound);
            }

            let asset_filename = source_asset
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("{}.img", Uuid::new_v4()));
            let backup_asset = backup_assets_dir.join(&asset_filename);
            fs::copy(&source_asset, &backup_asset)?;

            manifest_items.push(BackupItem {
                name: item.name,
                asset_file: format!("assets/{asset_filename}"),
                original_filename: item.original_filename,
            });
        }

        let manifest = BackupManifest {
            version: 1,
            items: manifest_items,
        };
        let manifest_path = backup_dir.join("manifest.json");
        let manifest_json = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| StoreError::Io(std::io::Error::other(error)))?;
        fs::write(manifest_path, manifest_json)?;

        Ok(backup_dir)
    }

    pub fn import_backup(&self, source_dir: impl AsRef<Path>) -> Result<usize, StoreError> {
        let source_dir = source_dir.as_ref();
        let manifest_path = source_dir.join("manifest.json");
        let requests = if manifest_path.exists() {
            self.import_requests_from_manifest(source_dir, &manifest_path)?
        } else {
            self.import_requests_from_installation(source_dir)?
        };

        let imported = self.import_items(requests)?;
        Ok(imported.len())
    }

    fn import_one(
        &self,
        connection: &Connection,
        request: ImportRequest,
    ) -> Result<StickerRecord, StoreError> {
        let source_path = request.path;
        let source_metadata = source_path.metadata()?;
        if !source_metadata.is_file() {
            return Err(StoreError::UnsupportedFormat(
                source_path.display().to_string(),
            ));
        }

        let format = image::ImageFormat::from_path(&source_path)
            .map_err(|_| StoreError::UnsupportedFormat(source_path.display().to_string()))?;
        let extension = extension_for_format(format);
        let mime_type = mime_guess::from_path(&source_path)
            .first_raw()
            .unwrap_or("application/octet-stream")
            .to_string();
        let kind = if mime_type == "image/gif" {
            "gif".to_string()
        } else {
            "static_image".to_string()
        };

        let generated_name = request
            .name
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| file_stem_or_name(&source_path));

        let asset_filename = format!("{}.{}", Uuid::new_v4(), extension);
        let thumb_filename = format!("{}.png", Uuid::new_v4());
        let asset_path = self.assets_dir.join(asset_filename);
        let thumb_path = self.thumbs_dir.join(thumb_filename);

        fs::copy(&source_path, &asset_path)?;

        let image = ImageReader::open(&asset_path)?
            .with_guessed_format()?
            .decode()?;
        let (width, height) = image.dimensions();
        let thumb = image.thumbnail(THUMB_SIZE, THUMB_SIZE);
        thumb.save(&thumb_path)?;

        let now = unix_now();
        let original_filename = request.original_filename.unwrap_or_else(|| {
            source_path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| generated_name.clone())
        });

        connection.execute(
            "INSERT INTO stickers (
                name,
                kind,
                original_filename,
                asset_path,
                thumb_path,
                mime_type,
                width,
                height,
                created_at,
                updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                generated_name,
                kind,
                original_filename,
                asset_path.to_string_lossy(),
                thumb_path.to_string_lossy(),
                mime_type,
                i64::from(width),
                i64::from(height),
                now,
                now
            ],
        )?;

        let id = connection.last_insert_rowid() as u64;
        self.get_item(id)
    }

    fn connection(&self) -> Result<Connection, StoreError> {
        Ok(Connection::open(&self.db_path)?)
    }

    fn migrate(&self, connection: &Connection) -> Result<(), StoreError> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS stickers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                original_filename TEXT NOT NULL,
                asset_path TEXT NOT NULL,
                thumb_path TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_stickers_name ON stickers(name);",
        )?;
        Ok(())
    }

    fn prune_missing_assets(&self, connection: &Connection) -> Result<(), StoreError> {
        let mut statement =
            connection.prepare("SELECT id, asset_path, thumb_path FROM stickers ORDER BY id")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)? as u64,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        for row in rows {
            let (id, asset_path, thumb_path) = row?;
            if !Path::new(&asset_path).exists() {
                connection.execute("DELETE FROM stickers WHERE id = ?1", [id as i64])?;
                self.remove_if_exists(Path::new(&thumb_path))?;
            }
        }

        Ok(())
    }

    fn import_requests_from_manifest(
        &self,
        source_dir: &Path,
        manifest_path: &Path,
    ) -> Result<Vec<ImportRequest>, StoreError> {
        let manifest_bytes = fs::read(manifest_path)?;
        let manifest: BackupManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|error| StoreError::Io(std::io::Error::other(error)))?;

        Ok(manifest
            .items
            .into_iter()
            .map(|item| ImportRequest {
                path: source_dir.join(item.asset_file),
                name: Some(item.name),
                original_filename: Some(item.original_filename),
            })
            .collect())
    }

    fn import_requests_from_installation(
        &self,
        source_dir: &Path,
    ) -> Result<Vec<ImportRequest>, StoreError> {
        let source_db = source_dir.join("stickers.db");
        if !source_db.exists() {
            return Err(StoreError::InvalidBackup);
        }

        let connection = Connection::open(source_db)?;
        let mut statement = connection.prepare(
            "SELECT
                name,
                original_filename,
                asset_path
             FROM stickers
             ORDER BY id",
        )?;

        let rows = statement.query_map([], |row| {
            Ok(ImportRequest {
                path: PathBuf::from(row.get::<_, String>(2)?),
                name: Some(row.get(0)?),
                original_filename: Some(row.get(1)?),
            })
        })?;

        let requests = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(requests)
    }

    fn remove_if_exists(&self, path: &Path) -> Result<(), StoreError> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(StoreError::Io(error)),
        }
    }

    fn row_to_record(row: &rusqlite::Row<'_>) -> Result<StickerRecord, rusqlite::Error> {
        Ok(StickerRecord {
            id: row.get::<_, i64>(0)? as u64,
            name: row.get(1)?,
            kind: row.get(2)?,
            original_filename: row.get(3)?,
            asset_path: row.get(4)?,
            thumb_path: row.get(5)?,
            mime_type: row.get(6)?,
            width: row.get::<_, i64>(7)? as u32,
            height: row.get::<_, i64>(8)? as u32,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }
}

fn file_stem_or_name(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sticker".to_string())
}

fn extension_for_format(format: image::ImageFormat) -> &'static str {
    match format {
        image::ImageFormat::Gif => "gif",
        image::ImageFormat::Jpeg => "jpg",
        image::ImageFormat::Png => "png",
        image::ImageFormat::Bmp => "bmp",
        image::ImageFormat::WebP => "webp",
        _ => "img",
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use tempfile::TempDir;

    fn setup_store() -> (TempDir, StickerStore) {
        let temp_dir = TempDir::new().expect("temp dir");
        let store = StickerStore::initialize(temp_dir.path()).expect("store init");
        (temp_dir, store)
    }

    fn write_png(path: &Path, color: [u8; 4]) {
        let image = RgbaImage::from_pixel(48, 48, Rgba(color));
        image.save(path).expect("write png");
    }

    #[test]
    fn imports_into_managed_storage() {
        let (_temp_dir, store) = setup_store();
        let source_dir = TempDir::new().expect("source dir");
        let source_file = source_dir.path().join("wave.png");
        write_png(&source_file, [255, 0, 0, 255]);

        let imported = store
            .import_items([ImportRequest {
                path: source_file.clone(),
                name: Some("Wave".to_string()),
                original_filename: None,
            }])
            .expect("import");

        assert_eq!(imported.len(), 1);
        let record = &imported[0];
        assert_eq!(record.name, "Wave");
        assert!(Path::new(&record.asset_path).exists());
        assert!(Path::new(&record.thumb_path).exists());
        assert_ne!(Path::new(&record.asset_path), source_file.as_path());
    }

    #[test]
    fn search_is_case_insensitive() {
        let (_temp_dir, store) = setup_store();
        let source_dir = TempDir::new().expect("source dir");
        let alpha = source_dir.path().join("alpha.png");
        let beta = source_dir.path().join("beta.png");
        write_png(&alpha, [255, 0, 0, 255]);
        write_png(&beta, [0, 255, 0, 255]);

        store
            .import_items([
                ImportRequest {
                    path: alpha,
                    name: Some("Party Blob".to_string()),
                    original_filename: None,
                },
                ImportRequest {
                    path: beta,
                    name: Some("Serious Cat".to_string()),
                    original_filename: None,
                },
            ])
            .expect("import");

        let matches = store.list_items("blob").expect("search");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Party Blob");
    }

    #[test]
    fn delete_removes_database_row_and_files() {
        let (_temp_dir, store) = setup_store();
        let source_dir = TempDir::new().expect("source dir");
        let source_file = source_dir.path().join("delete-me.png");
        write_png(&source_file, [0, 0, 255, 255]);

        let imported = store
            .import_items([ImportRequest {
                path: source_file,
                name: None,
                original_filename: None,
            }])
            .expect("import");
        let record = imported[0].clone();

        store.delete_item(record.id).expect("delete");

        assert!(!Path::new(&record.asset_path).exists());
        assert!(!Path::new(&record.thumb_path).exists());
        assert!(matches!(
            store.get_item(record.id),
            Err(StoreError::NotFound)
        ));
    }

    #[test]
    fn delete_all_removes_everything() {
        let (_temp_dir, store) = setup_store();
        let source_dir = TempDir::new().expect("source dir");
        let first = source_dir.path().join("first.png");
        let second = source_dir.path().join("second.png");
        write_png(&first, [0, 0, 255, 255]);
        write_png(&second, [255, 255, 0, 255]);

        let imported = store
            .import_items([
                ImportRequest {
                    path: first,
                    name: Some("First".to_string()),
                    original_filename: None,
                },
                ImportRequest {
                    path: second,
                    name: Some("Second".to_string()),
                    original_filename: None,
                },
            ])
            .expect("import");

        let deleted_count = store.delete_all_items().expect("delete all");

        assert_eq!(deleted_count, 2);
        assert!(store.list_items("").expect("list").is_empty());
        for record in imported {
            assert!(!Path::new(&record.asset_path).exists());
            assert!(!Path::new(&record.thumb_path).exists());
        }
    }

    #[test]
    fn duplicate_filenames_do_not_collide() {
        let (_temp_dir, store) = setup_store();
        let first_dir = TempDir::new().expect("first source");
        let second_dir = TempDir::new().expect("second source");
        let first = first_dir.path().join("same-name.png");
        let second = second_dir.path().join("same-name.png");
        write_png(&first, [1, 2, 3, 255]);
        write_png(&second, [4, 5, 6, 255]);

        let imported = store
            .import_items([
                ImportRequest {
                    path: first,
                    name: None,
                    original_filename: None,
                },
                ImportRequest {
                    path: second,
                    name: None,
                    original_filename: None,
                },
            ])
            .expect("import");

        assert_ne!(imported[0].asset_path, imported[1].asset_path);
        assert_ne!(imported[0].thumb_path, imported[1].thumb_path);
    }

    #[test]
    fn unsupported_formats_are_rejected() {
        let (_temp_dir, store) = setup_store();
        let source_dir = TempDir::new().expect("source dir");
        let source_file = source_dir.path().join("note.txt");
        fs::write(&source_file, "not an image").expect("write txt");

        let result = store.import_items([ImportRequest {
            path: source_file,
            name: Some("Bad".to_string()),
            original_filename: None,
        }]);

        assert!(matches!(result, Err(StoreError::UnsupportedFormat(_))));
    }

    #[test]
    fn startup_prunes_missing_assets() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store = StickerStore::initialize(temp_dir.path()).expect("store init");
        let source_dir = TempDir::new().expect("source dir");
        let source_file = source_dir.path().join("ghost.png");
        write_png(&source_file, [20, 20, 20, 255]);

        let imported = store
            .import_items([ImportRequest {
                path: source_file,
                name: Some("Ghost".to_string()),
                original_filename: None,
            }])
            .expect("import");
        let record = imported[0].clone();

        fs::remove_file(&record.asset_path).expect("remove asset");

        let reopened = StickerStore::initialize(temp_dir.path()).expect("reopen store");
        assert!(reopened.list_items("").expect("list").is_empty());
        assert!(!Path::new(&record.thumb_path).exists());
    }

    #[test]
    fn rename_updates_stored_name() {
        let (_temp_dir, store) = setup_store();
        let source_dir = TempDir::new().expect("source dir");
        let source_file = source_dir.path().join("rename-me.png");
        write_png(&source_file, [200, 10, 10, 255]);

        let imported = store
            .import_items([ImportRequest {
                path: source_file,
                name: Some("Before".to_string()),
                original_filename: None,
            }])
            .expect("import");

        let renamed = store
            .rename_item(imported[0].id, "After")
            .expect("rename succeeds");

        assert_eq!(renamed.name, "After");
        let matches = store.list_items("after").expect("search renamed");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, imported[0].id);
    }

    #[test]
    fn rename_rejects_blank_names() {
        let (_temp_dir, store) = setup_store();
        let source_dir = TempDir::new().expect("source dir");
        let source_file = source_dir.path().join("blank-name.png");
        write_png(&source_file, [10, 200, 10, 255]);

        let imported = store
            .import_items([ImportRequest {
                path: source_file,
                name: Some("Original".to_string()),
                original_filename: None,
            }])
            .expect("import");

        let result = store.rename_item(imported[0].id, "   ");
        assert!(matches!(result, Err(StoreError::InvalidName)));
    }

    #[test]
    fn exported_backup_can_be_imported() {
        let (_temp_dir, store) = setup_store();
        let source_dir = TempDir::new().expect("source dir");
        let source_file = source_dir.path().join("backup-me.png");
        write_png(&source_file, [33, 66, 99, 255]);

        store
            .import_items([ImportRequest {
                path: source_file,
                name: Some("Backup Me".to_string()),
                original_filename: None,
            }])
            .expect("import");

        let export_root = TempDir::new().expect("export dir");
        let backup_dir = store.export_backup(export_root.path()).expect("export");

        let restore_root = TempDir::new().expect("restore dir");
        let restored = StickerStore::initialize(restore_root.path()).expect("restore store");
        let imported_count = restored.import_backup(&backup_dir).expect("import backup");

        assert_eq!(imported_count, 1);
        let items = restored.list_items("backup").expect("list restored");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Backup Me");
    }

    #[test]
    fn can_import_from_previous_installation_directory() {
        let previous_root = TempDir::new().expect("previous install");
        let previous_store =
            StickerStore::initialize(previous_root.path()).expect("previous store");
        let source_dir = TempDir::new().expect("source dir");
        let source_file = source_dir.path().join("old-install.png");
        write_png(&source_file, [120, 30, 90, 255]);

        previous_store
            .import_items([ImportRequest {
                path: source_file,
                name: Some("Old Install".to_string()),
                original_filename: None,
            }])
            .expect("import previous");

        let current_root = TempDir::new().expect("current install");
        let current_store = StickerStore::initialize(current_root.path()).expect("current store");
        let imported_count = current_store
            .import_backup(previous_root.path())
            .expect("import previous install");

        assert_eq!(imported_count, 1);
        let items = current_store.list_items("old").expect("list imported");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Old Install");
    }
}
