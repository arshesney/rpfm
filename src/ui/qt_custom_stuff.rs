// Here it goes ffi stuff, like subclassing and stuff like that.

use qt_core::string_list::StringList;
use qt_core::object::Object;

/// This function gives the column you want of the given TableView a custom StyledItemDelegate using Combos instead of LineEdits.
/// You can pass it a list of strings to populate the Combos and can make it editable or non-editable. 
extern "C" { pub fn new_combobox_item_delegate(table_view: *mut Object, column: i32, list: *const StringList, is_editable: bool); }
extern "C" { pub fn new_spinbox_item_delegate(table_view: *mut Object, column: i32, integer_type: i32); }
extern "C" { pub fn new_doublespinbox_item_delegate(table_view: *mut Object, column: i32); }
