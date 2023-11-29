mod utils;
use std::{vec, collections::HashMap, cmp::Ordering};
use anyhow::{anyhow, Context};
use chrono::{Utc, NaiveDateTime};
use openapiv3::{Schema, ReferenceOr, OpenAPI, SchemaKind, Type, VariantOrUnknownOrEmpty, StringFormat};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json, Number};
use convert_case::Casing;
use indexmap::IndexMap;
use regex;
use rufs_base_rust::{openapi::{RufsOpenAPI, SchemaPlace}, rufs_micro_service::{Role}};
#[cfg(target_arch = "wasm32")]
use web_log::println;

#[derive(Debug,PartialEq,Clone,Copy,Default,Deserialize,Serialize)]
pub enum FieldSortType {
    #[default] None,
    Asc,
    Desc
}

#[derive(Debug,Clone,Copy,Default,Deserialize,Serialize)]
pub struct FieldSort {
    sort_type: FieldSortType,
    order_index: i64,
    table_visible: bool,
    hidden: bool,
}

#[derive(Default)]
struct HttpRestRequest {
    url :String,
    // message_working :String,
    // message_error :String,
    token: Option<String>,
    //http_error: String,
}

impl HttpRestRequest {

	fn new(url :&str) -> Self {
		//if url.endsWith("/") == true) url = url.substring(0, url.length-1);
		// TODO : change "rest" by openapi.server.base
        Self {url :format!("{}/{}", url, "rest"), ..Default::default()}
	}

/*
	static urlSearchParamsToJson(urlSearchParams, properties) {
		const convertSearchParamsTypes = (searchParams, properties) => {
			let reservedParams = ["primaryKey", "overwrite", "filter", "filterRange", "filterRangeMin", "filterRangeMax"];

			for name of reservedParams {
				let obj = searchParams[name];

				if obj != undefined {
					for [fieldName, value] of Object.entries(obj) {
						let field = properties[fieldName];

						if field != undefined {
							if field.type == "integer")
								obj[fieldName] = Number.parseInt(value);
							else if field.type == "number")
								obj[fieldName] = Number.parseFloat(value);
							else if field.type.startsWith("date") == true)
								obj[fieldName] = new Date(value);
						}
					}
				}
			}
		}

		if urlSearchParams.is_none() || urlSearchParams == null)
			return {};

		let _Qs = HttpRestRequest.Qs != null ? HttpRestRequest.Qs : Qs;
		let searchParams = _Qs.parse(urlSearchParams, {ignoreQueryPrefix: true, allowDots: true});
		if properties != undefined) convertSearchParamsTypes(searchParams, properties);
		return searchParams;
	}
*/
/*
	async fn login_basic(&mut self, path :&str, username :&str, password :&str) -> Result<LoginResponseClient, Box<std::error::Error>> {
        let client = reqwest::Client::new();
        let resp = client.post(&format!("{}/{}", self.url, path)).basic_auth(username, Some(password)).send().await?;

        if resp.status() != reqwest::StatusCode::OK {
            println!("[login_basic] : {:?}", resp);
        }

        let login_response_client = resp.json::<LoginResponseClient>().await?;
        self.token = Some(login_response_client.jwt_header.clone());
        Ok(login_response_client)
    }
 */
    async fn request_text(&self, path :&str, method :Method, params :&Value, data_out :&Value) -> Result<String, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let query_string = serde_qs::to_string(params).unwrap();
        let url = format!("{}{}?{}", self.url, path, query_string);

        let request = if method == Method::POST || method == Method::PUT {
            client.request(method.clone(), &url).json(&data_out)
        } else {
            client.request(method.clone(), &url)
        };
        
        let request = if let Some(token) = &self.token {
            request.bearer_auth(token)
        } else {
            request
        };

        println!("[HttpRestRequest::request_text] : waiting for {} {} ...", method, url);
        let response = request.send().await?;
        let status = response.status();
        let data_in = response.text().await?;
        println!("[HttpRestRequest::request_text] : ... returned {} from {}", status, url);

        if status != reqwest::StatusCode::OK {
            return Err(data_in)?;
        }

        Ok(data_in)
	}

    async fn request(&self, path :&str, method :Method, params :&Value, data_out :&Value) -> Result<Value, Box<dyn std::error::Error>> {
        let data_in = self.request_text(path, method, params, &data_out).await?;
        Ok(serde_json::from_str(&data_in)?)
    }

	async fn login(&mut self, path :&str, username :&str, password :&str) -> Result<LoginResponseClient, Box<dyn std::error::Error>> {
        let data_out = json!({"user": username, "password": password});
        let data_in = self.request_text(path, Method::POST, &Value::Null, &data_out).await?;
        let login_response_client = serde_json::from_str::<LoginResponseClient>(&data_in)?;
        self.token = Some(login_response_client.jwt_header.clone());
        Ok(login_response_client)
    }
    
	async fn save(&self, path :&str, item_send :&Value) -> Result<Value, Box<dyn std::error::Error>> {
		self.request(path, Method::POST, &Value::Null, item_send).await
	}

	async fn update(&self, path :&str, params :&Value, item_send :&Value) -> Result<Value, Box<dyn std::error::Error>> {
		self.request(path, Method::PUT, params, item_send).await
	}

	async fn query(&self, path :&str, params :&Value) -> Result<Value, Box<dyn std::error::Error>> {
		self.request(path, Method::GET, params, &Value::Null).await
	}

	async fn get(&self, path :&str, params :&Value) -> Result<Value, Box<dyn std::error::Error>> {
		let value = self.request(path, Method::GET, params, &Value::Null).await?;

        match value {
            Value::Array(list) => if list.len() == 1 {Ok(list[0].clone())} else {Ok(Value::Array(list))},
            _ => Ok(value),
        }
    }

	async fn remove(&self, path :&str, params :&Value) -> Result<Value, Box<dyn std::error::Error>> {
		self.request(path, Method::DELETE, params, &Value::Null).await
    }
/*
	async fn patch(&self, path :&str, item_send :&Value) -> Result<Value, anyhow::Error> {
		self.request(path, Method::PATCH, &Value::Null, item_send).await
	}
*/
}

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct Pagination {
    page: Option<usize>,
    page_size: Option<usize>,
}

#[derive(PartialEq,Clone,Copy,Debug)]
pub enum DataViewProcessAction {
    Search,
    New,
    Edit,
    View
}

impl std::fmt::Display for DataViewProcessAction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DataViewProcessAction::Search => write!(f, "search"),
            DataViewProcessAction::New => write!(f, "new"),
            DataViewProcessAction::Edit => write!(f, "edit"),
            DataViewProcessAction::View => write!(f, "view"),
        }
    }
}

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct DataViewProcessParams {
    primary_key: Option<Value>,
    filter :Option<Value>,
    filter_range :Option<Value>,
    filter_range_min :Option<Value>,
    filter_range_max :Option<Value>,
    aggregate :Option<Value>,
    sort: Option<HashMap<String, FieldSort>>,
    pagination: Option<Pagination>,
    overwrite: Option<Value>,
    select_out: Option<String>,
}

pub struct Service {
    schema_name : String,
    path: String,
    properties: IndexMap<String, ReferenceOr<Box<Schema>>>,
    short_description_list : Vec<String>,

    primary_keys : Vec<String>,
    list: Vec<Value>,
    list_str: Vec<String>,
}

impl Service {

