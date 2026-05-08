use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    sync::mpsc,
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result, anyhow};
#[cfg(target_os = "linux")]
use futures_util::StreamExt;
use gtk::{
    Align, Box as GtkBox, Button, CssProvider, DrawingArea, DropTarget, Entry,
    EventControllerMotion, FileDialog, FileFilter, FlowBox, GestureClick, Label, Orientation,
    Overlay, PolicyType, ScrolledWindow, SearchEntry, SelectionMode, gdk, gio, glib, prelude::*,
};
use libadwaita as adw;
use libadwaita::prelude::*;
use universal_stickers_core::{ImportRequest, StickerRecord, StickerStore};

const APP_ID: &str = "dev.trude.UniversalStickers";
const LEGACY_ORG_DIR: &str = "UniversalStickers";
const LEGACY_APP_DIR: &str = "Universal Stickers";
#[cfg(target_os = "linux")]
const SHORTCUT_ID: &str = "toggle-picker";
const DEFAULT_DISPLAY_SCALE: u32 = 100;
const MIN_DISPLAY_SCALE: u32 = 50;
const MAX_DISPLAY_SCALE: u32 = 200;
const DISPLAY_SCALE_STEP: u32 = 5;
const BASE_CARD_SIZE: i32 = 170;
const GRID_GAP: i32 = 12;
const GRID_MARGIN: i32 = 16;
const DEFAULT_WINDOW_WIDTH: i32 = 1040;
const GIF_ANIMATION_INTERVAL: Duration = Duration::from_millis(100);

struct AppState {
    store: StickerStore,
    window: adw::ApplicationWindow,
    search: SearchEntry,
    grid: FlowBox,
    status: Label,
    records: Rc<RefCell<Vec<StickerRecord>>>,
    search_reload_source: RefCell<Option<glib::SourceId>>,
    gif_tiles: RefCell<Vec<GifTile>>,
    gif_animation_source: RefCell<Option<glib::SourceId>>,
    display_scale: Rc<Cell<u32>>,
    settings_path: PathBuf,
}

struct GifTile {
    area: glib::WeakRef<DrawingArea>,
    iter: gtk::gdk_pixbuf::PixbufAnimationIter,
    frame: Rc<RefCell<Option<gtk::gdk_pixbuf::Pixbuf>>>,
}

fn main() {
    adw::init().expect("initialize libadwaita");

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| install_css());
    app.connect_activate(build_application);
    app.run();
}

fn build_application(app: &adw::Application) {
    if let Err(error) = build_application_inner(app) {
        show_startup_error(app, &error);
    }
}

fn build_application_inner(app: &adw::Application) -> Result<()> {
    let data_dir = legacy_data_dir()?;
    let store = StickerStore::initialize(&data_dir)?;
    let settings_path = data_dir.join("settings.conf");
    let display_scale = Rc::new(Cell::new(load_display_scale(&settings_path)));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Universal Stickers")
        .default_width(DEFAULT_WINDOW_WIDTH)
        .default_height(720)
        .build();
    window.set_icon_name(Some(APP_ID));

    let root = GtkBox::new(Orientation::Vertical, 0);
    let toolbar = adw::HeaderBar::new();

    let search = SearchEntry::builder()
        .placeholder_text("Search stickers")
        .hexpand(true)
        .width_request(260)
        .build();
    toolbar.set_title_widget(Some(&search));

    let add_button = Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("Add stickers")
        .build();
    toolbar.pack_start(&add_button);

    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Main menu")
        .build();
    toolbar.pack_end(&menu_button);
    root.append(&toolbar);

    let grid = FlowBox::builder()
        .selection_mode(SelectionMode::None)
        .homogeneous(true)
        .column_spacing(GRID_GAP as u32)
        .row_spacing(GRID_GAP as u32)
        .margin_top(GRID_MARGIN)
        .margin_bottom(GRID_MARGIN)
        .margin_start(GRID_MARGIN)
        .margin_end(GRID_MARGIN)
        .halign(Align::Fill)
        .valign(Align::Start)
        .hexpand(true)
        .build();
    grid.set_min_children_per_line(1);
    grid.set_max_children_per_line(128);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .propagate_natural_width(false)
        .min_content_width(0)
        .child(&grid)
        .vexpand(true)
        .build();
    root.append(&scroll);

    let status = Label::builder()
        .xalign(0.0)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(10)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    root.append(&status);

    window.set_content(Some(&root));

    let state = AppState {
        store,
        window: window.clone(),
        search: search.clone(),
        grid: grid.clone(),
        status: status.clone(),
        records: Rc::new(RefCell::new(Vec::new())),
        search_reload_source: RefCell::new(None),
        gif_tiles: RefCell::new(Vec::new()),
        gif_animation_source: RefCell::new(None),
        display_scale,
        settings_path,
    };
    let state = Rc::new(state);

    install_main_popover(&menu_button, &state);
    install_actions(app, &state);
    install_drag_and_drop(&state);

    {
        let state = state.clone();
        add_button.connect_clicked(move |_| choose_import_files(&state));
    }
    {
        let state = state.clone();
        search.connect_search_changed(move |_| schedule_reload_items(&state));
    }

    reload_items(&state);
    window.present();
    setup_global_shortcut(Rc::downgrade(&state));
    Ok(())
}

