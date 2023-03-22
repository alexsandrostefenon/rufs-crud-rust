mod utils;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
// #[cfg(feature = "wee_alloc")]
// #[global_allocator]
// static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// #[wasm_bindgen]
// extern {
//     fn alert(s: &str);
// }

// #[wasm_bindgen]
// pub fn greet() {
//     alert("Hello, {{project-name}}!");
// }

use js_sys::Array;
use serde::Deserialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use openapiv3::*;
use rufs_base_rust::openapi::{RufsOpenAPI, SchemaPlace};

// Called when the wasm module is instantiated
#[wasm_bindgen(start)]
fn main() -> Result<(), JsValue> {
    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    // Manufacture the element we're gonna append
    let val = document.create_element("p")?;
    val.set_inner_html("Hello from Rust!");

    body.append_child(&val)?;

    Ok(())
}

#[wasm_bindgen]
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[derive(Deserialize)]
#[wasm_bindgen(js_name = OpenAPI)]
pub struct OpenAPIWrapper(OpenAPI);

#[wasm_bindgen(js_class = OpenAPI)]
impl OpenAPIWrapper {

    pub fn from_str(json_str: &str) -> Result<OpenAPIWrapper, JsValue> {
        serde_json::from_str::<OpenAPIWrapper>(json_str).map_err(|err| JsValue::from(err.to_string()))
    }

    pub fn get_schema_name_from_ref(&self, reference: &str) -> String {
        let ret = OpenAPI::get_schema_name_from_ref(reference);
        ret
    }

    pub fn get_schema(&self, path :&str, method :&str, schema_place :&str) -> Result<JsValue, JsValue> {
        let schema = self.0.get_schema(path, method, &SchemaPlace::from_str(schema_place), false).map_err(|err| JsValue::from(err.to_string()))?;
        js_sys::JSON::parse(&serde_json::to_string(schema).unwrap())
    }

    pub fn get_paths(&self) -> Result<JsValue, JsValue> {
        js_sys::JSON::parse(&serde_json::to_string(&self.0.paths).unwrap())
    }

    pub fn get_dependencies(&self, schema_name_or_ref: &str) -> Array {
        let mut list = vec![];
        self.0.get_dependencies(schema_name_or_ref, &mut list);
        list.into_iter().map(|x| JsValue::from_str(&x)).collect::<Array>()
    }

    pub fn copy_fields(&self, path :&str, method :&str, schema_place :&str, may_be_array: bool, value_in: JsValue, ignorenil: bool, ignore_hiden: bool, only_primary_keys: bool) -> Result<JsValue, JsValue> {
        //web_sys::console::log_1(&format!("[rufs-crud-rust::copy_fields({}, {}, {}, {:?}, {}, {}, {})]", path, method, schema_place, value_in, ignorenil, ignore_hiden, only_primary_keys).into());
        let value_in: String = js_sys::JSON::stringify(&value_in).unwrap().into();
        //web_sys::console::log_1(&format!("[rufs-crud-rust::copy_fields({}, {}, {}, {:?}, {}, {}, {})]", path, method, schema_place, value_in, ignorenil, ignore_hiden, only_primary_keys).into());
        let value_in = serde_json::from_str(&value_in).unwrap();
        //let value_in = serde_wasm_bindgen::from_value(value_in).unwrap();
        //web_sys::console::log_1(&format!("[rufs-crud-rust::copy_fields({}, {}, {}, {:?}, {}, {}, {})]", path, method, schema_place, value_in, ignorenil, ignore_hiden, only_primary_keys).into());
        //openapi.copy_fields("/rufs_user", "get", &SchemaPlace::response, false, &json!({}), false, false, false)
        let value_out = self.0.copy_fields(path, method, &SchemaPlace::from_str(schema_place), may_be_array, &value_in, ignorenil, ignore_hiden, only_primary_keys).map_err(|err| JsValue::from(err.to_string()))?;
        //Ok(serde_wasm_bindgen::to_value(&value_out).unwrap())
        js_sys::JSON::parse(&serde_json::to_string(&value_out).unwrap())
    }

    pub fn get_value_from_schema(&self, schema_name :&str, property_name :&str, obj: JsValue) -> JsValue {
        if let Some(schema) = self.0.get_schema_from_schemas(schema_name) {
            let obj_value = serde_wasm_bindgen::from_value::<Value>(obj).unwrap();

            if let Some(value) = self.0.get_value_from_schema(schema, property_name, &obj_value) {
                let value = serde_wasm_bindgen::to_value(value).unwrap();
                return value;
            }
        }

        JsValue::NULL
    }

    pub fn get_dependents(&self, schema_name_target: &str, only_in_document :bool) -> Result<JsValue, JsValue> {
        let dependents = self.0.get_dependents(schema_name_target, only_in_document);
        js_sys::JSON::parse(&serde_json::to_string(&dependents).unwrap())
    }

    pub fn copy_value(&self, path :&str, method :&str, schema_place :&str, property_name :&str, value_in :JsValue) -> Result<JsValue, JsValue> {
        let value_in = serde_wasm_bindgen::from_value(value_in).unwrap();
        let value_out = self.0.copy_value(path, method, &SchemaPlace::from_str(schema_place), false, property_name, &value_in).map_err(|err| JsValue::from(err.to_string()))?;
        js_sys::JSON::parse(&serde_json::to_string(&value_out).unwrap())
        //let value_out = serde_wasm_bindgen::to_value(&value_out).unwrap();
        //web_sys::console::log_1(&format!("[rufs-crud-rust::copy_value({}, {}, {}, {}, {:?})] : {:?}", path, method, schema_place, property_name, value_in, value_out).into());
        //Ok(value_out)
    }

    pub fn get_primary_key_foreign(&self, schema_name :&str, property_name :&str, obj :JsValue) -> Result<JsValue, JsValue> {
        let value_in = serde_wasm_bindgen::from_value(obj).unwrap();

        if let Some(value_out) = self.0.get_primary_key_foreign(schema_name, property_name, &value_in).map_err(|err| JsValue::from(err.to_string()))? {
            //Ok(serde_wasm_bindgen::to_value(&value_out).unwrap())
            js_sys::JSON::parse(&serde_json::to_string(&value_out).unwrap())
        } else {
            Ok(JsValue::NULL)
        }
    }

    pub fn get_foreign_key(&self, schema: &str, property_name: &str, value_in: JsValue) -> Result<JsValue, JsValue> {
        let value_in = serde_wasm_bindgen::from_value(value_in).unwrap();
        let ret = self.0.get_foreign_key(schema, property_name, &value_in).map_err(|err| JsValue::from(err.to_string()))?;
        match ret {
            Some(value_out) => Ok(serde_wasm_bindgen::to_value(&value_out).unwrap()),
            None => Ok(JsValue::NULL),
        }
    }

}
