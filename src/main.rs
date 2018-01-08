// In this file we create the UI of the RPFM, and control it (events, updates, etc...).

#![windows_subsystem = "windows"]

#[macro_use]
extern crate serde_derive;
extern crate gtk;
extern crate gdk;
extern crate sourceview;
extern crate num;

use std::path::PathBuf;
use std::cell::RefCell;
use std::rc::Rc;
use std::error;
use std::io::{
    Error, ErrorKind
};

use gtk::prelude::*;
use gtk::{
    AboutDialog, Box, Builder, MenuItem, Window, WindowPosition, FileChooserDialog,
    TreeView, TreeSelection, TreeStore, MessageDialog, ScrolledWindow, Orientation,
    CellRendererText, TreeViewColumn, Popover, Entry, CheckMenuItem, Button
};

use sourceview::{
    Buffer, BufferExt, View, ViewExt, Language, LanguageManager, LanguageManagerExt
};

use packfile::packfile::PackFile;
use common::coding_helpers;
use common::*;
use packedfile::loc::Loc;
use packedfile::db::DB;
use packedfile::rigidmodel::RigidModel;

mod common;
mod ui;
mod packfile;
mod packedfile;

/// This macro is used to clone the variables into the closures without the compiler protesting.
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

// This constant get the version of the program from the "Cargo.toml", so we don't have to change it
// in two different places in every update.
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