fn install_actions(app: &adw::Application, state: &Rc<AppState>) {
    app.add_action_entries([
        gio::ActionEntry::builder("import")
            .activate({
                let state = state.clone();
                move |_, _, _| choose_import_files(&state)
            })
            .build(),
        gio::ActionEntry::builder("export-backup")
            .activate({
                let state = state.clone();
                move |_, _, _| choose_export_folder(&state)
            })
            .build(),
        gio::ActionEntry::builder("import-backup")
            .activate({
                let state = state.clone();
                move |_, _, _| choose_backup_folder(&state)
            })
            .build(),
        gio::ActionEntry::builder("delete-all")
            .activate({
                let state = state.clone();
                move |_, _, _| confirm_delete_all(&state)
            })
            .build(),
    ]);
}

fn install_main_popover(menu_button: &gtk::MenuButton, state: &Rc<AppState>) {
    let popover = gtk::Popover::new();
    let menu = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .spacing(2)
        .width_request(230)
        .build();

    let size_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .margin_top(4)
        .margin_bottom(4)
        .margin_start(8)
        .margin_end(8)
        .build();
    let decrease_button = Button::builder()
        .icon_name("value-decrease-symbolic")
        .tooltip_text("Smaller cards")
        .css_classes(["flat"])
        .build();
    let size_label = Label::builder()
        .hexpand(true)
        .xalign(0.5)
        .width_request(78)
        .build();
    let increase_button = Button::builder()
        .icon_name("value-increase-symbolic")
        .tooltip_text("Larger cards")
        .css_classes(["flat"])
        .build();
    size_row.append(&decrease_button);
    size_row.append(&size_label);
    size_row.append(&increase_button);
    menu.append(&size_row);
    menu.append(&separator());

    update_size_controls(state, &size_label, &decrease_button, &increase_button);

    {
        let state = state.clone();
        let size_label = size_label.clone();
        let decrease_button_for_update = decrease_button.clone();
        let increase_button = increase_button.clone();
        decrease_button.connect_clicked(move |_| {
            adjust_display_scale(&state, -(DISPLAY_SCALE_STEP as i32));
            update_size_controls(
                &state,
                &size_label,
                &decrease_button_for_update,
                &increase_button,
            );
        });
    }
    {
        let state = state.clone();
        let size_label = size_label.clone();
        let decrease_button = decrease_button.clone();
        let increase_button_for_update = increase_button.clone();
        increase_button.connect_clicked(move |_| {
            adjust_display_scale(&state, DISPLAY_SCALE_STEP as i32);
            update_size_controls(
                &state,
                &size_label,
                &decrease_button,
                &increase_button_for_update,
            );
        });
    }

    let export_button = popover_button("Export Backup");
    {
        let state = state.clone();
        let popover = popover.clone();
        export_button.connect_clicked(move |_| {
            popover.popdown();
            choose_export_folder(&state);
        });
    }
    menu.append(&export_button);

    let import_backup_button = popover_button("Import Backup");
    {
        let state = state.clone();
        let popover = popover.clone();
        import_backup_button.connect_clicked(move |_| {
            popover.popdown();
            choose_backup_folder(&state);
        });
    }
    menu.append(&import_backup_button);
    menu.append(&separator());

    let delete_all_button = popover_button("Delete All Stickers");
    {
        let state = state.clone();
        let popover = popover.clone();
        delete_all_button.connect_clicked(move |_| {
            popover.popdown();
            confirm_delete_all(&state);
        });
    }
    menu.append(&delete_all_button);

    popover.set_child(Some(&menu));
    menu_button.set_popover(Some(&popover));
}

