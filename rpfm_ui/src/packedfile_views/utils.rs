//---------------------------------------------------------------------------//
// Copyright (c) 2017-2020 Ismael Gutiérrez González. All rights reserved.
//
// This file is part of the Rusted PackFile Manager (RPFM) project,
// which can be found here: https://github.com/Frodo45127/rpfm.
//
// This file is licensed under the MIT license, which can be found here:
// https://github.com/Frodo45127/rpfm/blob/master/LICENSE.
//---------------------------------------------------------------------------//

/*!
Module with extra functions for `PackedFileView`.
!*/

use crate::app_ui::AppUI;
use crate::pack_tree::*;
use crate::packfile_contents_ui::PackFileContentsUI;
use crate::UI_STATE;

//-------------------------------------------------------------------------------//
//                              Enums & Structs
//-------------------------------------------------------------------------------//

/// This function sets the `is_modified` state of the open PackFile, setting also the visual state of the provided PackedFile in the process.
pub unsafe fn set_modified(is_modified: bool, path: &[String], app_ui: &mut AppUI, pack_file_contents_ui: &mut PackFileContentsUI) {
    let path = if path.is_empty() { TreePathType::PackFile } else { TreePathType::File(path.to_vec()) };
    if is_modified {
        pack_file_contents_ui.packfile_contents_tree_view.update_treeview(true, TreeViewOperation::Modify(vec![path; 1]));
        UI_STATE.set_is_modified(true, app_ui, pack_file_contents_ui);
    }
    else {
        pack_file_contents_ui.packfile_contents_tree_view.update_treeview(true, TreeViewOperation::Undo(vec![path; 1]));
    }
}