/// One Function to rule them all, One Function to find them,
/// One Function to bring them all and in the darkness bind them.
fn main() {

    // Init GTK3. Boilerplate code.
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    // We import the Glade design and get all the UI objects into variables.
    let glade_design = include_str!("glade/main.glade");
    let builder = Builder::new_from_string(glade_design);

    let window: Window = builder.get_object("gtk_window").expect("Couldn't get gtk_window");

    let packed_file_data_display: Box = builder.get_object("gtk_packed_file_data_display").expect("Couldn't get gtk_packed_file_data_display");

    let window_about: AboutDialog = builder.get_object("gtk_window_about").expect("Couldn't get gtk_window_about");
    let error_dialog: MessageDialog = builder.get_object("gtk_error_dialog").expect("Couldn't get gtk_error_dialog");
    let success_dialog: MessageDialog = builder.get_object("gtk_success_dialog").expect("Couldn't get gtk_success_dialog");
    let rename_popover: Popover = builder.get_object("gtk_rename_popover").expect("Couldn't get gtk_rename_popover");

    let rename_popover_text_entry: Entry = builder.get_object("gtk_rename_popover_text_entry").expect("Couldn't get gtk_rename_popover_text_entry");

    let file_chooser_open_packfile_dialog: FileChooserDialog = builder.get_object("gtk_file_chooser_open_packfile").expect("Couldn't get gtk_file_chooser_open_packfile");
    let file_chooser_save_packfile_dialog: FileChooserDialog = builder.get_object("gtk_file_chooser_save_packfile").expect("Couldn't get gtk_file_chooser_save_packfile");
    let file_chooser_add_file_to_packfile: FileChooserDialog = builder.get_object("gtk_file_chooser_add_file_to_packfile").expect("Couldn't get gtk_file_chooser_add_file_to_packfile");
    let file_chooser_add_folder_to_packfile: FileChooserDialog = builder.get_object("gtk_file_chooser_add_folder_to_packfile").expect("Couldn't get gtk_file_chooser_add_folder_to_packfile");
    let file_chooser_add_from_packfile_dialog: FileChooserDialog = builder.get_object("gtk_file_chooser_add_from_packfile").expect("Couldn't get gtk_file_chooser_add_from_packfile");
    let file_chooser_extract_file: FileChooserDialog = builder.get_object("gtk_file_chooser_extract_file").expect("Couldn't get gtk_file_chooser_extract_file");
    let file_chooser_extract_folder: FileChooserDialog = builder.get_object("gtk_file_chooser_extract_folder").expect("Couldn't get gtk_file_chooser_extract_folder");
    let file_chooser_packedfile_loc_import_csv: FileChooserDialog = builder.get_object("gtk_file_chooser_packedfile_loc_import_csv").expect("Couldn't get gtk_file_chooser_packedfile_loc_import_csv");
    let file_chooser_packedfile_loc_export_csv: FileChooserDialog = builder.get_object("gtk_file_chooser_packedfile_loc_export_csv").expect("Couldn't get gtk_file_chooser_packedfile_loc_export_csv");

    let top_menu_file: MenuItem = builder.get_object("gtk_top_menu_file").expect("Couldn't get gtk_top_menu_file");
    let top_menu_special_stuff: MenuItem = builder.get_object("gtk_top_menu_special_stuff").expect("Couldn't get gtk_top_menu_special_stuff");

    let context_menu_tree_view: Popover = builder.get_object("gtk_context_menu_tree_view").expect("Couldn't get gtk_context_menu_tree_view");
    let context_menu_tree_view_packed_file_loc: Popover = builder.get_object("gtk_context_menu_tree_view_packedfile_loc").expect("Couldn't get gtk_context_menu_tree_view_packedfile_loc");
    let context_menu_tree_view_packed_file_db: Popover = builder.get_object("gtk_context_menu_tree_view_packedfile_db").expect("Couldn't get gtk_context_menu_tree_view_packedfile_db");

    let tree_view_add_file: Button = builder.get_object("gtk_context_menu_tree_view_add_file").expect("Couldn't get gtk_context_menu_tree_view_add_file");
    let tree_view_add_folder: Button = builder.get_object("gtk_context_menu_tree_view_add_folder").expect("Couldn't get gtk_context_menu_tree_view_add_folder");
    let tree_view_add_from_packfile: Button = builder.get_object("gtk_context_menu_tree_view_add_from_packfile").expect("Couldn't get gtk_context_menu_tree_view_add_from_packfile");
    let tree_view_delete_file: Button = builder.get_object("gtk_context_menu_tree_view_delete_file").expect("Couldn't get gtk_context_menu_tree_view_delete_file");
    let tree_view_extract_file: Button = builder.get_object("gtk_context_menu_tree_view_extract_file").expect("Couldn't get gtk_context_menu_tree_view_extract_file");

    let tree_view_packedfile_loc_add_rows: Button = builder.get_object("gtk_context_menu_tree_view_loc_packedfile_add_rows").expect("Couldn't get gtk_context_menu_tree_view_loc_packedfile_add_rows");
    let tree_view_packedfile_loc_add_rows_number: Entry = builder.get_object("gtk_context_menu_tree_view_loc_packedfile_add_rows_number").expect("Couldn't get gtk_context_menu_tree_view_loc_packedfile_add_rows_number");
    let tree_view_packedfile_loc_delete_row: Button = builder.get_object("gtk_context_menu_tree_view_packedfile_loc_delete_row").expect("Couldn't get gtk_context_menu_tree_view_packedfile_loc_delete_row");
    let tree_view_packedfile_loc_import_csv: Button = builder.get_object("gtk_context_menu_tree_view_packedfile_loc_import_csv").expect("Couldn't get gtk_context_menu_tree_view_packedfile_loc_import_csv");
    let tree_view_packedfile_loc_export_csv: Button = builder.get_object("gtk_context_menu_tree_view_packedfile_loc_export_csv").expect("Couldn't get gtk_context_menu_tree_view_packedfile_loc_export_csv");

    let tree_view_packedfile_db_add_rows: Button = builder.get_object("gtk_context_menu_tree_view_db_packedfile_add_rows").expect("Couldn't get gtk_context_menu_tree_view_db_packedfile_add_rows");
    let tree_view_packedfile_db_add_rows_number: Entry = builder.get_object("gtk_context_menu_tree_view_db_packedfile_add_rows_number").expect("Couldn't get gtk_context_menu_tree_view_db_packedfile_add_rows_number");
    let tree_view_packedfile_db_delete_row: Button = builder.get_object("gtk_context_menu_tree_view_packedfile_db_delete_row").expect("Couldn't get gtk_context_menu_tree_view_packedfile_db_delete_row");

    let top_menu_file_new_packfile: MenuItem = builder.get_object("gtk_top_menu_file_new_packfile").expect("Couldn't get gtk_top_menu_file_new_packfile");
    let top_menu_file_open_packfile: MenuItem = builder.get_object("gtk_top_menu_file_open_packfile").expect("Couldn't get gtk_top_menu_file_open_packfile");
    let top_menu_file_save_packfile: MenuItem = builder.get_object("gtk_top_menu_file_save_packfile").expect("Couldn't get gtk_top_menu_file_save_packfile");
    let top_menu_file_save_packfile_as: MenuItem = builder.get_object("gtk_top_menu_file_save_packfile_as").expect("Couldn't get gtk_top_menu_file_save_packfile_as");
    let top_menu_file_quit: MenuItem = builder.get_object("gtk_top_menu_file_quit").expect("Couldn't get gtk_top_menu_file_quit");
    let top_menu_special_patch_ai: MenuItem = builder.get_object("gtk_top_menu_special_patch_ai").expect("Couldn't get gtk_top_menu_special_patch_ai");
    let top_menu_about_about: MenuItem = builder.get_object("gtk_top_menu_about_about").expect("Couldn't get gtk_top_menu_about_about");

    let top_menu_file_change_packfile_type: MenuItem = builder.get_object("gtk_top_menu_file_select_packfile_type").expect("Couldn't get gtk_top_menu_file_select_packfile_type");
    let top_menu_file_change_packfile_type_boot: CheckMenuItem = builder.get_object("gtk_top_menu_file_select_packfile_type1").expect("Couldn't get gtk_top_menu_file_select_packfile_type1");
    let top_menu_file_change_packfile_type_release: CheckMenuItem = builder.get_object("gtk_top_menu_file_select_packfile_type2").expect("Couldn't get gtk_top_menu_file_select_packfile_type2");
    let top_menu_file_change_packfile_type_patch: CheckMenuItem = builder.get_object("gtk_top_menu_file_select_packfile_type3").expect("Couldn't get gtk_top_menu_file_select_packfile_type3");
    let top_menu_file_change_packfile_type_mod: CheckMenuItem = builder.get_object("gtk_top_menu_file_select_packfile_type4").expect("Couldn't get gtk_top_menu_file_select_packfile_type4");
    let top_menu_file_change_packfile_type_movie: CheckMenuItem = builder.get_object("gtk_top_menu_file_select_packfile_type5").expect("Couldn't get gtk_top_menu_file_select_packfile_type5");

    let folder_tree_view: TreeView = builder.get_object("gtk_folder_tree_view").expect("Couldn't get gtk_folder_tree_view");
    let folder_tree_selection: TreeSelection = builder.get_object("gtk_folder_tree_view_selection").expect("Couldn't get gtk_folder_tree_view_selection");

    // The TreeView's stuff is created manually here, as I had problems creating it in Glade.
    let folder_tree_store = TreeStore::new(&[String::static_type()]);
    folder_tree_view.set_model(Some(&folder_tree_store));

    let column = TreeViewColumn::new();
    let cell = CellRendererText::new();
    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", 0);

    folder_tree_view.append_column(&column);
    folder_tree_view.set_enable_search(false);
    folder_tree_view.set_rules_hint(true);
    window.set_position(WindowPosition::Center);

    // This variable is used to "Lock" and "Unlock" the "Decode on select" feature of the TreeView.
    // We need it to lock this feature when we open a secondary PackFile and want to move some folders
    // from one PackFile to another.
    let is_folder_tree_view_locked = Rc::new(RefCell::new(false));

    // Here we set the TreeView as "drag_dest", so we can drag&drop things to it.
    let targets = vec![
        // This one is for dragging PackFiles into the TreeView.
        gtk::TargetEntry::new("text/uri-list", gtk::TargetFlags::OTHER_APP, 0),
    ];
    folder_tree_view.drag_dest_set(gtk::DestDefaults::ALL, &targets, gdk::DragAction::COPY);

    // Then we display the "Tips" text.
    ui::display_help_tips(&packed_file_data_display);

    // Then we set all the stuff of the "About" dialog (except the Icon).
    window_about.set_program_name("Rusted PackFile Manager");
    window_about.set_version(VERSION);
    window_about.set_license_type(gtk::License::MitX11);
    window_about.set_website("https://github.com/Frodo45127/rpfm");
    window_about.set_website_label("Source code and more info here:)");
    window_about.set_comments(Some("Made by modders, for modders."));

    window_about.add_credit_section("Created and Programmed by", &["Frodo45127"]);
    window_about.add_credit_section("Icon by", &["Maruka"]);
    window_about.add_credit_section("RigidModel research by", &["Mr.Jox", "Der Spaten", "Maruka", "Frodo45127"]);
    window_about.add_credit_section("DB Schemas by", &["PFM team"]);
    window_about.add_credit_section("Special thanks to", &["- PFM team (for providing the community\n   with awesome modding tools).", "- CA (for being a mod-friendly company)."]);

    // We bring up the main window.
    window.show_all();

    // We also create a dummy PackFile we're going to use to store all the data from the opened Packfile,
    // and an extra dummy PackFile for situations were we need two PackFiles opened at the same time.
    let pack_file_decoded = Rc::new(RefCell::new(PackFile::new()));
    let pack_file_decoded_extra = Rc::new(RefCell::new(PackFile::new()));

    // And we import the master_schema for the DB tables.
    let master_schema = include_str!("packedfile/db/master_schema.xml");

    // End of the "Getting Ready" part.
    // From here, it's all event handling.

    // First, we catch the close window event, and close the program when we do it.
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    /*
    --------------------------------------------------------
                     Superior Menu: "File"
    --------------------------------------------------------
    */

    // When we open the menu, we check if we need to enable or disable his buttons first.
    top_menu_file.connect_activate(clone!(
        top_menu_file_save_packfile,
        top_menu_file_save_packfile_as,
        top_menu_file_change_packfile_type,
        pack_file_decoded => move |_| {

        // If the current PackFile has no name, we haven't open or created one, so disable all the
        // options that need a PackFile opened. Otherwise enable them.
        if pack_file_decoded.borrow().pack_file_extra_data.file_name.is_empty() {
            top_menu_file_save_packfile.set_sensitive(false);
            top_menu_file_save_packfile_as.set_sensitive(false);
            top_menu_file_change_packfile_type.set_sensitive(false);
        }
        else {
            top_menu_file_save_packfile.set_sensitive(true);
            top_menu_file_save_packfile_as.set_sensitive(true);
            top_menu_file_change_packfile_type.set_sensitive(true);
        }
    }));


    // When we hit the "New PackFile" button.
    top_menu_file_new_packfile.connect_activate(clone!(
        window,
        pack_file_decoded,
        folder_tree_store,
        top_menu_file_change_packfile_type_mod => move |_| {

        // We just create a new PackFile with a name, set his type to Mod and update the
        // TreeView to show it.
        *pack_file_decoded.borrow_mut() = packfile::new_packfile("unkown.pack".to_string());
        ui::update_tree_view(&folder_tree_store, &*pack_file_decoded.borrow());
        window.set_title(&format!("Rusted PackFile Manager -> {}", pack_file_decoded.borrow().pack_file_extra_data.file_name));

        top_menu_file_change_packfile_type_mod.set_active(true);
    }));


    // When we hit the "Open PackFile" button.
    top_menu_file_open_packfile.connect_activate(clone!(
        window,
        error_dialog,
        pack_file_decoded,
        folder_tree_store,
        top_menu_file_change_packfile_type_boot,
        top_menu_file_change_packfile_type_release,
        top_menu_file_change_packfile_type_patch,
        top_menu_file_change_packfile_type_mod,
        top_menu_file_change_packfile_type_movie => move |_| {

        // When we select the file to open, we get his path, open it and, if there has been no
        // errors, decode it, update the TreeView to show it and check his type for the Change PackFile
        // Type option in the File menu.
        if file_chooser_open_packfile_dialog.run() == gtk::ResponseType::Ok.into() {
            let pack_file_path = file_chooser_open_packfile_dialog.get_filename().expect("Couldn't open file");
            match packfile::open_packfile(pack_file_path) {
                Ok(pack_file_opened) => {

                    *pack_file_decoded.borrow_mut() = pack_file_opened;
                    ui::update_tree_view(&folder_tree_store, &*pack_file_decoded.borrow());
                    window.set_title(&format!("Rusted PackFile Manager -> {}", pack_file_decoded.borrow().pack_file_extra_data.file_name));

                    // We choose the right option, depending on our PackFile.
                    if pack_file_decoded.borrow().pack_file_header.pack_file_type == 0u32 {
                        top_menu_file_change_packfile_type_boot.set_active(true);
                    }
                    else if pack_file_decoded.borrow().pack_file_header.pack_file_type == 1u32{
                        top_menu_file_change_packfile_type_release.set_active(true);
                    }
                    else if pack_file_decoded.borrow().pack_file_header.pack_file_type == 2u32{
                        top_menu_file_change_packfile_type_patch.set_active(true);
                    }
                    else if pack_file_decoded.borrow().pack_file_header.pack_file_type == 3u32{
                        top_menu_file_change_packfile_type_mod.set_active(true);
                    }
                    else if pack_file_decoded.borrow().pack_file_header.pack_file_type == 4u32{
                        top_menu_file_change_packfile_type_movie.set_active(true);
                    }
                }
                Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
            }
        }
        file_chooser_open_packfile_dialog.hide_on_delete();
    }));


    // When we hit the "Save PackFile" button
    top_menu_file_save_packfile.connect_activate(clone!(
        window,
        success_dialog,
        error_dialog,
        pack_file_decoded,
        folder_tree_view,
        folder_tree_store,
        folder_tree_selection,
        file_chooser_save_packfile_dialog => move |_| {

        // First, we check if our PackFile has a path. If it doesn't have it, we launch the Save
        // Dialog and set the current name in the entry of the dialog to his name.
        // When we hit "Accept", we get the selected path, encode the PackFile, and save it to that
        // path. After that, we update the TreeView to reflect the name change and hide the dialog.
        let mut pack_file_path: Option<PathBuf> = None;
        if pack_file_decoded.borrow().pack_file_extra_data.file_path.is_empty() {
            file_chooser_save_packfile_dialog.set_current_name(&pack_file_decoded.borrow().pack_file_extra_data.file_name);
            if file_chooser_save_packfile_dialog.run() == gtk::ResponseType::Ok.into() {
                pack_file_path = Some(file_chooser_save_packfile_dialog.get_filename().expect("Couldn't open file"));

                let mut success = false;
                match packfile::save_packfile(&mut *pack_file_decoded.borrow_mut(), pack_file_path) {
                    Ok(result) => {
                        success = true;
                        ui::show_dialog(&success_dialog, result);
                    },
                    Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
                }
                if success {
                    // If saved, we reset the title to unmodified.
                    window.set_title(&format!("Rusted PackFile Manager -> {}", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                    ui::update_tree_view_expand_path(
                        &folder_tree_store,
                        &*pack_file_decoded.borrow(),
                        &folder_tree_selection,
                        &folder_tree_view,
                        false
                    );
                }

            }
            file_chooser_save_packfile_dialog.hide_on_delete();
        }

        // If the PackFile has a path, we just encode it and save it into that path.
        else {
            let mut success = false;
            match packfile::save_packfile(&mut *pack_file_decoded.borrow_mut(), pack_file_path) {
                Ok(result) => {
                    success = true;
                    ui::show_dialog(&success_dialog, result);
                },
                Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
            }
            if success {
                // If saved, we reset the title to unmodified.
                window.set_title(&format!("Rusted PackFile Manager -> {}", pack_file_decoded.borrow().pack_file_extra_data.file_name));
            }
        }
    }));


    // When we hit the "Save PackFile as" button.
    top_menu_file_save_packfile_as.connect_activate(clone!(
        window,
        success_dialog,
        error_dialog,
        pack_file_decoded,
        folder_tree_view,
        folder_tree_store,
        folder_tree_selection,
        file_chooser_save_packfile_dialog => move |_| {

        // We first set the current file of the Save dialog to the PackFile's name. Then we just
        // encode it and save it in the path selected. After that, we update the TreeView to reflect
        // the name change and hide the dialog.
        file_chooser_save_packfile_dialog.set_current_name(&pack_file_decoded.borrow().pack_file_extra_data.file_name);
        if file_chooser_save_packfile_dialog.run() == gtk::ResponseType::Ok.into() {
            let mut success = false;
            match packfile::save_packfile(
               &mut *pack_file_decoded.borrow_mut(),
               Some(file_chooser_save_packfile_dialog.get_filename().expect("Couldn't open file"))) {
                    Ok(result) => {
                        success = true;
                        ui::show_dialog(&success_dialog, result);
                    },
                    Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
            }
            if success {
                window.set_title(&format!("Rusted PackFile Manager -> {}", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                ui::update_tree_view_expand_path(
                    &folder_tree_store,
                    &*pack_file_decoded.borrow(),
                    &folder_tree_selection,
                    &folder_tree_view,
                    false
                );
            }
        }
        file_chooser_save_packfile_dialog.hide_on_delete();
    }));


    // When changing the type of the PackFile... we just change his pack_file_type variable. Nothing complex.
    top_menu_file_change_packfile_type_boot.connect_toggled(clone!(
        top_menu_file_change_packfile_type_boot,
        pack_file_decoded => move |_| {
        if top_menu_file_change_packfile_type_boot.get_active() {
            pack_file_decoded.borrow_mut().pack_file_header.pack_file_type = 0;
        }
    }));
    top_menu_file_change_packfile_type_release.connect_toggled(clone!(
        top_menu_file_change_packfile_type_release,
        pack_file_decoded => move |_| {
        if top_menu_file_change_packfile_type_release.get_active() {
            pack_file_decoded.borrow_mut().pack_file_header.pack_file_type = 1;
        }
    }));
    top_menu_file_change_packfile_type_patch.connect_toggled(clone!(
        top_menu_file_change_packfile_type_patch,
        pack_file_decoded => move |_| {
        if top_menu_file_change_packfile_type_patch.get_active() {
            pack_file_decoded.borrow_mut().pack_file_header.pack_file_type = 2;
        }
    }));
    top_menu_file_change_packfile_type_mod.connect_toggled(clone!(
        top_menu_file_change_packfile_type_mod,
        pack_file_decoded => move |_| {
        if top_menu_file_change_packfile_type_mod.get_active() {
            pack_file_decoded.borrow_mut().pack_file_header.pack_file_type = 3;
        }
    }));
    top_menu_file_change_packfile_type_movie.connect_toggled(clone!(
        top_menu_file_change_packfile_type_movie,
        pack_file_decoded => move |_| {
        if top_menu_file_change_packfile_type_movie.get_active() {
            pack_file_decoded.borrow_mut().pack_file_header.pack_file_type = 4;
        }
    }));


    // When we hit the "Quit" button.
    top_menu_file_quit.connect_activate(|_| {
        gtk::main_quit();
    });

    /*
    --------------------------------------------------------
                 Superior Menu: "Special Stuff"
    --------------------------------------------------------
    */

    // When we open the menu, we check if we need to enable or disable his buttons first.
    top_menu_special_stuff.connect_activate(clone!(
        top_menu_special_patch_ai,
        pack_file_decoded => move |_| {
        if pack_file_decoded.borrow().pack_file_extra_data.file_name.is_empty() {
            top_menu_special_patch_ai.set_sensitive(false);
        }
        else {
            top_menu_special_patch_ai.set_sensitive(true);
        }
    }));


    // When we hit the "Patch SiegeAI" button.
    top_menu_special_patch_ai.connect_activate(clone!(
    success_dialog,
    error_dialog,
    pack_file_decoded,
    folder_tree_view,
    folder_tree_store,
    folder_tree_selection => move |_| {

        // First, we try to patch the PackFile. If there are no errors, we save the result in a tuple.
        // Then we check that tuple and, if it's a success, we save the PackFile and update the TreeView.
        let mut sucessful_patching = (false, String::new());
        match packfile::patch_siege_ai(&mut *pack_file_decoded.borrow_mut()) {
            Ok(result) => sucessful_patching = (true, result),
            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
        }
        if sucessful_patching.0 {
            let mut success = false;
            match packfile::save_packfile( &mut *pack_file_decoded.borrow_mut(), None) {
                Ok(result) => {
                    success = true;
                    ui::show_dialog(&success_dialog, format!("{}\n\n{}", sucessful_patching.1, result));
                },
                Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
            }
            if success {
                ui::update_tree_view_expand_path(
                    &folder_tree_store,
                    &*pack_file_decoded.borrow(),
                    &folder_tree_selection,
                    &folder_tree_view,
                    false
                );
            }
        }
    }));

    /*
    --------------------------------------------------------
                    Superior Menu: "About"
    --------------------------------------------------------
    */

    // When we hit the "About" button.
    top_menu_about_about.connect_activate(move |_| {
        window_about.run();
        window_about.hide_on_delete();
    });


    /*
    --------------------------------------------------------
                   Contextual TreeView Popup
    --------------------------------------------------------
    */

    // When we right-click the TreeView, we check if we need to enable or disable his buttons first.
    // Then we calculate the position where the popup must aim, and show it.
    //
    // NOTE: REMEMBER, WE OPEN THE POPUP HERE, BUT WE NEED TO CLOSED IT WHEN WE HIT HIS BUTTONS.
    folder_tree_view.connect_button_release_event(clone!(
        pack_file_decoded,
        folder_tree_view,
        folder_tree_selection,
        tree_view_add_file,
        tree_view_add_folder,
        tree_view_add_from_packfile,
        tree_view_extract_file,
        tree_view_delete_file,
        context_menu_tree_view => move |_, button| {

        let button_val = button.get_button();
        if button_val == 3 && folder_tree_selection.count_selected_rows() > 0 {
            let tree_path = ui::get_tree_path_from_selection(&folder_tree_selection, false);
            for i in &*pack_file_decoded.borrow().pack_file_data.packed_files {
                // If the selected thing is a file
                if i.packed_file_path == tree_path {
                    tree_view_add_file.set_sensitive(false);
                    tree_view_add_folder.set_sensitive(false);
                    tree_view_add_from_packfile.set_sensitive(false);
                    tree_view_extract_file.set_sensitive(true);
                    tree_view_delete_file.set_sensitive(true);
                    break;
                }
                else {
                    tree_view_add_file.set_sensitive(true);
                    tree_view_add_folder.set_sensitive(true);
                    tree_view_add_from_packfile.set_sensitive(true);
                    tree_view_extract_file.set_sensitive(true);
                    tree_view_delete_file.set_sensitive(true);
                }
            }
            if tree_path.len() == 0 {
                tree_view_add_file.set_sensitive(true);
                tree_view_add_folder.set_sensitive(true);
                tree_view_add_from_packfile.set_sensitive(true);
                tree_view_extract_file.set_sensitive(false);
                tree_view_delete_file.set_sensitive(false);
            }
            let rect = ui::get_rect_for_popover(&folder_tree_view, Some(button.get_position()));

            context_menu_tree_view.set_pointing_to(&rect);
            context_menu_tree_view.popup();
        }
        Inhibit(false)
    }));


    // When we hit the "Add file" button.
    tree_view_add_file.connect_button_release_event(clone!(
        window,
        error_dialog,
        pack_file_decoded,
        folder_tree_view,
        folder_tree_store,
        folder_tree_selection,
        context_menu_tree_view => move |_,_| {

        // First, we hide the context menu, then we pick the file selected and add it to the Packfile.
        // After that, we update the TreeView.
        context_menu_tree_view.popdown();

        if file_chooser_add_file_to_packfile.run() == gtk::ResponseType::Ok.into() {

            let paths = file_chooser_add_file_to_packfile.get_filenames();
            for path in paths.iter() {

                //let file_path = file_chooser_add_file_to_packfile.get_filename().expect("Couldn't open file");
                let tree_path = ui::get_tree_path_from_pathbuf(&path, &folder_tree_selection, true);
                let mut success = false;
                match packfile::add_file_to_packfile(&mut *pack_file_decoded.borrow_mut(), path, tree_path) {
                    Ok(_) => success = true,
                    Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
                }
                if success {
                    window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                    ui::update_tree_view_expand_path(
                        &folder_tree_store,
                        &*pack_file_decoded.borrow(),
                        &folder_tree_selection,
                        &folder_tree_view,
                        false
                    );
                }
            }
        }
        file_chooser_add_file_to_packfile.hide_on_delete();

        Inhibit(false)
    }));


    // When we hit the "Add folder" button.
    tree_view_add_folder.connect_button_release_event(clone!(
        window,
        error_dialog,
        pack_file_decoded,
        folder_tree_view,
        folder_tree_store,
        folder_tree_selection,
        context_menu_tree_view => move |_,_| {

        // First, we hide the context menu. Then we get the folder selected and we get all the files
        // in him and his subfolders. After that, for every one of those files, we strip his path,
        // leaving then with only the part that will be added to the PackedFile and we add it to the
        // PackFile. After all that, if we added any of the files to the PackFile, we update the
        // TreeView.
        context_menu_tree_view.popdown();
        if file_chooser_add_folder_to_packfile.run() == gtk::ResponseType::Ok.into() {
            let folders = file_chooser_add_folder_to_packfile.get_filenames();
            for folder in folders.iter() {

                let mut big_parent_prefix = folder.clone();
                big_parent_prefix.pop();
                let file_path_list = ::common::get_files_from_subdir(&folder);
                let mut file_errors = 0;
                for i in file_path_list {
                    match i.strip_prefix(&big_parent_prefix) {
                        Ok(filtered_path) => {
                            let tree_path = ui::get_tree_path_from_pathbuf(&filtered_path.to_path_buf(), &folder_tree_selection, false);
                            if let Err(_) = packfile::add_file_to_packfile(&mut *pack_file_decoded.borrow_mut(), &i.to_path_buf(), tree_path) {
                                file_errors += 1;
                            }
                        }
                        Err(_) => {
                            panic!("Error while trying to filter the path. This should never happen unless I break something while I'm getting the paths.");
                        }
                    }
                }
                if file_errors > 0 {
                    ui::show_dialog(&error_dialog, format!("{} file/s that you wanted to add already exist in the Packfile.", file_errors));
                }
                window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                ui::update_tree_view_expand_path(
                    &folder_tree_store,
                    &*pack_file_decoded.borrow(),
                    &folder_tree_selection,
                    &folder_tree_view,
                    false
                );
            }
        }
        file_chooser_add_folder_to_packfile.hide_on_delete();

        Inhibit(false)
    }));

    // When we hit the "Add file/folder from PackFile" button.
    tree_view_add_from_packfile.connect_button_release_event(clone!(
        window,
        error_dialog,
        pack_file_decoded,
        pack_file_decoded_extra,
        packed_file_data_display,
        folder_tree_view,
        folder_tree_store,
        folder_tree_selection,
        is_folder_tree_view_locked,
        context_menu_tree_view => move |_,_| {

        // First, we hide the context menu, then we pick the PackFile selected.
        // After that, we update the TreeView.
        context_menu_tree_view.popdown();

        // Then, we destroy any childrens that the packed_file_data_display we use may have, cleaning it.
        let childrens_to_utterly_destroy = packed_file_data_display.get_children();
        if !childrens_to_utterly_destroy.is_empty() {
            for i in childrens_to_utterly_destroy.iter() {
                i.destroy();
            }
        }

        if file_chooser_add_from_packfile_dialog.run() == gtk::ResponseType::Ok.into() {
            let pack_file_path = file_chooser_add_from_packfile_dialog.get_filename().expect("Couldn't open file");
            match packfile::open_packfile(pack_file_path) {

                // If the extra PackFile is valid, we create a box with a button to exit this mode
                // and a TreeView of the PackFile data.
                Ok(pack_file_opened) => {

                    // We put a "Save" button in the top part, and left the lower part for an horizontal
                    // Box with the "Copy" button and the TreeView.
                    let folder_tree_view_extra_exit_button = Button::new_with_label("Exit \"Add file/folder from PackFile\" mode");
                    packed_file_data_display.add(&folder_tree_view_extra_exit_button);

                    let packed_file_data_display_horizontal_box = Box::new(Orientation::Horizontal, 0);
                    packed_file_data_display.pack_end(&packed_file_data_display_horizontal_box, true, true, 0);

                    // First, we create the "Copy" Button.
                    let folder_tree_view_extra_copy_button = Button::new_with_label("<=");
                    packed_file_data_display_horizontal_box.add(&folder_tree_view_extra_copy_button);

                    // Second, we create the new TreeView (in a ScrolledWindow) and his TreeStore.
                    let folder_tree_view_extra = TreeView::new();
                    let folder_tree_store_extra = TreeStore::new(&[String::static_type()]);
                    folder_tree_view_extra.set_model(Some(&folder_tree_store_extra));

                    let column_extra = TreeViewColumn::new();
                    let cell_extra = CellRendererText::new();
                    column_extra.pack_start(&cell_extra, true);
                    column_extra.add_attribute(&cell_extra, "text", 0);

                    folder_tree_view_extra.append_column(&column_extra);
                    folder_tree_view_extra.set_enable_tree_lines(true);
                    folder_tree_view_extra.set_enable_search(false);
                    folder_tree_view_extra.set_rules_hint(true);
                    folder_tree_view_extra.set_headers_visible(false);

                    let folder_tree_view_extra_scroll = ScrolledWindow::new(None, None);
                    folder_tree_view_extra_scroll.add(&folder_tree_view_extra);

                    packed_file_data_display_horizontal_box.pack_end(&folder_tree_view_extra_scroll, true, true, 0);

                    // And show everything and lock the main PackFile's TreeView.
                    packed_file_data_display.show_all();
                    *is_folder_tree_view_locked.borrow_mut() = true;

                    *pack_file_decoded_extra.borrow_mut() = pack_file_opened;
                    ui::update_tree_view(&folder_tree_store_extra, &*pack_file_decoded_extra.borrow());

                    // We need to check here if the selected destiny is not a file. Otherwise
                    // we disable the "Copy" button.
                    folder_tree_selection.connect_changed(clone!(
                    folder_tree_view_extra_copy_button,
                    pack_file_decoded => move |folder_tree_selection| {
                        let tree_path = ui::get_tree_path_from_selection(&folder_tree_selection, true);

                        // Only in case it's not a file, we enable the "Copy" Button.
                        match get_type_of_selected_tree_path(&tree_path, &*pack_file_decoded.borrow()) {
                            TreePathType::File(_) => folder_tree_view_extra_copy_button.set_sensitive(false),
                            TreePathType::Folder(_) | TreePathType::PackFile | TreePathType::None => folder_tree_view_extra_copy_button.set_sensitive(true),
                        }
                    }));

                    // When we click in the "Copy" button (<=).
                    folder_tree_view_extra_copy_button.connect_button_release_event(clone!(
                        window,
                        error_dialog,
                        pack_file_decoded,
                        pack_file_decoded_extra,
                        folder_tree_view,
                        folder_tree_store,
                        folder_tree_selection,
                        folder_tree_view_extra => move |_,_| {

                        let tree_path_source = ui::get_tree_path_from_selection(&folder_tree_view_extra.get_selection(), true);
                        let tree_path_destination = ui::get_tree_path_from_selection(&folder_tree_selection, true);
                        let mut packed_file_added = false;
                        match packfile::add_packedfile_to_packfile(
                            &*pack_file_decoded_extra.borrow(),
                            &mut *pack_file_decoded.borrow_mut(),
                            tree_path_source,
                            tree_path_destination,
                        ) {
                            Ok(_) => packed_file_added = true,
                            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                        }
                        if packed_file_added {
                            window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                            ui::update_tree_view_expand_path(
                                &folder_tree_store,
                                &*pack_file_decoded.borrow(),
                                &folder_tree_selection,
                                &folder_tree_view,
                                false
                            );
                        }

                        Inhibit(false)
                    }));

                    // When we click in the "Exit "Add file/folder from PackFile" mode" button.
                    folder_tree_view_extra_exit_button.connect_button_release_event(clone!(
                        packed_file_data_display,
                        is_folder_tree_view_locked => move |_,_| {
                        *is_folder_tree_view_locked.borrow_mut() = false;

                        // We need to destroy any children that the packed_file_data_display we use may have, cleaning it.
                        let children_to_utterly_destroy = packed_file_data_display.get_children();
                        if !children_to_utterly_destroy.is_empty() {
                            for i in children_to_utterly_destroy.iter() {
                                i.destroy();
                            }
                        }
                        ui::display_help_tips(&packed_file_data_display);

                        Inhibit(false)
                    }));

                }
                Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
            }
        }
        file_chooser_add_from_packfile_dialog.hide_on_delete();

        Inhibit(false)
    }));

    // When we hit the "Delete file/folder" button.
    tree_view_delete_file.connect_button_release_event(clone!(
        window,
        error_dialog,
        pack_file_decoded,
        folder_tree_view,
        folder_tree_store,
        folder_tree_selection,
        context_menu_tree_view => move |_,_|{

        // We hide the context menu, then we get the selected file/folder, delete it and update the
        // TreeView. Pretty simple, actually.
        context_menu_tree_view.popdown();

        let tree_path = ui::get_tree_path_from_selection(&folder_tree_selection, true);
        let mut success = false;
        match packfile::delete_from_packfile(&mut *pack_file_decoded.borrow_mut(), tree_path) {
            Ok(_) => success = true,
            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
        }
        if success {
            window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
            ui::update_tree_view_expand_path(
                &folder_tree_store,
                &*pack_file_decoded.borrow(),
                &folder_tree_selection,
                &folder_tree_view,
                true
            );
        }

        Inhibit(false)
    }));


    // When we hit the "Extract file/folder" button.
    tree_view_extract_file.connect_button_release_event(clone!(
        success_dialog,
        error_dialog,
        pack_file_decoded,
        folder_tree_selection,
        context_menu_tree_view => move |_,_|{

        // First, we hide the context menu.
        context_menu_tree_view.popdown();

        let tree_path = ui::get_tree_path_from_selection(&folder_tree_selection, true);

        // Then, we check with the correlation data if the tree_path is a folder or a file.
        // Both (folder and file) are processed in the same way but we need a different
        // FileChooser for files and folders, so we check first what it's.
        match get_type_of_selected_tree_path(&tree_path, &*pack_file_decoded.borrow()) {
            TreePathType::File(_) => {
                file_chooser_extract_file.set_current_name(&tree_path.last().unwrap());
                if file_chooser_extract_file.run() == gtk::ResponseType::Ok.into() {
                    match packfile::extract_from_packfile(
                        &*pack_file_decoded.borrow(),
                        tree_path,
                        file_chooser_extract_file.get_filename().expect("Couldn't open file")) {
                        Ok(result) => ui::show_dialog(&success_dialog, result),
                        Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
                    }
                }
                file_chooser_extract_file.hide_on_delete();
            },
            TreePathType::Folder(_) => {
                if file_chooser_extract_folder.run() == gtk::ResponseType::Ok.into() {
                    match packfile::extract_from_packfile(
                        &*pack_file_decoded.borrow(),
                        tree_path,
                        file_chooser_extract_folder.get_filename().expect("Couldn't open file")) {
                        Ok(result) => ui::show_dialog(&success_dialog, result),
                        Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
                    }
                }
                file_chooser_extract_folder.hide_on_delete();
            }
            TreePathType::PackFile => ui::show_dialog(&error_dialog, format!("Extracting an entire PackFile is not implemented. Yet.")),
            TreePathType::None => ui::show_dialog(&error_dialog, format!("You can't extract non-existant files.")),
        }

        Inhibit(false)
    }));

    /*
    --------------------------------------------------------
                        Special Events
    --------------------------------------------------------
    */

    // When we double-click something in the TreeView (or click something already selected).
    folder_tree_view.connect_row_activated(clone!(
        window,
        error_dialog,
        pack_file_decoded,
        folder_tree_view,
        folder_tree_store,
        folder_tree_selection,
        rename_popover,
        rename_popover_text_entry => move |_,_,_| {

        // First, we get the variable for the new name and spawn the popover.
        let new_name: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

        let rect = ui::get_rect_for_popover(&folder_tree_view, None);
        rename_popover.set_pointing_to(&rect);
        rename_popover.popup();

        // Now, in the "New Name" popup, we wait until "Enter" (65293) is hit AND released.
        // In that point, we try to rename the file/folder selected. If we success, the TreeView is
        // updated. If not, we get a Dialog saying why.
        rename_popover.connect_key_release_event(clone!(
            window,
            error_dialog,
            pack_file_decoded,
            folder_tree_view,
            folder_tree_store,
            folder_tree_selection,
            rename_popover,
            rename_popover_text_entry,
            new_name => move |_, key| {

            let key_val = key.get_keyval();
            if key_val == 65293 {
                let mut name_changed = false;
                let tree_path = ui::get_tree_path_from_selection(&folder_tree_selection, true);
                *new_name.borrow_mut() = rename_popover_text_entry.get_buffer().get_text();
                match packfile::rename_packed_file(&mut *pack_file_decoded.borrow_mut(), tree_path.to_vec(), &*new_name.borrow()) {
                    Ok(_) => {
                        rename_popover.popdown();
                        name_changed = true;
                    }
                    Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
                }
                if name_changed {
                    ui::update_tree_view_expand_path(
                        &folder_tree_store,
                        &*pack_file_decoded.borrow(),
                        &folder_tree_selection,
                        &folder_tree_view,
                        true
                    );
                    window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                }
                rename_popover_text_entry.get_buffer().set_text("");
            }
            // We need to set this to true to avoid the Enter re-fire this event again and again.
            Inhibit(true)
        }));
        Inhibit(true);
    }));


    // When you select a file in the TreeView, decode it with his codec, if it's implemented.
    folder_tree_view.connect_cursor_changed(clone!(
        window,
        error_dialog,
        pack_file_decoded,
        folder_tree_selection,
        tree_view_packedfile_loc_add_rows,
        tree_view_packedfile_loc_add_rows_number,
        tree_view_packedfile_loc_delete_row,
        tree_view_packedfile_loc_import_csv,
        tree_view_packedfile_loc_export_csv,
        is_folder_tree_view_locked,
        context_menu_tree_view_packed_file_loc => move |_| {

        // Before anything else, we need to check if the TreeView is unlocked. Otherwise we don't
        // execute anything from here.
        if !(*is_folder_tree_view_locked.borrow()) {

            // First, we destroy any childrens that the packed_file_data_display we use may have, cleaning it.
            let childrens_to_utterly_destroy = packed_file_data_display.get_children();
            if !childrens_to_utterly_destroy.is_empty() {
                for i in childrens_to_utterly_destroy.iter() {
                    i.destroy();
                }
            }

            // Then, we get the tree_path selected, and check if it's a folder or a file.
            let tree_path = ui::get_tree_path_from_selection(&folder_tree_selection, false);

            let mut is_a_file = false;
            let mut index: i32 = 0;
            for i in &*pack_file_decoded.borrow().pack_file_data.packed_files {
                if i.packed_file_path == tree_path {
                    is_a_file = true;
                    break;
                }
                index += 1;
            }

            // Only in case it's a file, we do something.
            if is_a_file {

                // First, we get his type to decode it properly
                let mut packed_file_type: &str = "None";
                if tree_path.last().unwrap().ends_with(".loc") {
                    packed_file_type = "LOC";
                }
                else if tree_path.last().unwrap().ends_with(".txt") ||
                        tree_path.last().unwrap().ends_with(".xml") ||
                        tree_path.last().unwrap().ends_with(".csv") ||
                        tree_path.last().unwrap().ends_with(".lua") {
                    packed_file_type = "TEXT";
                }
                else if tree_path.last().unwrap().ends_with(".rigid_model_v2") {
                    packed_file_type = "RIGIDMODEL"
                }
                else if tree_path[0] == "db" {
                    packed_file_type = "DB";
                }

                // Then, depending of his type we decode it properly (if we have it implemented support
                // for his type).
                match packed_file_type {
                    "LOC" => {

                        // We check if it's decodeable before trying it.
                        let packed_file_data_encoded = &*pack_file_decoded.borrow().pack_file_data.packed_files[index as usize].packed_file_data;
                        let packed_file_data_decoded = Loc::read(packed_file_data_encoded.to_vec());
                        match packed_file_data_decoded {
                            Ok(packed_file_data_decoded) => {

                                let packed_file_data_decoded = Rc::new(RefCell::new(packed_file_data_decoded));
                                // First, we create the new TreeView and all the needed stuff, and prepare it to
                                // display the data from the Loc file.
                                let packed_file_tree_view_stuff = ui::packedfile_loc::PackedFileLocTreeView::create_tree_view(&packed_file_data_display);
                                let packed_file_tree_view = packed_file_tree_view_stuff.packed_file_tree_view;
                                let packed_file_list_store = packed_file_tree_view_stuff.packed_file_list_store;
                                let packed_file_tree_view_selection = packed_file_tree_view_stuff.packed_file_tree_view_selection;
                                let packed_file_tree_view_cell_key = packed_file_tree_view_stuff.packed_file_tree_view_cell_key;
                                let packed_file_tree_view_cell_text = packed_file_tree_view_stuff.packed_file_tree_view_cell_text;
                                let packed_file_tree_view_cell_tooltip = packed_file_tree_view_stuff.packed_file_tree_view_cell_tooltip;

                                // We enable "Multiple" selection mode, so we can do multi-row operations.
                                packed_file_tree_view_selection.set_mode(gtk::SelectionMode::Multiple);

                                // Now we set the new TreeView as parent of the context menu Popover.
                                context_menu_tree_view_packed_file_loc.set_relative_to(Some(&packed_file_tree_view));

                                // Then we populate the TreeView with the entries of the Loc PackedFile.
                                ui::packedfile_loc::PackedFileLocTreeView::load_data_to_tree_view(&packed_file_data_decoded.borrow().packed_file_data, &packed_file_list_store);

                                // Here they come!!! This is what happen when we edit the cells.
                                // This is the key column. Here we need to restrict the String to not having " ",
                                // be empty or repeated.
                                packed_file_tree_view_cell_key.connect_edited(clone!(
                                    window,
                                    error_dialog,
                                    pack_file_decoded,
                                    packed_file_data_decoded,
                                    packed_file_tree_view,
                                    packed_file_list_store => move |_,tree_path , new_text|{

                                    // First we need to check if the value has changed. Otherwise we do nothing.
                                    let edited_cell = packed_file_list_store.get_iter(&tree_path);
                                    let edited_cell_column = packed_file_tree_view.get_cursor();
                                    let old_text: String = packed_file_list_store.get_value(&edited_cell.unwrap(), edited_cell_column.1.unwrap().get_sort_column_id()).get().unwrap();

                                    // If the value has changed, then we need to check that the new value is
                                    // valid, as this is a key column.
                                    if old_text != new_text {
                                        let current_line = packed_file_list_store.get_iter_first().unwrap();
                                        let mut key_already_exists = false;
                                        let mut done = false;
                                        while !done {
                                            let key: String = packed_file_list_store.get_value(&current_line, 1).get().unwrap();
                                            if key == new_text {
                                                key_already_exists = true;
                                                break;
                                            }
                                            else if !packed_file_list_store.iter_next(&current_line) {
                                                done = true;
                                            }
                                        }

                                        if new_text.is_empty() {
                                            ui::show_dialog(&error_dialog, format!("Only my hearth can be empty."));
                                        }
                                        else if new_text.contains(" ") {
                                            ui::show_dialog(&error_dialog, format!("Spaces are not valid characters."));
                                        }
                                        else if key_already_exists {
                                            ui::show_dialog(&error_dialog, format!("This key is already in the Loc PackedFile."));
                                        }

                                        // If it has passed all the checks without error, we update the Loc PackedFile
                                        // and save the changes.
                                        else {
                                            let edited_cell = packed_file_list_store.get_iter(&tree_path);
                                            let edited_cell_column = packed_file_tree_view.get_cursor();
                                            packed_file_list_store.set_value(&edited_cell.unwrap(), edited_cell_column.1.unwrap().get_sort_column_id() as u32, &new_text.to_value());

                                            // Get the data from the table and turn it into a Vec<u8> to write it.
                                            packed_file_data_decoded.borrow_mut().packed_file_data = ui::packedfile_loc::PackedFileLocTreeView::return_data_from_tree_view(&packed_file_list_store);
                                            ::packfile::update_packed_file_data_loc(
                                                &*packed_file_data_decoded.borrow_mut(),
                                                &mut *pack_file_decoded.borrow_mut(),
                                                index as usize);
                                            window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                                        }
                                    }
                                }));


                                packed_file_tree_view_cell_text.connect_edited(clone!(
                                    window,
                                    pack_file_decoded,
                                    packed_file_data_decoded,
                                    packed_file_tree_view,
                                    packed_file_list_store => move |_,tree_path , new_text|{

                                    let edited_cell = packed_file_list_store.get_iter(&tree_path);
                                    let edited_cell_column = packed_file_tree_view.get_cursor();
                                    packed_file_list_store.set_value(&edited_cell.unwrap(), edited_cell_column.1.unwrap().get_sort_column_id() as u32, &new_text.to_value());

                                    // Get the data from the table and turn it into a Vec<u8> to write it.
                                    packed_file_data_decoded.borrow_mut().packed_file_data = ui::packedfile_loc::PackedFileLocTreeView::return_data_from_tree_view(&packed_file_list_store);
                                    ::packfile::update_packed_file_data_loc(
                                        &*packed_file_data_decoded.borrow_mut(),
                                        &mut *pack_file_decoded.borrow_mut(),
                                        index as usize);
                                    window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                                }));


                                packed_file_tree_view_cell_tooltip.connect_toggled(clone!(
                                    window,
                                    pack_file_decoded,
                                    packed_file_data_decoded,
                                    packed_file_tree_view,
                                    packed_file_list_store => move |cell, tree_path|{

                                    let tree_iter = packed_file_list_store.get_iter(&tree_path).unwrap();
                                    // Get (Option<TreePath>, Option<TreeViewColumn>)
                                    let edited_cell_column: u32 = packed_file_tree_view.get_cursor().1.unwrap().get_sort_column_id() as u32;
                                    let new_value: bool = packed_file_list_store.get_value(&tree_iter, edited_cell_column as i32).get().unwrap();
                                    let new_value_bool = (!new_value).to_value();
                                    cell.set_active(!new_value);
                                    packed_file_list_store.set_value(&tree_iter, edited_cell_column, &new_value_bool);

                                    // Get the data from the table and turn it into a Vec<u8> to write it.
                                    packed_file_data_decoded.borrow_mut().packed_file_data = ui::packedfile_loc::PackedFileLocTreeView::return_data_from_tree_view(&packed_file_list_store);
                                    ::packfile::update_packed_file_data_loc(
                                        &*packed_file_data_decoded.borrow_mut(),
                                        &mut *pack_file_decoded.borrow_mut(),
                                        index as usize);
                                    window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                                }));


                                // When we right-click the TreeView, we check if we need to enable or disable his buttons first.
                                // Then we calculate the position where the popup must aim, and show it.
                                //
                                // NOTE: REMEMBER, WE OPEN THE POPUP HERE, BUT WE NEED TO CLOSED IT WHEN WE HIT HIS BUTTONS.
                                packed_file_tree_view.connect_button_release_event(clone!(
                                    packed_file_tree_view_selection,
                                    tree_view_packedfile_loc_delete_row,
                                    context_menu_tree_view_packed_file_loc => move |packed_file_tree_view, button| {

                                    let button_val = button.get_button();
                                    if button_val == 3 {
                                        if packed_file_tree_view_selection.count_selected_rows() > 0 {
                                            tree_view_packedfile_loc_delete_row.set_sensitive(true);
                                        }
                                        else {
                                            tree_view_packedfile_loc_delete_row.set_sensitive(false);
                                        }
                                        let rect = ui::get_rect_for_popover(&packed_file_tree_view, Some(button.get_position()));

                                        context_menu_tree_view_packed_file_loc.set_pointing_to(&rect);
                                        context_menu_tree_view_packed_file_loc.popup();
                                    }

                                    Inhibit(false)
                                }));

                                // When we hit the "Add row" button.
                                tree_view_packedfile_loc_add_rows.connect_button_release_event(clone!(
                                    window,
                                    error_dialog,
                                    pack_file_decoded,
                                    packed_file_data_decoded,
                                    packed_file_list_store,
                                    tree_view_packedfile_loc_add_rows_number,
                                    context_menu_tree_view_packed_file_loc => move |_,_| {

                                    // We hide the context menu, then we get the selected file/folder, delete it and update the
                                    // TreeView. Pretty simple, actually.
                                    context_menu_tree_view_packed_file_loc.popdown();

                                    // First, we check if the input is a valid number, as I'm already seeing people
                                    // trying to add "two" rows.
                                    let number_rows = tree_view_packedfile_loc_add_rows_number.get_buffer().get_text();
                                    match number_rows.parse::<u32>() {
                                        Ok(number_rows) => {
                                            // Then we make this the new line's "Key" field unique, so there are no
                                            // duplicate keys in the Loc PackedFile.
                                            for _ in 0..number_rows {
                                                let mut new_key = String::new();

                                                // Before checking for duplicates, we need to check if there is at least
                                                // a row.
                                                if let Some(mut current_line) = packed_file_list_store.get_iter_first() {
                                                    let mut done = false;
                                                    let mut j = 1;

                                                    while !done {
                                                        let key: String = packed_file_list_store.get_value(&current_line, 1).get().unwrap();

                                                        if key == format!("New_line_{}", j) {
                                                            current_line = packed_file_list_store.get_iter_first().unwrap();
                                                            j += 1;
                                                        }
                                                        else if !packed_file_list_store.iter_next(&current_line) {
                                                            new_key = format!("New_line_{}", j);
                                                            done = true;
                                                        }
                                                    }
                                                }
                                                else {
                                                    new_key = format!("New_line_1");
                                                }

                                                packed_file_list_store.insert_with_values(None, &[0, 1, 2, 3], &[&"New".to_value(), &new_key.to_value(), &"New_line_text".to_value(), &true.to_value()]);
                                            }

                                            // Get the data from the table and turn it into a Vec<u8> to write it.
                                            packed_file_data_decoded.borrow_mut().packed_file_data = ui::packedfile_loc::PackedFileLocTreeView::return_data_from_tree_view(&packed_file_list_store);
                                            ::packfile::update_packed_file_data_loc(
                                                &*packed_file_data_decoded.borrow_mut(),
                                                &mut *pack_file_decoded.borrow_mut(),
                                                index as usize);
                                            window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                                        }
                                        Err(error) => ui::show_dialog(&error_dialog, format!("You can only add an \"ENTIRE NUMBER\" of rows. Like 4, or 6. Maybe 5, who knows? But definetly not \"{}\".", error::Error::description(&error).to_string())),
                                    }
                                    Inhibit(false)
                                }));

                                // When we hit the "Delete row" button.
                                tree_view_packedfile_loc_delete_row.connect_button_release_event(clone!(
                                    window,
                                    pack_file_decoded,
                                    packed_file_data_decoded,
                                    packed_file_list_store,
                                    packed_file_tree_view_selection,
                                    context_menu_tree_view_packed_file_loc => move |_,_| {

                                    // We hide the context menu, then we get the selected file/folder, delete it and update the
                                    // TreeView. Pretty simple, actually.
                                    context_menu_tree_view_packed_file_loc.popdown();

                                    // (Vec<TreePath>, TreeModel)
                                    let mut selected_rows = packed_file_tree_view_selection.get_selected_rows();

                                    // Only in case there is something selected (so we have at least a TreePath)
                                    // we delete rows. We sort the rows selected and reverse them. This is because
                                    // it's the only way I found to always delete the rows in reverse (from last
                                    // to beginning) so we avoid getting missing iters due to the rest of the rows
                                    // repositioning themselves after deleting one of them.
                                    if !selected_rows.0.is_empty() {
                                        selected_rows.0.sort();
                                        for i in (0..selected_rows.0.len()).rev() {
                                            let selected_row_iter = packed_file_list_store.get_iter(&selected_rows.0[i]).unwrap();
                                            packed_file_list_store.remove(&selected_row_iter);
                                        }

                                        // Get the data from the table and turn it into a Vec<u8> to write it.
                                        packed_file_data_decoded.borrow_mut().packed_file_data = ui::packedfile_loc::PackedFileLocTreeView::return_data_from_tree_view(&packed_file_list_store);
                                        ::packfile::update_packed_file_data_loc(
                                            &*packed_file_data_decoded.borrow_mut(),
                                            &mut *pack_file_decoded.borrow_mut(),
                                            index as usize);
                                        window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                                    }

                                    Inhibit(false)
                                }));

                                // When we hit the "Import to CSV" button.
                                tree_view_packedfile_loc_import_csv.connect_button_release_event(clone!(
                                    window,
                                    error_dialog,
                                    pack_file_decoded,
                                    packed_file_data_decoded,
                                    packed_file_list_store,
                                    file_chooser_packedfile_loc_import_csv,
                                    context_menu_tree_view_packed_file_loc => move |_,_|{

                                    // We hide the context menu first.
                                    context_menu_tree_view_packed_file_loc.popdown();

                                    // First we ask for the file to import.
                                    if file_chooser_packedfile_loc_import_csv.run() == gtk::ResponseType::Ok.into() {
                                        match packedfile::import_from_csv(file_chooser_packedfile_loc_import_csv.get_filename().expect("Couldn't open file")) {

                                            // If the file we choose has been processed into a LocData, we replace
                                            // our old LocData with that one, and then re-create the ListStore.
                                            // After that, we save the PackedFile to memory with the new data.
                                            Ok(result) => {
                                                packed_file_data_decoded.borrow_mut().packed_file_data = result;
                                                ui::packedfile_loc::PackedFileLocTreeView::load_data_to_tree_view(&packed_file_data_decoded.borrow().packed_file_data, &packed_file_list_store);

                                                // Get the data from the table and turn it into a Vec<u8> to write it.
                                                packed_file_data_decoded.borrow_mut().packed_file_data = ui::packedfile_loc::PackedFileLocTreeView::return_data_from_tree_view(&packed_file_list_store);
                                                ::packfile::update_packed_file_data_loc(
                                                    &*packed_file_data_decoded.borrow_mut(),
                                                    &mut *pack_file_decoded.borrow_mut(),
                                                    index as usize);
                                                window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                                            }
                                            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
                                        }
                                    }
                                    file_chooser_packedfile_loc_import_csv.hide_on_delete();

                                    Inhibit(false)
                                }));

                                // When we hit the "Export to CSV" button.
                                tree_view_packedfile_loc_export_csv.connect_button_release_event(clone!(
                                    error_dialog,
                                    success_dialog,
                                    packed_file_data_decoded,
                                    folder_tree_selection,
                                    file_chooser_packedfile_loc_export_csv,
                                    context_menu_tree_view_packed_file_loc => move |_,_|{

                                    // We hide the context menu first.
                                    context_menu_tree_view_packed_file_loc.popdown();

                                    let tree_path = ui::get_tree_path_from_selection(&folder_tree_selection, false);
                                    file_chooser_packedfile_loc_export_csv.set_current_name(format!("{}.csv",&tree_path.last().unwrap()));

                                    if file_chooser_packedfile_loc_export_csv.run() == gtk::ResponseType::Ok.into() {
                                        match packedfile::export_to_csv(&packed_file_data_decoded.borrow_mut().packed_file_data, file_chooser_packedfile_loc_export_csv.get_filename().expect("Couldn't open file")) {
                                            Ok(result) => ui::show_dialog(&success_dialog, result),
                                            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
                                        }
                                    }
                                    file_chooser_packedfile_loc_export_csv.hide_on_delete();
                                    Inhibit(true)
                                }));
                            }
                            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                        }

                    }

                    // If it's a DB, we try to decode it
                    "DB" => {
                        let table = &*tree_path[1];
                        let packed_file_data_encoded = &*pack_file_decoded.borrow().pack_file_data.packed_files[index as usize].packed_file_data;
                        let packed_file_data_decoded = DB::read(packed_file_data_encoded.to_vec(), table, master_schema.clone());
                        match packed_file_data_decoded {
                            Ok(packed_file_data_decoded) => {

                                let packed_file_data_decoded = Rc::new(RefCell::new(packed_file_data_decoded));
                                // ONLY if we have found an schema, we decode it, otherwise we do nothing.
                                let packed_file_data_structure = &packed_file_data_decoded.borrow().packed_file_data.packed_file_data_structure;
                                match *packed_file_data_structure {
                                    Some(ref packed_file_data_structure) => {
                                        let packed_file_tree_view_stuff = match ui::packedfile_db::PackedFileDBTreeView::create_tree_view(&packed_file_data_display, &*packed_file_data_decoded.borrow()) {
                                            Ok(data) => data,
                                            Err(error) => return ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
                                        };
                                        let packed_file_tree_view = packed_file_tree_view_stuff.packed_file_tree_view;
                                        let packed_file_list_store = packed_file_tree_view_stuff.packed_file_list_store;

                                        let packed_file_tree_view_selection = packed_file_tree_view.get_selection();

                                        // We enable "Multiple" selection mode, so we can do multi-row operations.
                                        packed_file_tree_view_selection.set_mode(gtk::SelectionMode::Multiple);

                                        // Now we set the new TreeView as parent of the context menu Popover.
                                        context_menu_tree_view_packed_file_db.set_relative_to(Some(&packed_file_tree_view));

                                        if let Err(error) = ui::packedfile_db::PackedFileDBTreeView::load_data_to_tree_view(
                                            (&packed_file_data_decoded.borrow().packed_file_data.packed_file_data).to_vec(),
                                            &packed_file_list_store,
                                        ) {
                                            return ui::show_dialog(&error_dialog, error::Error::description(&error).to_string())
                                        };

                                        // These are the events to save edits in cells, one loop for every type of cell.
                                        // This loop takes care of the interaction with string cells.
                                        for edited_cell in packed_file_tree_view_stuff.packed_file_tree_view_cell_string.iter() {
                                            edited_cell.connect_edited(clone!(
                                            window,
                                            error_dialog,
                                            pack_file_decoded,
                                            packed_file_data_decoded,
                                            packed_file_tree_view,
                                            packed_file_list_store => move |_ ,tree_path , new_text| {

                                            let edited_cell = packed_file_list_store.get_iter(&tree_path);
                                            let edited_cell_column = packed_file_tree_view.get_cursor();
                                            packed_file_list_store.set_value(&edited_cell.unwrap(), edited_cell_column.1.unwrap().get_sort_column_id() as u32, &new_text.to_value());

                                            // Get the data from the table and turn it into a Vec<u8> to write it.
                                            let packed_file_data_structure = &packed_file_data_decoded.borrow().packed_file_data.packed_file_data_structure.clone();
                                            match ui::packedfile_db::PackedFileDBTreeView::return_data_from_tree_view(&packed_file_data_structure ,&packed_file_list_store) {
                                                Ok(data) => {
                                                    packed_file_data_decoded.borrow_mut().packed_file_data.packed_file_data = data;
                                                    if let Err(error) = ::packfile::update_packed_file_data_db(&*packed_file_data_decoded.borrow_mut(), &mut *pack_file_decoded.borrow_mut(), index as usize) {
                                                        ui::show_dialog(&error_dialog, error::Error::description(&error).to_string());
                                                    }
                                                    window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));

                                                }
                                                Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                                                }
                                            }));

                                        }

                                        // This loop takes care of the interaction with optional_string cells.
                                        for edited_cell in packed_file_tree_view_stuff.packed_file_tree_view_cell_optional_string.iter() {
                                            edited_cell.connect_edited(clone!(
                                            window,
                                            error_dialog,
                                            pack_file_decoded,
                                            packed_file_data_decoded,
                                            packed_file_tree_view,
                                            packed_file_list_store => move |_ ,tree_path , new_text|{

                                            let edited_cell = packed_file_list_store.get_iter(&tree_path);
                                            let edited_cell_column = packed_file_tree_view.get_cursor();
                                            packed_file_list_store.set_value(&edited_cell.unwrap(), edited_cell_column.1.unwrap().get_sort_column_id() as u32, &new_text.to_value());

                                            // Get the data from the table and turn it into a Vec<u8> to write it.
                                            let packed_file_data_structure = &packed_file_data_decoded.borrow().packed_file_data.packed_file_data_structure.clone();
                                            match ui::packedfile_db::PackedFileDBTreeView::return_data_from_tree_view(&packed_file_data_structure ,&packed_file_list_store) {
                                                Ok(data) => {
                                                    packed_file_data_decoded.borrow_mut().packed_file_data.packed_file_data = data;
                                                    if let Err(error) = ::packfile::update_packed_file_data_db(&*packed_file_data_decoded.borrow_mut(), &mut *pack_file_decoded.borrow_mut(), index as usize) {
                                                        ui::show_dialog(&error_dialog, error::Error::description(&error).to_string());
                                                    }
                                                    window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));

                                                }
                                                Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                                                }
                                            }));
                                        }

                                        // This loop takes care of the interaction with U32 cells.
                                        for edited_cell in packed_file_tree_view_stuff.packed_file_tree_view_cell_integer.iter() {
                                            edited_cell.connect_edited(clone!(
                                            window,
                                            error_dialog,
                                            pack_file_decoded,
                                            packed_file_data_decoded,
                                            packed_file_tree_view,
                                            packed_file_list_store => move |_ ,tree_path , new_text|{

                                                let new_number = new_text.parse();
                                                match new_number {
                                                    Ok(new_number) => {
                                                        let new_number: u32 = new_number;
                                                        let edited_cell = packed_file_list_store.get_iter(&tree_path);
                                                        let edited_cell_column = packed_file_tree_view.get_cursor();
                                                        packed_file_list_store.set_value(&edited_cell.unwrap(), edited_cell_column.1.unwrap().get_sort_column_id() as u32, &new_number.to_value());

                                                        // Get the data from the table and turn it into a Vec<u8> to write it.
                                                        let packed_file_data_structure = &packed_file_data_decoded.borrow().packed_file_data.packed_file_data_structure.clone();
                                                        match ui::packedfile_db::PackedFileDBTreeView::return_data_from_tree_view(&packed_file_data_structure ,&packed_file_list_store) {
                                                            Ok(data) => {
                                                                packed_file_data_decoded.borrow_mut().packed_file_data.packed_file_data = data;
                                                                if let Err(error) = ::packfile::update_packed_file_data_db(&*packed_file_data_decoded.borrow_mut(), &mut *pack_file_decoded.borrow_mut(), index as usize) {
                                                                    ui::show_dialog(&error_dialog, error::Error::description(&error).to_string());
                                                                }
                                                                window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));

                                                            }
                                                            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                                                        }
                                                    }
                                                    Err(_) => {
                                                        let edited_cell = packed_file_list_store.get_iter(&tree_path).unwrap();
                                                        let edited_cell_column = packed_file_tree_view.get_cursor().1.unwrap().get_sort_column_id();
                                                        let old_number: u32 = packed_file_list_store.get_value(&edited_cell, edited_cell_column as i32).get().unwrap();
                                                        packed_file_list_store.set_value(&edited_cell, edited_cell_column as u32, &old_number.to_value());
                                                    }
                                                }
                                            }));
                                        }

                                        // This loop takes care of the interaction with F32 cells.
                                        // TODO: Delete the trailing zeros.
                                        for edited_cell in packed_file_tree_view_stuff.packed_file_tree_view_cell_float.iter() {
                                            edited_cell.connect_edited(clone!(
                                            window,
                                            error_dialog,
                                            pack_file_decoded,
                                            packed_file_data_decoded,
                                            packed_file_tree_view,
                                            packed_file_list_store => move |_ ,tree_path , new_text|{

                                                let new_number = new_text.parse();
                                                match new_number {
                                                    Ok(new_number) => {
                                                        let new_number: f32 = new_number;
                                                        let edited_cell = packed_file_list_store.get_iter(&tree_path);
                                                        let edited_cell_column = packed_file_tree_view.get_cursor();
                                                        packed_file_list_store.set_value(&edited_cell.unwrap(), edited_cell_column.1.unwrap().get_sort_column_id() as u32, &new_number.to_value());

                                                        // Get the data from the table and turn it into a Vec<u8> to write it.
                                                        let packed_file_data_structure = &packed_file_data_decoded.borrow().packed_file_data.packed_file_data_structure.clone();
                                                        match ui::packedfile_db::PackedFileDBTreeView::return_data_from_tree_view(&packed_file_data_structure ,&packed_file_list_store) {
                                                            Ok(data) => {
                                                                packed_file_data_decoded.borrow_mut().packed_file_data.packed_file_data = data;
                                                                if let Err(error) = ::packfile::update_packed_file_data_db(&*packed_file_data_decoded.borrow_mut(), &mut *pack_file_decoded.borrow_mut(), index as usize) {
                                                                    ui::show_dialog(&error_dialog, error::Error::description(&error).to_string());
                                                                }
                                                                window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));

                                                            }
                                                            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                                                        }
                                                    }
                                                    Err(_) => {
                                                        let edited_cell = packed_file_list_store.get_iter(&tree_path).unwrap();
                                                        let edited_cell_column = packed_file_tree_view.get_cursor().1.unwrap().get_sort_column_id();
                                                        let old_number: f32 = packed_file_list_store.get_value(&edited_cell, edited_cell_column as i32).get().unwrap();
                                                        packed_file_list_store.set_value(&edited_cell, edited_cell_column as u32, &old_number.to_value());
                                                    }
                                                }
                                            }));
                                        }

                                        // This loop takes care of the interaction with bool cells.
                                        for edited_cell in packed_file_tree_view_stuff.packed_file_tree_view_cell_bool.iter() {
                                            edited_cell.connect_toggled(clone!(
                                            window,
                                            error_dialog,
                                            pack_file_decoded,
                                            packed_file_data_decoded,
                                            packed_file_tree_view,
                                            packed_file_list_store => move |cell, tree_path|{

                                            let tree_iter = packed_file_list_store.get_iter(&tree_path).unwrap();
                                            // Get (Option<TreePath>, Option<TreeViewColumn>)
                                            let edited_cell_column: u32 = packed_file_tree_view.get_cursor().1.unwrap().get_sort_column_id() as u32;
                                            let new_value: bool = packed_file_list_store.get_value(&tree_iter, edited_cell_column as i32).get().unwrap();
                                            let new_value_bool = (!new_value).to_value();
                                            cell.set_active(!new_value);
                                            packed_file_list_store.set_value(&tree_iter, edited_cell_column, &new_value_bool);

                                            // Get the data from the table and turn it into a Vec<u8> to write it.
                                            let packed_file_data_structure = &packed_file_data_decoded.borrow().packed_file_data.packed_file_data_structure.clone();
                                            match ui::packedfile_db::PackedFileDBTreeView::return_data_from_tree_view(&packed_file_data_structure ,&packed_file_list_store) {
                                                Ok(data) => {
                                                    packed_file_data_decoded.borrow_mut().packed_file_data.packed_file_data = data;
                                                    if let Err(error) = ::packfile::update_packed_file_data_db(&*packed_file_data_decoded.borrow_mut(), &mut *pack_file_decoded.borrow_mut(), index as usize) {
                                                        ui::show_dialog(&error_dialog, error::Error::description(&error).to_string());
                                                    }
                                                    window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));

                                                }
                                                Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                                                }
                                            }));
                                        }

                                        // When we right-click the TreeView, we check if we need to enable or disable his buttons first.
                                        // Then we calculate the position where the popup must aim, and show it.
                                        //
                                        // NOTE: REMEMBER, WE OPEN THE POPUP HERE, BUT WE NEED TO CLOSED IT WHEN WE HIT HIS BUTTONS.
                                        packed_file_tree_view.connect_button_release_event(clone!(
                                            packed_file_tree_view_selection,
                                            tree_view_packedfile_db_delete_row,
                                            context_menu_tree_view_packed_file_db => move |packed_file_tree_view, button| {

                                            let button_val = button.get_button();
                                            if button_val == 3 {
                                                if packed_file_tree_view_selection.count_selected_rows() > 0 {
                                                    tree_view_packedfile_db_delete_row.set_sensitive(true);
                                                }
                                                else {
                                                    tree_view_packedfile_db_delete_row.set_sensitive(false);
                                                }
                                                let rect = ui::get_rect_for_popover(&packed_file_tree_view, Some(button.get_position()));

                                                context_menu_tree_view_packed_file_db.set_pointing_to(&rect);
                                                context_menu_tree_view_packed_file_db.popup();
                                            }

                                            Inhibit(false)
                                        }));

                                        // When we hit the "Add row" button.
                                        tree_view_packedfile_db_add_rows.connect_button_release_event(clone!(
                                            window,
                                            error_dialog,
                                            pack_file_decoded,
                                            packed_file_data_decoded,
                                            packed_file_list_store,
                                            packed_file_data_structure,
                                            tree_view_packedfile_db_add_rows_number,
                                            context_menu_tree_view_packed_file_db => move |_,_|{

                                            // We hide the context menu, then we get the selected file/folder, delete it and update the
                                            // TreeView. Pretty simple, actually.
                                            context_menu_tree_view_packed_file_db.popdown();

                                            // First, we check if the input is a valid number, as I'm already seeing people
                                            // trying to add "two" rows.
                                            let number_rows = tree_view_packedfile_db_add_rows_number.get_buffer().get_text();
                                            let mut error_while_adding_rows = false;
                                            let mut error_description = Error::new(ErrorKind::Other, format!("This is just so the compiler doesn't complain about not initialized value"));
                                            match number_rows.parse::<u32>() {
                                                Ok(number_rows) => {

                                                    let column_amount = packed_file_data_structure.len();
                                                    for _ in 0..number_rows {

                                                        // Due to issues with types and gtk-rs, we need to create an empty line and then add the
                                                        // values to it, one by one.
                                                        let current_row = packed_file_list_store.append();
                                                        let mut index = 0;

                                                        for column in 0..(column_amount + 1) {

                                                            let gtk_value_field;

                                                            // First column it's always the index.
                                                            if column == 0 {
                                                                gtk_value_field = gtk::ToValue::to_value(&format!("New"));
                                                            }
                                                            else {
                                                                let field = packed_file_data_structure.get_index((column as usize) - 1).unwrap();
                                                                let field_type = field.1;

                                                                match &**field_type {
                                                                    "boolean" => {
                                                                        gtk_value_field = gtk::ToValue::to_value(&false);
                                                                    }
                                                                    "string" | "string_ascii" => {
                                                                        gtk_value_field = gtk::ToValue::to_value(&String::new());
                                                                    }
                                                                    "optstring" | "optstring_ascii" => {
                                                                        gtk_value_field = gtk::ToValue::to_value(&String::new());
                                                                    }
                                                                    "int" => {
                                                                        gtk_value_field = gtk::ToValue::to_value(&0);
                                                                    }
                                                                    "float" => {
                                                                        gtk_value_field = gtk::ToValue::to_value(&0.0);
                                                                    }
                                                                    _ => {
                                                                        // If this fires up, the table has a non-implemented field.
                                                                        error_while_adding_rows = true;
                                                                        error_description = Error::new(ErrorKind::Other, format!("Error while trying to create a new row: Unknown field_type \"{}\".", field_type));
                                                                        break;

                                                                    }
                                                                }
                                                            }
                                                            packed_file_list_store.set_value(&current_row, index, &gtk_value_field);
                                                            index += 1;
                                                        }

                                                        // If errors have been found, stop.
                                                        if error_while_adding_rows {
                                                            break;
                                                        }
                                                    }
                                                    if error_while_adding_rows {
                                                        ui::show_dialog(&error_dialog, error::Error::description(&error_description).to_string());
                                                    }
                                                    else {
                                                        // Get the data from the table and turn it into a Vec<u8> to write it.
                                                        let packed_file_data_structure = &packed_file_data_decoded.borrow().packed_file_data.packed_file_data_structure.clone();
                                                        match ui::packedfile_db::PackedFileDBTreeView::return_data_from_tree_view(&packed_file_data_structure ,&packed_file_list_store) {
                                                            Ok(data) => {
                                                                packed_file_data_decoded.borrow_mut().packed_file_data.packed_file_data = data;
                                                                if let Err(error) = ::packfile::update_packed_file_data_db(&*packed_file_data_decoded.borrow_mut(), &mut *pack_file_decoded.borrow_mut(), index as usize) {
                                                                    ui::show_dialog(&error_dialog, error::Error::description(&error).to_string());
                                                                }
                                                                window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));

                                                            }
                                                            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                                                        }
                                                    }
                                                }
                                                Err(_) => ui::show_dialog(&error_dialog, format!("You can only add an \"ENTIRE NUMBER\" of rows. Like 4, or 6. Maybe 5, who knows?")),
                                            }
                                            Inhibit(false)
                                        }));

                                        // When we hit the "Delete row" button.
                                        tree_view_packedfile_db_delete_row.connect_button_release_event(clone!(
                                            window,
                                            error_dialog,
                                            pack_file_decoded,
                                            packed_file_data_decoded,
                                            packed_file_list_store,
                                            packed_file_tree_view_selection,
                                            context_menu_tree_view_packed_file_db => move |_,_|{

                                            // We hide the context menu, then we get the selected file/folder, delete it and update the
                                            // TreeView. Pretty simple, actually.
                                            context_menu_tree_view_packed_file_db.popdown();

                                            // (Vec<TreePath>, TreeModel)
                                            let mut selected_rows = packed_file_tree_view_selection.get_selected_rows();

                                            // Only in case there is something selected (so we have at least a TreePath)
                                            // we delete rows. We sort the rows selected and reverse them. This is because
                                            // it's the only way I found to always delete the rows in reverse (from last
                                            // to beginning) so we avoid getting missing iters due to the rest of the rows
                                            // repositioning themselves after deleting one of them.
                                            if !selected_rows.0.is_empty() {
                                                selected_rows.0.sort();
                                                for i in (0..selected_rows.0.len()).rev() {
                                                    let selected_row_iter = packed_file_list_store.get_iter(&selected_rows.0[i]).unwrap();
                                                    packed_file_list_store.remove(&selected_row_iter);
                                                }

                                                // Get the data from the table and turn it into a Vec<u8> to write it.
                                                let packed_file_data_structure = &packed_file_data_decoded.borrow().packed_file_data.packed_file_data_structure.clone();
                                                match ui::packedfile_db::PackedFileDBTreeView::return_data_from_tree_view(&packed_file_data_structure ,&packed_file_list_store) {
                                                    Ok(data) => {
                                                        packed_file_data_decoded.borrow_mut().packed_file_data.packed_file_data = data;
                                                        if let Err(error) = ::packfile::update_packed_file_data_db(&*packed_file_data_decoded.borrow_mut(), &mut *pack_file_decoded.borrow_mut(), index as usize) {
                                                            ui::show_dialog(&error_dialog, error::Error::description(&error).to_string());
                                                        }
                                                        window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));

                                                    }
                                                    Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                                                }
                                            }

                                            Inhibit(false)
                                        }));
                                    }
                                    None => {
                                        return ui::show_dialog(&error_dialog, format!("Schema to decode this DB PackedFile Type not yet implemented."))
                                    }
                                }
                            }
                            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                        }
                    }

                    // If it's a plain text file, we create a source view and try to get highlighting for
                    // his language, if it's an specific language file.
                    "TEXT" => {

                        // Before doing anything, we try to decode the data. Only if we success, we create
                        // the SourceView and add the data to it.
                        let packed_file_data_encoded = &*pack_file_decoded.borrow().pack_file_data.packed_files[index as usize].packed_file_data;
                        match coding_helpers::decode_string_u8(packed_file_data_encoded.to_vec()) {
                            Ok(string) => {

                                // First, we create a vertical Box, put a "Save" button in the top part, and left
                                // the lower part for the SourceView.
                                let packed_file_source_view_save_button = Button::new_with_label("Save to PackedFile");
                                packed_file_data_display.add(&packed_file_source_view_save_button);

                                // Second, we create the new SourceView (in a ScrolledWindow) and his buffer,
                                // get his buffer and put the text in it.
                                let packed_file_source_view_scroll = ScrolledWindow::new(None, None);
                                packed_file_data_display.pack_end(&packed_file_source_view_scroll, true, true, 0);

                                let packed_file_source_view_buffer: Buffer = Buffer::new(None);
                                let packed_file_source_view = View::new_with_buffer(&packed_file_source_view_buffer);

                                // Third, we config the SourceView for our needs.
                                packed_file_source_view.set_tab_width(4);
                                packed_file_source_view.set_show_line_numbers(true);
                                packed_file_source_view.set_indent_on_tab(true);
                                packed_file_source_view.set_highlight_current_line(true);

                                // Then, we get the Language of the file.
                                let language_manager = LanguageManager::get_default().unwrap();
                                let packedfile_language: Option<Language>;
                                if tree_path.last().unwrap().ends_with(".xml") {
                                    packedfile_language = language_manager.get_language("xml");
                                }
                                else if tree_path.last().unwrap().ends_with(".lua") {
                                    packedfile_language = language_manager.get_language("lua");
                                }
                                else if tree_path.last().unwrap().ends_with(".csv") {
                                    packedfile_language = language_manager.get_language("csv");
                                }
                                else {
                                    packedfile_language = None;
                                }

                                // Then we set the Language of the file, if it has one.
                                if let Some(language) = packedfile_language {
                                    packed_file_source_view_buffer.set_language(&language);
                                }

                                packed_file_source_view_buffer.set_text(&*string);

                                // And show everything.
                                packed_file_source_view_scroll.add(&packed_file_source_view);
                                packed_file_data_display.show_all();

                                // When we click in the "Save to PackedFile" button
                                packed_file_source_view_save_button.connect_button_release_event(clone!(
                                    window,
                                    pack_file_decoded => move |_,_| {
                                    let packed_file_data_decoded = coding_helpers::encode_string_u8(packed_file_source_view.get_buffer().unwrap().get_slice(
                                        &packed_file_source_view.get_buffer().unwrap().get_start_iter(),
                                        &packed_file_source_view.get_buffer().unwrap().get_end_iter(),
                                        true).unwrap());

                                    ::packfile::update_packed_file_data_text(
                                        packed_file_data_decoded,
                                        &mut *pack_file_decoded.borrow_mut(),
                                        index as usize);

                                    window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));

                                    Inhibit(false)
                                }));
                            }
                            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                        }
                    }

                    // If it's a rigidmodel, we decode it and take care of his update events.
                    "RIGIDMODEL" => {
                        let packed_file_data_encoded = &*pack_file_decoded.borrow().pack_file_data.packed_files[index as usize].packed_file_data;
                        let packed_file_data_decoded = RigidModel::read(packed_file_data_encoded.to_vec());
                        match packed_file_data_decoded {
                            Ok(packed_file_data_decoded) => {
                                let packed_file_data_view_stuff = ui::packedfile_rigidmodel::PackedFileRigidModelDataView::create_data_view(&packed_file_data_display, &packed_file_data_decoded);
                                let packed_file_save_button = packed_file_data_view_stuff.packed_file_save_button;
                                let rigid_model_game_patch_button = packed_file_data_view_stuff.rigid_model_game_patch_button;
                                let rigid_model_game_label = packed_file_data_view_stuff.rigid_model_game_label;
                                let packed_file_texture_paths = packed_file_data_view_stuff.packed_file_texture_paths;
                                let packed_file_data_decoded = Rc::new(RefCell::new(packed_file_data_decoded));

                                // When we hit the "Patch to Warhammer 1&2" button.
                                rigid_model_game_patch_button.connect_button_release_event(clone!(
                                    window,
                                    error_dialog,
                                    success_dialog,
                                    pack_file_decoded,
                                    packed_file_data_decoded => move |rigid_model_game_patch_button, _| {

                                    let packed_file_data_patch_result = packfile::patch_rigid_model_attila_to_warhammer(&mut *packed_file_data_decoded.borrow_mut());
                                    match packed_file_data_patch_result {
                                        Ok(result) => {
                                            rigid_model_game_patch_button.set_sensitive(false);
                                            rigid_model_game_label.set_text("RigidModel compatible with: \"Warhammer 1&2\".");

                                            let mut success = false;
                                            match ::packfile::update_packed_file_data_rigid(
                                                &*packed_file_data_decoded.borrow(),
                                                &mut *pack_file_decoded.borrow_mut(),
                                                index as usize
                                            ) {
                                                Ok(_) => {
                                                    success = true;
                                                    ui::show_dialog(&success_dialog, result);
                                                },
                                                Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                                            }
                                            if success {
                                                window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                                            }
                                        },
                                        Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                                    }
                                    Inhibit(false)
                                }));

                                // When we hit the "Save to PackFile" button.
                                packed_file_save_button.connect_button_release_event(clone!(
                                    window,
                                    error_dialog,
                                    success_dialog,
                                    pack_file_decoded,
                                    packed_file_texture_paths,
                                    packed_file_data_decoded => move |_ ,_|{

                                    let new_data = ui::packedfile_rigidmodel::PackedFileRigidModelDataView::return_data_from_data_view(
                                        packed_file_texture_paths.to_vec(),
                                        &mut (*packed_file_data_decoded.borrow_mut()).packed_file_data.packed_file_data_lods_data.to_vec()
                                    );

                                    packed_file_data_decoded.borrow_mut().packed_file_data.packed_file_data_lods_data = new_data;

                                    let mut success = false;
                                    match ::packfile::update_packed_file_data_rigid(
                                        &*packed_file_data_decoded.borrow(),
                                        &mut *pack_file_decoded.borrow_mut(),
                                        index as usize
                                    ) {
                                        Ok(result) => {
                                            success = true;
                                            ui::show_dialog(&success_dialog, result)
                                        },
                                        Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                                    }
                                    if success {
                                        window.set_title(&format!("Rusted PackFile Manager -> {}(modified)", pack_file_decoded.borrow().pack_file_extra_data.file_name));
                                    }
                                    Inhibit(false)
                                }));
                            }
                            Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                        }
                    }

                    // If we reach this point, the coding to implement this type of file is not done yet,
                    // so we ignore the file.
                    _ => {
                        ui::display_help_tips(&packed_file_data_display);
                    }
                }
            }

            // If it's a folder, then we need to display the Tips.
            else {
                ui::display_help_tips(&packed_file_data_display);
            }
        }
        Inhibit(false);
    }));

    // This allow us to open a PackFile by "Drag&Drop" it into the folder_tree_view.
    folder_tree_view.connect_drag_data_received(clone!(
        window,
        error_dialog,
        pack_file_decoded,
        folder_tree_store,
        top_menu_file_change_packfile_type_boot,
        top_menu_file_change_packfile_type_release,
        top_menu_file_change_packfile_type_patch,
        top_menu_file_change_packfile_type_mod,
        top_menu_file_change_packfile_type_movie => move |_, _, _, _, selection_data, info, _| {
        match info {
            0 => {
                let pack_file_path: PathBuf;
                if cfg!(target_os = "linux") {
                    pack_file_path = PathBuf::from(selection_data.get_uris()[0].replace("file:///", "/").replace("%20", " "));
                }
                else {
                    pack_file_path = PathBuf::from(selection_data.get_uris()[0].replace("file:///", "").replace("%20", " "));
                }
                match packfile::open_packfile(pack_file_path) {
                    Ok(pack_file_opened) => {

                        *pack_file_decoded.borrow_mut() = pack_file_opened;
                        ui::update_tree_view(&folder_tree_store, &*pack_file_decoded.borrow());
                        window.set_title(&format!("Rusted PackFile Manager -> {}", pack_file_decoded.borrow().pack_file_extra_data.file_name));

                        // We choose the right option, depending on our PackFile.
                        if pack_file_decoded.borrow().pack_file_header.pack_file_type == 0u32 {
                            top_menu_file_change_packfile_type_boot.set_active(true);
                        }
                        else if pack_file_decoded.borrow().pack_file_header.pack_file_type == 1u32{
                            top_menu_file_change_packfile_type_release.set_active(true);
                        }
                        else if pack_file_decoded.borrow().pack_file_header.pack_file_type == 2u32{
                            top_menu_file_change_packfile_type_patch.set_active(true);
                        }
                        else if pack_file_decoded.borrow().pack_file_header.pack_file_type == 3u32{
                            top_menu_file_change_packfile_type_mod.set_active(true);
                        }
                        else if pack_file_decoded.borrow().pack_file_header.pack_file_type == 4u32{
                            top_menu_file_change_packfile_type_movie.set_active(true);
                        }
                    }
                    Err(error) => ui::show_dialog(&error_dialog, error::Error::description(&error).to_string()),
                }
            }
            _ => ui::show_dialog(&error_dialog, format!("This type of event is not yet used.")),
        }
    }));

    // We start GTK. Yay
    gtk::main();
}