fn popover_button(label: &str) -> Button {
    Button::builder()
        .label(label)
        .halign(Align::Fill)
        .hexpand(true)
        .css_classes(["flat"])
        .build()
}

fn separator() -> gtk::Separator {
    gtk::Separator::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(4)
        .margin_bottom(4)
        .build()
}

fn adjust_display_scale(state: &Rc<AppState>, delta: i32) {
    let current = state.display_scale.get() as i32;
    let next = (current + delta).clamp(MIN_DISPLAY_SCALE as i32, MAX_DISPLAY_SCALE as i32) as u32;
    if next == state.display_scale.get() {
        return;
    }
    state.display_scale.set(next);
    save_display_scale(&state.settings_path, next);
    rebuild_grid(state);
}

fn update_size_controls(
    state: &Rc<AppState>,
    label: &Label,
    decrease_button: &Button,
    increase_button: &Button,
) {
    let scale = state.display_scale.get();
    label.set_text(&format!("{scale}%"));
    decrease_button.set_sensitive(scale > MIN_DISPLAY_SCALE);
    increase_button.set_sensitive(scale < MAX_DISPLAY_SCALE);
}

fn install_drag_and_drop(state: &Rc<AppState>) {
    let drop_target = DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    {
        let state = state.clone();
        drop_target.connect_drop(move |_, value, _, _| {
            let Ok(file_list) = value.get::<gdk::FileList>() else {
                return false;
            };
            let paths = file_list
                .files()
                .iter()
                .filter_map(|file| file.path())
                .filter(|path| is_supported_import_path(path))
                .collect::<Vec<_>>();

            if paths.is_empty() {
                return false;
            }

            import_paths_with_prompts(&state, paths);
            true
        });
    }
    state.window.add_controller(drop_target);
}

fn schedule_reload_items(state: &Rc<AppState>) {
    if let Some(source_id) = state.search_reload_source.borrow_mut().take() {
        source_id.remove();
    }

    let callback_state = state.clone();
    let source_id = glib::timeout_add_local_once(Duration::from_millis(150), move || {
        *callback_state.search_reload_source.borrow_mut() = None;
        reload_items(&callback_state);
    });
    *state.search_reload_source.borrow_mut() = Some(source_id);
}

fn reload_items(state: &Rc<AppState>) {
    match state.store.list_items(state.search.text().as_str()) {
        Ok(items) => {
            *state.records.borrow_mut() = items;
            rebuild_grid(state);
        }
        Err(error) => show_error(state, "Loading stickers failed", &error.to_string()),
    }
}

fn rebuild_grid(state: &Rc<AppState>) {
    state.gif_tiles.borrow_mut().clear();
    while let Some(child) = state.grid.first_child() {
        state.grid.remove(&child);
    }

    let records = state.records.borrow();
    if records.is_empty() {
        let empty = Label::builder()
            .label("No stickers found. Import a few to get started.")
            .margin_top(64)
            .css_classes(["dim-label"])
            .build();
        state.grid.insert(&empty, -1);
        return;
    }

    for record in records.iter().cloned() {
        let tile = build_sticker_tile(state, record);
        state.grid.insert(&tile, -1);
    }
}