	pub fn new(openapi: &OpenAPI, path: &str) -> Result<Self, Box<dyn std::error::Error>> {
		fn get_max_field_size(openapi: &OpenAPI, schema: &Schema, property_name: &str) -> usize {
			let field = openapi.get_property_from_schema(schema, property_name).unwrap();

            match &field.schema_kind {
                SchemaKind::Type(typ) => match  typ {
                    Type::String(typ) => match &typ.format {
                        VariantOrUnknownOrEmpty::Item(format) => match format {
                            StringFormat::Date => 30,
                            StringFormat::DateTime => 30,
                            StringFormat::Password => todo!(),
                            StringFormat::Byte => todo!(),
                            StringFormat::Binary => todo!(),
                        },
                        _ => {
                            if let Some(max_len) = typ.max_length {
                                max_len
                            } else {
                                100
                            }
                        },
                    },
                    Type::Number(_) => {
                        if let Some(precision) = field.schema_data.extensions.get("x-precision") {
                            precision.as_i64().unwrap() as usize
                        } else {
                            15
                        }
                    },
                    Type::Integer(_) => 9,
                    Type::Object(_) => todo!(),
                    Type::Array(_) => todo!(),
                    Type::Boolean {  } => 5,
                },
                _ => todo!(),
            }
		}

        let mut short_description_list = vec![];
        let schema = openapi.get_schema(path, "get", &SchemaPlace::Response, false)?;
        let extensions = &schema.schema_data.extensions;
        //self.foreignKeys = self.schema["x-foreignKeys"] || {};
        //self.primaryKeys = self.schema["x-primaryKeys"] || [];
        let primary_keys = if let Some(list) = extensions.get("x-primaryKeys") {
            if let Value::Array(list) = list {
                list.iter().map(|v| v.as_str().unwrap().to_string()).collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let unique_keys = if let Some(v) = extensions.get("x-uniqueKeys") {
            v.clone()
        } else {
            json!({})
        };

        let mut properties = openapi.get_properties_from_schema(schema).unwrap().clone();
        let num_properties = properties.len();
        let mut not_table_visible = vec![];

        for (field_name, field) in &mut properties {
            if let ReferenceOr::Item(field) = field {
                let extensions = &mut field.schema_data.extensions;
                if extensions.get("x-shortDescription").is_none() {extensions.insert("x-shortDescription".to_string(), json!(false));};
                if extensions.get("x-orderIndex").is_none() {extensions.insert("x-orderIndex".to_string(), json!(num_properties));};
    
                if let Some(hidden) = extensions.get("x-hidden") {
                    if let Value::Bool(hidden) = hidden {
                        if hidden == &true {
                            not_table_visible.push(field_name.clone());
                        }
                    }
                }
    
                if let Some(short_description) = extensions.get("x-shortDescription") {
                    if let Value::Bool(short_description) = short_description {
                        if short_description == &true {
                            short_description_list.push(field_name.clone());
                        }
                    }
                }
            }
        }
        // Se não foi definido manualmente o shortDescriptionList, monta em modo automático usando os uniqueMaps
        if short_description_list.len() == 0 {
            if unique_keys.as_object().unwrap().len() > 0 {
                for (field_name, field) in &mut properties {
                    if let ReferenceOr::Item(field) = field {
                        let extensions = &mut field.schema_data.extensions;
    
                        if extensions.get("x-hidden").is_none() && extensions.get("x-identityGeneration").is_some() {
                            extensions.insert("x-hidden".to_string(), json!(true));
                            not_table_visible.push(field_name.clone());
                        }
                    }
                }
            }

            let mut short_description_list_size = 0;
            
            if primary_keys.iter().find(|field_name| not_table_visible.contains(field_name)) == None {
                for field_name in &primary_keys {
                    short_description_list.push(field_name.clone());
                    short_description_list_size += get_max_field_size(&openapi, schema, field_name);
                }
            }

            for (_, list) in unique_keys.as_object().unwrap() {
                let list = list.as_array().unwrap().iter().map(|field_name| field_name.as_str().unwrap().to_string()).collect::<Vec<String>>();

                if list.iter().find(|field_name| not_table_visible.contains(field_name)) == None {
                    for field_name in &list {
                        if short_description_list.contains(field_name) == false {
                            short_description_list.push(field_name.clone());
                            short_description_list_size += get_max_field_size(&openapi, schema, field_name);
                        }
                    }

                    if short_description_list.len() > 3 || short_description_list_size > 30 {
                        break
                    }
                }
            }

            for (field_name, _) in &properties {
                if short_description_list.len() > 3 || short_description_list_size > 30 {
                    break
                }

                if not_table_visible.contains(field_name) == false && short_description_list.contains(field_name) == false {
                    short_description_list.push(field_name.clone());
                    short_description_list_size += get_max_field_size(&openapi, schema, field_name);
                }
            }
        }

        Ok(Self{
            path: path.to_string(), 
            schema_name: path[1..].to_string().to_case(convert_case::Case::Camel), 
            primary_keys, 
            short_description_list, 
            properties, 
            list: vec![], 
            list_str: vec![], 
        })
	}

	pub fn get_primary_key(&self, obj: &Value) -> Option<Value> {
		// private, projected for extract primaryKey and uniqueKeys
		fn copy_fields_from_list(data_in: &Value, field_names: &Vec<String>, retutn_null_if_any_empty: bool) -> Option<Value> {
			let mut ret = json!({});
	
			for field_name in field_names {
				if let Some(value) = data_in.get(field_name) {
					ret[field_name] = value.clone();
				} else {
					if retutn_null_if_any_empty == true {
						return None;
					}
				}
			}
	
			Some(ret)
		}
		
		copy_fields_from_list(obj, &self.primary_keys, true)
	}

    async fn query_remote(&self, server_connection: &ServerConnection, params :&Value) -> Result<(Vec<Value>, Vec<String>), Box<dyn std::error::Error>> {
        let access = server_connection.login_response.roles.iter().find(|role| role.path == self.path).unwrap().mask;

        if access & 1 != 0 {
            //console.log("[ServerConnection] loading", service.label, "...");
            //callback_partial("loading... " + service.label);
            let value = server_connection.http_rest.query(&self.path, params).await?;

            let list = match value {
                Value::Array(list) => list,
                Value::Null => todo!(),
                Value::Bool(_) => todo!(),
                Value::Number(_) => todo!(),
                Value::String(_) => todo!(),
                Value::Object(_) => todo!(),
            };
    
            let list_str = self.build_list_str(server_connection, &list)?;
            /*
            let dependents = server_connection.login_response.openapi.get_dependents(&self.name, false);
            let mut list_processed = vec![];
            // também atualiza a lista de nomes de todos os serviços que dependem deste
            for item in &dependents {
                if list_processed.contains(&item.schema) == false {
                    if let Some(service) = server_connection.services.get_mut(&item.schema) {
                        service.list_str = service.build_list_str(server_connection);
                        list_processed.push(item.schema.clone());
                    }
                }
            }
            */

            if list.len() != list_str.len() {
                println!("[DEBUG - query_remote - {} - list.len({}) != list_str.len({})]", self.path, list.len(), list_str.len());
            }

            return Ok((list, list_str));
        }

        Ok((vec![], vec![]))
    }

    //find<'a>(list: &'a Vec<Value>, filter: &'a Value) -> Vec<&'a Value>
	pub fn find<'a>(&'a self, params: &'a Value) -> Vec<&'a Value> {
        rufs_base_rust::data_store::Filter::find(&self.list, params).unwrap()
	}

	pub fn find_pos(&self, key: &Value) -> Option<usize> {
		rufs_base_rust::data_store::Filter::find_index(&self.list, key).unwrap()
	}

	pub fn find_one(&self, key: &Value) -> Option<&Value> {
		if let Some(pos) = self.find_pos(key) {
            self.list.get(pos)
        } else {
            None
        }
	}
	// private, use in get, save, update and remove
	pub fn update_list(&mut self, value: Value, pos: Option<usize>) -> usize {
        #[cfg(test)]
        if value.is_array() {
            for _value in &self.list {
                //println!("[DEBUG - {:?} - {:?}]", self.get_primary_key(value), value);
            }
        }

        let ret = if let Some(pos) = pos {
            self.list[pos] = value;

            if self.list.len() > self.list_str.len() + 1 {
                println!("[DEBUG - update_list - {} - 1 - rufs_service.list.len({}) != rufs_service.list_str.len({})]", self.path, self.list.len(), self.list_str.len());
            }
    
            pos
        } else {
			if let Some(key) = self.get_primary_key(&value) {
                if let Some(pos) = self.find_pos(&key) {
                    self.list[pos] = value;

                    if self.list.len() > self.list_str.len() + 1 {
                        println!("[DEBUG - update_list - {} - 2 - rufs_service.list.len({}) != rufs_service.list_str.len({})]", self.path, self.list.len(), self.list_str.len());
                    }
            
                    pos
                } else {
                    self.list.push(value);

                    if self.list.len() > self.list_str.len() + 1 {
                        #[cfg(test)]
                        for _value in &self.list {
                            //println!("[DEBUG - {:?} - {:?}]", self.get_primary_key(value), value);
                        }
                    }
            
                    self.list.len()-1
                }
            } else {
                self.list.push(value);

                if self.list.len() > self.list_str.len() + 1 {
                    println!("[DEBUG - update_list - {} - 4 - rufs_service.list.len({}) != rufs_service.list_str.len({})]", self.path, self.list.len(), self.list_str.len());
                }
        
                self.list.len()-1
            }
        };

        ret
    }

	fn build_field_str(server_connection: &ServerConnection, parent_name: &Option<String>, schema_name: &str, field_name: &str, obj: &Value)  -> Result<String, Box<dyn std::error::Error>> {
        fn build_field_reference(server_connection: &ServerConnection, schema_name: &str, field_name: &str, obj: &Value, _reference: &String) -> Result<String, Box<dyn std::error::Error>> {
            let item = server_connection.login_response.openapi.get_primary_key_foreign(schema_name, field_name, obj).unwrap().unwrap();

            if item.valid == false {
                return Ok("".to_string());
            }

            let service = server_connection.service_map.get(&item.schema).context(format!("Don't found service {}", item.schema))?;
            let primary_key = item.primary_key;
            let pos = service.find_pos(&primary_key).context(format!("Don't found item {} in service {}.\ncandidates:{:?}\n", primary_key, item.schema, service.list))?;
            let str = service.list_str[pos].clone();
            Ok(str)
        }

		let value = if let Some(value) = obj.get(field_name) {
			match value {
                //Value::Null => return,
                //Value::Bool(_) => todo!(),
                //Value::Number(_) => todo!(),
                Value::String(str) => if str.is_empty() {
                    return Ok("".to_string());
                },
                Value::Array(_array) => {
                    //println!("[build_field()] array = {:?}", array);
                    //todo!()
                    return Ok("".to_string());
                },
                Value::Object(_) => {
                    //string_buffer.push(value.to_string());
                    return Ok("".to_string());
                },
                _ => {},
            }

            value
		} else {
            return Ok("".to_string());
		};

        let properties = server_connection.login_response.openapi.get_properties_from_schema_name(parent_name, schema_name, &SchemaPlace::Schemas).context(format!("Missing properties in openapi schema {}", schema_name))?;
        let field = properties.get(field_name).context(format!("Don't found field {} in properties", field_name))?;

        match field {
            ReferenceOr::Reference { reference } => {
                return build_field_reference(server_connection, schema_name, field_name, obj, reference);
            },
            ReferenceOr::Item(field) => {
                let extensions = &field.schema_data.extensions;

                if let Some(reference) = extensions.get("x-$ref") {
                    if let Value::String(reference)  = reference {
                        return build_field_reference(server_connection, schema_name, field_name, obj, reference);
                    }
                }
            },
        }

        // TODO : verificar se o uso do "trim" não tem efeitos colaterais.
        let str = match value {
            Value::String(str) => str.trim().to_string(),
            Value::Null => "".to_string(),
            Value::Bool(value) => value.to_string(),
            Value::Number(value) => {
                if field_name == "id" && value.is_u64() {
                    format!("{:04}", value.as_u64().unwrap())
                } else {
                    value.to_string()
                }
            },
            Value::Array(_) => "".to_string(),
            Value::Object(_) => "".to_string(),
        };

        Ok(str)
    }
    // Instance section
    /*
	async fn request(&self, server_connection: &mut ServerConnection, path :&str, method :Method, params :&Value, obj_send :&Value) -> Result<Value, anyhow::Error> {
        server_connection.http_rest.request(&format!("{}/{}", self.path, path), method, params, obj_send).await
	}
*/
    fn build_item_str(&self, server_connection: &ServerConnection, item :&Value) -> Result<String, Box<dyn std::error::Error>> {
		let mut string_buffer = vec![];

		for field_name in &self.short_description_list {
			let str = Service::build_field_str(server_connection, &None, &self.schema_name, field_name, item)?;
            string_buffer.push(str);//trim
        }

		let str = string_buffer.join(" - ");
        Ok(str)
    }

    fn build_list_str(&self, server_connection: &ServerConnection, list: &Vec<Value>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    	let mut list_out = vec![];

        for item in list {
    		let str = self.build_item_str(server_connection, item)?;

			if let Some(pos) = list_out.iter().position(|s| s == &str) {
                println!("already str in list, position {}", pos);
                println!("item = {}", item);
                println!("item[{}] = {}", pos, list[pos]);
				self.build_item_str(server_connection, item)?;
                todo!()
			}

    		list_out.push(str);
    	}

        if self.list.len() != self.list_str.len() {
            println!("[DEBUG - build_list_str - {} - rufs_service.list.len({}) != rufs_service.list_str.len({})]", self.schema_name, self.list.len(), self.list_str.len());
        }

    	Ok(list_out)
    }

	fn remove_internal(&mut self, primary_key: &Value) -> Result<Option<usize>, Box<dyn std::error::Error>> {
        let index = self.find_pos(primary_key);

		// for listener in self.remote_listeners {
        //     listener.on_notify(schema_name, primary_key, "delete");
        // }

		//console.log("DataStore.removeInternal : pos = ", pos, ", data :", service.list[pos]);
        if let Some(index) = &index {
            //let value = &service.list[pos];
            //service.update_list(value, Some(pos));
	        //service.update_list_str(response);
            if *index >= self.list.len() {
                return Err(anyhow!(format!("[remove_internal({}, {})] index {} out of service.list.len {}", self.path, primary_key, index, self.list.len())))?;
            }

            if *index >= self.list_str.len() {
                return Err(anyhow!(format!("[remove_internal({}, {})] index {} out of service.list_str.len {}", self.path, primary_key, index, self.list_str.len())))?;
            }

            self.list.remove(*index);
            self.list_str.remove(*index);
        }

        Ok(index)
	}

}

#[derive(Serialize,Default,Debug)]
pub struct DataViewResponse {
    changes: Value,
    form_id: String,
    instance: String,
    html_search: String,
    table: String,
}

#[derive(PartialEq)]
pub enum DataViewType {
    Primary,
    ObjectProperty,
    Dependent,    
}

pub struct DataView {
    form_id: String, // parent_name + schema_name + edit
    typ: DataViewType,
    schema_name: String,
    parent_name: Option<String>,
    // schema
    path: Option<String>,
    short_description_list : Vec<String>,
    extensions: IndexMap<String, Value>,
    properties: IndexMap<String, ReferenceOr<Box<Schema>>>,
    properties_modified : IndexMap<String, Value>,

    //property_name: Option<String>,
    //method: String,
    //schema_place: SchemaPlace,
    action: DataViewProcessAction,
    //data_view_method_place : Vec<DataStoreMethodPlace>,
    //label :String,
    // data instance
    active_primary_key: Option<Value>, // active index of filter_results
    instance: Value,
    instance_flags: HashMap<String, Vec<bool>>,
    original: Value,
    // data list
    active_index: Option<usize>, // active index of filter_results
    filter_results: Vec<Value>,
    field_filter_results: IndexMap<String, Value>,
    field_results: IndexMap<String, Vec<Value>>,
    field_results_str: IndexMap<String, Vec<String>>,
    field_external_references_str: IndexMap<String, String>,
    //list: Vec<Value>,
    //list_str: Vec<String>,
    current_page: usize,
    page_size: usize,
    // data list aggregate
    instance_aggregate_range: Value,
    aggregate_results: Value,
    // data list filter
    instance_filter: Value,
    instance_filter_range: Value,
    instance_filter_range_min: Value,
    instance_filter_range_max: Value,
    // data list sort
    fields_sort: HashMap<String, FieldSort>,
    // ui
    fields_table: Vec<String>,
    childs: Vec<DataView>
}

impl DataView {

    pub fn form_id(schema_name: &str, _action: &str) -> String {
        let action = "new";
        format!("{}-{}", action.to_string(), schema_name.to_case(convert_case::Case::Camel))
    }

    pub fn form_id_with_parent(parent_name :&str, schema_name: &str, _action: &str) -> String {
        let action = "new";
        format!("{}-{}-{}", action, parent_name.to_case(convert_case::Case::Camel), schema_name.to_case(convert_case::Case::Camel))
    }

    pub fn form_id_with_regex(cap: &regex::Captures, parent_only: bool) -> Result<String, Box<dyn std::error::Error>> {
        // TODO
        let action = "new";//cap.name("action").context("context action")?.as_str();
        let name = cap.name("name").context("context name")?.as_str();

        if parent_only {
            if let Some(parent) = cap.name("parent") {
                Ok(format!("{}-{}", action, parent.as_str().to_case(convert_case::Case::Camel)))
            } else {
                Ok(format!("{}-{}", action, name.to_case(convert_case::Case::Camel)))
            }
        } else {
            if let Some(parent) = cap.name("parent") {
                Ok(format!("{}-{}", action, parent.as_str().to_case(convert_case::Case::Camel)))
            } else {
                Ok(format!("{}-{}", action, name.to_case(convert_case::Case::Camel)))
            }
        }
    }

	pub fn new(path_or_name: &str, typ :DataViewType, parent_name :Option<String>, action: DataViewProcessAction) -> Self {
        let (path, schema_name) = if path_or_name.starts_with("/") {
            (Some(path_or_name.to_string()), path_or_name[1..].to_string().to_case(convert_case::Case::Camel))
        } else {
            (None, path_or_name.to_string())
        };
        /*
        let access = server_connection.login_response.roles.iter().find(|role| role.path == path).unwrap().mask;
		let path_item_object = server_connection.login_response.openapi.paths.paths.get(path).unwrap().as_item().unwrap();

        let label = if let Some(summary) = &path_item_object.summary {
            summary.clone()
        } else {
            path.to_string().to_case(convert_case::Case::Title)
        };
        */

        let form_id = if let Some(parent_name) = &parent_name {
            DataView::form_id_with_parent(parent_name, &schema_name, &action.to_string())
        } else {
            DataView::form_id(&schema_name, &action.to_string())
        };

        Self{
            form_id,
            parent_name,
            path, 
            //property_name, 
            //access,
            action,
            //method: String::default(), 
            //schema_place: SchemaPlace::Schemas, 
            //data_view_method_place: vec![],
            schema_name, 
            //label,
            short_description_list: vec![],
            properties: IndexMap::default(), 
            properties_modified: IndexMap::default(),
            extensions: IndexMap::default(), 
            field_filter_results: IndexMap::default(),
            field_results: IndexMap::default(),
            field_results_str: IndexMap::default(),
            field_external_references_str: IndexMap::default(),
            //list: vec![], 
            //list_str: vec![], 
            active_index: None,
            filter_results: vec![], 
            current_page: 1, 
            page_size: 25, 
            active_primary_key: None,
            instance: json!({}), 
            instance_flags: HashMap::default(), 
            original: json!({}), 
            instance_aggregate_range: json!({}), 
            aggregate_results: json!({}), 
            instance_filter: json!({}), 
            instance_filter_range: json!({}), 
            instance_filter_range_min: json!({}), 
            instance_filter_range_max: json!({}), 
            fields_sort: HashMap::default(), 
            fields_table: vec![],
            childs: vec![],
            typ,
        }
	}

	pub fn set_schema(&mut self, server_connection: &ServerConnection) -> Result<(), Box<dyn std::error::Error>> {
        let Some(path) = &self.path else {
            return Ok(())
        };

        let (method, schema_place) = match self.action {
            DataViewProcessAction::New => ("post", SchemaPlace::Request),
            DataViewProcessAction::Edit => ("put", SchemaPlace::Request),
            _ => ("get", SchemaPlace::Response),
        };

        //self.method = method.to_string();
        //self.schema_place = schema_place;
        // TODO : adicionar o paraâmetro "property_name" no Rust.
        //self.schema = self.openapi.get_schema(self.path, method, schema_place);
        //self.properties = self.schema.properties || self.schema.items.properties;
        //let schema_name = openapi.get_schema_name(path, method, false)?;
        let /*mut*/ schema = server_connection.login_response.openapi.get_schema(path, method, &schema_place, false)?;

        self.properties = /*if let Some(property_name) = &self.property_name {
            schema = server_connection.login_response.openapi.get_property_from_schema(schema, property_name).unwrap();
            server_connection.login_response.openapi.get_properties_from_schema(schema).unwrap().clone()
        } else*/ {
            server_connection.login_response.openapi.get_properties_from_schema(schema).unwrap().clone()
        };

        self.short_description_list = server_connection.service_map.get(&self.schema_name).context("Missing service")?.short_description_list.clone();

        if let Some(property) = self.properties.get_mut("rufsGroupOwner") {
            match property {
                ReferenceOr::Item(property) => {
                    property.schema_data.extensions.insert("x-hidden".to_string(), Value::Bool(true));
                    property.schema_data.default = Some(Value::Number(Number::from(server_connection.login_response.rufs_group_owner)));
                },
                _ => todo!(),
            };
        }
        
        Ok(())
	}

	pub fn clear(&mut self) {
        self.original = json!({});
        self.instance = json!({});
        self.instance_flags.clear();
        self.field_external_references_str.clear();
        //self.set_values(json!({}), true, true); // set default values
    }

    fn build_instance(data_view_manager :&DataViewManager, data_view :&DataView, url_params :&DataViewProcessParams, action :&DataViewProcessAction) -> Result<String, Box<dyn std::error::Error>> {

        fn build_form_instance(data_view :&DataView, _url_params :&DataViewProcessParams) -> Result<String, Box<dyn std::error::Error>> {
            let form_id = &data_view.form_id;

            let mut hmtl_fields = vec![];
            //println!("[build_form_instance] DEBUG 1");
    
            for (field_name, field) in &data_view.properties {
                let field = field.as_item().context("field is reference")?;
                let extension = &field.schema_data.extensions;
                //println!("[build_form_instance] DEBUG 1.1");
                let hidden = extension.get("x-hidden").unwrap_or(&Value::Bool(false)).as_bool().unwrap_or(false);
                //println!("[build_form_instance] DEBUG 1.2");
    
                if hidden {
                    continue;
                }
    
                //println!("[build_form_instance] DEBUG 1.3");
                let typ = match &field.schema_kind {
                    SchemaKind::Type(typ) => typ,
                    SchemaKind::Any(_) => todo!(),
                    _ => continue
                };
    
                //println!("[build_form_instance] DEBUG 1.4");
                let (html_input_typ, html_input_step, html_input_pattern, html_input_max_length, col_size) = match typ {
                    Type::String(typ) => {
                        let max_length = typ.max_length.unwrap_or(1024);
    
                        let col_size = if max_length > 110 {
                            11
                        } else {
                            (max_length / 10) + 1
                        };
        
                        ("text", "", "", max_length, col_size)
                    },
                    Type::Number(_typ) => {
                        ("number", "", "", 15, 2)
                    },
                    Type::Integer(_typ) => {
                        if let Some(_reference) = extension.get("x-$ref") {
                            ("text", "", "", 1024, 8)
                        } else {
                            ("number", r#"step="1"#, r#"pattern="\d+""#, 15, 2)
                        }   
                    },
                    Type::Boolean {  } => ("checkbox", "", "", 0, 1),
                    Type::Object(_) => ("", "", "", 0, 0),
                    Type::Array(_) => ("", "", "", 0, 0),
                };
    
                //println!("[build_form_instance] DEBUG 1.5");
                let html_input = match typ {
                    Type::Object(_) => {
                        format!(r##"
    
                        "##)
                    },
                    Type::Array(_) =>  {
                        format!(r##"
                        
                        "##)
                    },
                    _ =>  {
                        let mut html_options = vec![];
    
                        if data_view.action != DataViewProcessAction::View {
                            if let Some(list) = data_view.field_results_str.get(field_name) {
                                for str in list {
                                    html_options.push(format!(r##"
                                    <option value="{str}">{str}</option>
                                    "##));
                                }
                            }
                        }
    
                        let html_options_str = html_options.join("\n");
    
                        if data_view.action != DataViewProcessAction::View && html_options.len() > 0 && html_options.len() <= 20 {
                            format!(r##"
                            <select class="form-control" id="instance-{form_id}-{field_name}" name="{field_name}" ng-required="field.essential == true && field.nullable != true" ng-disabled="{{field.readOnly == true}}">
                                <option value=""></option>
                                {html_options_str}
                            </select>
                            "##)
                        } else {
                            // ng-disabled="{{field.readOnly == true}}"
                            let disabled = if data_view.action == DataViewProcessAction::View {
                                "disabled"
                            } else {
                                ""
                            };

                            format!(r##"
                            <input class="form-control" id="instance-{form_id}-{field_name}" name="{field_name}" type="{html_input_typ}" {html_input_step} {html_input_pattern} maxlength="{html_input_max_length}" placeholder="{{field.placeholder}}" ng-required="field.essential == true && field.nullable != true" {disabled} list="list-{form_id}-{field_name}" autocomplete="off">
                            <datalist ng-if="field.filterResultsStr.length >  20" id="list-{form_id}-{field_name}">
                                {html_options_str}
                            </datalist>
                            "##)
                        }
                    },
                };
    
                //println!("[build_form_instance] DEBUG 1.6");
                let html_references = if let Some(_reference) = extension.get("x-$ref") {
                    //let reference = reference.as_str().context("not string content")?;
    
                    let mut list = vec![];
                    list.push(format!(r##"
                    <a id="reference-view-{form_id}-{field_name}" name="reference-view-{field_name}" class="btn btn-secondary" href="{{vm.goToField(fieldName, 'view', vm.instance, false)}}"><span class="glyphicon glyphicon-eye-open"></span></a>
                    "##));
    
                    if data_view.action != DataViewProcessAction::View {
                        list.push(format!(r##"
                        <a id="reference-create-{form_id}-{field_name}" name="reference-create-{field_name}" class="btn btn-secondary" href="{{vm.goToField(fieldName, 'new')}}"><span class="glyphicon glyphicon-plus"></span></a>
                        "##));
                        list.push(format!(r##"
                        <a id="reference-search-{form_id}-{field_name}" name="reference-search-{field_name}" class="btn btn-secondary" href ng-click="vm.goToField(fieldName, 'search', vm.instance, true)"><span class="glyphicon glyphicon-search"></span></a>
                        "##));
                    }
        
                    list.join("\n")
                } else {
                    "".to_string()
                };
    
                //println!("[build_form_instance] DEBUG 1.7");
                let html_flags = if let Some(flags) = extension.get("x-flags") {
                    let flags = flags.as_array().context(format!("Not array content in extension 'x-flags' of field {}, content : {}", field_name, flags))?;
                    let mut list = vec![];
                    let mut index = 0;
    
                    for label in flags {
                        let label = label.as_str().context("not string content")?;
    
                        list.push(format!(r##"
                        <div class="form-group form-group row">
                            <label class="col-offset-1 control-label">
                                <input type="checkbox" id="instance-{form_id}-{field_name}-{index}" name="{field_name}-{index}"/>
                                {label}
                            </label>
                        </div>
                        "##));
                        index += 1;
                    }
    
                    list.join("\n")
                } else {
                    "".to_string()
                };
    
                //println!("[build_form_instance] DEBUG 1.8");
                let label = field_name.to_case(convert_case::Case::Title);
                let str = format!(r##"
                <div class="col-{col_size}">
                    <label for="instance-{form_id}-{field_name}" class="form-label">{label}</label>
                    {html_input}
                    {html_references}
                    {html_flags}
                </div>
                "##);
                hmtl_fields.push(str);
                //println!("[build_form_instance] DEBUG 1.9");
            }
    
            //println!("[build_form_instance] DEBUG 2");
            let html_fields = hmtl_fields.join("\n");
            let label = data_view.schema_name.to_case(convert_case::Case::Title);
            let str = format!(r##"
                <form id="instance-{form_id}" name="instance-{form_id}" class="row">
                    <h5>{label}</h5>
                    {html_fields}
                    <div class="form-group">
                        <button id="instance-save-{form_id}"   name="save"   class="btn btn-primary"><span class="glyphicon glyphicon-save"  ></span> Save  </button>
                        <button id="instance-clear-{form_id}"  name="clear"  class="btn btn-default"><span class="glyphicon glyphicon-erase" ></span> Clear </button>
                        <button id="instance-cancel-{form_id}" name="cancel" class="btn btn-default"><span class="glyphicon glyphicon-remove"></span> Cancel</button>
                        <button id="instance-delete-{form_id}" name="delete" class="btn btn-default"><span class="glyphicon glyphicon-remove"></span> Remove</button>
                    </div>
                </form>
            "##);
            //println!("[build_form_instance] DEBUG 3");
            Ok(str)
        }
    
        fn build_crud_item(data_view :&DataView, url_params :&DataViewProcessParams) -> Result<String, Box<dyn std::error::Error>> {
            println!("[build_crud_item] DEBUG 1");
            let html_instance = build_form_instance(data_view, url_params)?;
            println!("[build_crud_item] DEBUG 2");

            let html_form_instance = if data_view.action != DataViewProcessAction::View {
                format!(r##"
                    {html_instance}
                "##)
            } else {
                "".to_string()
            };
            let html_page = "";
            let str = format!(r##"
                {html_form_instance}
                {html_page}            
            "##);
            println!("[build_crud_item] DEBUG 3");
            Ok(str)
        }

        let label = match action {
            DataViewProcessAction::New => "New",
            DataViewProcessAction::Edit => "Edit",
            DataViewProcessAction::View => "View",
            DataViewProcessAction::Search => "Filter",
        };

        println!("[build_instance] DEBUG 1");
        let mut crud_item_json = vec![];

        for data_view in &data_view.childs {
            println!("[build_instance] DEBUG 2.1");
            let form_instance = build_crud_item(data_view, url_params)?;
            let form_id = &data_view.form_id;
            let params_search = DataViewProcessParams{..Default::default()};
            println!("[build_instance] DEBUG 2.2");
            let table = DataView::build_page(data_view_manager, data_view, &params_search)?;
            crud_item_json.push(format!(r##"
            <div id="div-{form_id}">
                <form id="instance-{form_id}" name="instance-{form_id}" class="row">
                    {form_instance}
                </form>
                {table}
            </div>
            "##));
            println!("[build_instance] DEBUG 2.3");
        }

        println!("[build_instance] DEBUG 2");
        let html_instance = build_form_instance(data_view, url_params)?;
        println!("[build_instance] DEBUG 3");
        let html_crud_items = crud_item_json.join("\n");
        let ret = format!(r##"
            <h4>{label}</h4>
            {html_instance}
            {html_crud_items}
        "##);
        println!("[build_instance] DEBUG 9");
        Ok(ret)
    }

    fn build_changes(data_view_manager :&mut DataViewManager) -> Result<Value, Box<dyn std::error::Error>> {
        println!("[build_changes] 1");
        let mut forms = json!({});

        for (form_id, data_view) in &mut data_view_manager.data_view_map {
            println!("[build_changes] 2.1 {}", form_id);
            let mut form = json!({});

            for (field_name, value) in &data_view.properties_modified {
                println!("[build_changes] 2.2.1 {}.{} = {}", form_id, field_name, value);
                form[field_name] = json!(value);
            }

            forms[form_id] = form;
            data_view.properties_modified.clear();
            let mut i = 0;

            for data_view in &mut data_view.childs {
                println!("[build_changes] 2.3.1 {}.{} [{}]", form_id, data_view.schema_name, i);
                i += 1;
                let mut form_child = json!({});

                for (field_name, value) in &data_view.properties_modified {
                    println!("[build_changes] 2.3.2.1 {}.{}.{} = {}", form_id, data_view.schema_name, field_name, value);
                    form_child[field_name] = json!(value);
                }
    
                println!("[build_changes] 2.3.3 {}.{}", form_id, data_view.schema_name);
                forms[&data_view.form_id] = form_child;
                data_view.properties_modified.clear();
            }
        }

        Ok(forms)
    }

    fn build_search(data_view :&DataView, _params_search :&DataViewProcessParams) -> Result<String, Box<dyn std::error::Error>> {
        let form_id = &data_view.form_id;

        let href_new = if data_view.path.is_some() {
            DataView::build_location_hash(&data_view.form_id, "new", &json!({}))?
        } else {
            "".to_string()
        };

        let hidden_chart = "hidden";

        let ret = format!(r##"
        <form id="{form_id}" name="{form_id}" class="form-horizontal" role="form">
            <div class="form-group">
                <div class="col-md-offset-2 col-sm-2">
                    <a href="{href_new}" id="create-{form_id}" class="btn btn-primary"><span class="glyphicon glyphicon-plus"></span> New</a>
                </div>
            </div>
        </form>
        
        <div {hidden_chart}>
            <canvas id="aggregate-chart"></canvas>
        </div>
        
        <div class="panel panel-default" ng-if="vm.rufsService.list.length > 0 || vm.rufsService.access.get == true">
            <div class="panel-heading" role="tab">
                <ul class="nav nav-tabs">
                    <li ng-class="{{active: vm.activeTab == 1}}"><a id="btn-collapse-form-filter-{form_id}"    ng-click="vm.activeTab = vm.activeTab == 1 ? 0 : 1">Filtro</a></li>
                    <li ng-class="{{active: vm.activeTab == 2}}"><a id="btn-collapse-form-aggregate-{form_id}" ng-click="vm.activeTab = vm.activeTab == 2 ? 0 : 2">Relatório</a></li>
                    <li ng-class="{{active: vm.activeTab == 3}}"><a id="btn-collapse-form-sort-{form_id}"      ng-click="vm.activeTab = vm.activeTab == 3 ? 0 : 3">Ordenamento</a></li>
                </ul>
            </div>
        
            <div ng-show="vm.activeTab == 1" class="panel-body">
                <form id="search-filter-{form_id}" name="filter" class="form-horizontal" role="form">
                    <div class="form-group">
                        <div class="col-sm-offset-2 col-sm-10">
                            <button id="search-filter-apply-{form_id}" name="filter" class="btn btn-primary" ng-click="vm.clickFilter()"><span class="glyphicon glyphicon-search"></span> Aplicar</button>
                            <button id="search-filter-cancel-{form_id}" name="cancel" class="btn btn-default" ng-click="vm.clear_filter()"><span class="glyphicon glyphicon-remove"></span> Limpar</button>
                            <button id="search-filter-exit-{form_id}" name="exit" class="btn btn-default" ng-click="vm.activeTab = 0"><span class="glyphicon glyphicon-remove"></span> Sair</button>
                        </div>
                    </div>
        
                    <div ng-include="'./templates/crud-model_form_body_filter.html'"></div>
                </form>
            </div>
        
            <div ng-show="vm.activeTab == 2" class="panel-body">
                <form id="search-aggregate-{form_id}" name="aggregate" class="form-horizontal" role="form">
                    <div class="form-group">
                        <div class="col-sm-offset-2 col-sm-10">
                            <button id="search-aggregate-apply-{form_id}" name="aggregate" class="btn btn-primary" ng-click="vm.apply_aggregate()"><span class="glyphicon glyphicon-search"></span> Aplicar</button>
                            <button id="search-aggregate-cancel-{form_id}" name="cancel" class="btn btn-default" ng-click="vm.clear_aggregate()"><span class="glyphicon glyphicon-remove"></span> Limpar</button>
                            <button id="search-aggregate-exit-{form_id}" name="exit" class="btn btn-default" ng-click="vm.activeTab = 0"><span class="glyphicon glyphicon-exit"></span> Sair</button>
                        </div>
                    </div>
        
                    <div ng-include="'./templates/crud-model_form_body_aggregate.html'"></div>
                </form>
            </div>
        
            <div ng-show="vm.activeTab == 3" class="panel-body">
                <form id="search-sort-{form_id}" name="sort" class="form-horizontal" role="form">
                    <div class="form-group">
                        <div class="col-sm-offset-2 col-sm-10">
                            <button id="search-sort-apply-{form_id}" name="sort" class="btn btn-primary" ng-click="vm.apply_sort(null)"><span class="glyphicon glyphicon-search"></span> Aplicar</button>
                            <button id="search-sort-cancel-{form_id}" name="cancel" class="btn btn-default" ng-click="vm.clear_sort()"><span class="glyphicon glyphicon-remove"></span> Limpar</button>
                            <button id="search-sort-exit-{form_id}" name="exit" class="btn btn-default" ng-click="vm.activeTab = 0"><span class="glyphicon glyphicon-exit"></span> Sair</button>
                        </div>
                    </div>
        
                    <div ng-include="'./templates/crud-model_form_body_sort.html'"></div>
                </form>
            </div>
        </div>
        "##);
        Ok(ret)
    }

    fn build_page(data_view_manager :&DataViewManager, data_view :&DataView, params_search :&DataViewProcessParams) -> Result<String, Box<dyn std::error::Error>> {
        fn build_href(data_view_manager :&DataViewManager, data_view :&DataView, item: &Value, action: &str) -> Result<String, Box<dyn std::error::Error>> {
            let str = if data_view.path.is_some() {
                let service = data_view_manager.server_connection.service_map.get(&data_view.schema_name).context("Missing service")?;
                let primary_key = &service.get_primary_key(item).context(format!("Missing primary key"))?;
                DataView::build_location_hash(&data_view.form_id, action, primary_key)?
            } else {
                "".to_string()
            };

            Ok(str)
        }
        
        //println!("DEBUG  - build_page 1");
        let form_id = &data_view.form_id;
        //println!("DEBUG  - build_page 2 - {}", data_view.schema_name);

        let list = if data_view.path.is_none() || data_view.filter_results.len() > 0 {
            &data_view.filter_results
        } else {
            let service = data_view_manager.server_connection.service_map.get(&data_view.schema_name).context("Missing service in service_map")?;
            &service.list
        };
        //println!("DEBUG  - build_page 3 - {}", data_view.schema_name);

        if list.len() == 0 {
            println!("DEBUG  - build_page 3.1 - {}", data_view.schema_name);
            return Ok("".to_string());
        }

        //println!("DEBUG  - build_page 4 - {}", data_view.schema_name);
        let mut hmtl_header = vec![];

        for field_name in &data_view.short_description_list {
            let label = field_name.to_case(convert_case::Case::Title);
            let col = format!(r##"
            <th>
                <a href id="sort_left-{field_name}"><span class="glyphicon glyphicon-arrow-left"></span> </a>
                <a href id="sort_toggle-{field_name}"> {label}</a>
                <a href id="sort_rigth-{field_name}"><span class="glyphicon glyphicon-arrow-right"></span> </a>
            </th>
            "##);
            hmtl_header.push(col);
        }

        //println!("DEBUG  - build_page 5 - {}", data_view.schema_name);
        let mut offset_ini = (data_view.current_page-1) * data_view.page_size;
    
        if offset_ini > list.len() {
            offset_ini = list.len();
        }

        //println!("DEBUG  - build_page 6 - {}", data_view.schema_name);
        let mut offset_end = data_view.current_page * data_view.page_size;

        if offset_end > list.len() {
            offset_end = list.len();
        }

        if form_id == "new-rufsUser-roles" {
            println!("DEBUG  - build_page 7 - {}", data_view.schema_name);
        }

        let mut hmtl_rows = vec![];
        let mut item_index = 0;

        for index in offset_ini..offset_end {
            let item = list.get(index).context(format!("Broken: missing item at index"))?;
            let mut html_cols = vec![];

            for field_name in &data_view.short_description_list {
                let href_go_to_field = data_view.build_go_to_field(&data_view_manager.server_connection, field_name, "view", item, false)?;
                let href_go_to_field = href_go_to_field.unwrap_or("".to_string());

                let parent_name = if data_view.path.is_none() {
                    &data_view.parent_name
                } else {
                    &None
                };

                let field_str = Service::build_field_str(&data_view_manager.server_connection, parent_name, &data_view.schema_name, field_name, item)?;
                html_cols.push(format!(r#"<td><a id="table-row-col-{form_id}-{field_name}-{index}" href="{href_go_to_field}">{field_str}</a></td>"#));
            }

            let html_cols = html_cols.join("\n");

            let html_a_search_select = if let Some(select_out) = &params_search.select_out {
                format!(r#"<a href id="search_select-{form_id}-{select_out}-{item_index}"><span class="glyphicon glyphicon-ok"></span> Select</a>"#)
            } else {
                "".to_string()
            };

            let href_view = build_href(data_view_manager, data_view, item, "view")?;
            let href_edit = build_href(data_view_manager, data_view, item, "edit")?;
            let href_item_move = format!(r##"
            <a id="table-row-remove-{form_id}-{index}" ng-if="edit == true" href><span class="glyphicon glyphicon-trash">     </span> Delete</a>
            <a id="table-row-up-{form_id}-{index}"     ng-if="edit == true" href><span class="glyphicon glyphicon-arrow-up">  </span> Up</a>
            <a id="table-row-down-{form_id}-{index}"   ng-if="edit == true" href><span class="glyphicon glyphicon-arrow-down"></span> Down</a>
            "##);
            let row = format!(r##"
            <tr>
                <td>
                    <a id="table-row-view-{form_id}-{index}" href="{href_view}"><span class="glyphicon glyphicon-eye-open"></span> View</a>
                    <a id="table-row-edit-{form_id}-{index}" href="{href_edit}"><span class="glyphicon glyphicon-eye-open"></span> Edit</a>
                    {html_a_search_select}
                    {href_item_move}
                </td>
                {html_cols}
            </tr>
            "##);
            hmtl_rows.push(row);
            item_index += 1;
        }

        //println!("DEBUG  - build_page 8 - {}", data_view.schema_name);
        let html_page_control = if list.len() > data_view.page_size {
            let max_page = if list.len() % data_view.page_size == 0 {
                list.len() / data_view.page_size
            } else {
                (list.len() / data_view.page_size) + 1
            };

            let mut html_pages = vec![];

            for page in 1..max_page {
                html_pages.push(format!(r##"<li class="page-item"><a class="page-link" id="selected_page-{form_id}-{page}" href="#">{page}</a></li>"##));
            }

            let html_pages = html_pages.join("\n");
            let page_size = data_view.page_size;
            format!(r##"
            <nav aria-label="Page navigation">
                <ul class="pagination">
                    <li class="page-item">
                        <a class="page-link" href="#" aria-label="Previous">
                            <span aria-hidden="true">&laquo;</span>
                        </a>
                    </li>
                    {html_pages}
                    <li class="page-item">
                        <a class="page-link" href="#" aria-label="Next">
                            <span aria-hidden="true">&raquo;</span>
                        </a>
                    </li>
                </ul>
            </nav>

            <div class="form-group row" ng-if="vm.filterResults.length > vm.pageSize">
                <label for="page-size" class="col-2 col-form-label">Page size</label>

                <div class="col-2">
                    <input class="form-control" id="page_size-{form_id}" name="page_size" type="number" step="1" value="{page_size}">
                </div>
            </div>
            "##)

        } else {
            "".to_string()
        };

        let html_header = hmtl_header.join("\n");
        let html_rows = hmtl_rows.join("\n");
        let ret = format!(r##"
        <div id="div-{form_id}-table" class="table-responsive" style="white-space: nowrap;">
            <table id="table-{form_id}" class="table table-responsive table-bordered table-striped clearfix">
                <thead>
                    <tr>
                        <th></th>
                        {html_header}
                    </tr>
                </thead>
                <tbody>
                {html_rows}
                </tbody>
            </table>
        </div>
        {html_page_control}
        "##);
        println!("[build_page] 9 - {}", data_view.form_id);
        Ok(ret)
    }

    fn paginate(&mut self, page_size: Option<usize>, page: Option<usize>) -> Result<(), Box<dyn std::error::Error>> {
        self.page_size = page_size.unwrap_or(25);
        self.current_page = page.unwrap_or(1);
        //let result = self.filter_results.len().div_ceil(self.page_size);
        //self.numPages = (result == 0) ? 1 : result;
        Ok(())
    }
/*
	fn paginate(usize pageSize, usize page) {
		if pageSize == null {
			pageSize = CrudUiSkeleton.calcPageSize();
		}
		
		if pageSize < 10 {
			pageSize = 10;
		}
		
		super.paginate(pageSize, 1);
		//self.setPage(1);
	}
 */
    /*
    fn set_page_size(&mut self, page_size: usize) {
    	return self.paginate(Some(page_size), None);
    }
*/
/*
    fn set_page(&mut self, page: Option<usize>) {
    	return self.paginate(Some(self.page_size), page);
        //self.dataStoreManager.getDocuments(service, self.listPage);
    }
*/
/*
	fn is_valid(&self) -> bool {
		//let properties = self.properties || self.schema.properties;
		let mut ret = true;

		for (field_name, property) in &self.properties {
            if let ReferenceOr::Item(schema) = property {
                let extension = &schema.schema_data.extensions;
                let essential = extension.get("x-essential").unwrap_or(&Value::Bool(false)).as_bool().unwrap_or(false);
                let identity_generation = extension.get("x-identityGeneration");

                if essential == true && identity_generation.is_none() {
                    let value = self.instance.get(field_name);
    
                    if let Some(value) = value {
                        let nullable = extension.get("x-nullable").unwrap_or(&Value::Bool(false)).as_bool().unwrap_or(false);

                        if value == &Value::Null && nullable != true {
                            ret = false;
                            break;
                        }
                    } else {
                        ret = false;
                        break;
                    }
                }
            }
		}

		ret
	}
*/
/*
	fn is_changed(&self) -> bool {
		let mut ret = false;

		for (field_name, _) in &self.properties {
			if self.instance[field_name] != self.original[field_name] {
				ret = true;
				break;
			}
		}

		return ret;
	}
*/
    // Aggregate Section
    fn clear_aggregate(&mut self) {
        self.instance_aggregate_range = json!({});
        self.aggregate_results = json!({});
    }

	fn apply_aggregate(&mut self, _aggregate: &Value) {
        /*
		if aggregate.is_none()) aggregate = self.instance_aggregate_range; else self.instance_aggregate_range = aggregate;
		let dateRanges = ["secound", "minute", "hora", "dia", "mês", "ano"];
		
		let labelFromDate = (date, range) => {
			let type = dateRanges.indexOf(range);
			let str = "";
			if type <= 5) str = date.getFullYear() + " " + str;
			if type <= 4) str = date.getMonth()+1 + "/" + str;
			if type <= 3) str = date.getDate() + "/" + str;
			if type <= 2) str = date.getHours() + " " + str;
			return str;
		};
		
		self.aggregate_results = new Map();
		
		for item of self.filterResults {
			let label = "";
			
			for fieldName in aggregate {
				let value = item[fieldName];
				let range = aggregate[fieldName];
				let field = self.properties[fieldName];
				
				if range != false && range != "" && range != 0 {
					if field.$ref != undefined {
						label = label + self.buildFieldStr(fieldName, item) + ",";
					} else if field.flags != null {
						label = label + value.toString(16) + ",";
					} else if field.enum != undefined {
						let pos = field.filterResults.indexOf(JSON.stringify(value));
						label = label + field.filterResultsStr[pos] + ",";
					} else if field.htmlType == "number" {
						label = label + Math.trunc(value / range) * range + ",";
					} else if field.htmlType.includes("date") || field.htmlType.includes("time") {
						label = label + labelFromDate(value, range) + ",";
					}
				}
			}
			
			if label.length > 0 {
				if self.aggregate_results.has(label) == true {
					self.aggregate_results.set(label, self.aggregateResults.get(label) + 1);
				} else {
					self.aggregate_results.set(label, 1);
				}
			}
		}
        */
	}
// Filter section
    fn clear_filter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // hora corrente, hora anterior, uma hora, hoje, ontem, um dia, semana corrente, semana anterior, uma semana, quinzena corrente, quinzena anterior, 15 dias, mês corrente, mês anterior, 30 dias, ano corrente, ano anterior, 365 dias
        self.instance_filter = json!({});
        self.instance_filter_range = json!({});
        self.instance_filter_range_min = json!({});
        self.instance_filter_range_max = json!({});
        //self.filter_results = self.list.clone();
        //self.filter_results.clear();
        self.clear();
        Ok(())
    }

	fn set_filter_range(&mut self, _field_name: &str, _range: &str) {
        /*
		let period_labels = [" minuto ", " hora ", " dia ", " semana ", " quinzena ",    " mês ",     " ano "];
		let periods = [60.0,     3600.0,   86400.0,  7.0 * 86400.0,   15.0 * 86400.0, 30.0 * 86400.0, 365.0 * 86400.0];
		let mut period;
		
		for i in 0..period_labels.len() {
			if range.contains(period_labels[i]) {
				period = periods[i] * 1000.0;
				break;
			}
		}
		
		let now = js_sys::Date::new_0().value_of();
		let nowPeriodTrunc = js_sys::Math::trunc(now / period as f64) * period as f64; 
		let dateEnd = None;
		
		let dateIni = if range.contains(" corrente ") {
			js_sys::Date::new(&JsValue::from(nowPeriodTrunc))
		} else if range.contains(" anterior ") {
			dateEnd = Some(js_sys::Date::new(&JsValue::from(nowPeriodTrunc)));
			js_sys::Date::new(&JsValue::from(nowPeriodTrunc - period))
		} else {
			js_sys::Date::new(&JsValue::from(now - period))
		};
		
		let nowDate = new Date(); 
		let dayActiveStart = dateFns.startOfDay(nowDate);
		let dayLastStart = dateFns.startOfDay(nowDate);
		dayLastStart.setDate(dayLastStart.getDate()-1);
		let weekActiveStart = dateFns.startOfWeek(nowDate);
		let weekLastStart = new Date(weekActiveStart);
		weekLastStart.setDate(weekLastStart.getDate()-7);
		let monthActiveStart = dateFns.startOfMonth(nowDate);
		let monthLastStart = new Date(monthActiveStart);
		monthLastStart.setMonth(monthLastStart.getMonth()-1);
		let yearActiveStart = dateFns.startOfYear(nowDate);
		let yearLastStart = new Date(yearActiveStart);
		yearLastStart.setFullYear(yearLastStart.getFullYear()-1);
		
		if range.contains("dia corrente") {
			dateIni = dayActiveStart;
		} else if range.contains("dia anterior") {
			dateIni = dayLastStart;
			dateEnd = dayActiveStart;
		} else if range.contains("semana corrente") {
			dateIni = weekActiveStart;
		} else if range.contains("semana anterior") {
			dateIni = weekLastStart;
			dateEnd = weekActiveStart;
		} else if range.contains("quinzena corrente") {
			dateIni = nowDate.getDate() <= 15 ? monthActiveStart : new Date(monthActiveStart.setDate(15));
		} else if range.contains("quinzena anterior") {
			dateEnd = nowDate.getDate() <= 15 ? monthActiveStart : new Date(monthActiveStart.setDate(15));
			dateIni = new Date(dateEnd);
			if dateEnd.getDate() > 15 {dateIni.setDate(15);} else {dateIni.setDate(1);} 
		} else if range.contains("mês corrente") {
			dateIni = monthActiveStart;
		} else if range.contains("mês anterior") {
			dateIni = monthLastStart;
			dateEnd = monthActiveStart;
		} else if range.contains("ano corrente") {
			dateIni = yearActiveStart;
		} else if range.contains("ano anterior") {
			dateIni = yearLastStart;
			dateEnd = yearActiveStart;
		}
		
		self.instance_filter_range_min[fieldName] = dateIni;
		self.instance_filter_range_max[fieldName] = dateEnd;
        */
	}

	fn apply_filter(&mut self, _filter: &Option<Value>,_filter_range_minn: &Option<Value>, _filter_range_max: &Option<Value>) {
        /*
		if filter == Value::Null {filter = self.instance_filter;} else {self.instance_filter = filter;}
		if filter_range_min == Value::Null {filter_range_min = self.instance_filter_range_min;} else {self.instance_filter_range_min = filter_range_min;}
		if filter_range_max == Value::Null {filter_range_max = self.instance_filter_range_max;} else {self.instance_filter_range_max = filter_range_max;}
		//console.log(`DataStoreItem.applyFilter() :`, filter, filterRangeMin, filterRangeMax);

		let process_foreign = |fieldFilter, obj, fieldName, compareType| {
			let compareFunc = (candidate, expected, compareType) => {
				return Filter.matchObject(expected, candidate, (a,b,fieldName) => fieldName.is_none() ? (compareType == 0 ? a == b : (compareType < 0 ? a < b : a > b)) : false, false);
			}
			
			let item = self.dataStoreManager.getPrimaryKeyForeign(self.rufsService, fieldName, obj);
			let service = self.dataStoreManager.getSchema(item.schema);
			let primaryKey = item.primary_key;
			let candidate = service.findOne(primaryKey);
			let flag = compareFunc(candidate, fieldFilter.filter, 0);

			if flag == true {
				flag = compareFunc(candidate, fieldFilter.filterRangeMin, -1);

				if flag == true {
					flag = compareFunc(candidate, fieldFilter.filterRangeMax, 1);
				}
			}

			return flag;
		};

		const process = (expectedFields, expectedFieldsMin, expectedFieldsMax, list) => {
			const compareFunc = (candidate, expected, compareType) => {
				return Filter.matchObject(expected, candidate, (a,b,fieldName) => fieldName.is_none() ? (compareType == 0 ? a == b : (compareType < 0 ? a < b : a > b)) : processForeign(a,candidate,fieldName, compareType), true);
			}
			
			return list.filter(candidate => {
				let flag = compareFunc(candidate, expectedFields, 0);

				if flag == true {
					flag = compareFunc(candidate, expectedFieldsMin, -1);

					if flag == true {
						flag = compareFunc(candidate, expectedFieldsMax, 1);
					}
				}

				return flag;
			});
		}

		const getFilteredItems = (objFilter, objFilterMin, objFilterMax) => {
			var list = [];

			if objFilter != undefined && objFilter != null {
				list = process(objFilter, objFilterMin, objFilterMax, self.list);
			} else {
				list = self.list;
			}

			return list;
		}
	
		self.filterResults = getFilteredItems(filter, filterRangeMin, filterRangeMax);
		console.log(`[${constructor.name}.applyFilter()] self.filterResults = `, self.filterResults);
		self.paginate(null, null);
        */
	}
// Sort section
	// sortType, orderIndex, tableVisible
	fn apply_sort(&mut self, sort: &Option<HashMap<String, FieldSort>>) -> Result<(), Box<dyn std::error::Error>> {
		if let Some(sort) = sort {
			for (field_name, field) in &mut self.fields_sort {
				if let Some(sort) = sort.get(field_name) {
					field.sort_type = sort.sort_type.clone();
					field.order_index = sort.order_index.clone();
					field.table_visible = sort.table_visible.clone();
				}
			}
		}
		// format fieldsTable in correct order;
		{
			let mut entries: Vec<(&String, &FieldSort)> = self.fields_sort.iter().collect();
			entries.sort_by(|a, b| b.1.order_index.cmp(&a.1.order_index));
			self.fields_table = vec![];

			for (field_name, field) in entries {
                if field.hidden != true && field.table_visible != false {
                    self.fields_table.push(field_name.clone());
                }
            }
		}

        let fields_table = self.fields_table.clone();
        let fields_sort = self.fields_sort.clone();

		self.filter_results.sort_by(|a, b| {
			let mut ret = Ordering::Equal;
			
			for field_name in &fields_table {
				let field = fields_sort.get(field_name).unwrap();
				
				if field.sort_type != FieldSortType::None {
					let val_a = a.get(field_name);
					let val_b = b.get(field_name);
					
					if val_a != val_b {
						ret = if val_b.is_none() {
                            Ordering::Less
                        } else if val_a.is_none() {
                            Ordering::Greater
                        } else {
                            format!("{:0>9}", val_b.unwrap().to_string()).cmp(&format!("{:0>8}", val_a.unwrap().to_string()))
                        };

						if field.sort_type == FieldSortType::Desc {
                            ret = ret.reverse()
                        }

						if ret != Ordering::Equal {
                            break;
                        }
					}
				}
			}
			
			ret
		});

		Ok(())
	}

    fn clear_sort(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.fields_sort.clear();
        //let properties = self.schemaResponse != undefined ? self.schemaResponse.properties : self.properties;

        for (field_name, field) in &self.properties {
            if let ReferenceOr::Item(schema) = field {
                let extension = &schema.schema_data.extensions;
                let table_visible = extension.get("x-tableVisible").unwrap_or(&Value::Bool(false)).as_bool().unwrap_or(false);
                let hidden = extension.get("x-hidden").unwrap_or(&Value::Bool(false)).as_bool().unwrap_or(false);
                let order_index = extension.get("x-orderIndex").unwrap_or(&Value::from(0)).as_i64().unwrap_or(0);
                self.fields_sort.insert(field_name.clone(), FieldSort{ sort_type: FieldSortType::None, order_index, table_visible, hidden });
            }
        }

        self.apply_sort(&None)
    }
/*
    fn sort_toggle(&mut self, field_name: &str) {
        let mut field = self.fields_sort[field_name].clone();

        field.sort_type = if field.sort_type == FieldSortType::Asc {
            FieldSortType::Desc
        } else {
            FieldSortType::Asc
        };

        self.fields_sort.insert(field_name.to_string(), field);
        self.apply_sort(&None);
    }

    fn sort_left(&mut self, field_name: &str) {
        let mut field = self.fields_sort[field_name].clone();
        field.order_index -= 1;
        self.fields_sort.insert(field_name.to_string(), field);
        self.apply_sort(&None);
    }

    fn sort_rigth(&mut self, field_name: &str) {
        let mut field = self.fields_sort[field_name].clone();
        field.order_index += 1;
        self.fields_sort.insert(field_name.to_string(), field);
        self.apply_sort(&None);
    }
*/
/*
	fn update_fields(&self) {
/*
		for fieldName in self.properties {
			let field = self.properties[fieldName];
			field.filter = json!({});
			field.htmlType = "text";
			field.htmlStep = "any";

			if field.type == "boolean" {
				field.htmlType = "checkbox";
			} else if field.type == "integer" {
				field.htmlType = "number";
				field.htmlStep = "1";
			} else if field.type == "number" {
				field.htmlType = "number";
				
				if field.precision == 1 {
					field.htmlStep = "0.1";
				} else if field.precision == 2 {
					field.htmlStep = "0.01";
				} else {
					field.htmlStep = "0.001";
				}
			} else if field.type == "date" {
				field.htmlType = "date";
			} else if field.type == "time" {
				field.htmlType = "time";
			} else if field.format == "date-time" {
				field.htmlType = "datetime-local";
			}

			if field.enum == undefined && field.enumLabels == undefined && field.type == "string" && field.maxLength == 1 && (field.default == "S" || field.default == "N" {
				field.filterResults = field.enum = ["S", "N"];
				field.filterResultsStr = field.enumLabels = ["Sim", "Não"];
			}

			if field.htmlType == "number" || field.htmlType.includes("date") || field.htmlType.includes("time" {
				field.htmlTypeIsRangeable = true;
			} else {
				field.htmlTypeIsRangeable = false;
			}
			
			if field.label == undefined {
				if field.description != undefined && field.description.length <= 30 {
					field.label = field.description;
				} else {
					let label = self.dataStoreManager.convertCaseAnyToLabel(fieldName);
					field.label = label;
				}
			}

			if field.flags != null && Array.isArray(field.flags) == false {
				field.flags = field.flags.split(",");
				field.htmlTypeIsRangeable = false;
			}

			if field.enum != undefined {
				if Array.isArray(field.enum) == false) field.enum = field.enum.split(",");
				field.htmlTypeIsRangeable = false;
			}

			if field.enumLabels != undefined {
				if Array.isArray(field.enumLabels) == false) field.enumLabels = field.enumLabels.split(",");
				field.htmlTypeIsRangeable = false;
			}
			
			if field.$ref != undefined {
				field.htmlTypeIsRangeable = false;
			}
		}
*/
    }
*/
/*
	fn clear_form(&mut self, server_connection: &ServerConnection) {
		// self.serverConnection.selectOut = {};
		// self.serverConnection.useHistoryState = false;
		return self.clear();
	}
*/
    fn set_value(&mut self, child_name: Option<&str>, server_connection: &ServerConnection, watcher: &impl DataViewWatch, field_name: &str, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
        fn get_value_old_or_default_or_null(field: &Schema, value_old: &Value) -> Value {
            let value_default = if let Some(default) = &field.schema_data.default {
                println!("get_default_value : {}", default);

                match &field.schema_kind {
                    SchemaKind::Type(typ) => match typ {
                        Type::String(typ) => match &typ.format {
                            VariantOrUnknownOrEmpty::Item(item) => match item {
                                StringFormat::Date => json!(Utc::now().to_rfc3339()),
                                StringFormat::DateTime => json!(Utc::now().to_rfc3339()),
                                _ => default.clone(),
                            },
                            _ => default.clone(),
                        },
                        _ => default.clone(),
                    },
                    _ => todo!(),
                }
            } else {
                Value::Null
            };

            if value_default.is_null() == false && value_old.is_null() == false {
                value_old.clone()
            } else {
                value_default
            }
        }

        fn u64_to_flags(value_in: u64) -> Vec<bool> {
            let mut flags = vec![];

            for k in 0..64 {
                let bit = 1 << k;
                let value = value_in & bit;
                flags.push(value != 0);
            }

            flags
        }

        fn set_value_process(data_view: &mut DataView, server_connection: &ServerConnection, field_name: &str, value: &Value) -> Result<(Value, Value, Value), Box<dyn std::error::Error>> {
            //data_view.field_external_references_str.remove(field_name);
            let value_old = data_view.instance.get(field_name).unwrap_or(&Value::Null);
    
            let field = match data_view.properties.get_mut(field_name).context("set_value_process 1.0 : context")? {
                ReferenceOr::Reference { reference: _ } => todo!(),
                ReferenceOr::Item(schema) => schema.as_mut(),
            };

            println!("set_value_process 1 : {}.{}.{} = {} (old {}) (nullable {}) (action {})", "?", data_view.schema_name, field_name, value, value_old, field.schema_data.nullable, data_view.action);

            let value = if value.is_null() {
                let value = get_value_old_or_default_or_null(field, value_old);

                if value.is_null() {
                    let force_enable_null = if data_view.action == DataViewProcessAction::New {
                        true
                    } else {
                        false
                    };
            
                    // set_value_process 1 : ?."requestPayment".id
                    if data_view.schema_name == "requestPayment" && field_name == "id" {
                        println!("set_value_process 1 : {}.{}.{} = {} (old {}) (nullable {}) (force_enable_null {})", "?", data_view.schema_name, field_name, value, value_old, field.schema_data.nullable, force_enable_null);
                    }

                    if force_enable_null || field.schema_data.nullable {
                        value
                    } else {
                        return None.context(format!("set_value_process 2 : received value null in {}.{}, force_enable_null = {}, field.schema_data.nullable = {}, data_view.action = {}", data_view.form_id, field_name, force_enable_null, field.schema_data.nullable, data_view.action))?;
                    }
                } else {
                    value
                }                
            } else {
                value.clone()
            };
    
            let schema_data = &mut field.schema_data;
            let extensions = &mut schema_data.extensions;
    
            println!("set_value_process 2 : {}.{:?}.{} = {} ({})", "?", data_view.schema_name, field_name, value, value_old);

            if extensions.contains_key("x-$ref") {
                if value.is_null() {
                    data_view.field_external_references_str.insert(field_name.to_string(), "".to_string());
                } else {
                    let service = server_connection.service_map.get(&data_view.schema_name).unwrap();
                    let mut obj = data_view.instance.clone();
                    obj[field_name] = value.clone();
                    let external_references_str = Service::build_field_str(server_connection, &None, &service.schema_name, field_name, &obj)?;
                    data_view.field_external_references_str.insert(field_name.to_string(), external_references_str.clone());
                }
            } else if extensions.contains_key("x-flags") && value.is_u64() {
                // field.flags : String[], vm.instanceFlags[fieldName] : Boolean[]
                data_view.instance_flags.insert(field_name.to_string(), u64_to_flags(value.as_u64().unwrap_or(0)));
            } else if extensions.contains_key("x-enum") {
                let empty_list = &Vec::<String>::new();
                let field_results_str = data_view.field_results_str.get(field_name).unwrap_or(empty_list);

                if value.is_object() {
                    let str_value = value.to_string();

                    if let Some(pos) = field_results_str.iter().position(|s| s == &str_value) {
                        //extensions.insert("x-externalReferencesStr".to_string(), json!(field_results_str[pos].clone()));
                        data_view.field_external_references_str.insert(field_name.to_string(), field_results_str[pos].clone());
                    } else {
                        //console.error(`${self.constructor.name}.setValue(${fieldName}) : don\'t found\nvalue:`, value, `\nstr:\n`, field.externalReferences, `\noptions:\n`, field.filterResultsStr);
                    }
                } else if value.is_null() {
                    data_view.field_external_references_str.insert(field_name.to_string(), "".to_string());
                } else {
                    if let Some(pos) = data_view.filter_results.iter().position(|v| v == &value) {
                        //extensions.insert("x-externalReferencesStr".to_string(), json!(field_results_str[pos].clone()));
                        data_view.field_external_references_str.insert(field_name.to_string(), field_results_str[pos].clone());
                    } else {
                        //console.error(`${self.constructor.name}.setValue(${fieldName}) : don\'t found\nvalue:`, value, `\nstr:\n`, field.externalReferences, `\noptions:\n`, field.filterResultsStr);
                    }
                }
            }

            let hidden = extensions.contains_key("x-hidden");
            //console.log(`[${self.constructor.name}.setValues().setValue()] openapi.copy_value("${self.path}", "${self.method}", SchemaPlace::${self.schema_place}, "${fieldName}", ${JSON.stringify(value)})`);
            let value = if !value.is_null() {
                //server_connection.login_response.openapi.copy_value(&data_view.path, &data_view.method, &data_view.schema_place, false /*true*/, field_name, &value)?//value || {}
                server_connection.login_response.openapi.copy_value_field(field, true, &value)?
            } else {
                value
            };

            println!("set_value_process 3 : {}.{:?}.{} = {} ({})", "?", data_view.schema_name, field_name, value, value_old);

            let value_view = if hidden {
                Value::Null
            } else if let Some(value) = data_view.field_external_references_str.get(field_name) {
                json!(value)
            } else {
                value.clone()
            };

            println!("set_value_process 4 : {}.{:?}.{} = {} ({}), view = {}", "?", data_view.schema_name, field_name, value, value_old, value_view);
            Ok((value_old.clone(), value, value_view))
        }

        let (value_old, field_value, field_value_str) = if let Some(child_name) = child_name {
            let data_view = self.childs.iter_mut().find(|item| item.schema_name == child_name).context(format!("set_value 1 : Missing item {} in data_view {}", child_name, self.schema_name))?;
            set_value_process(data_view, server_connection, field_name, value)?
        } else {
            set_value_process(self, server_connection, field_name, value)?
        };

        //if field_value.is_null() == false {
            if value_old != field_value && watcher.check_set_value(self, child_name, server_connection, field_name, &field_value)? == true {
                fn set_value_show(data_view :&mut DataView, field_name :&str, field_value_str :Value) -> Result<(), Box<dyn std::error::Error>> {
                    let field = data_view.properties.get(field_name).context(format!("Missing field {} in data_view {}", field_name, data_view.schema_name))?;
                    let schema = field.as_item().context(format!("field {} in data_view {} is reference", field_name, data_view.schema_name))?;
                    let extension = &schema.schema_data.extensions;
                    let hidden = extension.get("x-hidden").unwrap_or(&Value::Bool(false)).as_bool().unwrap_or(false);
                                
                    if hidden == false /*&&(data_view.properties_modified.contains_key(field_name) || field_value_str.is_null() == false)*/ {
                        data_view.properties_modified.insert(field_name.to_string(), field_value_str);
                    }

                    Ok(())
                }

                println!("set_value : {}.{:?}.{} = {}", self.schema_name, child_name, field_name, field_value);

                if self.schema_name == "rufsUser" && field_name == "roles" && field_value.is_array() {
                    println!("DEBUG set_value : {}.{:?}.{} = {}", self.schema_name, child_name, field_name, field_value);
                }

                if let Some(child_name) = child_name {
                    let data_view = self.childs.iter_mut().find(|item| item.schema_name == child_name).context(format!("set_value 2 : Missing item {} in data_view {}", child_name, self.schema_name))?;

                    match &field_value {
                        Value::Array(array) => {
                            data_view.filter_results = array.clone();
                        },
                        Value::Object(_obj) => todo!(),
                        _ => data_view.instance[field_name] = field_value.clone(),
                    }

                    set_value_show(data_view, field_name, field_value_str)?;
                } else {                    
                    match &field_value {
                        Value::Array(array) => {
                            let data_view = self.childs.iter_mut().find(|item| item.schema_name == field_name).context(format!("set_value 3 : Missing item {} in data_view {}", field_name, self.schema_name))?;
                            data_view.filter_results = array.clone();
                        },
                        Value::Object(_obj) => todo!(),
                        _ => {},
                    }

                    self.instance[field_name] = field_value.clone();
                    set_value_show(self, field_name, field_value_str)?;
                }
            }
        //}

        Ok(())
    }

    fn set_values(&mut self, server_connection: &ServerConnection, watcher: &impl DataViewWatch, obj: &Value) -> Result<(), Box<dyn std::error::Error>> {
        fn set_values_process(data_view: &mut DataView, child_name: Option<&str>, server_connection: &ServerConnection, watcher: &impl DataViewWatch, obj: &Value) -> Result<(), Box<dyn std::error::Error>> {
            let keys = if let Some(child_name) = child_name {
                let data_view = data_view.childs.iter_mut().find(|item| item.schema_name == child_name).context(format!("set_values 1 : Missing item {} in data_view {}", child_name, data_view.schema_name))?;
                data_view.properties.iter().map(|item| item.0.to_string()).collect::<Vec<String>>()
            } else {
                data_view.properties.iter().map(|item| item.0.to_string()).collect::<Vec<String>>()
            };

            for field_name in &keys {
                let value = obj/*_out*/.get(field_name).unwrap_or(&Value::Null);
                println!("set_values_process : {}.{:?}.{} = {}", data_view.schema_name, child_name, field_name, value);
                data_view.set_value(child_name, server_connection, watcher, field_name, value)?;
            }

            Ok(())
        }
        // const list = Object.entries(data_view.properties);
        // let filter = list.filter(([fieldName, field]) => field.hidden != true && field.readOnly != true && field.essential == true && field.type != "object" && field.type != "array" && data_view.instance[fieldName] == undefined);
        // if filter.length == 0) filter = list.filter(([fieldName, field]) => field.hidden != true && field.readOnly != true && field.essential == true && field.type != "object" && field.type != "array");
        // if filter.length == 0) filter = list.filter(([fieldName, field]) => field.hidden != true && field.readOnly != true && field.essential == true);
        // if filter.length == 0) filter = list.filter(([fieldName, field]) => field.hidden != true && field.readOnly != true);
        // if filter.length == 0) filter = list.filter(([fieldName, field]) => field.hidden != true);
        //self.get_document(self, obj, false);
        let obj = &server_connection.login_response.openapi.copy_fields_using_properties(&self.properties, &self.extensions, false /*true*/, obj, true, true, false)?;//value || {}
        set_values_process(self, None, server_connection, watcher, obj)?;

        for data_view in &mut self.childs {
            if data_view.typ == DataViewType::ObjectProperty {
                if let Some(obj) = obj.get(&data_view.schema_name) {
                    data_view.set_values(server_connection, watcher, obj)?;
                }
            }
        }

        Ok(())
    }

    pub async fn save(&self, server_connection: &mut ServerConnection) -> Result<Value, Box<dyn std::error::Error>> {
        println!("[DataViewManager] {}.save 1", self.schema_name);

        if self.action == DataViewProcessAction::New {
            println!("[DataViewManager] {}.save new 1.1 : {}", self.schema_name, self.instance);
            server_connection.save(&self.schema_name, &self.instance).await
        } else {
            println!("[DataViewManager] {}.save update 1.2 : {}", self.schema_name, self.instance);
            println!("[DataViewManager] {}.save update 1.2 : {}", self.schema_name, self.instance);
            server_connection.update(&self.schema_name, &self.instance).await
        }
    }

    fn build_location_hash(form_id: &str, action: &str, params: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let query_string = serde_qs::to_string(params).unwrap();
        let re = regex::Regex::new(r"(?P<action>\w+)-((?P<parent>\w+)-)?(?P<name>\w+)$")?;

        if let Some(cap) = re.captures(form_id) {
            let name = cap.name("name").context("context")?.as_str().to_case(convert_case::Case::Snake);

            if let Some(parent) = cap.name("parent") {
                Ok(format!("#!/app/{}-{}/{}?{}", parent.as_str().to_case(convert_case::Case::Snake), name, action, query_string))
            } else {
                Ok(format!("#!/app/{}/{}?{}", name, action, query_string))
            }
        } else {
            Ok(format!("#!/app/{}/{}?{}", form_id, action, query_string))
        }
    }

    fn build_go_to_field(&self, server_connection: &ServerConnection, field_name: &str, action: &str, obj: &Value, is_go_now: bool) -> Result<Option<String>, Box<dyn std::error::Error>> {
        fn super_go_to_field(data_view: &DataView, server_connection: &ServerConnection, field_name: &str, action: &str, obj: &Value, is_go_now: bool) -> Result<Option<String>, Box<dyn std::error::Error>> {
            let field = data_view.properties.get(field_name).context("Missing field in properties")?;
            let field = field.as_item().context("field is reference")?;
            let extensions = &field.schema_data.extensions;
    
            let Some(reference) = extensions.get("$ref") else {
                let Some(value) = obj.get(field_name) else {
                    return Ok(None);
                };

                let Some(value) = value.as_str() else {
                    return Ok(None);
                };

                if value.starts_with("#") {
                    return Ok(Some(value.to_string()));
                } else {
                    return Ok(None);
                }
            };

            let schema_name = reference.as_str().unwrap();
            let item = server_connection.login_response.openapi.get_primary_key_foreign(schema_name, field_name, obj)?.context("Missing primary_key_foreign")?;
            let service_name = &item.schema;
            let mut query_obj = json!({});
    
            if action == "search" && is_go_now == true {
                query_obj["selectOut"] = json!(field_name);
                let filter = json!({});
    /*
                if item.is_unique_key == false {
                    for (field_name, value) in item.primary_key {
                        if value.is_null() == false {
                            filter[field_name] = value;
                        }
                    }
                }
     */
                query_obj["filter"] = filter;
                //server_connection.useHistoryState = true;
                //window.history.replaceState(this.instance, "Edited values");
            } else {
                query_obj = item.primary_key;
            }
    
            let url = DataView::build_location_hash(&service_name.to_case(convert_case::Case::Snake), action, &query_obj)?;
            Ok(Some(url))
        }
            
        let url = super_go_to_field(self, server_connection, field_name, action, obj, is_go_now)?;

        let url = if let Some(url) = url {
            Ok(Some(url))
        } else {
            if self.path.is_some() {
                let service = server_connection.service_map.get(&self.schema_name).context("Missing service")?;
                let primary_key = &service.get_primary_key(obj).context(format!("Missing primary key"))?;
                Ok(Some(DataView::build_location_hash(&self.form_id, action, primary_key)?))
            } else {
                Ok(None)
            }
        };

        return url;
    }                        
        
}

trait RemoteListener {
    fn on_notify(schema_name: &str, primary_key: &Value, action: &str);
}

trait CallbackPartial {
    
}

#[derive(Default,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponseClient {
    //token_payload : TokenPayload,
    //user_proteced: RufsUserProteced,
    pub id: u64,
    pub name: String,
    pub rufs_group_owner: u64,
    pub groups: Vec<u64>,
    pub roles: Vec<Role>,
    pub ip: String,
    //user_public: RufsUserPublic,
    //routes: Vec<Route>,
    //menu: Vec<MenuItem>,
    pub path: String,
    pub jwt_header: String,
    pub title: String,
    pub openapi: OpenAPI,
}

#[derive(Default)]
pub struct ServerConnection {
    http_rest :HttpRestRequest,
    login_response :LoginResponseClient,
    service_map: HashMap<String, Service>,
    //pathname: String,
    //remote_listeners: Vec<dyn RemoteListener>,
    //web_socket :Option<WebSocket>,
}

impl ServerConnection {
    pub fn new(server_url: &str) -> Self {
        Self {http_rest: HttpRestRequest::new(server_url), ..Default::default()}
    }

    // ignoreCache is used in websocket notifications
    async fn get(&mut self, schema_name: &str, primary_key: &Value) -> Result<&Value, Box<dyn std::error::Error>> {
        let service = self.service_map.get_mut(schema_name).context(format!("Missing service {} in service_map", schema_name))?;
        let pos = service.find_pos(primary_key);

        let pos = if let Some(pos) = pos {
            pos
        } else {
            let data = self.http_rest.get(&service.path, primary_key).await?;

            if data.is_array() {
                return Err(format!("Missing parameter {} in query string {}.", "primary_key", ""))?;
            }

            service.update_list(data, None)
        };

        let ret = service.list.get(pos).unwrap();
        Ok(ret)
    }
/*
	async fn query_remote(&mut self, server_connection: &ServerConnection, schema_name: &str, params :&Value) -> Result<(), Box<std::error::Error>> {
        if let Some(data_view) = self.service_map.get_mut(schema_name) {
            data_view.query_remote(server_connection, self, params).await?;
        }

        Ok(())
	}
*/
    fn update_list_str(&mut self, schema_name: &str, data :&Value, old_pos :Option<usize>, new_pos :usize) -> Result<(), Box<dyn std::error::Error>> {
        fn assert_exists(list :&Vec<String>, str: &str, _old_pos :Option<usize>, new_pos :usize) -> Result<(), anyhow::Error> {
            let pos = list.iter().position(|s| s == str);

            if let Some(pos) = pos {
                if pos != new_pos {
                    println!("[DEBUG] assert_exists(str: {}, old_pos: {:?}, new_pos: {})", str, _old_pos, new_pos);
                    todo!()
                }
            }

            Ok(())
        }

        let data_view = self.service_map.get(schema_name).unwrap();
        let str = data_view.build_item_str(self, data)?;
        let data_view = self.service_map.get_mut(schema_name).unwrap();

        if let Some(old_pos) = old_pos {
            if new_pos == old_pos {
                // replace
                assert_exists(&data_view.list_str, &str, Some(old_pos), new_pos)?;
                data_view.list_str[new_pos] = str;
            } else {
                // remove and add
                data_view.list_str.remove(old_pos);
                assert_exists(&data_view.list_str, &str, Some(old_pos), new_pos)?;
                data_view.list_str[new_pos] = str;
            }
        } else {
            assert_exists(&data_view.list_str, &str, None, new_pos)?;
            data_view.list_str.push(str);
        }

        Ok(())
    }

    async fn save(&mut self, schema_name: &str, item_send :&Value) -> Result<Value, Box<dyn std::error::Error>> {
        let service = self.service_map.get_mut(schema_name).context(format!("[ServerConnection.save({})] missing service {}", schema_name, schema_name))?;
        let schema_place = SchemaPlace::Request;//data_view.schema_place
        let method = "post";//data_view.method
		let data_out = self.login_response.openapi.copy_fields(&service.path, method, &schema_place, false, item_send, false, false, false)?;
        let data = self.http_rest.save(&service.path, &data_out).await?;
        let new_pos = service.update_list(data.clone(), None);
        self.update_list_str(schema_name, &data, None, new_pos)?;
        let service = self.service_map.get(schema_name).unwrap();

        if service.list.len() != service.list_str.len() {
            println!("[DEBUG - {} - service.list.len({}) != service.list_str.len({})]", service.schema_name, service.list.len(), service.list_str.len());
        }

        Ok(data)
    }

    async fn update(&mut self, schema_name: &str, item_send :&Value) -> Result<Value, Box<dyn std::error::Error>> {
        let service = self.service_map.get_mut(schema_name).unwrap();
        let schema_place = SchemaPlace::Request;//data_view.schema_place
        let method = "put";//data_view.method
		let data_out = self.login_response.openapi.copy_fields(&service.path, method, &schema_place, false, item_send, false, false, false)?;
        let primary_key = &service.get_primary_key(&data_out).context(format!("Missing primary key"))?;
        let data = self.http_rest.update(&service.path, primary_key, &data_out).await?;
        let old_pos = service.find_pos(primary_key);
        let new_pos = service.update_list(data.clone(), old_pos);
        self.update_list_str(schema_name, &data, old_pos, new_pos)?;
        let service = self.service_map.get(schema_name).unwrap();

        if service.list.len() != service.list_str.len() {
            println!("[DEBUG - {} - service.list.len({}) != service.list_str.len({})]", service.schema_name, service.list.len(), service.list_str.len());
        }

        Ok(data)
    }

    async fn remove(&mut self, schema_name: &str, primary_key: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let service = self.service_map.get_mut(schema_name).context(format!("Missing service {} in service_map", schema_name))?;
        let old_value = self.http_rest.remove(&service.path, primary_key).await?;
        //#[cfg(test)]
        service.remove_internal(primary_key)?;
        //.then(data => self.serverConnection.remove_internal(self.name, primaryKey))
    	//.then(response => self.updateListStr(response));
        Ok(old_value)
	}
/*
	async fn patch(&self, item_send :&Value) -> Value {
    	let data = self.http_rest.patch(self.path, self.openapi.copy_fields(self.path, self.method, self.schema_place, item_send)).await;
        self.update_list(&data);
        data
	}
*/
/*
	fn getDocument(service, obj, merge, tokenPayload) {
		const getPrimaryKeyForeignList = (schema, obj) => {
			let list = [];

			for [fieldName, field] of Object.entries(schema.properties) {
				if field.$ref != undefined {
					let item = self.getPrimaryKeyForeign(schema, fieldName, obj);

					if item.valid == true && list.find(candidate => candidate.fieldName == fieldName).is_none() {
						list.push({"fieldName": fieldName, item});
					}
				}
			}

			return list;
		}

		let document;

		if merge != true {
			document = {};
		} else {
			document = obj;
		}

		let promises = [];
		// One To One
		{
			const next = (document, list) => {
				if list.length == 0) return;
				let data = list.shift();
				let schemaRef = self.getSchema(data.item.schema);
				
				if schemaRef.is_none() {
					console.error(data);
					self.getSchema(data.item.schema);
				}

				let promise;

				if Object.entries(data.item.primary_key).length > 0 {
					promise = self.get(schemaRef.name, data.item.primary_key);
				} else {
					promise = Promise.resolve({});
				}

				
				return promise.
				then(objExternal => document[data.fieldName] = objExternal).
				catch(err => console.error(err)).
//				then(() => next(document, list));
				finally(() => next(document, list));
			}

			let listToGet = getPrimaryKeyForeignList(service, obj);
			promises.push(next(document, listToGet));
		}
		// One To Many
		{
			let dependents = self.openapi.get_dependents(service.name, true, self.services);

			for item of dependents {
				let rufsServiceOther = self.getSchema(item.schema, tokenPayload);
				if rufsServiceOther == null) continue;
				let field = rufsServiceOther.properties[item.field];
				let foreignKey = Object.fromEntries(self.openapi.get_foreign_key(rufsServiceOther.name, item.field, obj));
				// TODO : check to findRemote
				promises.push(service.find(foreignKey).then(list => document[field.document] = list));
			}
		}

		return Promise.all(promises).then(() => document);
	}

	fn getDocument(service, obj, merge, tokenData) {
		return super.getDocument(service, obj, merge, tokenData).then(() => {
			if service.primary_keys.length > 0 {
				let primaryKey = service.get_primary_key(obj);

				if primaryKey != null {
					let pos = service.find_pos(primaryKey);

					if pos >= 0 {
						if service.updateListStr != undefined {
							service.updateListStr({data: obj, oldPos: pos, newPos: pos});
						} else {
							console.error(`[${self.constructor.name}.getDocument()] : missing updateListStr`);
						}
					}
				}
			}
		});
	}

	fn getDocuments(service, list, index) {
		if list == null || list.length == 0) return Promise.resolve();
		if index == null) index = 0;
		if index >= list.length) return Promise.resolve();
		let item = list[index];
		console.log(`[${self.constructor.name}.getDocuments(${service.name}, ${index})] : updating references to register ${index}, item = ${JSON.stringify(item)}, list = `, list);
		return self.getDocument(service, item, false).then(() => self.getDocuments(service, list, ++index));
	}
*/
    // devolve o rufsService apontado por field
    fn get_foreign_service<'a>(&'a self, service: &Service, field_name: &str, debug: bool) -> Option<&Service> {
    	// TODO : refatorar consumidores da função getForeignService(field), pois pode haver mais de uma referência
    	let field = if service.properties.len() > 0 {
    		match service.properties[field_name].as_item() {
                Some(schema) => Some(schema.as_ref()),
                None => None,
            }
    	} else {
    		self.login_response.openapi.get_property(&service.schema_name, field_name)
    	};

        match field {
            Some(field) => {
                match field.schema_data.extensions.get("x-$ref") {
                    Some(reference) => {
                        let reference = reference.as_str().unwrap();
                        let schema_name = OpenAPI::get_schema_name_from_ref(reference)/*.to_case(convert_case::Case::Snake) */;
                        self.service_map.get(&schema_name)
                    },
                    None => {
                        if debug {
                            self.get_foreign_service(service, field_name, true)
                        } else {
                            None
                        }
                    },
                }
            },
            None => {
                if debug {
                    self.get_foreign_service(service, field_name, true)
                } else {
                    None
                }
            },
        }
    }

/*
	fn clear_remote_listeners(&mut self) {
		self.remote_listeners.clear();
	}

	fn add_remote_listener(&self, listener_instance: &RemoteListener) {
		self.remote_listeners.push(listener_instance);
	}
*/
	// private -- used in login()
	fn web_socket_connect(&self, _path: &str) {
        /*
        struct WebSocketData {
            service :String,
            action :String,
            primary_key : Value,
        }
        */
		// Open a WebSocket connection
		// 'wss://localhost:8443/xxx/websocket'
        /*
		let mut url = if self.http_rest.url.starts_with("https://") {
			format!("wss://{}", self.http_rest.url[..8].to_string())
		} else if self.http_rest.url.starts_with("http://") {
			format!("ws://{}", self.http_rest.url[..7].to_string())
		} else {
			format!("ws://{}", self.http_rest.url.to_string())
		};

		if url.ends_with("/") == false {
            url = url + "/";
        }

		url = url + path;

		if url.ends_with("/") == false {
            url = url + "/";
        }
        */
/*
		let url = url + "websocket";
		self.web_socket = WebSocket::new(url);

    	self.web_socket.onopen = |event| self.web_socket.send(self.http_rest.get_token());

    	self.web_socket.onmessage = |event| {
			let item: WebSocketData = serde_json::from_str(event.data);
            //console.log("[ServerConnection] webSocketConnect : onMessage :", item);
            if let Some(service) = self.services.get(item.service) {
        		if item.action == "delete" {
        			if let Some(primary_key) = service.find_one(item.primary_key) {
            			self.remove_internal(&item.service, primary_key);
        			} else {
        	            //console.log("[ServerConnection] webSocketConnect : onMessage : delete : alread removed", item);
        			}
        		} else {
        			if let Some(res) = self.get(&item.service, &item.primary_key, true).await {
                        /* 
						for listener in self.remote_listeners {
                            listener.on_notify(&item.service, &item.primary_key, &item.action);
                        }
                        */
        			}
        		}
            }
		};
        */
	}
    // public
    pub async fn login(&mut self, login_path: &str, username: &str, password: &str/*, callback_partial: CallbackPartial*/) -> Result<(), Box<dyn std::error::Error>> {
        self.service_map.clear();
        let password = md5::compute(password);
        let password = format!("{:x}", password);
        self.login_response = self.http_rest.login(login_path, username, &password).await?;
        let mut list_dependencies = vec![];
        // depois carrega os serviços autorizados
        for role in self.login_response.roles.clone() {
            let schema_name = role.path[1..].to_string().to_case(convert_case::Case::Camel);
            let service = Service::new(&self.login_response.openapi, &role.path)?;
            /*
            let methods = ["get", "post", "put", "delete"];

            for i in 0..methods.len() {
                let method = methods[i];

                if role.mask & (1 << i)) != 0 {
                    login_response.access[method] = true;
                } else {
                    login_response.access[method] = false;
                }
            }

            if service.properties.rufs_group_owner.is_some() && server_connection.login_response.rufs_group_owner != 1 {
                service.properties.rufs_group_owner.hidden = true;
            }
            
            if service.properties.rufs_group_owner.is_some() && service.properties.rufs_group_owner.default.is_none() {
                service.properties.rufs_group_owner.default = server_connection.login_response.rufs_group_owner;
            }
*/            
            self.service_map.insert(schema_name.clone(), service);
            self.login_response.openapi.get_dependencies(&schema_name, &mut list_dependencies);

            if list_dependencies.contains(&schema_name) == false {
                list_dependencies.push(schema_name);
            }
        }

//    		if user == "admin") listDependencies = ["rufsUser", "rufsGroupOwner", "rufsGroup", "rufsGroupUser"];

        for schema_name in list_dependencies {
            //console.log(`login ${schemaName}`)
            let service = self.service_map.get(&schema_name);

            if let Some(service) = service {
                let (list, list_str) = service.query_remote(self, &Value::Null).await?;

                if list.len() != list_str.len() {
                    println!("[DEBUG - {} - list.len({}) != list_str.len({})]", schema_name, list.len(), list_str.len());
                }
    
                let service = self.service_map.get_mut(&schema_name).unwrap();
                service.list = list;
                println!("login 1.1 : service {}, list_str.len = {}", schema_name, list_str.len());
                service.list_str = list_str;
            }
        }

        self.web_socket_connect("websocket");
        Ok(())
    }
    // public
    /*
    fn logout(&mut self) {
        // limpa todos os dados da sessão anterior
		//self.web_socket.close();
   		//self.http_rest.set_token(None);
        //self.services.clear();
    }
*/
}

pub trait DataViewWatch {
    fn check_set_value(&self, data_view :&mut DataView, child_name: Option<&str>, server_connection :&ServerConnection, field_name: &str, field_value: &Value) -> Result<bool, Box<dyn std::error::Error>>;
    fn check_save(&self, data_view :&mut DataView, child_name: Option<&str>, server_connection: &ServerConnection) -> Result<(bool, DataViewProcessAction), Box<dyn std::error::Error>>;
}

//#[derive(Default)]
pub struct DataViewManager {
    server_connection: ServerConnection,
    //active_form: Option<String>,
    data_view_map: HashMap<String, DataView>,
}

macro_rules! data_view_get {
    ($data_view_manager:tt, $cap:tt) => {
        {
            let schema_name = $cap.name("name").context("missing schema_name")?.as_str().to_case(convert_case::Case::Camel);
            let action = $cap.name("action").context("missing action")?.as_str();
    
            let data_view = if let Some(parent) = $cap.name("parent") {
                let form_id = DataView::form_id(parent.as_str(), action);
                println!("[DataViewManager] get_data_view 1.1, form_id = {}", form_id);
                let data_view = $data_view_manager.data_view_map.get(&form_id).context(format!("Missing parent schema {} in data_view_manager", form_id))?;
                data_view.childs.iter().find(|item| item.schema_name == schema_name).context(format!("Missing item {} in data_view {}", schema_name, parent.as_str()))?
            } else {
                let form_id = DataView::form_id(&schema_name, action);
                println!("[DataViewManager] data_view_get! 1.2, form_id = {}", form_id);
                $data_view_manager.data_view_map.get(&form_id).context(format!("[process_click_target] Missing form {} in data_view_manager (2).", form_id))?
            };
    
            data_view
        }
    }
}    

macro_rules! data_view_get_mut {
    ($data_view_manager:tt, $cap:tt) => {
        {
            let schema_name = $cap.name("name").context("missing schema_name")?.as_str().to_case(convert_case::Case::Camel);
            let action = $cap.name("action").context("missing action")?.as_str();
    
            let data_view = if let Some(parent) = $cap.name("parent") {
                let form_id = DataView::form_id(parent.as_str(), action);
                println!("[DataViewManager] get_data_view 1.1, form_id = {}", form_id);

                if $data_view_manager.data_view_map.contains_key(&form_id) == false {
                    println!("data_view_manager.data_view_map.keys() : {:?}", $data_view_manager.data_view_map.keys());
                }

                let data_view = $data_view_manager.data_view_map.get_mut(&form_id).context(format!("Missing parent schema mut {} in data_view_manager", form_id))?;
                data_view.childs.iter_mut().find(|item| item.schema_name == schema_name).context(format!("Missing item {} in data_view {}", schema_name, parent.as_str()))?
            } else {
                let form_id = DataView::form_id(&schema_name, action);
                println!("[DataViewManager] data_view_get_mut! 1.2, form_id = {}", form_id);
                $data_view_manager.data_view_map.get_mut(&form_id).context(format!("[process_click_target] Missing form {} in data_view_manager (2).", form_id))?
            };
    
            data_view
        }
    }
}    

impl DataViewManager {

	pub fn new(path: &str) -> Self {
        let server_connection = ServerConnection::new(path);
        Self{server_connection, data_view_map: Default::default()}
    }

	pub fn reset(&mut self, path: &str) {
        self.data_view_map.clear();
        self.server_connection = ServerConnection::new(path);
    }

    async fn process(&mut self, watcher: &(impl DataViewWatch + std::marker::Sync), cap: &regex::Captures<'_>, action :DataViewProcessAction, params_search :&DataViewProcessParams, params_extra :&Value) -> Result<Option<DataViewResponse>, Box<dyn std::error::Error>> {
        fn build_field_filter_results(server_connection: &ServerConnection, data_view: &mut DataView) -> Result<(), Box<dyn std::error::Error>> {
            let service = server_connection.service_map.get(&data_view.schema_name).context(format!("[build_field_filter_results] Missing service {} in server_connection.service_map.", data_view.schema_name))?;
            //console.log(`buildFieldFilterResults :`, data_view.properties);
            for (_field_name, field) in &data_view.properties {
                let field = field.as_item().unwrap();
                let extensions = &field.schema_data.extensions;
    
                if let Some(reference) = extensions.get("$ref") {
                    let reference = reference.as_str().unwrap();
    
                    if let Some(_service) = server_connection.service_map.get(reference) {
                        //data_view.serverConnection.getDocuments(service, service.list).await;
                    }
                }
            }
            // faz uma referencia local a field.filterResultsStr, para permitir opção filtrada, sem alterar a referencia global
            let mut lists = vec![];
    
            for (field_name, field) in &data_view.properties {
                let field = field.as_item().unwrap();
                let extensions = &field.schema_data.extensions;
    
                let pair = if let Some(reference) = extensions.get("x-$ref") {
                    if let Some(service) = server_connection.get_foreign_service(service, field_name, true) {
                        let mut filter = if let Some(filter) = data_view.field_filter_results.get(field_name) {
                            filter.clone()
                        } else {
                            json!({})
                        };
    
                        if filter.as_object().unwrap().is_empty() {
                            let reference = reference.as_str().unwrap();
    
                            if let Some(pos) = reference.chars().position(|c| c == '?') {
                                //let primaryKey = Qs.parse(reference[pos..], json!({ignoreQueryPrefix: true, allowDots: true}));
                                let primary_key = queryst::parse(&reference[pos..]).unwrap();
    
                                for (field_name, value) in primary_key.as_object().unwrap() {
                                    if let Some(value) = value.as_str() {
                                        if value.starts_with("*") {
                                            let value = json!(value[1..]);
                                            let field = data_view.properties.get(field_name).context("process 1 : context")?.as_item().context("as_ref")?;
                                            filter[field_name] = server_connection.login_response.openapi.copy_value_field(field, true, &value).unwrap();
                                        }
                                    }
                                }
                            }
                        }
    
                        if filter.as_object().unwrap().is_empty() == false {
                            let list = vec![];
                            let list_str = vec![];
                            //let i = 0;
    
                            for _candidate in &service.list {                         
                                // if Filter::match(filter, candidate) {
                                //     list.push(candidate);
                                //     let str = rufs_service.list_str[i];
                                //     list_str.push(str);
                                // }
                            }
    
                            println!("build_field_filter_results 1.1 : dataview {}, field {}, list_str.len = {}", data_view.schema_name, field_name, list_str.len());
                            (list, list_str)
                        } else {
                            println!("build_field_filter_results 1.2 : dataview {}, field {}, list_str.len = {}", data_view.schema_name, field_name, service.list_str.len());
                            (service.list.clone(), service.list_str.clone())
                        }
                    } else {
                        //console.warn("don't have acess to service ", field.$ref);
                        (vec![], vec![])
                    }
                } else if let Some(enumeration) = extensions.get("x-enum") {
                    let list_str = if let Some(enum_labels) = extensions.get("x-enumLabels") {
                        enum_labels.as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect()
                    } else {
                        enumeration.as_array().unwrap().iter().map(|s| s.to_string()).collect()
                    };
    
                    (enumeration.as_array().unwrap().clone(), list_str)
                } else {
                    (vec![], vec![])
                };
    
                lists.push(pair);
            }
    
            for (field_name, _) in &mut data_view.properties {
                let (list, list_str) = lists.remove(0);
                data_view.field_results.insert(field_name.clone(), list);
                data_view.field_results_str.insert(field_name.clone(), list_str);
            }
    
            // if field.htmlType.includes("date") {
            //     field.filterRangeOptions = [
            //         " hora corrente ", " hora anterior ", " uma hora ",
            //         " dia corrente ", " dia anterior ", " um dia ",
            //         " semana corrente ", " semana anterior ", " uma semana ", 
            //         " quinzena corrente ", " quinzena anterior ", " uma quinzena ",
            //         " mês corrente ", " mês anterior ", " um mês ",
            //         " ano corrente ", " ano anterior ", " um ano "
            //     ];
            //     field.aggregateRangeOptions = ["", "hora", "dia", "mês", "ano"];
            // }
    
/*
            for data_view in &mut data_view.items {
                build_field_filter_results(data_view, server_connection);
            }
            */
            Ok(())
        }

        //#[async_recursion::async_recursion]
        async fn data_view_get(watcher: &(impl DataViewWatch + std::marker::Sync), data_view: &mut DataView, server_connection: &mut ServerConnection, primary_key: &Value) -> Result<(), Box<dyn std::error::Error>> {
            let service = server_connection.service_map.get(&data_view.schema_name).context(format!("[data_view_get] Missing service {} in server_connection.service_map.", data_view.schema_name))?;
            println!("[process.data_view_get] DEBUG  1, schema = {}, primaryKey = {}", data_view.schema_name, primary_key);
            let primary_key = service.get_primary_key(primary_key).context(format!("wrong primary key {} for service {}", primary_key, service.schema_name))?;
            println!("[process.data_view_get] DEBUG  2, schema = {}, primaryKey = {}", data_view.schema_name, primary_key);
            let value = server_connection.get(&data_view.schema_name, &primary_key).await?.clone();
            println!("[process.data_view_get] DEBUG  3, schema = {}, primaryKey = {}", data_view.schema_name, primary_key);
            let dependents = server_connection.login_response.openapi.get_dependents(&data_view.schema_name, false);
            println!("[process.data_view_get] DEBUG  3.2, schema = {}, primaryKey = {}", data_view.schema_name, primary_key);

            for item in &dependents {
                let Some(data_view_item) = data_view.childs.iter_mut().find(|child| child.schema_name == item.schema) else {
                    continue;
                };

                println!("[process.data_view_get] DEBUG  3.3.1.1.3, schema = {}, primaryKey = {}", data_view.schema_name, primary_key);
                let foreign_key = server_connection.login_response.openapi.get_foreign_key(&item.schema, &item.field, &primary_key)?;
                println!("[process.data_view_get] DEBUG  3.3.1.1.4, schema = {}, primaryKey = {}", data_view.schema_name, primary_key);

                let foreign_key = foreign_key.context(format!("Missing foreign value {} in {}, field {}.", primary_key, item.schema, item.field))?;

                for (field_name, value) in foreign_key.as_object().unwrap() {
                    println!("[process.data_view_get] DEBUG  3.3.1.1.4.1, schema = {}, primaryKey = {}", data_view.schema_name, primary_key);
                    let property = data_view_item.properties.get_mut(field_name).context(format!("Missing field {} in {}", field_name, data_view.schema_name))?;

                    match property {
                        ReferenceOr::Reference { reference: _ } => todo!(),
                        ReferenceOr::Item(property) => {
                            println!("[process.data_view_get] DEBUG  3.3.1.1.4.1.1, schema = {}, primaryKey = {}", data_view.schema_name, primary_key);
                            property.schema_data.default = Some(value.clone());
                        },
                    }
                }

                println!("[process.data_view_get] DEBUG  3.3.1.1.5, schema = {}, primaryKey = {}", data_view.schema_name, primary_key);
                build_field_filter_results(server_connection, data_view_item)?;
                println!("[process.data_view_get] DEBUG  3.3.1.1.6, {}.{}, foreign_key = {}", data_view_item.schema_name, data_view.schema_name, foreign_key);
                //data_view_get(watcher, &mut data_view_item, server_connection, &foreign_key).await?;
                data_view_item.set_values(server_connection, watcher, &foreign_key)?;
                println!("[process.data_view_get] DEBUG  3.3.1.1.7 : {}.{}.instance = {}", data_view.schema_name, data_view_item.schema_name, data_view_item.instance);
            }

            data_view.active_primary_key = Some(primary_key);
            println!("[process.data_view_get] DEBUG  5, form_id = {}, value = {}", data_view.form_id, value);
            data_view.set_values(server_connection, watcher, &value)
        }

        let form_id_parent = DataView::form_id_with_regex(&cap, true)?;
        println!("[process] DEBUG  1.0, form_id_parent = {}, action = {}, data_view_map.keys = {:?}", form_id_parent, action, self.data_view_map.keys());

        if self.data_view_map.contains_key(&form_id_parent) == false {
            let name = cap.name("name").context("context")?.as_str().to_case(convert_case::Case::Snake);
    
            let path = if let Some(parent) = cap.name("parent") {
                format!("/{}", parent.as_str().to_case(convert_case::Case::Snake))
            } else {
                format!("/{}", name.to_case(convert_case::Case::Snake))
            };

            let mut data_view = DataView::new(&path, DataViewType::Primary, None, action);
            data_view.set_schema(&self.server_connection)?;

            {
                let dependents = self.server_connection.login_response.openapi.get_dependents(&data_view.schema_name, false);
                println!("[process.data_view_get] DEBUG  3.2, {}.action = {}", data_view.schema_name, data_view.action);
    
                for item in &dependents {
                    if let Some(field) = self.server_connection.login_response.openapi.get_property(&item.schema, &item.field) {
                        let extensions = &field.schema_data.extensions;
                        //println!("[process.data_view_get] DEBUG  3.3.1.1, form_id = {}, primaryKey = {}", data_view.schema_name, primary_key);
                
                        if let Some(_enumeration) = extensions.get("x-title") {
                            //println!("[process.data_view_get] DEBUG  3.3.1.1.1, form_id = {}, primaryKey = {}", data_view.schema_name, primary_key);
                            let path = format!("/{}", item.schema.to_case(convert_case::Case::Snake));
                            let mut data_view_item = DataView::new(&path, DataViewType::Dependent, Some(data_view.schema_name.clone()), DataViewProcessAction::New);
                            //println!("[process.data_view_get] DEBUG  3.3.1.1.X, form_id = {}, dependent = {}, data_view_item = {}, primaryKey = {}", data_view.schema_name, item.schema, data_view_item.schema_name, primary_key);
                            //println!("[process.data_view_get] DEBUG  3.3.1.1.2, form_id = {}, primaryKey = {}", data_view.schema_name, primary_key);
                            data_view_item.set_schema(&self.server_connection)?;
                            println!("[process.data_view_get] DEBUG  3.3.1.1.8 : {}.childs.push({}); data_view_item.action = {}", data_view.schema_name, data_view_item.schema_name, data_view_item.action);
                            data_view.childs.push(data_view_item);
                        }
                    }
                }
    
                //println!("[process.data_view_get] DEBUG  4, form_id = {}, primaryKey = {}", data_view.schema_name, primary_key);
    
                for (field_name, field) in &data_view.properties {
                    if data_view.childs.iter().find(|child| &child.schema_name == field_name).is_some() {
                        // TODO : verificar se a duplicidade pode ser um bug
                        continue;
                    }
    
                    let field = field.as_item().context("data_view_get 1 : context")?;
    
                    match &field.schema_kind {
                        SchemaKind::Type(typ) => match &typ {
                            Type::Array(array) => {
                                let field = array.items.as_ref().context("data_view_get 2 : context")?;
                                let field = field.as_item().context("data_view_get 3 : context")? ;
    
                                match &field.schema_kind {
                                    SchemaKind::Type(typ) => {
                                        match typ {
                                            Type::Object(schema) => {
                                                let mut data_view_item = DataView::new(field_name, DataViewType::ObjectProperty, Some(data_view.schema_name.clone()), DataViewProcessAction::New);
                                                data_view_item.properties = schema.properties.clone();
                                                println!("[process.data_view_get] DEBUG  4.1 : {}.childs.push({}) - data_view_item = {};", data_view.schema_name, field_name, data_view_item.action);
                                                data_view.childs.push(data_view_item);
                                            },
                                            _ => {}
                                        }
                                    },
                                    SchemaKind::Any(schema) => {
                                        let mut data_view_item = DataView::new(field_name, DataViewType::ObjectProperty, Some(data_view.schema_name.clone()), DataViewProcessAction::New);
                                        data_view_item.properties = schema.properties.clone();
                                        data_view_item.short_description_list = data_view_item.properties.keys().map(|x| x.clone()).collect();
                                        println!("[process.data_view_get] DEBUG  4.2 : {}.childs.push({}) - data_view_item.action = {}", data_view.schema_name, field_name, data_view_item.action);
                                        data_view.childs.push(data_view_item);
                                    },
                                    _ => todo!(),
                                }
    
                            },
                            _ => {},
                        },
                        _ => {},
                    }
                }
    
            }

            self.data_view_map.insert(form_id_parent.to_string(), data_view);
        }

        let form_id = DataView::form_id_with_regex(&cap, false)?;
        let data_view = data_view_get_mut!(self, cap);
        println!("[process] DEBUG  1.1, form_id = {}", &form_id);
        data_view.clear();
        data_view.clear_filter()?;
        data_view.clear_sort()?;
        data_view.clear_aggregate();
        println!("[process] DEBUG  1.2, form_id = {}, data_view.action = {}", &form_id, data_view.action);

        if data_view.action != action {
            data_view.action = action;
            data_view.set_schema(&self.server_connection)?;

            for data_view in &mut data_view.childs {
                data_view.action = DataViewProcessAction::New;
                data_view.set_schema(&self.server_connection)?;
            }
        }
    
        println!("[process] DEBUG  1.3, form_id = {}, data_view.action = {}", &form_id, data_view.action);

        if data_view.action == DataViewProcessAction::Search {
            // if params.filter != undefined || params.filterRangeMin != undefined || params.filterRangeMax != undefined {
            //     return data_view.queryRemote(data_view.serverConnection.openapi, params);
            // }
        }
    
        println!("[process] DEBUG  1.4, form_id = {}", &form_id);

        if data_view.path.is_some() {
            build_field_filter_results(&self.server_connection, data_view)?;
        }

        println!("[process] DEBUG  1.5, form_id = {}", &form_id);

        match &data_view.action {
            DataViewProcessAction::Search => {
                println!("[process] DEBUG  1.5.1, form_id = {}", form_id);
    
                if params_search.filter.is_some() || params_search.filter_range.is_some() || params_search.filter_range_min.is_some() || params_search.filter_range_max.is_some() {
                    if let Some(filter_range) = &params_search.filter_range {
                        for (field_name, value) in filter_range.as_object().unwrap() {
                            data_view.set_filter_range(field_name, value.as_str().unwrap());
                        }
                    }
    
                    data_view.apply_filter(&params_search.filter, &params_search.filter_range_min, &params_search.filter_range_max);
                    //data_view.setPage(1);
                }
    
                if let Some(aggregate) = &params_search.aggregate {
                    data_view.apply_aggregate(aggregate);
                }
    
                if params_search.sort.is_some() {
                    data_view.apply_sort(&params_search.sort)?;
                }
    
                if let Some(pagination) = &params_search.pagination {
                    data_view.paginate(pagination.page_size, pagination.page)?;
                }
            },
            DataViewProcessAction::New => {
                println!("[process] DEBUG  1.5.2, form_id = {}", form_id);
    
                if let Some(overwrite) = &params_search.overwrite {
                    data_view.set_values(&self.server_connection, watcher, overwrite)?;
                } else {
                    data_view.set_values(&self.server_connection, watcher, params_extra)?;
                }
            },
            DataViewProcessAction::Edit | DataViewProcessAction::View => {
                println!("[process] DEBUG  1.5.3, form_id = {}", form_id);

                if data_view.path.is_some() {
                    if let Some(primary_key) = &params_search.primary_key {
                        println!("[process] DEBUG  1.5.3.1, form_id = {}", form_id);
                        data_view_get(watcher, data_view, &mut self.server_connection, primary_key).await?
                    } else {
                        println!("[process] DEBUG  1.5.3.2, form_id = {}", form_id);
                        data_view_get(watcher, data_view, &mut self.server_connection, params_extra).await?
                    }
                }
            },
        }
    
        let mut data_view_response = DataViewResponse{form_id: data_view.form_id.clone(), ..Default::default()};
        println!("[process] DEBUG  1.6, form_id = {}", &form_id);
        data_view_response.changes = DataView::build_changes(self)?;
        println!("[process] DEBUG  1.7, form_id = {}", &form_id);
        let data_view = data_view_get!(self, cap);
        data_view_response.table = DataView::build_page(self, data_view, &params_search)?;
        println!("[process] DEBUG  1.8, form_id = {}", &form_id);

        match &action {
            DataViewProcessAction::Search => data_view_response.html_search = DataView::build_search(data_view, &params_search)?,
            _ => data_view_response.instance = DataView::build_instance(self, data_view, &params_search, &action)?,
        }

        println!("[process] DEBUG  1.9, form_id = {}", &form_id);
        Ok(Some(data_view_response))
    }

    pub async fn process_click_target(data_view_manager :&mut DataViewManager, target: &str, watcher: &(impl DataViewWatch + std::marker::Sync)) -> Result<Option<DataViewResponse>, Box<dyn std::error::Error>> {
        let re = regex::Regex::new(r"#!/app/((?P<parent>\w+)-)?(?P<name>\w+)/(?P<action>\w+)(?P<query_string>\?[\w\.=&]+)?")?;

        if let Some(cap) = re.captures(target) {
            let action = match cap.name("action").unwrap().as_str() {
                "new" => crate::DataViewProcessAction::New,
                "edit" => crate::DataViewProcessAction::Edit,
                "view" => crate::DataViewProcessAction::View,
                _ => crate::DataViewProcessAction::Search,
            };

            let mut params_search = DataViewProcessParams{..Default::default()};

            let params_extra = if let Some(query_string) = cap.name("query_string") {
                let str = query_string.as_str();

                let pairs = if str.len() > 0 {
                    let str = &str[1..];
                    //serde_qs::from_str::<Value>(str)?;
                    nested_qs::from_str::<Value>(str)?
                } else {
                    json!({})
                };

                println!("[process_click_target] target 1.3.1 = {}, pairs = {:?}", target, pairs);

                if let Some(obj_in) = pairs.as_object() {
                    let mut obj_out = json!({});
    
                    for (field_name, value) in obj_in {
                        let fields = field_name.split(".");
                        let mut obj_out = &mut obj_out;
    
                        for field_name in fields {
                            if obj_out.get(field_name).is_none() {
                                obj_out[field_name] = json!({});
                            }
    
                            obj_out = obj_out.get_mut(field_name).unwrap();
                        }
    
                        *obj_out = value.clone();
                    }
    
                    println!("[process_click_target] target 1.5 = {}, params_search = {:?}", target, params_search);
                    params_search = serde_json::from_value::<DataViewProcessParams>(obj_out.clone())?;
                    obj_out
                } else {
                    json!({})
                }    
            } else {
                json!({})
            };

            //data_view_manager.active_form = Some(schema_name.to_string());
            return data_view_manager.process(watcher, &cap, action.clone(), &params_search, &params_extra).await;
        }

        let re = regex::Regex::new(r"id=(?P<action>create)-(?P<name>\w+)")?;

        if let Some(cap) = re.captures(target) {
            println!("DEBUG target 2 = {}", target);
            let params_search = DataViewProcessParams{..Default::default()};
            let params_extra = json!({});
            return data_view_manager.process(watcher, &cap, crate::DataViewProcessAction::New, &params_search, &params_extra).await;
        }

        let re = regex::Regex::new(r"instance-delete-(?P<action>\w+)-((?P<parent>\w+)-)?(?P<name>\w+)")?;

        if let Some(cap) = re.captures(target) {
            let schema_name = cap.name("name").unwrap().as_str().to_case(convert_case::Case::Camel);
            let data_view = data_view_get!(data_view_manager, cap);
            let primary_key = data_view.active_primary_key.as_ref().context(format!("don't opened item in form_id {}", data_view.form_id))?;
            let _old_value = data_view_manager.server_connection.remove(&schema_name, primary_key).await?;
            let params_search = DataViewProcessParams{..Default::default()};
            let params_extra = json!({});
            return data_view_manager.process(watcher, &cap, crate::DataViewProcessAction::Search, &params_search, &params_extra).await;
        }

        let re = regex::Regex::new(r"instance-save-(?P<action>\w+)-((?P<parent>\w+)-)?(?P<name>\w+)$")?;

        if let Some(cap) = re.captures(target) {
            println!("[DataViewManager] process_click_target 4.1 : regex = {}, taget = {}", re.as_str(), target);
            let schema_name = cap.name("name").unwrap().as_str().to_case(convert_case::Case::Camel);
            let action = cap.name("action").unwrap().as_str();

            let (data_view, child_name) = if let Some(parent) = cap.name("parent") {
                let form_id = DataView::form_id(parent.as_str(), action);
                let data_view = data_view_manager.data_view_map.get_mut(&form_id).context(format!("4.1.1.1 Missing parent schema {} in data_view_manager", form_id))?;
                //data_view.items.iter_mut().find(|item| item.schema_name == schema_name).context(format!("Missing item {} in data_view {}", schema_name, parent.as_str()))?
                (data_view, Some(schema_name.as_str()))
            } else {
                let form_id = DataView::form_id(&schema_name, action);
                let data_view = data_view_manager.data_view_map.get_mut(&form_id).context(format!("[process_click_target] Missing form {} in data_view_manager (2).", form_id))?;
                (data_view, None)
            };

            println!("[DataViewManager] process_click_target 4.2 - data_view.action = {}", data_view.action);
            let (is_ok, action) = watcher.check_save(data_view, child_name, &data_view_manager.server_connection)?;

            let data_view_response = if is_ok {
                println!("[DataViewManager] process_click_target 4.2.1, action = {}", action);
                let obj_in = data_view.save(&mut data_view_manager.server_connection).await?;

                if let Some(child_name) = child_name {
                    let data_view = data_view.childs.iter_mut().find(|item| item.schema_name == child_name).context(format!("process_click_target 1 : Missing item {} in data_view {}", child_name, data_view.schema_name))?;
                    //let obj_in = 
                    if data_view.path.is_some() {
                        data_view.save(&mut data_view_manager.server_connection).await?;
                    }
                }

                println!("[DataViewManager] process_click_target(target = {}) 4.2.3 : data_view.active_index = {:?}", target, data_view.active_index);

                if let Some(index) = data_view.active_index {
                    data_view.filter_results[index] = obj_in.clone();
                }

                let params_extra = if action == DataViewProcessAction::Edit {
                    obj_in
                } else {
                    json!({})
                };

                let params_search = DataViewProcessParams{..Default::default()};
                data_view_manager.process(watcher, &cap, action, &params_search, &params_extra).await?
            } else {
                Some(DataViewResponse{..Default::default()})
            };

            println!("[DataViewManager] process_click_target 4.9");
            return Ok(data_view_response);
        }

        let re = regex::Regex::new(r"table-row-(?P<act>\w+)-(?P<action>\w+)-((?P<parent>\w+)-)?(?P<name>\w+)-(?P<field_name>\w+)-(?P<index>\d+)")?;

        if let Some(cap) = re.captures(target) {
            println!("[DataViewManager] process_click_target 5.1, reexp = {}, target = {}", re.as_str(), target);
            let data_view = data_view_get_mut!(data_view_manager, cap);
            /*
            if let Some(field_name) = cap.name("field_name") {
                let field_name = field_name.as_str();
                let field_value = data_view.instance.get_mut(field_name).context(format!("[process_click_target({}, {}, {:?})] Missing field {} in data_view {}", target, schema_name, child_name, field_name, data_view.schema_name))?;
                println!("[DataViewManager] process_click_target 5.3.1");

                match field_value {
                    Value::Bool(value) => {
                        if *value == true {
                            *value = false;
                        } else {
                            *value = true;
                        }
                    },
                    Value::Array(array) => {
                        let index = cap.name("index").context("broken")?.as_str().parse::<usize>()?;
                        let value = array.get_mut(index).context("process_click_target 1 : context")?;

                        match value {
                            Value::Bool(value) => {
                                if *value == true {
                                    *value = false;
                                } else {
                                    *value = true;
                                }
                            },
                            Value::Null => todo!(),
                            Value::Number(_) => todo!(),
                            Value::String(_) => todo!(),
                            Value::Array(_) => todo!(),
                            Value::Object(_) => {
                                let data_view = data_view.childs.iter_mut().find(|data_view| data_view.schema_name == field_name).context(format!("Missing child {} in form {}", field_name, data_view.form_id))?;
                                data_view.set_values(&data_view_manager.server_connection, watcher, value)?;                                
                            },
                        }
                    },
                    Value::Number(value) => {
                        if let Some(value) = &mut value.as_u64() {
                            let field = data_view.properties.get(field_name).context("process_click_target 2 : context")?.as_item().context("process_click_target 3 : context")?;

                            if field.schema_data.extensions.contains_key("x-flags") {
                                let index = cap.name("index").context("broken")?.as_str().parse::<usize>()?;
                                let bitmask = 1 << index;
                                *value = *value ^ bitmask;
                            }
                        }
                    },
                    _ => {},
                }
            } else {
            }
            */
            let field_name = cap.name("field_name").context("broken")?.as_str();
            let data_view = data_view.childs.iter_mut().find(|data_view| data_view.schema_name == field_name).context(format!("Missing child {} in {}", field_name, data_view.form_id))?;
            let active_index = cap.name("index").context("broken")?.as_str().parse::<usize>()?;
            data_view.instance = data_view.filter_results.get(active_index).context(format!("Missing {}.filter_results[{}], size = {}", data_view.schema_name, active_index, data_view.filter_results.len()))?.clone();
            println!("[DataViewManager] process_click_target(target = {}) 5.3.2 : active_index = {}", target, active_index);
            data_view.active_index = Some(active_index);
            //data_view_manager.active_form = Some(schema_name.to_string());
            let data_view_response = DataViewResponse{..Default::default()};
            return Ok(Some(data_view_response));
        }

        let re = regex::Regex::new(r"instance-(?P<action>\w+)-((?P<parent>\w+)-)?(?P<name>\w+)-(?P<field_name>\w+)-(?P<index>\d+)")?;

        if let Some(cap) = re.captures(target) {
            let data_view = data_view_get_mut!(data_view_manager, cap);
            let field_name = cap.name("field_name").context("context")?.as_str();
            let field = data_view.properties.get(field_name).context("process_click_target 2 : context")?.as_item().context("process_click_target 3 : context")?;

            if field.schema_data.extensions.contains_key("x-flags") {
                println!("[DataViewManager] process_click_target 6.2.1");
                let field_value = data_view.instance.get_mut(field_name).context(format!("[process_click_target({})] Missing field {} in data_view {}", target, field_name, data_view.form_id))?;
                let mut field_value = field_value.as_u64().context("Is not u64")?;
                let index = cap.name("index").context("broken")?.as_str().parse::<usize>()?;
                let bitmask = 1 << index;
                field_value = field_value ^ bitmask;
                data_view.set_value(None, &data_view_manager.server_connection, watcher, field_name, &json!(field_value))?;
                let value = data_view.filter_results.get_mut(data_view.active_index.context("Missing data_view.active_index")?).context("Missing data from index")?;
                value[field_name] = json!(field_value);
            }

            let mut data_view_response = DataViewResponse{form_id: data_view.form_id.clone(), ..Default::default()};
            println!("[DataViewManager] process_click_target 6.3 : form_id = {}", data_view.form_id);
            data_view_response.changes = DataView::build_changes(data_view_manager)?;
            let data_view = data_view_get!(data_view_manager, cap);
            println!("[DataViewManager] process_click_target 6.4 : form_id = {}", data_view.form_id);
            let params_search = DataViewProcessParams{..Default::default()};
            data_view_response.table = DataView::build_page(data_view_manager, data_view, &params_search)?;
            println!("[DataViewManager] process_click_target 6.9");
            return Ok(Some(data_view_response));
        }

        println!("[DataViewManager] process_click_target 9 : regex = {}, taget = {}", re.as_str(), target);
        Ok(None)
    }

    pub async fn process_edit_target(data_view_manager :&mut DataViewManager, target: &str, watcher: &impl DataViewWatch, value: &str) -> Result<Option<DataViewResponse>, Box<dyn std::error::Error>> {
        // fazer clone da função para parse_value_filter()
        fn parse_value(data_view :&mut DataView, child_name: Option<&str>, server_connection: &ServerConnection, watcher: &impl DataViewWatch, field_name: &str, value :&str) -> Result<(), Box<dyn std::error::Error>> {
            // faz o inverso da funcao strAsciiHexToFlags
            fn flags_to_str_ascii_hex(flags: &Vec<bool>) -> String {
                let mut value = 0;
        
                for i in 0..flags.len() {
                    let flag = flags[i];
                    let bit = 1 << i;
        
                    if flag == true {
                        value = value | bit;
                    }
                }
        
                format!("{:X}", value)
            }

            fn parse_value_process(data_view :&mut DataView, server_connection: &ServerConnection, field_name: &str, value :&str) -> Result<Value, Box<dyn std::error::Error>> {
                //data_view.field_external_references_str.insert(field_name.to_string(), value.to_string());
                let field = data_view.properties.get(field_name).context(format!("[process_edit_target.parse_value()] Missing field {}.{}", data_view.schema_name, field_name))?;
                let field =  field.as_item().context("[process_edit_target.parse_value({})] broken")?;
                let extensions = &field.schema_data.extensions;
                //let enumeration = field.schema_kind.

                let value = if let Some(_) = extensions.get("x-flags") {
                    let flags = u32::from_str_radix(&flags_to_str_ascii_hex(data_view.instance_flags.get(field_name).unwrap()), 16).unwrap();
                    json!(flags)
                } else if let Some(_reference) = extensions.get("x-$ref") {
                    if value.len() > 0 {
                        let field_results = data_view.field_results.get(field_name).context("Missing field_results")?;
                        let field_results_str = data_view.field_results_str.get(field_name).context("value not found in field_results_str")?;
                        let pos = field_results_str.iter().position(|s| s.as_str() == value).context(format!("Missing foreign description {} in {}.", value, field_name))?;
                        let foreign_data = field_results.get(pos).context("broken 1 in parse_value")?;
                        let foreign_key = server_connection.login_response.openapi.get_foreign_key(&data_view.schema_name, field_name, foreign_data).unwrap().unwrap();
                        foreign_key.get(field_name).context("broken 1 in parse_value")?.clone()
                    } else {
                        Value::Null
                    }
                } else if let Some(enumeration) = extensions.get("x-enum") {
                    let enumeration = enumeration.as_array().context("is not array")?;

                    if let Some(enum_labels) = extensions.get("x-enumLabels") {
                        let enum_labels = enum_labels.as_array().context("is not array")?;
                        let pos = enum_labels.iter().position(|item| {
                            if let Some(enum_label) = item.as_str() {
                                if enum_label == value {
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }).context(format!("Missing foreign description {} in {}.", value, field_name))?;

                        enumeration.get(pos).context("expected value at pos")?.clone()
                    } else {
                        json!(value)
                    }
                } else {
                    json!(value)
                };

                Ok(value)
            }

            let value = if let Some(child_name) = child_name {
                println!("data view {} have {} childs", data_view.form_id, data_view.childs.len());
                //data_view.childs.iter().for_each(|data_view_child| println!("child {} : {}", data_view_child.schema_name, data_view_child.form_id));
                let data_view = data_view.childs.iter_mut().find(|item| item.schema_name == child_name).context(format!("process_edit_target 1 : Missing item {} in data_view {}", child_name, data_view.schema_name))?;
                parse_value_process(data_view, server_connection, field_name, value)?
            } else {
                parse_value_process(data_view, server_connection, field_name, value)?
            };

            data_view.set_value(child_name, server_connection, watcher, field_name, &value)
        }

        let mut data_view_response = DataViewResponse{..Default::default()};
        let re = regex::Regex::new(r"instance-(?P<action>\w+)-((?P<parent>\w+)-)?(?P<name>\w+)-(?P<field_name>\w+)")?;
        println!("[DataViewManager] process_edit_target 1 : regex = {}, taget = {}, value = {}", re.as_str(), target, value);

        if let Some(cap) = re.captures(target) {
            println!("[DataViewManager] process_edit_target 1.1 : regex = {}, taget = {}", re.as_str(), target);
            let schema_name = cap.name("name").unwrap().as_str()/*.to_case(convert_case::Case::Snake) */;
            let action = cap.name("action").unwrap().as_str();
            let field_name = cap.name("field_name").unwrap().as_str();

            let (schema_name, child_name) = if let Some(parent) = cap.name("parent") {
                (parent.as_str(), Some(schema_name))
            } else {
                (schema_name, None)
            };

            let form_id = DataView::form_id(schema_name, action);
            let data_view = data_view_manager.data_view_map.get_mut(&form_id).context(format!("[process_edit_target] Missing form {} in data_view_manager.", form_id))?;
            parse_value(data_view, child_name, &data_view_manager.server_connection, watcher, field_name, &value)?;
            data_view_response.changes = DataView::build_changes(data_view_manager)?;
            return Ok(Some(data_view_response));
        }

        let re = regex::Regex::new(r"login-(?P<name>\w+)")?;
        println!("[DataViewManager] process_edit_target 2 : regex = {}, taget = {}", re.as_str(), target);

        for cap in re.captures_iter(target) {
            println!("[DataViewManager] process_edit_target 2.1 : regex = {}, taget = {}", re.as_str(), target);
            let name = cap.name("name").unwrap().as_str();

            if ["user", "password"].contains(&name) {
                return Ok(Some(data_view_response));
            }
        }

        println!("[DataViewManager] process_edit_target 3 : regex = {}, taget = {}", re.as_str(), target);
        Ok(None)
    }

}

struct RufsNfe {}

#[derive(Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
struct Request {
     rufs_group_owner: usize,
     id              : usize,
     #[serde(rename = "type")]
     typ            : usize,
     state           : usize,
     person           : String,
     person_dest      : String,
     date             : NaiveDateTime,
     additional_data  : Option<String>,
     products_value   : Option<f64>,
     services_value   : Option<f64>,
     transport_value  : Option<f64>,
     desc_value       : Option<f64>,
     sum_value        : Option<f64>,
     payments_value   : Option<f64>,
 }

#[derive(Debug,Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestProduct {
    id :Option<usize>,
    rufs_group_owner :usize,
    request :usize,
    product :usize,
    quantity :f64,
    value :f64,
    value_item :Option<f64>,
    value_desc :Option<f64>,
    value_freight :Option<f64>,
    cfop :Option<usize>,
    value_all_tax :Option<f64>,
    serials :Option<String>,   
 }

impl RufsNfe {
    
    fn request_payment_adjusts(data_view_payment : &mut DataView, watcher : &impl DataViewWatch, server_connection: &ServerConnection, request: &Request, typ :Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
        let remaining_payment = request.sum_value.unwrap_or(0.0) - request.payments_value.unwrap_or(0.0);
        let value = data_view_payment.instance.get("value").unwrap_or(&json!(0.0)).as_f64().unwrap_or(0.0);

        if value == 0.0 {
            let value = json!(remaining_payment);
            data_view_payment.set_value(None, server_connection, watcher, "value", &value)?;
        }

        let account = data_view_payment.instance.get("account").unwrap_or(&Value::Null);
        println!("[request_payment_adjusts] 5 : old account  = {}", account);

        if account.is_null() {
            let accounts = data_view_payment.field_results.get("account").context("expected list of accounts")?;

            let typ = if let Some(typ) = typ {
                typ
            } else {
                data_view_payment.instance.get("type").unwrap_or(&json!(1)).as_u64().unwrap_or(1)
            };
    
            if typ == 1 {
                if accounts.len() > 0 {
                    let account = accounts[accounts.len()-1].get("id").context("missing field id in account")?.clone();//accounts[0].id;//
                    data_view_payment.set_value(None, server_connection, watcher, "account", &account)?;
                }
            } else {
                if accounts.len() > 1 {
                    let account = accounts[accounts.len()-2].get("id").context("missing field id in account")?.clone();//accounts.len()-2
                    data_view_payment.set_value(None, server_connection, watcher, "account", &account)?;
                }
            }
        }

        Ok(())
    }

}

impl DataViewWatch for RufsNfe {

    fn check_set_value(&self, data_view :&mut DataView, child_name: Option<&str>, server_connection: &ServerConnection, field_name: &str, field_value: &Value) -> Result<bool, Box<dyn std::error::Error>> {
        println!("check_set_value 1 {}.{:?}.{} = {}", data_view.schema_name, child_name, field_name, field_value);

        if data_view.schema_name == "request" {
            println!("check_set_value 1.1 {}.{:?}.{} = {}", data_view.schema_name, child_name, field_name, field_value);

            if let Some(child_name) = child_name {
                println!("check_set_value 1.1.1 {}.{:?}.{} = {}", data_view.schema_name, child_name, field_name, field_value);
                println!("check_set_value 1.1.2 {}.{:?}.{} = {}", data_view.schema_name, child_name, field_name, field_value);

                if child_name == "requestProduct" && data_view.instance.get("product").is_some() && ["quantity", "value"].contains(&field_name) {
                    if let Some(data_view) = data_view.childs.iter_mut().find(|item| item.schema_name == child_name) {
                        if data_view.instance.get("value").is_none() {
                            // TODO : se valor unitário está ausente, pegar o valor do cadastro de produtos.
                            data_view.set_value(None, server_connection, self, "value", &json!(0.0))?;
                        }

                        if data_view.instance.get("quantity").is_none() {
                            data_view.set_value(None, server_connection, self, "quantity", &json!(1.0))?;
                        }

                        let field_value :f64 = match field_value {
                            Value::Number(field_value) => field_value.as_f64().context("expected type is f64")?,
                            _ => todo!(),
                        };

                        let mut request_product: RequestProduct = serde_json::from_value(data_view.instance.clone())?;

                        if field_name == "quantity" {
                            request_product.value_item = Some(request_product.value * field_value);
                        } else if field_name == "value" {
                            request_product.value_item = Some(request_product.quantity * field_value);
                        }

                        data_view.instance = serde_json::to_value(request_product)?;
                    }
                }

                println!("check_set_value 1.1.3 {}.{:?}.{} = {}", data_view.schema_name, child_name, field_name, field_value);

                if child_name == "requestPayment" && ["type"].contains(&field_name) {
                    println!("check_set_value 1.1.3.1 {}.{:?}.{} = {}", data_view.schema_name, child_name, field_name, field_value);

                    if let Some(data_view_child) = data_view.childs.iter_mut().find(|item| item.schema_name == child_name) {
                        println!("check_set_value 1.1.3.1.1 {}.{:?}.{} = {}", data_view.schema_name, child_name, field_name, field_value);
                        let typ = field_value.as_u64().unwrap_or(1);
                        // due_date
                        if [1,4,10,11,12,13].contains(&typ) {
                            let value = data_view.instance.get("date").context("check_set_value 1 : context")?;
                            data_view_child.set_value(None, server_connection, self, "dueDate", value)?;
                        }
                        // payday
                        if [1,4,10,11,12,13].contains(&typ) {
                            let value = data_view.instance.get("date").context("check_set_value 2 : context")?;
                            //data_view_child.instance["payday"] = value.clone();
                            data_view_child.set_value(None, server_connection, self, "payday", value)?;
                        }

                        let request: Request = serde_json::from_value(data_view.instance.clone())?;
                        println!("check_set_value 1.1.3.1.8 {}", data_view_child.instance);
                        RufsNfe::request_payment_adjusts(data_view_child, self, server_connection, &request, Some(typ))?;
                        println!("check_set_value 1.1.3.1.9 {}", data_view_child.instance);
                    }
                }
            } else {
                /*
                if ["sumValue"].contains(&field_name) {
                    if let Some(data_view_payment) = data_view.childs.iter_mut().find(|item| item.schema_name == "requestPayment") {
                        let request: Request = serde_json::from_value(data_view.instance.clone())?;
                        RufsNfe::request_payment_adjusts(data_view_payment, self, server_connection, &request, None)?;
                    }
                }
                 */
            }
        }

        Ok(true)
    }
     
    fn check_save(&self, data_view :&mut DataView, child_name: Option<&str>, server_connection: &ServerConnection) -> Result<(bool, DataViewProcessAction), Box<dyn std::error::Error>> {
        let action = if ["rufsUser", "request"].contains(&data_view.schema_name.as_str()) {
            if let Some(schema_name_child) = child_name {
                if data_view.schema_name == "request" {

                    if schema_name_child == "requestProduct" {
                        let item = data_view.childs.iter().find(|item| item.schema_name == schema_name_child).context(format!("Missing child {} in parent {}", schema_name_child, data_view.schema_name))?;
                        println!("[RufsNfe.check_save.request.requestProduct] 1 : instance = {}", item.instance);
                        let request_product: RequestProduct = serde_json::from_value(item.instance.clone())?;
                        println!("[RufsNfe.check_save.request.requestProduct] 2 : RequestProduct = {:?}", request_product);
                        let product_value = f64::trunc(request_product.quantity * request_product.value * 1000.0) / 1000.0;
                        let products_desc_value = request_product.value_desc.unwrap_or(0.0);
                        let request: Request = serde_json::from_value(data_view.instance.clone())?;
                        let products_value_old = request.products_value.unwrap_or(0.0);
                        let desc_value_old = request.desc_value.unwrap_or(0.0);
                        let sum_value_old = request.sum_value.unwrap_or(0.0);
                        data_view.set_value(None, server_connection, self, "productsValue", &json!(products_value_old + product_value))?;
                        data_view.set_value(None, server_connection, self, "descValue", &json!(desc_value_old - products_desc_value))?;
                        let sum_value = f64::trunc((sum_value_old + product_value - products_desc_value)*1000.0)/1000.0;
                        data_view.set_value(None, server_connection, self, "sumValue", &json!(sum_value))?;
                        let data_view_payment = data_view.childs.iter_mut().find(|item| item.schema_name == "requestPayment").context(format!("Missing child {} in parent {}", "requestPayment", data_view.schema_name))?;
                        let request: Request = serde_json::from_value(data_view.instance.clone())?;
                        RufsNfe::request_payment_adjusts(data_view_payment, self, server_connection, &request, None)?;
                    }
/*
                    let data_view_request_payment = data_view.childs.iter_mut().find(|item| item.schema_name == "requestPayment").context(format!("Missing child requestPayment in parent {}", data_view.schema_name))?;

                    if schema_name_child != "requestPayment" {
                    }
*/
                }
            }

            DataViewProcessAction::Edit
        } else {
            DataViewProcessAction::Search
        };

        Ok((true, action))
    }
     
 }

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = DataViewManager)]
pub struct DataViewManagerWrapper {
    watcher: RufsNfe,
    data_view_manager :DataViewManager,
    //data_view_html_map: HashMap<String, String>,
}

#[wasm_bindgen(js_class = DataViewManager)]
impl DataViewManagerWrapper {
    #[wasm_bindgen(constructor)]
	pub fn new(path: &str) -> Self {
        Self {
            watcher: RufsNfe{},
            data_view_manager :DataViewManager::new(path),
            //data_view_html_map :Default::default()
        }
    }

	pub async fn login(&mut self, login_path: &str, user: &str, password: &str/*, callback_partial: CallbackPartial*/) -> Result<JsValue, JsValue> {
        let _ret = match self.data_view_manager.server_connection.login(login_path, user, password).await {
            Ok(ret) => ret,
            Err(err) => return Err(JsValue::from_str(&err.to_string())),
        };
        
        let menu = json!({
            "Cadastros": {
                "Clientes e Fornecedores": "person/search",
                "Produtos": "product/search",
                "Contas": "account/search",
                "Requisições": "request/search",
                "Usuários": "rufs_user/search",
            },
            "Movimento": {
                "Financeiro": "request_payment/search",
                "Estoque": "stock/search",
            },
            "Rotinas": {
                "Compra": "request/new?overwrite.type=1&overwrite.state=10",
                "Venda": "request/new?overwrite.type=2&overwrite.state=10",
                "Importar": "request/import?overwrite.type=1&overwrite.state=10",
            },
            "Tabelas": {
                "Confaz Cest": "confaz_cest/search"
            }
        });

        let login_response = json!({"menu": menu, "path": self.data_view_manager.server_connection.login_response.path});
        Ok(serde_wasm_bindgen::to_value(&login_response)?)
    }

    pub async fn process_click_target(&mut self, target: &str) -> Result<JsValue, JsValue> {
        println!("DEBUG DataViewManagerWrapper::process_click_target 1 {}", target);
        let res = DataViewManager::process_click_target(&mut self.data_view_manager, target, &self.watcher).await;
        println!("DEBUG DataViewManagerWrapper::process_click_target 2 {}", target);

        let data_view_response = match res {
            Ok(value) => value,
            Err(err) => return Err(JsValue::from_str(&err.to_string())),
        };
        println!("DEBUG DataViewManagerWrapper::process_click_target 3 {}", target);

        let Some(data_view_response) = data_view_response else {
            return Ok(JsValue::from_str("{}"))
        };
        println!("DEBUG DataViewManagerWrapper::process_click_target 4 {}", target);

        Ok(serde_wasm_bindgen::to_value(&data_view_response).unwrap())
    }

    pub async fn process_edit_target(&mut self, target: &str, value: &str) -> Result<JsValue, JsValue> {
        /*
        let value = match serde_wasm_bindgen::from_value::<Value>(value) {
            Ok(value) => value,
            Err(err) => return Err(JsValue::from_str(&err.to_string())),
        };
        */
        println!("DEBUG DataViewManagerWrapper::process_edit_target 1 {} = {}", target, value);
        let res = DataViewManager::process_edit_target(&mut self.data_view_manager, target, &self.watcher, value).await;
        println!("DEBUG DataViewManagerWrapper::process_edit_target 2 {}", target);

        let data_view_response = match res {
            Ok(value) => value,
            Err(err) => return Err(JsValue::from_str(&err.to_string())),
        };
        println!("DEBUG DataViewManagerWrapper::process_edit_target 3 {}", target);

        let Some(data_view_response) = data_view_response else {
            return Ok(JsValue::from_str("{}"))
        };
        println!("DEBUG DataViewManagerWrapper::process_edit_target 4 {}", target);

        Ok(serde_wasm_bindgen::to_value(&data_view_response).unwrap())
    }

}

/*
#[wasm_bindgen(js_name = DataView)]
pub struct DataViewWrapper {
    data_view :DataView,
    primary_keys :JsValue,
    short_description_list :JsValue,
    properties :JsValue,
    list: Vec<JsValue>,
}

#[wasm_bindgen(js_class = DataView)]
impl DataViewWrapper {
    #[wasm_bindgen(constructor)]
	pub fn new(path: &str) -> Self {
        Self {
            data_view: DataStore::new(path, None), 
            primary_keys: JsValue::NULL,
            short_description_list: JsValue::NULL,
            properties: JsValue::NULL, 
            list: vec![],
        }
    }

    fn convert_list_to_js(&mut self) {
        web_sys::console::log_1(&format!("[DataStoreWrapper.convert_list_to_js({})] list_in :", self.data_view.path).into());

        for value in &self.data_view.list {
            web_sys::console::log_1(&format!("{:?}", value).into());
        }
        
        self.list = self.data_view.list.iter().map(|value| serde_wasm_bindgen::to_value(value).unwrap()).collect();
        //        self.list = self.data_view.list.iter().map(|value| js_sys::JSON::parse(&serde_json::to_string(&value).unwrap()).unwrap()).collect();
        web_sys::console::log_1(&format!("[DataStoreWrapper.convert_list_to_js({})] list_out :", self.data_view.path).into());

        for value in &self.list {
            web_sys::console::log_1(value);
        }

        web_sys::console::log_1(&format!("[DataStoreWrapper.convert_list_to_js({})] list_out.length : {}", self.data_view.path, self.list.len()).into());
    }
    
	pub fn clear(&mut self) {
        self.data_view.clear()
	}

/* 	pub async fn process(&mut self, server_connection_wrapper: &mut ServerConnectionWrapper, data_view_manager_wrapper: &mut DataStoreManagerWrapper, action: &str, params: JsValue) {
        let params = serde_wasm_bindgen::from_value::<DataStoreProcessParams>(params).unwrap();
        self.data_view.process(&mut server_connection_wrapper.server_connection, &mut data_view_manager_wrapper.data_view_manager, action, &params).await
	}
 */
	pub fn find(&self, params: JsValue) -> Vec<JsValue> {
        let params = serde_wasm_bindgen::from_value::<Value>(params).unwrap();
        self.data_view.find(&params).iter().map(|value| serde_wasm_bindgen::to_value(value).unwrap()).collect()
	}

	pub fn find_pos(&self, key: JsValue) -> JsValue {
        web_sys::console::log_1(&format!("[DataStoreWrapper.find_pos({:?})] 1", key).into());
        let key = serde_wasm_bindgen::from_value::<Value>(key).unwrap();
        web_sys::console::log_1(&format!("[DataStoreWrapper.find_pos({})] 2", key).into());

        if let Some(pos) = self.data_view.find_pos(&key) {
            web_sys::console::log_1(&format!("[DataStoreWrapper.find_pos({})] : 1", key).into());
            JsValue::from(pos)
        } else {
            web_sys::console::log_1(&format!("[DataStoreWrapper.find_pos({})] : 2", key).into());
            JsValue::NULL
        }
	}

	pub fn find_one(&self, key: JsValue) -> JsValue {
        let key = serde_wasm_bindgen::from_value::<Value>(key).unwrap();

        if let Some(value) = self.data_view.find_one(&key) {
            serde_wasm_bindgen::to_value(value).unwrap()
        } else {
            JsValue::NULL
        }
	}
	// private, use in get, save, update and remove
	pub fn update_list(&mut self, value: JsValue, pos: JsValue) -> JsValue {
        let value = serde_wasm_bindgen::from_value::<Value>(value).unwrap();

        let pos = if pos.is_null() {
            None
        } else {
            let pos = pos.as_f64().unwrap();
            Some(pos as usize)
        };

        let pos = JsValue::from(self.data_view.update_list(value, pos));
        self.convert_list_to_js();
        pos
	}

	pub fn set_schema(&mut self, server_connection_wrapper: &mut ServerConnectionWrapper, method: &str, schema_place: &str) {
        self.data_view.set_schema(&mut server_connection_wrapper.server_connection, method, &SchemaPlace::from_str(schema_place));
        self.primary_keys = js_sys::JSON::parse(&serde_json::to_string(&self.data_view.primary_keys).unwrap()).unwrap();
        self.short_description_list = js_sys::JSON::parse(&serde_json::to_string(&self.data_view.short_description_list).unwrap()).unwrap();
        self.properties = js_sys::JSON::parse(&serde_json::to_string(&self.data_view.properties).unwrap()).unwrap();
    }

	pub fn get_primary_key(&self, obj: JsValue) -> JsValue {
        let obj = serde_wasm_bindgen::from_value::<Value>(obj).unwrap();

        if let Some(value) = self.data_view.get_primary_key(&obj) {
            let value = serde_wasm_bindgen::to_value(&value).unwrap();
            value
        } else {
            JsValue::NULL
        }
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.data_view.name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn path(&self) -> String {
        self.data_view.path.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn primary_keys(&self) -> JsValue {
        self.primary_keys.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn short_description_list(&self) -> JsValue {
        self.short_description_list.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn properties(&self) -> JsValue {
        self.properties.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn list(&self) -> Vec<JsValue> {
        web_sys::console::log_1(&format!("[DataStoreWrapper.list({})] length : {}", self.data_view.path, self.list.len()).into());
        self.list.clone()
    }

}
 */

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
// #[cfg(feature = "wee_alloc")]
// #[global_allocator]
// static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
//use workflow_websocket::client::WebSocket;
/*
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
*/

// wasm-pack build --target web --dev
// clear;find ./ | grep -F 'openapi-rufs_nfe-rust.json' | xargs rm ;PGHOST=localhost PGPORT=5432 PGUSER=development PGPASSWORD=123456 psql rufs_nfe_development -c "DROP DATABASE IF EXISTS rufs_nfe" && PGHOST=localhost PGPORT=5432 PGUSER=development PGPASSWORD=123456 psql rufs_nfe_development -c "CREATE DATABASE rufs_nfe" && cargo build && cargo test nfe -- --nocapture;
#[cfg(test)]
mod tests {
    use crate::{DataView};
    use std::fs;
    use anyhow::{anyhow, Context};
    use convert_case::Casing;
    use rufs_base_rust::data_store::Filter;
    use serde::Deserialize;
    use serde_json::{Value, json};
    use crate::{DataViewManager, DataViewWatch, DataViewProcessParams, RufsNfe};
  /*  
    fn pause() {
        let mut stdin = io::stdin();
        let mut stdout = io::stdout();
        // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
        write!(stdout, "Press any key to continue...").unwrap();
        stdout.flush().unwrap();
        // Read a single byte and discard
        let _ = stdin.read(&mut [0u8]).unwrap();
    }    
*/
    #[derive(Debug,Default,Deserialize)]
    struct SeleniumCommand {
        //id: String,
        //comment: String,
        command: String,
        target: String,
        //targets: Vec<Vec<String>>,
        value: String,
    }
    
    #[derive(Debug,Default,Deserialize)]
    struct SeleniumTest {
        id: String,
        name: String,
        commands: Vec<SeleniumCommand>,
    }

    #[derive(Debug,Default,Deserialize)]
    struct SeleniumSuite {
        //id: String,
        //name: String,
        //parallel: bool,
        //timeout: usize,
        tests: Vec<String>,
    }
    
    #[derive(Debug,Default,Deserialize)]
    struct SeleniumIde {
        //id: String,
        //version: String,
        //name: String,
        //url: String,
        tests: Vec<SeleniumTest>,
        suites: Vec<SeleniumSuite>,
        //urls: Vec<String>,
        //plugins: Vec<String>,
    }

    #[tokio::test]
    async fn selelium() -> Result<(), Box<dyn std::error::Error>> {
        #[async_recursion::async_recursion]
        async fn test_run(data_view_manager :&mut DataViewManager, watcher: &(impl DataViewWatch + std::marker::Sync), side: &SeleniumIde, id_or_name :&str) -> Result<(), Box<dyn std::error::Error>> {
            if let Some(test) = side.tests.iter().find(|test| test.id == id_or_name || test.name == id_or_name) {
                println!("\nRunning test {}...", test.name);

                for command in &test.commands {
                    if command.command.as_str().starts_with("//") {
                        continue;
                    }

                    let mut target = command.target.clone();
                    println!("\nRunning command {} in target {} with value {}...", command.command.as_str(), target, command.value);

                    match command.command.as_str() {
                        "open" => {
                            data_view_manager.reset("http://localhost:8080");
                            continue;
                        },
                        "run" => {
                            test_run(data_view_manager, watcher, side, &command.target).await?;
                            continue;
                        },
                        "click" | "clickAt" => {
                            if target.starts_with("id=menu-") {
                                continue;
                            }

                            match command.target.as_str() {
                                "id=login-send" => {
                                    if let Some(user) = test.commands.iter().find(|command| ["type", "sendKeys"].contains(&command.command.as_str()) && command.target == "id=login-user") {
                                        if let Some(password) = test.commands.iter().find(|command| ["type", "sendKeys"].contains(&command.command.as_str()) && command.target == "id=login-password") {
                                            match data_view_manager.server_connection.login("/login", &user.value, &password.value).await {
                                                Ok(_) => target = format!("#!/app/{}", data_view_manager.server_connection.login_response.path),
                                                Err(err) => {
                                                    if let Some(http_msg) = test.commands.iter().find(|command| command.command == "assertText" && command.target == "id=http-error") {
                                                        if err.to_string().ends_with(&http_msg.value) {
                                                            break;
                                                        } else {
                                                            println!("received : {}", err);
                                                            println!("expected : {}", http_msg.value);
                                                        }
                                                    }

                                                    let res = Err(err);
                                                    return res?;
                                                },
                                            }
                                        }
                                    }
                                },
                                _ => {}
                            }

                            if let Some(_) = DataViewManager::process_click_target(data_view_manager, &target, watcher).await? {
                                continue;
                            }

                            let re = regex::Regex::new(r"id=instance-((?P<parent>\w+)-)?(?P<name>\w+)-(?P<field_name>\w+)")?;

                            if let Some(_) = re.captures(&target) {
                                println!("[selelium.test_run.click] DEBUG 1 : target = {}", target);
                                continue;
                            }
                        },
                        "type" | "sendKeys" | "select" => {
                            let value = if command.value.starts_with("label=") {
                                &command.value[6..]
                            } else {
                                &command.value
                            };
        
                            if let Some(_) = DataViewManager::process_edit_target(data_view_manager, &command.target, watcher, value).await? {
                                continue;
                            }
                        },
                        "assertText" | "assertValue" | "assertSelectedValue" => {
                            let re = regex::Regex::new(r"id=(?P<name>\w+)")?;

                            if let Some(cap) = re.captures(&command.target) {
                                let name = cap.name("name").unwrap().as_str();

                                match name {
                                    "http-error" => {

                                    },
                                    _ => {}
                                }
                            }

                            let re = regex::Regex::new(r"id=(instance|table-row-col)-(?P<action>\w+)-((?P<parent>\w+)-)?(?P<name>\w+)-(?P<field_name>\w+)(-(?P<index>\d+))?")?;

                            let Some(cap) = re.captures(&target) else {
                                println!("\nDon't match target !\n");
                                continue;
                            };

                            let schema_name = cap.name("name").unwrap().as_str()/*.to_case(convert_case::Case::Snake) */;
                            let field_name = cap.name("field_name").unwrap().as_str();
        
                            let data_view = data_view_get_mut!(data_view_manager, cap);

                            let str = if let Some(index) = cap.name("index") {
                                let list = if data_view.path.is_none() || data_view.filter_results.len() > 0 {
                                    &data_view.filter_results
                                } else {
                                    let service = data_view_manager.server_connection.service_map.get(&data_view.schema_name).context("Missing service in service_map")?;
                                    &service.list
                                };

                                let index = index.as_str().parse::<usize>()?;
                                let value = list.get(index).context(format!("Don't found value of index {} in {}", index, data_view.form_id))?;
                                value.get(field_name).context(format!("[{}] target = {} : Don't found field {} in data_view {}, json = {}", command.command.as_str(), target, field_name, data_view.form_id, value))?.to_string()
                            } else if let Some(str) = data_view.field_external_references_str.get(field_name) {
                                str.clone()
                            } else if let Some(value) = data_view.instance.get(field_name) {
                                match value {
                                    Value::String(value) => value.to_string(),
                                    Value::Bool(value) => value.to_string(),
                                    Value::Null => "".to_string(),
                                    Value::Number(value) => value.to_string(),
                                    Value::Array(_) => todo!(),
                                    Value::Object(_) => todo!(),
                                }
                            } else {
                                "".to_string()
                            };

                            let value = if command.value.starts_with("string:") {
                                &command.value[7..]
                            } else {
                                &command.value
                            };

                            if value == &str {
                                continue;
                            } else {
                                let empty_list = vec![];
                                let options = data_view.field_results_str.get(field_name).unwrap_or(&empty_list).join("\n");
                                return Err(anyhow!("[{}({})] : In schema {}, field {}, value of instance ({}) don't match with expected ({}).\nfield_results_str:\n{}", command.command.as_str(), target, schema_name, field_name, str, value, options))?;
                            }
                        },
                        "assertElementNotPresent" => {
                            if target == "id=http-error" {
                                continue;
                            }

                            let re = regex::Regex::new(r"#!/app/((?P<parent>\w+)-)?(?P<name>\w+)/(?P<action>\w+)(?P<query_string>\?[^']+)?")?;

                            if let Some(cap) = re.captures(&target) {
                                let params_search = if let Some(query_string) = cap.name("query_string") {
                                    let str = query_string.as_str();
                                    serde_qs::from_str::<DataViewProcessParams>(str)?
                                } else {
                                    DataViewProcessParams{..Default::default()}
                                };
                    
                                let params_extra = if let Some(query_string) = cap.name("query_string") {
                                    let str = query_string.as_str();
                    
                                    if str.len() > 0 {
                                        let str = &str[1..];
                                        nested_qs::from_str::<Value>(str).unwrap()
                                    } else {
                                        json!({})
                                    }
                                } else {
                                    json!({})
                                };

                                let primary_key = if let Some(primary_key) = &params_search.primary_key {
                                    primary_key
                                } else {
                                    &params_extra
                                };
                
                                let data_view = data_view_get!(data_view_manager, cap);
                                println!("{:?}", data_view.action);

                                let is_broken = if data_view.path.is_some() {
                                    let service = data_view_manager.server_connection.service_map.get(&data_view.schema_name).context(format!("Missing service {}", &data_view.schema_name))?;

                                    if let Some(value) = service.find_one(primary_key) {
                                        println!("Unexpected existence of item in service.list : {}", value);
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                };

                                if let Some(index) = Filter::find_index(&data_view.filter_results, primary_key).unwrap() {
                                    println!("Unexpected existence of item of index {} in filter_results.", index);
                                } else if !is_broken {
                                    continue;
                                }
                            }                    
                        },
                        "waitForElementNotVisible" => {
                            if target == "id=http-error" {
                                continue;
                            }
                        },
                        "waitForElementVisible" => {
                            continue;
                        },
                        _ => {}
                    }

                    return Err(anyhow!("General error : {}", format!("unknow command : {:?}", command)))?;
                }
            
                println!("... test {} is finalized with successfull !\n", test.name);
            }

            Ok(())
        }

        let watcher = RufsNfe{};
        let mut data_view_manager = DataViewManager::new("http://localhost:8080");
        let file = fs::File::open("/home/alexsandro/Downloads/webapp-rust.side").expect("file should open read only");
        let side: SeleniumIde = serde_json::from_reader(file).expect("file should be proper JSON");

        for suite in &side.suites {
            println!("suite : {:?}", suite);

            for id in &suite.tests {
                test_run(&mut data_view_manager, &watcher, &side, &id).await?
            }
        }

        Ok(())
    }
/*
    #[test]
    fn login() {
        tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            assert!(true);
        })
 
    }
*/
}
