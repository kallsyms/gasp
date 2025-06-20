use log::debug;
use pyo3::prelude::*;
use pyo3::types::PyString;

use crate::python_types::PyTypeInfo;
use crate::tag_finder::TagFinder;
use crate::xml_parser::StreamParser as XmlStreamParser;

/// Wrapper for the StreamParser that handles typed conversions
#[derive(Debug)]
pub struct TypedStreamParser {
    tag_finder: TagFinder,
    xml_parser: XmlStreamParser,
    type_info: Option<PyTypeInfo>,
    is_done: bool,
    capturing: bool,
    is_pydantic_model: bool,
    xml_buffer: String,
    partial_instance: Option<PyObject>,
    current_field: Option<String>,
    current_field_content: String,
}

impl TypedStreamParser {
    pub fn new(wanted_tags: Vec<String>, ignored_tags: Vec<String>) -> Self {
        Self {
            tag_finder: TagFinder::new_with_filter(wanted_tags, ignored_tags),
            xml_parser: XmlStreamParser::new(),
            type_info: None,
            is_done: false,
            capturing: false,
            is_pydantic_model: false,
            xml_buffer: String::new(),
            partial_instance: None,
            current_field: None,
            current_field_content: String::new(),
        }
    }

    pub fn with_type(
        type_info: PyTypeInfo,
        wanted_tags: Vec<String>,
        ignored_tags: Vec<String>,
    ) -> Self {
        Self {
            tag_finder: TagFinder::new_with_filter(wanted_tags, ignored_tags),
            xml_parser: XmlStreamParser::new(),
            type_info: Some(type_info),
            is_done: false,
            capturing: false,
            is_pydantic_model: false,
            xml_buffer: String::new(),
            partial_instance: None,
            current_field: None,
            current_field_content: String::new(),
        }
    }