fn build_sticker_tile(state: &Rc<AppState>, record: StickerRecord) -> GtkBox {
    let scale = state.display_scale.get();
    let card_size = scaled_dimension(BASE_CARD_SIZE, scale);
    let tile = GtkBox::new(Orientation::Vertical, 0);
    tile.set_size_request(card_size, card_size);
    tile.set_halign(Align::Center);
    tile.set_valign(Align::Center);
    tile.set_hexpand(false);
    tile.set_vexpand(false);
    tile.set_overflow(gtk::Overflow::Hidden);
    tile.set_css_classes(&["sticker-tile"]);

    let root = Overlay::new();
    root.set_size_request(card_size, card_size);
    root.set_halign(Align::Center);
    root.set_valign(Align::Center);
    root.set_hexpand(false);
    root.set_vexpand(false);
    root.set_overflow(gtk::Overflow::Hidden);

    let actions = GtkBox::new(Orientation::Horizontal, 6);
    actions.set_halign(Align::Fill);
    actions.set_valign(Align::Start);
    actions.set_margin_top(6);
    actions.set_margin_start(6);
    actions.set_margin_end(6);
    actions.set_visible(false);
    let edit_button = Button::builder()
        .icon_name("document-edit-symbolic")
        .tooltip_text("Rename sticker")
        .css_classes(["flat", "sticker-overlay-button"])
        .build();
    let delete_button = Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Delete sticker")
        .css_classes(["flat", "sticker-overlay-button"])
        .build();
    actions.append(&edit_button);
    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    actions.append(&spacer);
    actions.append(&delete_button);

    let media = media_canvas(state, &record, card_size);
    root.set_child(Some(&media));
    root.add_overlay(&actions);

    let name = Label::builder()
        .label(&record.name)
        .wrap(true)
        .justify(gtk::Justification::Center)
        .xalign(0.5)
        .halign(Align::Fill)
        .valign(Align::End)
        .margin_start(6)
        .margin_end(6)
        .margin_bottom(6)
        .css_classes(["sticker-name-overlay"])
        .build();
    name.set_visible(false);
    root.add_overlay(&name);

    tile.append(&root);

    {
        let enter_actions = actions.clone();
        let enter_name = name.clone();
        let motion = EventControllerMotion::new();
        motion.connect_enter(move |_, _, _| {
            enter_actions.set_visible(true);
            enter_name.set_visible(true);
        });
        let leave_actions = actions.clone();
        let leave_name = name.clone();
        motion.connect_leave(move |_| {
            leave_actions.set_visible(false);
            leave_name.set_visible(false);
        });
        tile.add_controller(motion);
    }

    {
        let state = state.clone();
        let record = record.clone();
        edit_button.connect_clicked(move |_| rename_sticker(&state, record.id, &record.name));
    }
    {
        let state = state.clone();
        let record = record.clone();
        delete_button
            .connect_clicked(move |_| confirm_delete_item(&state, record.id, &record.name));
    }
    {
        add_copy_click_controller(&media, &tile, state, record.id);
        add_copy_click_controller(&name, &tile, state, record.id);
    }

    tile
}

fn add_copy_click_controller<W: IsA<gtk::Widget>>(
    widget: &W,
    tile: &GtkBox,
    state: &Rc<AppState>,
    record_id: u64,
) {
    let click = GestureClick::new();
    click.connect_pressed({
        let tile = tile.clone();
        move |_, _, _, _| tile.add_css_class("pressed")
    });
    click.connect_stopped({
        let tile = tile.clone();
        move |_| tile.remove_css_class("pressed")
    });
    click.connect_released({
        let state = state.clone();
        move |_, _, _, _| copy_sticker(&state, record_id)
    });
    widget.add_controller(click);
}

fn media_canvas(state: &Rc<AppState>, record: &StickerRecord, card_size: i32) -> DrawingArea {
    let area = DrawingArea::builder()
        .content_width(card_size)
        .content_height(card_size)
        .width_request(card_size)
        .height_request(card_size)
        .halign(Align::Center)
        .valign(Align::Center)
        .hexpand(false)
        .vexpand(false)
        .css_classes(["sticker-media"])
        .build();
    area.set_size_request(card_size, card_size);

    let frame = Rc::new(RefCell::new(load_grid_frame(record)));
    {
        let frame = frame.clone();
        area.set_draw_func(move |_, cr, width, height| {
            if let Some(pixbuf) = frame.borrow().as_ref() {
                draw_pixbuf_cover(cr, pixbuf, width, height);
            }
        });
    }

    if record.kind == "gif" {
        register_gif_tile(state, &area, record, frame);
    }

    area
}

fn load_grid_frame(record: &StickerRecord) -> Option<gtk::gdk_pixbuf::Pixbuf> {
    gtk::gdk_pixbuf::Pixbuf::from_file(&record.thumb_path)
        .or_else(|_| gtk::gdk_pixbuf::Pixbuf::from_file(&record.asset_path))
        .ok()
}

fn register_gif_tile(
    state: &Rc<AppState>,
    area: &DrawingArea,
    record: &StickerRecord,
    frame: Rc<RefCell<Option<gtk::gdk_pixbuf::Pixbuf>>>,
) {
    let Ok(animation) = gtk::gdk_pixbuf::PixbufAnimation::from_file(&record.asset_path) else {
        return;
    };

    state.gif_tiles.borrow_mut().push(GifTile {
        area: area.downgrade(),
        iter: animation.iter(Some(SystemTime::now())),
        frame,
    });
    ensure_gif_animation_ticker(state);
}

fn ensure_gif_animation_ticker(state: &Rc<AppState>) {
    if state.gif_animation_source.borrow().is_some() {
        return;
    }

    let weak_state = Rc::downgrade(state);
    let source_id = glib::timeout_add_local(GIF_ANIMATION_INTERVAL, move || {
        let Some(state) = weak_state.upgrade() else {
            return glib::ControlFlow::Break;
        };

        let mut tiles = state.gif_tiles.borrow_mut();
        if tiles.is_empty() {
            *state.gif_animation_source.borrow_mut() = None;
            return glib::ControlFlow::Break;
        }

        tiles.retain_mut(|tile| {
            let Some(area) = tile.area.upgrade() else {
                return false;
            };

            if !is_animation_area_visible(&area, &state.window) {
                return true;
            }

            if tile.iter.advance(SystemTime::now()) {
                *tile.frame.borrow_mut() = Some(tile.iter.pixbuf());
                area.queue_draw();
            }

            true
        });

        if tiles.is_empty() {
            *state.gif_animation_source.borrow_mut() = None;
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
    *state.gif_animation_source.borrow_mut() = Some(source_id);
}

fn is_animation_area_visible(area: &DrawingArea, window: &adw::ApplicationWindow) -> bool {
    if !area.is_mapped() || !window.is_active() {
        return false;
    }

    let Some(bounds) = area.compute_bounds(window) else {
        return false;
    };

    bounds.y() + bounds.height() >= 0.0 && bounds.y() <= window.height() as f32
}

fn draw_pixbuf_cover(
    cr: &gtk::cairo::Context,
    pixbuf: &gtk::gdk_pixbuf::Pixbuf,
    width: i32,
    height: i32,
) {
    let pixbuf_width = pixbuf.width();
    let pixbuf_height = pixbuf.height();
    if pixbuf_width <= 0 || pixbuf_height <= 0 || width <= 0 || height <= 0 {
        return;
    }

    let scale = (width as f64 / pixbuf_width as f64).max(height as f64 / pixbuf_height as f64);
    let draw_width = pixbuf_width as f64 * scale;
    let draw_height = pixbuf_height as f64 * scale;
    let x = (width as f64 - draw_width) / 2.0;
    let y = (height as f64 - draw_height) / 2.0;

    let _ = cr.save();
    cr.rectangle(0.0, 0.0, width as f64, height as f64);
    cr.clip();
    cr.translate(x, y);
    cr.scale(scale, scale);
    cr.set_source_pixbuf(pixbuf, 0.0, 0.0);
    let _ = cr.paint();
    let _ = cr.restore();
}

fn choose_import_files(state: &Rc<AppState>) {
    let dialog = FileDialog::builder()
        .title("Import Stickers")
        .modal(true)
        .filters(&image_filters())
        .build();

    let state = state.clone();
    glib::MainContext::default().spawn_local(async move {
        let Ok(model) = dialog.open_multiple_future(Some(&state.window)).await else {
            return;
        };
        let paths = paths_from_model(&model)
            .into_iter()
            .filter(|path| is_supported_import_path(path))
            .collect::<Vec<_>>();
        import_paths_with_prompts(&state, paths);
    });
}

fn import_paths_with_prompts(state: &Rc<AppState>, paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }

    let state = state.clone();
    glib::MainContext::default().spawn_local(async move {
        let mut requests = Vec::new();
        for path in paths {
            let default_name = file_stem_or_name(&path);
            let title = format!(
                "Name for {}",
                path.file_name()
                    .map(|value| value.to_string_lossy())
                    .unwrap_or_default()
            );
            let Some(name) =
                entry_dialog(&state.window, "Sticker Name", &title, &default_name).await
            else {
                continue;
            };
            requests.push(ImportRequest {
                path,
                name: Some(name),
                original_filename: None,
            });
        }

        if requests.is_empty() {
            return;
        }

        import_requests_in_background(&state, requests);
    });
}

fn import_requests_in_background(state: &Rc<AppState>, requests: Vec<ImportRequest>) {
    let count = requests.len();
    set_status(state, &format!("Importing {count} sticker(s)…"));

    let store = state.store.clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(store.import_items(requests));
    });

    let state = state.clone();
    glib::timeout_add_local(Duration::from_millis(50), move || {
        match receiver.try_recv() {
            Ok(Ok(items)) => {
                let count = items.len();
                reload_items(&state);
                set_status(&state, &format!("Imported {count} sticker(s)"));
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                show_error(&state, "Import failed", &error.to_string());
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                show_error(
                    &state,
                    "Import failed",
                    "The import worker stopped unexpectedly.",
                );
                glib::ControlFlow::Break
            }
        }
    });
}