    /// Process a chunk of XML data with support for partial field values
    pub fn step(&mut self, chunk: &str) -> PyResult<Option<PyObject>> {
        let mut result = None;

        // Feed to tag_finder and reconstruct filtered XML
        let mut events = Vec::new();
        self.tag_finder
            .push(chunk, |event| {
                events.push(event.clone());

                // Build filtered XML buffer from tag events
                match &event {
                    crate::tag_finder::TagEvent::Open(name) => {
                        self.xml_buffer.push('<');
                        self.xml_buffer.push_str(name);
                        self.xml_buffer.push('>');
                    }
                    crate::tag_finder::TagEvent::Bytes(content) => {
                        self.xml_buffer.push_str(content);
                    }
                    crate::tag_finder::TagEvent::Close(name) => {
                        self.xml_buffer.push_str("</");
                        self.xml_buffer.push_str(name);
                        self.xml_buffer.push('>');
                    }
                }
                Ok(())
            })
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Tag parsing error: {:?}", e))
            })?;

        // Process container types incrementally from buffer
        if let Some(type_info) = &self.type_info {
            match type_info.kind {
                crate::python_types::PyTypeKind::List => {
                // Create list on first <list> tag
                if self.xml_buffer.contains("<list") && self.partial_instance.is_none() {
                    self.partial_instance = Some(pyo3::Python::with_gil(|py| {
                        pyo3::types::PyList::empty(py).into()
                    }));
                    result = self.partial_instance.clone();
                }
                
                // Process all items in buffer (both complete and incomplete)
                if let Some(list_obj) = &self.partial_instance {
                    if let Some(element_type) = type_info.args.get(0) {
                        pyo3::Python::with_gil(|py| {
                            let py_list = list_obj.as_ref(py).downcast::<pyo3::types::PyList>().unwrap();
                            let current_len = py_list.len();
                            
                            // For nested lists, only count items with the correct type attribute
                            let mut item_starts = Vec::new();
                            let mut pos = 0;
                            
                            // Determine the expected type attribute for items
                            let expected_type = match element_type.kind {
                                crate::python_types::PyTypeKind::List => {
                                    // For List[List[X]], items should have type="list[X]"
                                    if let Some(inner_type) = element_type.args.get(0) {
                                        format!("list[{}]", inner_type.name)
                                    } else {
                                        "list".to_string()
                                    }
                                }
                                _ => element_type.name.clone()
                            };
                            
                            while let Some(start) = self.xml_buffer[pos..].find("<item") {
                                let item_start = pos + start;
                                
                                // Check if this item has the expected type
                                let item_end = self.xml_buffer[item_start..].find('>')
                                    .map(|e| item_start + e)
                                    .unwrap_or(self.xml_buffer.len());
                                let item_tag = &self.xml_buffer[item_start..item_end];
                                
                                // Extract type attribute
                                if let Some(type_start) = item_tag.find("type=\"") {
                                    let type_start = type_start + 6;
                                    if let Some(type_end) = item_tag[type_start..].find('"') {
                                        let item_type = &item_tag[type_start..type_start + type_end];
                                        
                                        // Only add this item if it matches the expected type
                                        if item_type == expected_type {
                                            item_starts.push(item_start);
                                        }
                                    }
                                }
                                
                                pos = item_start + 5;
                            }
                            
                            // Create items for any we haven't processed yet
                            for i in current_len..item_starts.len() {
                                // For Union types, we need to determine the actual type first
                                if element_type.kind == crate::python_types::PyTypeKind::Union {
                                    // Look at the item to determine its type
                                    let item_start = item_starts[i];
                                    let item_end = self.xml_buffer[item_start..].find("</item>")
                                        .map(|e| item_start + e)
                                        .unwrap_or(self.xml_buffer.len());
                                    let item_content = &self.xml_buffer[item_start..item_end];
                                    
                                    // Try to extract type attribute from item tag
                                    let type_name = if let Some(type_start) = item_content.find("type=\"") {
                                        let type_start = type_start + 6;
                                        if let Some(type_end) = item_content[type_start..].find('"') {
                                            Some(&item_content[type_start..type_start + type_end])
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };
                                    
                                    // Find the matching union member
                                    if let Some(type_name) = type_name {
                                        for member in &element_type.args {
                                            if member.name == type_name {
                                                if let Some(member_py_type) = &member.py_type {
                                                    let item = if member_py_type.as_ref(py).hasattr("__gasp_from_partial__").unwrap_or(false) {
                                                        let empty_dict = pyo3::types::PyDict::new(py);
                                                        member_py_type.as_ref(py).call_method1("__gasp_from_partial__", (empty_dict,)).unwrap()
                                                    } else {
                                                        member_py_type.as_ref(py).call0().unwrap()
                                                    };
                                                    py_list.append(item).unwrap();
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Non-union type, create instance based on element type kind
                                    let item: PyObject = match element_type.kind {
                                        crate::python_types::PyTypeKind::List => {
                                            // For nested lists, create an empty list
                                            pyo3::types::PyList::empty(py).into()
                                        }
                                        crate::python_types::PyTypeKind::Dict => {
                                            // For dicts, create an empty dict
                                            pyo3::types::PyDict::new(py).into()
                                        }
                                        crate::python_types::PyTypeKind::Set => {
                                            // For sets, create an empty set
                                            py.eval("set()", None, None).unwrap().into()
                                        }
                                        crate::python_types::PyTypeKind::Tuple => {
                                            // For tuples, create an empty list (will convert later)
                                            pyo3::types::PyList::empty(py).into()
                                        }
                                        _ => {
                                            // For other types, use the py_type if available
                                            if let Some(py_type) = &element_type.py_type {
                                                if py_type.as_ref(py).hasattr("__gasp_from_partial__").unwrap_or(false) {
                                                    let empty_dict = pyo3::types::PyDict::new(py);
                                                    py_type.as_ref(py).call_method1("__gasp_from_partial__", (empty_dict,)).unwrap().into()
                                                } else {
                                                    py_type.as_ref(py).call0().unwrap().into()
                                                }
                                            } else {
                                                py.None()
                                            }
                                        }
                                    };
                                    py_list.append(item).unwrap();
                                }
                            }
                            
                            // Update fields for ALL items (not just complete ones)
                            for (idx, item_start) in item_starts.iter().enumerate() {
                                if idx < py_list.len() {
                                    let item = py_list.get_item(idx).unwrap();
                                    
                                    // Extract content for this item
                                    let item_end = self.xml_buffer[*item_start..].find("</item>")
                                        .map(|e| item_start + e)
                                        .unwrap_or(self.xml_buffer.len());
                                    
                                    let item_content = &self.xml_buffer[*item_start..item_end];
                                    
                                    // Special handling for nested lists
                                    if element_type.kind == crate::python_types::PyTypeKind::List {
                                        // This is a nested list - find all inner items
                                        let inner_list = item.downcast::<pyo3::types::PyList>().unwrap();
                                        
                                        // Find all <item type="..."> tags within this list item
                                        let mut inner_pos = 0;
                                        while let Some(inner_start) = item_content[inner_pos..].find("<item") {
                                            let inner_start_abs = inner_pos + inner_start;
                                            
                                            // Check if this inner item is complete
                                            if let Some(inner_end) = item_content[inner_start_abs..].find("</item>") {
                                                let inner_end_abs = inner_start_abs + inner_end;
                                                
                                                // Extract the content between <item> and </item>
                                                if let Some(content_start) = item_content[inner_start_abs..].find('>') {
                                                    let content_start_abs = inner_start_abs + content_start + 1;
                                                    let content = &item_content[content_start_abs..inner_end_abs].trim();
                                                    
                                                    if !content.is_empty() {
                                                        // Parse based on inner element type
                                                        if let Some(inner_elem_type) = element_type.args.get(0) {
                                                            let py_value = match inner_elem_type.kind {
                                                                crate::python_types::PyTypeKind::Integer => {
                                                                    content.parse::<i64>().unwrap_or(0).into_py(py)
                                                                }
                                                                crate::python_types::PyTypeKind::Float => {
                                                                    content.parse::<f64>().unwrap_or(0.0).into_py(py)
                                                                }
                                                                crate::python_types::PyTypeKind::Boolean => {
                                                                    matches!(content.to_lowercase().as_str(), "true" | "1" | "yes").into_py(py)
                                                                }
                                                                _ => content.into_py(py)
                                                            };
                                                            
                                                            // Only append if we don't already have this item
                                                            // (to avoid duplicates during incremental parsing)
                                                            let current_inner_len = inner_list.len();
                                                            let expected_items = item_content.matches("</item>").count();
                                                            if current_inner_len < expected_items {
                                                                inner_list.append(py_value).unwrap();
                                                            }
                                                        }
                                                    }
                                                }
                                                
                                                inner_pos = inner_end_abs + 7; // Move past </item>
                                            } else {
                                                break; // No closing tag yet
                                            }
                                        }
                                    } else {
                                        // Non-list items - update fields as before
                                        for (field_name, field_info) in &element_type.fields {
                                            let field_start = format!("<{}", field_name);
                                            let field_end = format!("</{}>", field_name);
                                            
                                            if let Some(fs) = item_content.find(&field_start) {
                                                if let Some(tag_end) = item_content[fs..].find('>') {
                                                    let content_start = fs + tag_end + 1;
                                                    
                                                    // Find field content (either to closing tag or end of item)
                                                    let content_end = item_content[content_start..].find(&field_end)
                                                        .map(|e| content_start + e)
                                                        .unwrap_or(item_content.len());
                                                        
                                                    let content = &item_content[content_start..content_end];
                                                    
                                                    // Convert and set field value
                                                    let py_value = match field_info.kind {
                                                        crate::python_types::PyTypeKind::String => content.into_py(py),
                                                        crate::python_types::PyTypeKind::Integer => {
                                                            content.trim().parse::<i64>().unwrap_or(0).into_py(py)
                                                        }
                                                        crate::python_types::PyTypeKind::Float => {
                                                            content.trim().parse::<f64>().unwrap_or(0.0).into_py(py)
                                                        }
                                                        crate::python_types::PyTypeKind::Boolean => {
                                                            matches!(content.trim().to_lowercase().as_str(), "true" | "1" | "yes").into_py(py)
                                                        }
                                                        _ => py.None()
                                                    };
                                                    
                                                    let _ = item.setattr(field_name.as_str(), py_value);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            if py_list.len() > current_len {
                                result = self.partial_instance.clone();
                            }
                        });
                    }
                }
                }
                crate::python_types::PyTypeKind::Dict => {
                    // Create dict on first <dict> tag
                    if self.xml_buffer.contains("<dict") && self.partial_instance.is_none() {
                        self.partial_instance = Some(pyo3::Python::with_gil(|py| {
                            pyo3::types::PyDict::new(py).into()
                        }));
                        result = self.partial_instance.clone();
                    }
                    
                    // Process all entries in buffer incrementally
                    if let Some(dict_obj) = &self.partial_instance {
                        pyo3::Python::with_gil(|py| {
                            let py_dict = dict_obj.as_ref(py).downcast::<pyo3::types::PyDict>().unwrap();
                            
                            // Find all <entry> tags
                            let mut entry_starts = Vec::new();
                            let mut pos = 0;
                            while let Some(start) = self.xml_buffer[pos..].find("<entry") {
                                entry_starts.push(pos + start);
                                pos = pos + start + 6;
                            }
                            
                            // Process each entry
                            for entry_start in entry_starts {
                                // Extract key attribute
                                if let Some(key_start) = self.xml_buffer[entry_start..].find("key=\"") {
                                    let key_start = entry_start + key_start + 5;
                                    if let Some(key_end) = self.xml_buffer[key_start..].find('"') {
                                        let key = &self.xml_buffer[key_start..key_start + key_end];
                                        
                                        // Check if we already have this key
                                        if !py_dict.contains(key).unwrap_or(false) {
                                            // Find entry content
                                            if let Some(content_start) = self.xml_buffer[entry_start..].find('>') {
                                                let content_start = entry_start + content_start + 1;
                                                
                                                // Find content end (either closing tag or end of buffer)
                                                let content_end = self.xml_buffer[content_start..].find("</entry>")
                                                    .map(|e| content_start + e)
                                                    .unwrap_or(self.xml_buffer.len());
                                                
                                                let content = &self.xml_buffer[content_start..content_end].trim();
                                                
                                                // Only set if we have content
                                                if !content.is_empty() {
                                                    // For now, treat all values as strings
                                                    // TODO: Parse based on value type
                                                    let _ = py_dict.set_item(key, content);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            result = self.partial_instance.clone();
                        });
                    }
                }
                crate::python_types::PyTypeKind::Set => {
                    // Create set on first <set> tag
                    if self.xml_buffer.contains("<set") && self.partial_instance.is_none() {
                        self.partial_instance = Some(pyo3::Python::with_gil(|py| {
                            py.eval("set()", None, None).unwrap().into()
                        }));
                        result = self.partial_instance.clone();
                    }
                    
                    // Process all items in buffer incrementally
                    if let Some(set_obj) = &self.partial_instance {
                        pyo3::Python::with_gil(|py| {
                            let py_set = set_obj.as_ref(py);
                            let current_len = py_set.len().unwrap_or(0);
                            
                            // Count complete <item> tags
                            let item_count = self.xml_buffer.matches("</item>").count();
                            
                            // Add items we haven't processed yet
                            if item_count > current_len {
                                // Find item contents
                                let mut pos = 0;
                                let mut items_found = 0;
                                
                                while items_found < item_count {
                                    if let Some(item_start) = self.xml_buffer[pos..].find("<item") {
                                        let item_start = pos + item_start;
                                        if let Some(content_start) = self.xml_buffer[item_start..].find('>') {
                                            let content_start = item_start + content_start + 1;
                                            
                                            if let Some(content_end) = self.xml_buffer[content_start..].find("</item>") {
                                                let content_end = content_start + content_end;
                                                let content = &self.xml_buffer[content_start..content_end].trim();
                                                
                                                if items_found >= current_len && !content.is_empty() {
                                                    // Add to set
                                                    let py_content: PyObject = content.into_py(py);
                                                    let _ = py_set.call_method1("add", (py_content,));
                                                }
                                                
                                                items_found += 1;
                                                pos = content_end;
                                            } else {
                                                break;
                                            }
                                        } else {
                                            break;
                                        }
                                    } else {
                                        break;
                                    }
                                }
                            }
                            
                            result = self.partial_instance.clone();
                        });
                    }
                }
                crate::python_types::PyTypeKind::Tuple => {
                    // Create tuple placeholder on first <tuple> tag
                    if self.xml_buffer.contains("<tuple") && self.partial_instance.is_none() {
                        // For tuples, we'll use a list during construction and convert at the end
                        self.partial_instance = Some(pyo3::Python::with_gil(|py| {
                            pyo3::types::PyList::empty(py).into()
                        }));
                        result = self.partial_instance.clone();
                    }
                    
                    // Process items incrementally (similar to list)
                    if let Some(list_obj) = &self.partial_instance {
                        pyo3::Python::with_gil(|py| {
                            let py_list = list_obj.as_ref(py).downcast::<pyo3::types::PyList>().unwrap();
                            let current_len = py_list.len();
                            
                            // Count complete <item> tags
                            let item_count = self.xml_buffer.matches("</item>").count();
                            
                            // Add items we haven't processed yet
                            for i in current_len..item_count {
                                // Find the i-th item
                                let mut pos = 0;
                                let mut items_found = 0;
                                
                                while items_found <= i {
                                    if let Some(item_start) = self.xml_buffer[pos..].find("<item") {
                                        let item_start = pos + item_start;
                                        if let Some(content_start) = self.xml_buffer[item_start..].find('>') {
                                            let content_start = item_start + content_start + 1;
                                            
                                            if let Some(content_end) = self.xml_buffer[content_start..].find("</item>") {
                                                let content_end = content_start + content_end;
                                                
                                                if items_found == i {
                                                    let content = &self.xml_buffer[content_start..content_end].trim();
                                                    // Add to list (will convert to tuple later)
                                                    if !content.is_empty() {
                                                        py_list.append(content).unwrap();
                                                    } else {
                                                        py_list.append(py.None()).unwrap();
                                                    }
                                                    break;
                                                }
                                                
                                                items_found += 1;
                                                pos = content_end;
                                            } else {
                                                break;
                                            }
                                        } else {
                                            break;
                                        }
                                    } else {
                                        break;
                                    }
                                }
                            }
                            
                            // Return the list for now (will be converted to tuple when complete)
                            result = self.partial_instance.clone();
                        });
                    }
                }
                _ => {} // Other types handled elsewhere
            }
        }

        // Process tag events for other types
        for event in &events {
            match event {
                crate::tag_finder::TagEvent::Open(tag_name) => {
                    debug!("TagEvent::Open({})", tag_name);
                    if let Some(type_info) = &self.type_info {
                        match type_info.kind {
                            crate::python_types::PyTypeKind::Dict => {
                                if tag_name == "dict" && self.partial_instance.is_none() {
                                    // Create empty dict immediately when we see <dict>
                                    self.partial_instance = Some(pyo3::Python::with_gil(|py| {
                                        pyo3::types::PyDict::new(py).into()
                                    }));
                                    result = self.partial_instance.clone();
                                }
                            }
                            crate::python_types::PyTypeKind::Set => {
                                if tag_name == "set" && self.partial_instance.is_none() {
                                    // Create empty set immediately when we see <set>
                                    self.partial_instance = Some(pyo3::Python::with_gil(|py| {
                                        py.eval("set()", None, None).unwrap().into()
                                    }));
                                    result = self.partial_instance.clone();
                                }
                            }
                            crate::python_types::PyTypeKind::Class => {
                                if tag_name == &type_info.name && self.partial_instance.is_none() {
                                    // Create instance immediately when we see the class tag
                                    self.partial_instance = Some(pyo3::Python::with_gil(|py| {
                                        if let Some(py_type) = &type_info.py_type {
                                            if py_type
                                                .as_ref(py)
                                                .hasattr("__gasp_from_partial__")
                                                .unwrap_or(false)
                                            {
                                                let empty_dict = pyo3::types::PyDict::new(py);
                                                py_type
                                                    .as_ref(py)
                                                    .call_method1("__gasp_from_partial__", (empty_dict,))
                                                    .unwrap()
                                                    .into()
                                            } else {
                                                py_type.as_ref(py).call0().unwrap().into()
                                            }
                                        } else {
                                            py.None()
                                        }
                                    }));
                                    result = self.partial_instance.clone();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                crate::tag_finder::TagEvent::Close(tag_name) => {
                    debug!("TagEvent::Close({})", tag_name);
                    // Handle closing tags - check if we have a complete item/field to process
                    if let Some(type_info) = &self.type_info {
                        match type_info.kind {
                            crate::python_types::PyTypeKind::List => {
                                if tag_name == "item" && self.partial_instance.is_some() {
                                    // Item closed, clear current item tracking
                                    self.current_field = None;
                                    self.current_field_content.clear();
                                    result = self.partial_instance.clone();
                                } else if let Some(element_type) = type_info.args.get(0) {
                                    // Check if this is a field of the current item type
                                    if let Some(field_info) = element_type.fields.get(tag_name) {
                                        // We have a complete field for an item
                                        pyo3::Python::with_gil(|py| {
                                            if let Some(list_obj) = &self.partial_instance {
                                                let py_list = list_obj.as_ref(py).downcast::<pyo3::types::PyList>().unwrap();
                                                let list_len = py_list.len();
                                                
                                                if list_len > 0 {
                                                    // Get the last item (the one being built)
                                                    if let Ok(last_item) = py_list.get_item(list_len - 1) {
                                                        // Extract field content from buffer
                                                        let field_start = format!("<{}", tag_name);
                                                        let field_end = format!("</{}>", tag_name);
                                                        
                                                        if let Some(start_idx) = self.xml_buffer.rfind(&field_start) {
                                                            if let Some(end_idx) = self.xml_buffer.rfind(&field_end) {
                                                                if let Some(content_start) = self.xml_buffer[start_idx..].find('>') {
                                                                    let content_start_abs = start_idx + content_start + 1;
                                                                    let content = &self.xml_buffer[content_start_abs..end_idx];
                                                                    
                                                                    // Convert based on field type
                                                                    let py_value = match field_info.kind {
                                                                        crate::python_types::PyTypeKind::String => content.into_py(py),
                                                                        crate::python_types::PyTypeKind::Integer => {
                                                                            content.parse::<i64>().unwrap_or(0).into_py(py)
                                                                        }
                                                                        crate::python_types::PyTypeKind::Float => {
                                                                            content.parse::<f64>().unwrap_or(0.0).into_py(py)
                                                                        }
                                                                        crate::python_types::PyTypeKind::Boolean => {
                                                                            matches!(content.to_lowercase().as_str(), "true" | "1" | "yes").into_py(py)
                                                                        }
                                                                        _ => py.None()
                                                                    };
                                                                    
                                                                    // Update the field
                                                                    let _ = last_item.setattr(tag_name.as_str(), py_value);
                                                                    result = self.partial_instance.clone();
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }
                            }
                            crate::python_types::PyTypeKind::Class => {
                                // Check if this is a field closing tag
                                if let Some(field_info) = type_info.fields.get(tag_name) {
                                    if self.partial_instance.is_some() {
                                        // Extract field content and update instance
                                        let field_start = format!("<{}", tag_name);
                                        let field_end = format!("</{}>", tag_name);
                                        
                                        if let Some(start_idx) = self.xml_buffer.rfind(&field_start) {
                                            if let Some(end_idx) = self.xml_buffer.rfind(&field_end) {
                                                // Extract content between tags
                                                if let Some(content_start) = self.xml_buffer[start_idx..].find('>') {
                                                    let content_start_abs = start_idx + content_start + 1;
                                                    let content = &self.xml_buffer[content_start_abs..end_idx];
                                                    
                                                    // Update field with content
                                                    pyo3::Python::with_gil(|py| {
                                                        if let Some(instance) = &self.partial_instance {
                                                            // Convert based on field type
                                                            let py_value = match field_info.kind {
                                                                crate::python_types::PyTypeKind::String => content.into_py(py),
                                                                crate::python_types::PyTypeKind::Integer => {
                                                                    content.parse::<i64>().unwrap_or(0).into_py(py)
                                                                }
                                                                crate::python_types::PyTypeKind::Float => {
                                                                    content.parse::<f64>().unwrap_or(0.0).into_py(py)
                                                                }
                                                                crate::python_types::PyTypeKind::Boolean => {
                                                                    matches!(content.to_lowercase().as_str(), "true" | "1" | "yes").into_py(py)
                                                                }
                                                                _ => {
                                                                    // For complex types, parse the field XML
                                                                    let field_xml = &self.xml_buffer[start_idx..end_idx + field_end.len()];
                                                                    let mut field_parser = XmlStreamParser::new();
                                                                    if let Ok(field_events) = field_parser.step(field_xml) {
                                                                        if !field_events.is_empty() {
                                                                            let wrapped_events: Vec<Result<xml::Event, crate::xml_types::XmlError>> = 
                                                                                field_events.into_iter().map(Ok).collect();
                                                                            if let Ok(xml_value) = crate::xml_parser::events_to_xml_value(wrapped_events) {
                                                                                crate::python_types::xml_to_python(py, &xml_value, Some(field_info)).unwrap_or_else(|_| py.None())
                                                                            } else {
                                                                                py.None()
                                                                            }
                                                                        } else {
                                                                            py.None()
                                                                        }
                                                                    } else {
                                                                        py.None()
                                                                    }
                                                                }
                                                            };
                                                            let _ = instance.as_ref(py).setattr(tag_name.as_str(), py_value);
                                                        }
                                                    });
                                                    result = self.partial_instance.clone();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                crate::tag_finder::TagEvent::Bytes(content) => {
                    // For string fields, update incrementally
                    if !content.trim().is_empty() {
                        self.current_field_content.push_str(content);
                    }
                }
            }
        }

        // Process for incremental parsing
        if let Some(type_info) = &self.type_info {
            // Handle raw primitive types (str, int, float, bool)
            match type_info.kind {
                crate::python_types::PyTypeKind::String
                | crate::python_types::PyTypeKind::Integer
                | crate::python_types::PyTypeKind::Float
                | crate::python_types::PyTypeKind::Boolean
                | crate::python_types::PyTypeKind::None => {
                    // For raw primitives, look for any complete element
                    // Try to find a complete element in the buffer
                    let mut tag_start = None;
                    let mut tag_name = String::new();

                    // Find opening tag
                    if let Some(start_idx) = self.xml_buffer.find('<') {
                        if let Some(end_idx) = self.xml_buffer[start_idx..].find('>') {
                            let tag_content = &self.xml_buffer[start_idx + 1..start_idx + end_idx];
                            if !tag_content.starts_with('/') && !tag_content.is_empty() {
                                // Extract tag name (before any space or >)
                                tag_name = tag_content
                                    .split_whitespace()
                                    .next()
                                    .unwrap_or("")
                                    .to_string();
                                tag_start = Some(start_idx);
                            }
                        }
                    }

                    // Check if we have a complete element
                    if !tag_name.is_empty() {
                        let close_tag = format!("</{}>", tag_name);
                        if self.xml_buffer.contains(&close_tag) {
                            // We have a complete element, parse it
                            let xml_events =
                                self.xml_parser.step(&self.xml_buffer).map_err(|e| {
                                    pyo3::exceptions::PyValueError::new_err(format!(
                                        "XML parsing error: {:?}",
                                        e
                                    ))
                                })?;

                            if !xml_events.is_empty() {
                                // Extract the text content and convert to the primitive type
                                let xml_value = crate::xml_parser::events_to_xml_value(
                                    xml_events.into_iter().map(Ok).collect(),
                                )?;

                                if let crate::xml_types::XmlValue::Element(_, _, children) =
                                    xml_value
                                {
                                    // Check if children is empty (empty element)
                                    if children.is_empty() {
                                        // Handle empty element
                                        let parsed_result =
                                            pyo3::Python::with_gil(|py| match type_info.kind {
                                                crate::python_types::PyTypeKind::String => {
                                                    Ok("".to_string().into_py(py))
                                                }
                                                crate::python_types::PyTypeKind::Integer => {
                                                    Ok::<PyObject, PyErr>(py.None())
                                                }
                                                crate::python_types::PyTypeKind::Float => {
                                                    Ok::<PyObject, PyErr>(py.None())
                                                }
                                                crate::python_types::PyTypeKind::Boolean => {
                                                    Ok(false.into_py(py))
                                                }
                                                _ => Ok(py.None()),
                                            })?;

                                        self.is_done = true;
                                        return Ok(Some(parsed_result));
                                    }

                                    // Look for text content in children
                                    for child in children {
                                        if let crate::xml_types::XmlValue::Text(text) = child {
                                            // Convert text to the appropriate primitive type
                                            let parsed_result = pyo3::Python::with_gil(|py| {
                                                match type_info.kind {
                                                    crate::python_types::PyTypeKind::String => {
                                                        Ok(text.into_py(py))
                                                    }
                                                    crate::python_types::PyTypeKind::Integer => {
                                                        match text.parse::<i64>() {
                                                            Ok(val) => Ok(val.into_py(py)),
                                                            Err(_) => {
                                                                Ok::<PyObject, PyErr>(py.None())
                                                            }
                                                        }
                                                    }
                                                    crate::python_types::PyTypeKind::Float => {
                                                        match text.parse::<f64>() {
                                                            Ok(val) => Ok(val.into_py(py)),
                                                            Err(_) => {
                                                                Ok::<PyObject, PyErr>(py.None())
                                                            }
                                                        }
                                                    }
                                                    crate::python_types::PyTypeKind::Boolean => {
                                                        let val = match text.to_lowercase().as_str()
                                                        {
                                                            "true" | "1" | "yes" => true,
                                                            "false" | "0" | "no" => false,
                                                            _ => false,
                                                        };
                                                        Ok(val.into_py(py))
                                                    }
                                                    crate::python_types::PyTypeKind::None => {
                                                        Ok(py.None())
                                                    }
                                                    _ => Ok(py.None()),
                                                }
                                            })?;

                                            self.is_done = true;
                                            return Ok(Some(parsed_result));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // If we're still collecting data, check for partial content
                    if !tag_name.is_empty() && !self.is_done {
                        let open_tag = format!("<{}", tag_name);
                        let close_tag = format!("</{}>", tag_name);

                        if self.xml_buffer.contains(&open_tag)
                            && !self.xml_buffer.contains(&close_tag)
                        {
                            // We're inside the tag, extract partial content
                            if let Some(start_idx) = self.xml_buffer.rfind('>') {
                                if start_idx < self.xml_buffer.len() - 1 {
                                    let partial_content = &self.xml_buffer[start_idx + 1..];
                                    if !partial_content.trim().is_empty()
                                        && !partial_content.contains('<')
                                    {
                                        // Return partial content as string for now
                                        result = Some(pyo3::Python::with_gil(|py| {
                                            partial_content.to_string().into_py(py)
                                        }));
                                    }
                                }
                            }
                        }
                    }

                    return Ok(result);
                }
                crate::python_types::PyTypeKind::List
                | crate::python_types::PyTypeKind::Dict
                | crate::python_types::PyTypeKind::Tuple
                | crate::python_types::PyTypeKind::Set => {
                    // For raw container types, look for complete container elements
                    let container_tag = match type_info.kind {
                        crate::python_types::PyTypeKind::List => "list",
                        crate::python_types::PyTypeKind::Dict => "dict",
                        crate::python_types::PyTypeKind::Tuple => "tuple",
                        crate::python_types::PyTypeKind::Set => "set",
                        _ => return Ok(None),
                    };

                    let open_tag = format!("<{}", container_tag);
                    let close_tag = format!("</{}>", container_tag);

                    if self.xml_buffer.contains(&open_tag) && self.xml_buffer.contains(&close_tag) {
                        // We have a complete container element, parse it
                        let xml_events = self.xml_parser.step(&self.xml_buffer).map_err(|e| {
                            pyo3::exceptions::PyValueError::new_err(format!(
                                "XML parsing error: {:?}",
                                e
                            ))
                        })?;

                        if !xml_events.is_empty() {
                            // Convert XML events to Python object
                            let wrapped_events: Vec<
                                Result<xml::Event, crate::xml_types::XmlError>,
                            > = xml_events.into_iter().map(Ok).collect();

                            let parsed_result = pyo3::Python::with_gil(|py| {
                                crate::python_types::create_instance_from_xml_events(
                                    py,
                                    type_info,
                                    wrapped_events,
                                )
                            })?;

                            self.is_done = true;
                            return Ok(Some(parsed_result));
                        }
                    }

                    return Ok(result);
                }
                _ => {
                    // Continue with existing logic for non-primitive types
                }
            }

            // Check if we're inside the main element for non-union types
            if type_info.kind != crate::python_types::PyTypeKind::Union {
                if self.xml_buffer.contains(&format!("<{}>", type_info.name))
                    || self.xml_buffer.contains(&format!("<{} ", type_info.name))
                {
                    // We're inside the main element
                    if self.partial_instance.is_none() {
                        // Create instance on first encounter
                        self.partial_instance = Some(pyo3::Python::with_gil(|py| {
                            type_info
                                .py_type
                                .as_ref()
                                .unwrap()
                                .as_ref(py)
                                .call0()
                                .unwrap()
                                .into()
                        }));
                    }

                    // Check for field tags and extract partial content
                    for (field_name, field_info) in &type_info.fields {
                        let field_start = format!("<{}", field_name);
                        let field_end = format!("</{}>", field_name);

                        if self.xml_buffer.contains(&field_start)
                            && !self.xml_buffer.contains(&field_end)
                        {
                            // We're inside a field tag, extract partial content
                            if let Some(start_idx) = self.xml_buffer.rfind('>') {
                                if start_idx < self.xml_buffer.len() - 1 {
                                    let partial_content = &self.xml_buffer[start_idx + 1..];
                                    if !partial_content.trim().is_empty()
                                        && !partial_content.contains('<')
                                    {
                                        // Update the field with partial content
                                        pyo3::Python::with_gil(|py| {
                                            if let Some(instance) = &self.partial_instance {
                                                let _ = instance
                                                    .as_ref(py)
                                                    .setattr(field_name.as_str(), partial_content);
                                            }
                                        });
                                        result = self.partial_instance.clone();
                                    }
                                }
                            }
                        } else if self.xml_buffer.contains(&field_end) {
                            // Field is complete, extract final content
                            if let Some(start_idx) = self.xml_buffer.rfind(&field_start) {
                                if let Some(content_start) = self.xml_buffer[start_idx..].find('>')
                                {
                                    let content_start_abs = start_idx + content_start + 1;
                                    if let Some(end_idx) = self.xml_buffer.rfind(&field_end) {
                                        let content = &self.xml_buffer[content_start_abs..end_idx];
                                        // Update with final content
                                        pyo3::Python::with_gil(|py| {
                                            if let Some(instance) = &self.partial_instance {
                                                let _ = instance
                                                    .as_ref(py)
                                                    .setattr(field_name.as_str(), content);
                                            }
                                        });
                                        result = self.partial_instance.clone();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // For Union types and complete element detection, keep existing logic
        if let Some(type_info) = &self.type_info {
            if type_info.kind == crate::python_types::PyTypeKind::Union {
                // First check if we have the union wrapper tag (e.g., <MyUnion type="A">)
                let union_open_tag = format!("<{}", type_info.name);
                let union_close_tag = format!("</{}>", type_info.name);

                if self.xml_buffer.contains(&union_open_tag)
                    && self.xml_buffer.contains(&union_close_tag)
                {
                    // We have a complete union wrapper element, parse it
                    let xml_events = self.xml_parser.step(&self.xml_buffer).map_err(|e| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "XML parsing error: {:?}",
                            e
                        ))
                    })?;

                    if !xml_events.is_empty() {
                        // Convert XML events to Python object using the union type
                        // Wrap events in Ok() to match expected type
                        let wrapped_events: Vec<Result<xml::Event, crate::xml_types::XmlError>> =
                            xml_events.into_iter().map(Ok).collect();

                        let parsed_result = pyo3::Python::with_gil(|py| {
                            crate::python_types::create_instance_from_xml_events(
                                py,
                                type_info,
                                wrapped_events,
                            )
                        })?;

                        self.is_done = true;
                        return Ok(Some(parsed_result));
                    }
                }

                // Otherwise check if we have a complete element for any union member
                for member in &type_info.args {
                    let open_tag = format!("<{}", member.name);
                    let close_tag = format!("</{}>", member.name);

                    if self.xml_buffer.contains(&open_tag) && self.xml_buffer.contains(&close_tag) {
                        // We have a complete element, parse it
                        let xml_events = self.xml_parser.step(&self.xml_buffer).map_err(|e| {
                            pyo3::exceptions::PyValueError::new_err(format!(
                                "XML parsing error: {:?}",
                                e
                            ))
                        })?;

                        if !xml_events.is_empty() {
                            // Convert XML events to Python object using the union type
                            // Wrap events in Ok() to match expected type
                            let wrapped_events: Vec<
                                Result<xml::Event, crate::xml_types::XmlError>,
                            > = xml_events.into_iter().map(Ok).collect();

                            let result = pyo3::Python::with_gil(|py| {
                                crate::python_types::create_instance_from_xml_events(
                                    py,
                                    type_info,
                                    wrapped_events,
                                )
                            })?;

                            self.is_done = true;
                            return Ok(Some(result));
                        }
                    }
                }
            } else {
                // Non-union type, check for the type's tag
                let open_tag = format!("<{}", type_info.name);
                let close_tag = format!("</{}>", type_info.name);

                if self.xml_buffer.contains(&open_tag) && self.xml_buffer.contains(&close_tag) {
                    // We have a complete element, parse it
                    let xml_events = self.xml_parser.step(&self.xml_buffer).map_err(|e| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "XML parsing error: {:?}",
                            e
                        ))
                    })?;

                    if !xml_events.is_empty() {
                        // Convert XML events to Python object
                        // Wrap events in Ok() to match expected type
                        let wrapped_events: Vec<Result<xml::Event, crate::xml_types::XmlError>> =
                            xml_events.into_iter().map(Ok).collect();

                        let parsed_result = pyo3::Python::with_gil(|py| {
                            crate::python_types::create_instance_from_xml_events(
                                py,
                                type_info,
                                wrapped_events,
                            )
                        })?;

                        self.is_done = true;
                        // Store in partial_instance so it gets returned at the end
                        self.partial_instance = Some(parsed_result.clone());
                        return Ok(Some(parsed_result));
                    }
                }
            }
        }

        // Return the partial instance if we have one
        Ok(result)
    }

    /// Check if parsing is complete
    pub fn is_done(&self) -> bool {
        self.is_done
    }
}

/// Python wrapper for the TypedStreamParser
#[pyclass(name = "Parser", unsendable)]
pub struct PyParser {
    parser: TypedStreamParser,
    result: Option<PyObject>,
}

#[pymethods]
impl PyParser {
    #[new]
    #[pyo3(signature = (type_obj=None, ignored_tags=vec!["think".to_string(), "thinking".to_string(), "system".to_string(), "thought".to_string()]))]
    fn new(py: Python, type_obj: Option<&PyAny>, ignored_tags: Vec<String>) -> PyResult<Self> {
        debug!(
            "[PyParser::new] type_obj: {:?}",
            type_obj.map(|o| o
                .repr()
                .unwrap_or_else(|_| PyString::new(py, "Error getting repr").into()))
        );
        match type_obj {
            Some(obj) => {
                let mut type_info = PyTypeInfo::extract_from_python(obj)?;
                debug!(
                    "[PyParser::new] Extracted type_info: name='{}', kind='{:?}', origin='{:?}'",
                    type_info.name, type_info.kind, type_info.origin
                );

                if type_info.py_type.is_none() {
                    type_info.py_type = Some(obj.into_py(py));
                }

                // For Union types, we want to look for any of the union member tags
                // AND the union type name itself (for type aliases)
                let wanted_tags = match type_info.kind {
                    crate::python_types::PyTypeKind::Union => {
                        let mut tags: Vec<String> =
                            type_info.args.iter().map(|arg| arg.name.clone()).collect();
                        // Also add the union type name itself for type aliases
                        tags.push(type_info.name.clone());
                        tags
                    }
                    crate::python_types::PyTypeKind::String => {
                        vec!["str".to_string()]
                    }
                    crate::python_types::PyTypeKind::Integer => {
                        vec!["int".to_string()]
                    }
                    crate::python_types::PyTypeKind::Float => {
                        vec!["float".to_string()]
                    }
                    crate::python_types::PyTypeKind::Boolean => {
                        vec!["bool".to_string(), "boolean".to_string()]
                    }
                    crate::python_types::PyTypeKind::None => {
                        vec!["None".to_string(), "null".to_string()]
                    }
                    crate::python_types::PyTypeKind::List => {
                        vec!["list".to_string()]
                    }
                    crate::python_types::PyTypeKind::Dict => {
                        vec!["dict".to_string()]
                    }
                    crate::python_types::PyTypeKind::Tuple => {
                        vec!["tuple".to_string()]
                    }
                    crate::python_types::PyTypeKind::Set => {
                        vec!["set".to_string()]
                    }
                    _ => vec![type_info.name.clone()],
                };
                debug!("[PyParser::new] wanted_tags: {:?}", wanted_tags);
                let parser = TypedStreamParser::with_type(type_info, wanted_tags, ignored_tags);
                Ok(Self {
                    parser,
                    result: None,
                })
            }
            None => {
                debug!("[PyParser::new] No type_obj provided.");
                let parser = TypedStreamParser::new(Vec::new(), ignored_tags);
                Ok(Self {
                    parser,
                    result: None,
                })
            }
        }
    }

    /// Create a parser for a Pydantic model
    #[staticmethod]
    #[pyo3(text_signature = "(pydantic_model)")]
    fn from_pydantic(py: Python, pydantic_model: &PyAny) -> PyResult<Self> {
        let mut type_info = PyTypeInfo::extract_from_python(pydantic_model)?;
        if type_info.py_type.is_none() {
            type_info.py_type = Some(pydantic_model.into_py(py));
        }

        let wanted_tags = vec![type_info.name.clone()];
        let mut typed_stream_parser =
            TypedStreamParser::with_type(type_info, wanted_tags, Vec::new());
        typed_stream_parser.is_pydantic_model = true;

        Ok(Self {
            parser: typed_stream_parser,
            result: None,
        })
    }

    #[pyo3(text_signature = "($self, chunk)")]
    fn feed(&mut self, py: Python, chunk: &str) -> PyResult<Option<PyObject>> {
        debug!("Feeding chunk: {}", chunk);
        if let Some(res) = self.parser.step(chunk)? {
            self.result = Some(res);
        }
        Ok(self.result.clone())
    }

    #[pyo3(text_signature = "($self)")]
    fn is_complete(&self) -> bool {
        self.parser.is_done()
    }

    #[pyo3(text_signature = "($self)")]
    fn get_partial(&mut self, py: Python) -> PyResult<Option<PyObject>> {
        Ok(self.result.clone())
    }

    #[pyo3(text_signature = "($self)")]
    fn validate(&mut self, py: Python) -> PyResult<Option<PyObject>> {
        self.get_partial(py)
    }
}