fn rename_sticker(state: &Rc<AppState>, id: u64, current_name: &str) {
    let state = state.clone();
    let current_name = current_name.to_string();
    glib::MainContext::default().spawn_local(async move {
        let Some(new_name) =
            entry_dialog(&state.window, "Rename Sticker", "New name", &current_name).await
        else {
            return;
        };
        match state.store.rename_item(id, &new_name) {
            Ok(record) => {
                reload_items(&state);
                set_status(&state, &format!("Renamed sticker to {}", record.name));
            }
            Err(error) => show_error(&state, "Rename failed", &error.to_string()),
        }
    });
}

fn confirm_delete_item(state: &Rc<AppState>, id: u64, name: &str) {
    let state = state.clone();
    let name = name.to_string();
    glib::MainContext::default().spawn_local(async move {
        let confirmed = confirm_dialog(
            &state.window,
            "Delete Sticker",
            &format!("Delete \"{name}\" from the library?"),
            "Delete",
        )
        .await;
        if !confirmed {
            return;
        }

        match state.store.delete_item(id) {
            Ok(()) => {
                reload_items(&state);
                set_status(&state, &format!("Deleted {name}"));
            }
            Err(error) => show_error(&state, "Delete failed", &error.to_string()),
        }
    });
}

fn confirm_delete_all(state: &Rc<AppState>) {
    if state.records.borrow().is_empty() {
        set_status(state, "There are no stickers to delete");
        return;
    }

    let state = state.clone();
    glib::MainContext::default().spawn_local(async move {
        let count = state.records.borrow().len();
        let confirmed = confirm_dialog(
            &state.window,
            "Delete All Stickers",
            &format!("Delete all {count} stickers from the library? This cannot be undone."),
            "Delete All",
        )
        .await;
        if !confirmed {
            return;
        }

        match state.store.delete_all_items() {
            Ok(count) => {
                reload_items(&state);
                set_status(&state, &format!("Deleted {count} sticker(s)"));
            }
            Err(error) => show_error(&state, "Delete all failed", &error.to_string()),
        }
    });
}

fn choose_export_folder(state: &Rc<AppState>) {
    let dialog = FileDialog::builder()
        .title("Choose Backup Destination")
        .modal(true)
        .build();

    let state = state.clone();
    glib::MainContext::default().spawn_local(async move {
        let Ok(folder) = dialog.select_folder_future(Some(&state.window)).await else {
            return;
        };
        let Some(path) = folder.path() else {
            show_error(
                &state,
                "Export failed",
                "The selected folder is not a local path.",
            );
            return;
        };
        match state.store.export_backup(path) {
            Ok(path) => set_status(&state, &format!("Exported backup to {}", path.display())),
            Err(error) => show_error(&state, "Export failed", &error.to_string()),
        }
    });
}

fn choose_backup_folder(state: &Rc<AppState>) {
    let dialog = FileDialog::builder()
        .title("Choose Backup Or Previous Installation")
        .modal(true)
        .build();

    let state = state.clone();
    glib::MainContext::default().spawn_local(async move {
        let Ok(folder) = dialog.select_folder_future(Some(&state.window)).await else {
            return;
        };
        let Some(path) = folder.path() else {
            show_error(
                &state,
                "Import backup failed",
                "The selected folder is not a local path.",
            );
            return;
        };
        let confirmed = confirm_dialog(
            &state.window,
            "Import Backup",
            "Import stickers from this backup or previous installation into the current library?",
            "Import",
        )
        .await;
        if !confirmed {
            return;
        }

        match state.store.import_backup(path) {
            Ok(count) => {
                reload_items(&state);
                set_status(&state, &format!("Imported {count} sticker(s) from backup"));
            }
            Err(error) => show_error(&state, "Import backup failed", &error.to_string()),
        }
    });
}

fn copy_sticker(state: &Rc<AppState>, id: u64) {
    match state.store.get_item(id) {
        Ok(record) => {
            if let Some(display) = gdk::Display::default() {
                let clipboard = display.clipboard();
                let uri = gio::File::for_path(&record.asset_path).uri();
                let text = format!("{uri}\n{}", record.asset_path);
                let uri_bytes = glib::Bytes::from_owned(format!("{uri}\r\n").into_bytes());
                let text_bytes = glib::Bytes::from_owned(text.clone().into_bytes());
                let mut providers = vec![
                    gdk::ContentProvider::for_bytes("text/uri-list", &uri_bytes),
                    gdk::ContentProvider::for_bytes("text/plain;charset=utf-8", &text_bytes),
                ];

                match (
                    record.kind.as_str(),
                    gdk::Texture::from_filename(&record.asset_path),
                ) {
                    ("gif", _) => {}
                    (_, Ok(texture)) => {
                        providers.push(gdk::ContentProvider::for_value(&texture.to_value()));
                    }
                    _ => {}
                }

                let provider = gdk::ContentProvider::new_union(&providers);
                if clipboard.set_content(Some(&provider)).is_err() {
                    clipboard.set_text(&text);
                }
                set_status(state, &format!("Copied {} to clipboard", record.name));
            } else {
                show_error(state, "Copy failed", "No display clipboard is available.");
            }
        }
        Err(error) => show_error(state, "Copy failed", &error.to_string()),
    }
}

async fn entry_dialog(
    parent: &adw::ApplicationWindow,
    title: &str,
    body: &str,
    initial: &str,
) -> Option<String> {
    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();
    content.append(&Label::builder().label(body).xalign(0.0).wrap(true).build());

    let entry = Entry::builder()
        .text(initial)
        .activates_default(true)
        .build();
    content.append(&entry);

    let dialog = adw::AlertDialog::builder()
        .heading(title)
        .extra_child(&content)
        .close_response("cancel")
        .default_response("ok")
        .default_widget(&entry)
        .focus_widget(&entry)
        .build();
    dialog.add_responses(&[("cancel", "Cancel"), ("ok", "OK")]);

    let response = dialog.choose_future(Some(parent)).await;
    let value = entry.text().trim().to_string();

    if response == "ok" && !value.is_empty() {
        Some(value)
    } else {
        None
    }
}

async fn confirm_dialog(
    parent: &adw::ApplicationWindow,
    title: &str,
    body: &str,
    accept_label: &str,
) -> bool {
    let dialog = adw::AlertDialog::builder()
        .heading(title)
        .body(body)
        .close_response("cancel")
        .default_response("accept")
        .build();
    dialog.add_responses(&[("cancel", "Cancel"), ("accept", accept_label)]);
    dialog.set_response_appearance("accept", adw::ResponseAppearance::Destructive);

    dialog.choose_future(Some(parent)).await == "accept"
}

fn show_error(state: &Rc<AppState>, title: &str, message: &str) {
    let dialog = adw::AlertDialog::builder()
        .heading(title)
        .body(message)
        .close_response("ok")
        .default_response("ok")
        .build();
    dialog.add_response("ok", "OK");
    dialog.present(Some(&state.window));
}

fn show_startup_error(app: &adw::Application, error: &anyhow::Error) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Universal Stickers")
        .default_width(480)
        .default_height(160)
        .build();
    let root = GtkBox::new(Orientation::Vertical, 12);
    root.set_margin_top(24);
    root.set_margin_bottom(24);
    root.set_margin_start(24);
    root.set_margin_end(24);
    root.append(
        &Label::builder()
            .label("Failed to initialize the sticker library")
            .xalign(0.0)
            .build(),
    );
    root.append(
        &Label::builder()
            .label(error.to_string())
            .wrap(true)
            .xalign(0.0)
            .build(),
    );
    window.set_content(Some(&root));
    window.present();
}

#[cfg(target_os = "linux")]
fn setup_global_shortcut(state: Weak<AppState>) {
    glib::MainContext::default().spawn_local(async move {
        let Some(initial_state) = state.upgrade() else {
            return;
        };

        match bind_global_shortcut().await {
            Ok(mut stream) => {
                set_status(&initial_state, "Global shortcut active: Ctrl+Meta+Space");
                while let Some(activated) = stream.next().await {
                    if activated.shortcut_id() == SHORTCUT_ID {
                        if let Some(state) = state.upgrade() {
                            state.window.present();
                        } else {
                            break;
                        }
                    }
                }
            }
            Err(error) => set_status(
                &initial_state,
                &format!("Global shortcut unavailable: {error}"),
            ),
        }
    });
}

#[cfg(not(target_os = "linux"))]
fn setup_global_shortcut(_state: Weak<AppState>) {}

#[cfg(target_os = "linux")]
async fn bind_global_shortcut()
-> Result<impl futures_util::Stream<Item = ashpd::desktop::global_shortcuts::Activated>> {
    use ashpd::desktop::{
        CreateSessionOptions, Session,
        global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut},
    };

    let portal = GlobalShortcuts::new().await?;
    let session = portal
        .create_session(CreateSessionOptions::default())
        .await
        .context("create shortcut session")?;
    let shortcut = NewShortcut::new(SHORTCUT_ID, "Show Universal Stickers")
        .preferred_trigger(Some("<Control><Super>space"));
    let request = portal
        .bind_shortcuts(&session, &[shortcut], None, BindShortcutsOptions::default())
        .await
        .context("bind shortcut")?;
    let _ = request.response().context("shortcut response")?;

    // Leak the session to keep the portal shortcut alive for the process lifetime.
    let _session: &'static Session<GlobalShortcuts> = Box::leak(Box::new(session));
    portal
        .receive_activated()
        .await
        .context("listen for shortcut")
}

fn set_status(state: &Rc<AppState>, message: &str) {
    state.status.set_text(message);
}

fn image_filters() -> gio::ListStore {
    let store = gio::ListStore::new::<FileFilter>();
    let filter = FileFilter::new();
    filter.set_name(Some("Images"));
    for mime in [
        "image/png",
        "image/jpeg",
        "image/bmp",
        "image/webp",
        "image/gif",
    ] {
        filter.add_mime_type(mime);
    }
    store.append(&filter);
    store
}

fn paths_from_model(model: &gio::ListModel) -> Vec<PathBuf> {
    (0..model.n_items())
        .filter_map(|index| model.item(index))
        .filter_map(|object| object.downcast::<gio::File>().ok())
        .filter_map(|file| file.path())
        .collect()
}

fn is_supported_import_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "webp" | "gif"
            )
        })
        .unwrap_or(false)
}

fn file_stem_or_name(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sticker".to_string())
}

fn legacy_data_dir() -> Result<PathBuf> {
    dirs::data_dir()
        .map(|path| path.join(LEGACY_ORG_DIR).join(LEGACY_APP_DIR))
        .ok_or_else(|| anyhow!("could not resolve user data directory"))
}

fn scaled_dimension(base: i32, scale: u32) -> i32 {
    ((base as u32 * scale) / 100) as i32
}

fn load_display_scale(path: &Path) -> u32 {
    let Some(value) = std::fs::read_to_string(path)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
    else {
        return DEFAULT_DISPLAY_SCALE;
    };

    match value {
        0 => 75,
        1 => 100,
        2 => 125,
        value => {
            let scale = value as u32;
            scale.clamp(MIN_DISPLAY_SCALE, MAX_DISPLAY_SCALE)
        }
    }
}

fn save_display_scale(path: &Path, scale: u32) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, scale.to_string());
}

fn install_css() {
    let provider = CssProvider::new();
    provider.load_from_string(
        ".sticker-tile {
            border-radius: 8px;
            background: @card_bg_color;
            padding: 0;
        }
        .sticker-tile:hover {
            filter: brightness(1.05);
        }
        .sticker-tile.pressed {
            filter: brightness(0.9);
        }
        .sticker-media {
            background: alpha(currentColor, 0.05);
        }
        .sticker-overlay-button {
            border-radius: 999px;
            background: alpha(@window_bg_color, 0.78);
            color: @window_fg_color;
            padding: 4px;
        }
        .sticker-overlay-button:hover {
            background: alpha(@window_bg_color, 0.92);
        }
        .sticker-name-overlay {
            border-radius: 6px;
            background: alpha(black, 0.58);
            color: white;
            padding: 5px 7px;
            font-weight: 600;
        }",
    );

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
